use std::collections::VecDeque;
use std::num::NonZeroUsize;
use tcio::bytes::{Buf, Bytes, BytesMut};

use crate::h2::hpack::huffman::{self, HuffmanError};
use crate::headers::error::HeaderError;
use crate::headers::{HeaderMap, standard};
use crate::headers::{HeaderName, HeaderValue};

const MSB: u8 = 0b1000_0000;
const U7: u8 = 0b0111_1111;
const U6: u8 = 0b0011_1111;
const U5: u8 = 0b0001_1111;
const U4: u8 = 0b0000_1111;

//   0   1   2   3   4   5   6   7
// +---+---+---+---+---+---+---+---+
// | 1 |        Index (7+)         |
// +---+---------------------------+
const INDEXED: u8 = 0b1000_0000;
const INDEXED_INT: u8 = U7;
// +---+---+---+---+---+---+---+---+
// | 0 | 1 |      Index (6+)       |
// +---+---+-----------------------+
const LITERAL_INDEXED: u8 = 0b0100_0000;
const LITERAL_INDEXED_INT: u8 = U6;
// +---+---+---+---+---+---+---+---+
// | 0 | 0 | 1 |   Max size (5+)   |
// +---+---------------------------+
const SIZE_UPDATE: u8 = 0b0010_0000;
const SIZE_UPDATE_MASK: u8 = 0b1110_0000;
const SIZE_UPDATE_INT: u8 = U5;

// === Literal without indexing ====
// +---+---+---+---+---+---+---+---+
// | 0 | 0 | 0 | 0 |  Index (4+)   |
// +---+---+-----------------------+
// ===== Literal never indexed =====
// +---+---+---+---+---+---+---+---+
// | 0 | 0 | 0 | 1 |  Index (4+)   |
// +---+---+-----------------------+
// const LITERAL_NINDEX: u8 = 0b0001_0000;
const LITERAL_NINDEX_INT: u8 = U4;

/// 0bx1xx_xxxx = literal with indexed
/// 0bx0xx_xxxx = literal without/never indexed
const LITERAL_IS_INDEXED_MASK: u8 = 0b0100_0000;

/// HPACK Table.
#[derive(Debug)]
pub struct Table {
    fields: VecDeque<(HeaderName, HeaderValue)>,
    size: usize,
    max_size: usize,
}

impl Table {
    #[inline]
    pub const fn new(max_size: usize) -> Table {
        Self {
            fields: VecDeque::new(),
            size: 0,
            max_size,
        }
    }

    #[inline]
    pub fn with_capacity(max_size: usize, capacity: usize) -> Table {
        Self {
            fields: VecDeque::with_capacity(capacity),
            size: 0,
            max_size,
        }
    }

    fn default_inner() -> Table {
        Self {
            fields: VecDeque::new(),
            size: 0,
            max_size: 4096,
        }
    }

    fn update_size(&mut self, max_size: usize) {
        self.max_size = max_size;
        while self.max_size < self.size {
            self.evict_entry();
        }
    }

    fn insert(&mut self, name: HeaderName, val: HeaderValue) {
        let size = field_size(&name, &val);

        // It is not an error to attempt to add an entry that is larger than the maximum size; an
        // attempt to add an entry larger than the maximum size causes the table to be emptied of
        // all existing entries and results in an empty table.
        if self.max_size < size {
            self.fields.clear();
            return;
        }

        while self.max_size - self.size < size {
            self.evict_entry();
        }

        self.fields.push_front((name, val));
        self.size += size;

        debug_assert!(self.size <= self.max_size);
    }

    fn evict_entry(&mut self) -> Option<(HeaderName, HeaderValue)> {
        let (name, val) = self.fields.pop_back()?;
        let size = field_size(&name, &val);
        self.size -= size;
        Some((name, val))
    }

    pub fn decode_block(
        &mut self,
        mut block: Bytes,
        maps: &mut HeaderMap,
        write_buffer: &mut BytesMut,
    ) -> Result<(), DecodeError> {
        let Some(prefix) = block.first() else {
            return Ok(());
        };
        // Dynamic table size update MUST occur at the beginning of the first header block
        // following the change to the dynamic table size.
        if prefix & SIZE_UPDATE_MASK == SIZE_UPDATE {
            let prefix = block.get_u8();
            let max_size = decode_int!(SIZE_UPDATE_INT, prefix, &mut block);
            self.update_size(max_size);
        }

        while !block.is_empty() {
            let (name, value) = self.decode(&mut block, write_buffer)?;
            maps.append(name, value);
        }
        Ok(())
    }

    fn decode(
        &mut self,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<(HeaderName, HeaderValue), DecodeError> {
        use DecodeError as E;

        debug_assert!(write_buffer.is_empty());

        let prefix = bytes.try_get_u8().ok_or(E::Incomplete)?;

        // decoding

        let index = if prefix & INDEXED == INDEXED {
            let index = decode_int!(INDEXED_INT, prefix, bytes)
                .checked_sub(1)
                .ok_or(E::ZeroIndex)?;
            return match STATIC_HEADER.get(index) {
                Some((name, val)) => match val {
                    Some(val) => Ok((name.clone(), val.clone())),
                    None => Err(E::NotFound)
                },
                None => match self.fields.get(index.strict_sub(STATIC_HEADER.len())) {
                    Some(field) => Ok(field.clone()),
                    None => Err(E::NotFound),
                },
            }
        } else if prefix & LITERAL_INDEXED == LITERAL_INDEXED {
            decode_int!(LITERAL_INDEXED_INT, prefix, bytes)

        } else if prefix & SIZE_UPDATE == SIZE_UPDATE {
            return Err(E::InvalidSizeUpdate);

        } else {
            // Literal without/never indexed
            decode_int!(LITERAL_NINDEX_INT, prefix, bytes)
        };

        // processing

        let name = match NonZeroUsize::new(index) {
            Some(index) => {
                // HPACK is 1 indexed
                let index = index.get() - 1;
                match STATIC_HEADER.get(index) {
                    Some((name, _)) => name.clone(),
                    None => self
                        .fields
                        .get(index.strict_sub(STATIC_HEADER.len()))
                        .ok_or(E::NotFound)?
                        .0
                        .clone(),
                }
            }
            None => {
                HeaderName::from_bytes_lowercase(decode_string(bytes, write_buffer)?)?
            },
        };
        let value = HeaderValue::from_bytes(decode_string(bytes, write_buffer)?)?;

        let is_indexed = prefix & LITERAL_IS_INDEXED_MASK == LITERAL_IS_INDEXED_MASK;
        if is_indexed {
            self.insert(name.clone(), value.clone());
        }

        Ok((name, value))
    }
}

#[cfg(test)]
impl Table {
    pub(crate) fn fields(&self) -> &VecDeque<(HeaderName, HeaderValue)> {
        &self.fields
    }

    pub(crate) fn size(&self) -> usize {
        self.size
    }

    pub(crate) fn decode_test(
        &mut self,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<(HeaderName, HeaderValue), DecodeError> {
        self.decode(bytes, write_buffer)
    }
}

impl Default for Table {
    #[inline]
    fn default() -> Self {
        Self::default_inner()
    }
}

macro_rules! decode_int {
    ($int:expr, $prefix:expr, $bytes:expr) => {{
        let int = ($prefix & $int);
        if int != $int {
            int as usize
        } else {
            (int as usize) + continue_decode_int($bytes)?
        }
    }};
}

use {decode_int};

fn continue_decode_int(bytes: &mut Bytes) -> Result<usize, DecodeError> {
    let mut shift = 0;
    let mut value = 0;

    loop {
        let prefix = bytes.try_get_u8().ok_or(DecodeError::Incomplete)?;
        let u7 = prefix & U7;

        value += (u7 as usize) << shift;
        shift += 7;

        if prefix & MSB != MSB {
            break
        }
    }

    Ok(value)
}

fn decode_string(bytes: &mut Bytes, write_buffer: &mut BytesMut) -> Result<Bytes, DecodeError> {
    //   0   1   2   3   4   5   6   7
    // +---+---+---+---+---+---+---+---+
    // | H |    String Length (7+)     |
    // +---+---------------------------+
    // |  String Data (Length octets)  |
    // +-------------------------------+
    let prefix = bytes.try_get_u8().ok_or(DecodeError::Incomplete)?;

    let len = decode_int!(U7, prefix, bytes);
    let Some(value) = bytes.get(..len) else {
        return Err(DecodeError::Incomplete);
    };

    if prefix & MSB == MSB {
        huffman::decode(value, write_buffer)?;
        let value = write_buffer.split().freeze();
        bytes.advance(len);
        Ok(value)
    } else {
        bytes.try_split_to(len).ok_or(DecodeError::Incomplete)
    }
}

fn field_size(name: &HeaderName, val: &HeaderValue) -> usize {
    name.as_str().len() + val.as_bytes().len() + 32
}

/// HPACK Decoding Error.
#[derive(Debug)]
pub enum DecodeError {
    /// Bytes given is insufficient.
    Incomplete,
    /// Unknown header block kind.
    UnknownRepr,
    /// Found `0` index.
    ZeroIndex,
    /// Indexed header not found.
    NotFound,
    /// Huffman coding error.
    Huffman,
    /// Header name or value validation error.
    InvalidHeader,
    /// Size update is too large or is not at the beginning of header block.
    InvalidSizeUpdate,
}

impl std::error::Error for DecodeError { }
impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incomplete => f.write_str("data incomplete"),
            Self::UnknownRepr => f.write_str("unknown header block representation"),
            Self::ZeroIndex => f.write_str("index cannot be 0"),
            Self::NotFound => f.write_str("field with given index not found"),
            Self::Huffman => f.write_str("huffman coding error"),
            Self::InvalidHeader => f.write_str("invalid header"),
            Self::InvalidSizeUpdate => f.write_str("invalid size update"),
        }
    }
}

impl From<HeaderError> for DecodeError {
    fn from(_: HeaderError) -> Self {
        Self::InvalidHeader
    }
}

impl From<HuffmanError> for DecodeError {
    fn from(_: HuffmanError) -> Self {
        Self::Huffman
    }
}

static STATIC_HEADER: [(HeaderName, Option<HeaderValue>); 61] = [
    (standard::PSEUDO_AUTHORITY, None),
    (standard::PSEUDO_METHOD, Some(HeaderValue::from_static(b"GET"))),
    (standard::PSEUDO_METHOD, Some(HeaderValue::from_static(b"POST"))),
    (standard::PSEUDO_PATH, Some(HeaderValue::from_static(b"/"))),
    (standard::PSEUDO_PATH, Some(HeaderValue::from_static(b"/index.html"))),
    (standard::PSEUDO_SCHEME, Some(HeaderValue::from_static(b"http"))),
    (standard::PSEUDO_SCHEME, Some(HeaderValue::from_static(b"https"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"200"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"204"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"206"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"304"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"400"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"404"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"500"))),
    (standard::ACCEPT_CHARSET, None),
    (standard::ACCEPT_ENCODING, Some(HeaderValue::from_static(b"gzip, deflate"))),
    (standard::ACCEPT_LANGUAGE, None),
    (standard::ACCEPT_RANGES, None),
    (standard::ACCEPT, None),
    (standard::ACCESS_CONTROL_ALLOW_ORIGIN, None),
    (standard::AGE, None),
    (standard::ALLOW, None),
    (standard::AUTHORIZATION, None),
    (standard::CACHE_CONTROL, None),
    (standard::CONTENT_DISPOSITION, None),
    (standard::CONTENT_ENCODING, None),
    (standard::CONTENT_LANGUAGE, None),
    (standard::CONTENT_LENGTH, None),
    (standard::CONTENT_LOCATION, None),
    (standard::CONTENT_RANGE, None),
    (standard::CONTENT_TYPE, None),
    (standard::COOKIE, None),
    (standard::DATE, None),
    (standard::ETAG, None),
    (standard::EXPECT, None),
    (standard::EXPIRES, None),
    (standard::FROM, None),
    (standard::HOST, None),
    (standard::IF_MATCH, None),
    (standard::IF_MODIFIED_SINCE, None),
    (standard::IF_NONE_MATCH, None),
    (standard::IF_RANGE, None),
    (standard::IF_UNMODIFIED_SINCE, None),
    (standard::LAST_MODIFIED, None),
    (standard::LINK, None),
    (standard::LOCATION, None),
    (standard::MAX_FORWARDS, None),
    (standard::PROXY_AUTHENTICATE, None),
    (standard::PROXY_AUTHORIZATION, None),
    (standard::RANGE, None),
    (standard::REFERER, None),
    (standard::REFRESH, None),
    (standard::RETRY_AFTER, None),
    (standard::SERVER, None),
    (standard::SET_COOKIE, None),
    (standard::STRICT_TRANSPORT_SECURITY, None),
    (standard::TRANSFER_ENCODING, None),
    (standard::USER_AGENT, None),
    (standard::VARY, None),
    (standard::VIA, None),
    (standard::WWW_AUTHENTICATE, None),
];

#[test]
fn test_hpack_int() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Bytes::copy_from_slice(&[
        0b0001_1111,
        0b1001_1010,
        0b0000_1010,
    ]);
    let prefix = bytes.get_u8();
    let int = decode_int!(U5, prefix, &mut bytes);
    assert!(bytes.is_empty());
    assert_eq!(int, 1337);
    Ok(())
}

#[test]
fn test_hpack_int2() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Bytes::copy_from_slice(&[
        0b0001_1111,
        0b0000_0000,
    ]);
    let prefix = bytes.get_u8();
    let int = decode_int!(U5, prefix, &mut bytes);
    assert!(bytes.is_empty());
    assert_eq!(int, 31);
    Ok(())
}
