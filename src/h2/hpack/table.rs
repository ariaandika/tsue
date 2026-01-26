use std::collections::VecDeque;
use std::num::NonZeroUsize;
use tcio::bytes::{Buf, Bytes, BytesMut};

use crate::h2::hpack::huffman::{self, HuffmanError};
use crate::headers::error::{HeaderNameError, HeaderValueError};
use crate::headers::{HeaderMap, standard};
use crate::headers::{HeaderName, HeaderValue};

const MSB: u8 = 0b1000_0000;
const BIT7: u8 = 1 << 6;
const U5: u8 = u8::MAX >> 3;
const U4: u8 = u8::MAX >> 4;
const U6: u8 = u8::MAX >> 2;
const U7: u8 = u8::MAX >> 1;

/// HPACK Table.
#[derive(Debug)]
pub struct Table {
    fields: VecDeque<Field>,
    size: usize,
    max_size: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: HeaderName,
    pub value: HeaderValue,
}

impl Field {
    fn size(&self) -> usize {
        self.name.as_str().len() + self.value.as_bytes().len() + 32
    }
}

impl Table {
    #[inline]
    pub const fn new() -> Table {
        Self {
            fields: VecDeque::new(),
            size: 0,
            max_size: 4096,
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Table {
        Self {
            fields: VecDeque::with_capacity(capacity),
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

    fn insert(&mut self, field: Field) {
        let size = field.size();

        if self.max_size < size {
            self.fields.clear();
            return;
        }

        while self.max_size - self.size < size {
            self.evict_entry();
        }

        self.fields.push_front(field);
        self.size += size;

        debug_assert!(self.size <= self.max_size);
    }

    fn evict_entry(&mut self) -> Option<Field> {
        let evicted = self.fields.pop_back()?;
        let size = evicted.size();
        self.size -= size;
        Some(evicted)
    }

    pub fn decode_block(
        &mut self,
        mut block: Bytes,
        maps: &mut HeaderMap,
        write_buffer: &mut BytesMut,
    ) -> Result<(), DecodeError> {
        // +---+---+---+---+---+---+---+---+
        // | 0 | 0 | 1 |   Max size (5+)   |
        // +---+---------------------------+
        const SIZE_UPDATE: u8 = 0b0010_0000;
        const SIZE_UPDATE_MASK: u8 = 0b1110_0000;

        let Some(prefix) = block.first() else {
            return Ok(());
        };
        if prefix & SIZE_UPDATE_MASK == SIZE_UPDATE {
            let prefix = block.get_u8();
            let max_size = parse_int!(U5, prefix, &mut block);
            self.update_size(max_size);
        }

        while !block.is_empty() {
            let Field { name, value } = self.decode(&mut block, write_buffer)?;
            maps.append(name, value);
        }
        Ok(())
    }

    fn decode(&mut self, bytes: &mut Bytes, write_buffer: &mut BytesMut) -> Result<Field, DecodeError> {
        use DecodeError as E;

        //   0   1   2   3   4   5   6   7
        // +---+---+---+---+---+---+---+---+
        // | 1 |        Index (7+)         |
        // +---+---------------------------+
        const INDEXED: u8       = 0b1000_0000;
        // +---+---+---+---+---+---+---+---+
        // | 0 | 1 |      Index (6+)       |
        // +---+---+-----------------------+
        const LITERAL_INDEXED: u8 = 0b0100_0000;
        // +---+---+---+---+---+---+---+---+
        // | 0 | 0 | 1 |   Max size (5+)   |
        // +---+---------------------------+
        const SIZE_UPDATE: u8 = 0b0010_0000;

        // # Literal without indexing
        // +---+---+---+---+---+---+---+---+
        // | 0 | 0 | 0 | 0 |  Index (4+)   |
        // +---+---+-----------------------+
        // # Literal never indexed
        // +---+---+---+---+---+---+---+---+
        // | 0 | 0 | 0 | 1 |  Index (4+)   |
        // +---+---+-----------------------+

        debug_assert!(write_buffer.is_empty());

        let prefix = bytes.try_get_u8().ok_or(E::Incomplete)?;

        // decoding

        let index = if prefix & INDEXED == INDEXED {
            let index = parse_int!(U7, prefix, bytes).checked_sub(1).ok_or(E::ZeroIndex)?;
            return match STATIC_HEADER.get(index) {
                Some((name, val)) => match val {
                    Some(val) => Ok(Field {
                        name: name.clone(),
                        value: val.clone(),
                    }),
                    None => Err(E::NotFound)
                },
                None => match self.fields.get(index.strict_sub(STATIC_HEADER.len())) {
                    Some(field) => Ok(field.clone()),
                    None => Err(E::NotFound),
                },
            }
        } else if prefix & LITERAL_INDEXED == LITERAL_INDEXED {
            parse_int!(U6, prefix, bytes)

        } else if prefix & SIZE_UPDATE == SIZE_UPDATE {
            return Err(E::InvalidSizeUpdate);

        } else {
            // Literal without/never indexed
            parse_int!(U4, prefix, bytes)
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
                        .name
                        .clone(),
                }
            }
            None => {
                HeaderName::from_bytes_lowercase(parse_string(bytes, write_buffer)?)?
            },
        };
        let value = HeaderValue::from_bytes(parse_string(bytes, write_buffer)?)?;

        let field = Field {
            name,
            value,
        };

        let is_added = prefix & BIT7 == BIT7;
        if is_added {
            self.insert(field.clone());
        }

        Ok(field)
    }
}

#[cfg(test)]
impl Table {
    pub(crate) fn fields(&self) -> &VecDeque<Field> {
        &self.fields
    }

    pub(crate) fn size(&self) -> usize {
        self.size
    }

    pub(crate) fn decode_test(
        &mut self,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<Field, DecodeError> {
        self.decode(bytes, write_buffer)
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! parse_int {
    ($int:expr, $prefix:expr, $bytes:expr) => {{
        const CONTINUE_FLAG: u8 = 1 << ($int.count_ones() - 1);
        const INT_MASK: u8 = $int >> 1;
        let init = ($prefix & INT_MASK) as usize;
        if $prefix & CONTINUE_FLAG != CONTINUE_FLAG {
            init
        } else {
            init + continue_parse_int($bytes)?
        }
    }};
}

use {parse_int};

fn parse_string(bytes: &mut Bytes, write_buffer: &mut BytesMut) -> Result<Bytes, DecodeError> {
    //   0   1   2   3   4   5   6   7
    // +---+---+---+---+---+---+---+---+
    // | H |    String Length (7+)     |
    // +---+---------------------------+
    // |  String Data (Length octets)  |
    // +-------------------------------+
    let prefix = bytes.try_get_u8().ok_or(DecodeError::Incomplete)?;

    let len = parse_int!(U7, prefix, bytes);
    let Some(name) = bytes.get(..len) else {
        return Err(DecodeError::Incomplete);
    };

    if prefix & MSB == MSB {
        huffman::decode(name, write_buffer)?;
        Ok(write_buffer.split().freeze())
    } else {
        bytes.try_split_to(len).ok_or(DecodeError::Incomplete)
    }
}

fn continue_parse_int(bytes: &mut Bytes) -> Result<usize, DecodeError> {
    let mut shift = 0;
    let mut value = 0;

    loop {
        let prefix = bytes.try_get_u8().ok_or(DecodeError::Incomplete)?;
        let u7 = prefix & U7;

        value += (u7 as usize) << shift;
        shift += 1;

        if prefix & MSB != MSB {
            break
        }
    }

    Ok(value)
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

impl From<HeaderNameError> for DecodeError {
    fn from(_: HeaderNameError) -> Self {
        Self::InvalidHeader
    }
}

impl From<HeaderValueError> for DecodeError {
    fn from(_: HeaderValueError) -> Self {
        Self::InvalidHeader
    }
}

impl From<HuffmanError> for DecodeError {
    fn from(_: HuffmanError) -> Self {
        Self::Huffman
    }
}

static STATIC_HEADER: [(HeaderName, Option<HeaderValue>); 61] = [
    /* 1 */(standard::PSEUDO_AUTHORITY, None),
    /* 2 */(standard::PSEUDO_METHOD, Some(HeaderValue::from_static(b"GET"))),
    /* 3 */(standard::PSEUDO_METHOD, Some(HeaderValue::from_static(b"POST"))),
    /* 4 */(standard::PSEUDO_PATH, Some(HeaderValue::from_static(b"/"))),
    /* 5 */(standard::PSEUDO_PATH, Some(HeaderValue::from_static(b"/index.html"))),
    /* 6 */(standard::PSEUDO_SCHEME, Some(HeaderValue::from_static(b"http"))),
    /* 7 */(standard::PSEUDO_SCHEME, Some(HeaderValue::from_static(b"https"))),
    /* 8 */(standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"200"))),
    /* 9 */(standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"204"))),
    /* 10 */(standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"206"))),
    /* 11 */(standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"304"))),
    /* 12 */(standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"400"))),
    /* 13 */(standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"404"))),
    /* 14 */(standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"500"))),
    /* 15 */(HeaderName::from_static(b"accept-charset"), None),
    /* 16 */(standard::ACCEPT_ENCODING, Some(HeaderValue::from_static(b"gzip, deflate"))),
    /* 17 */(standard::ACCEPT_LANGUAGE, None),
    /* 18 */(standard::ACCEPT_RANGES, None),
    /* 19 */(standard::ACCEPT, None),
    /* 20 */(standard::ACCESS_CONTROL_ALLOW_ORIGIN, None),
    /* 21 */(standard::AGE, None),
    /* 22 */(standard::ALLOW, None),
    /* 23 */(standard::AUTHORIZATION, None),
    /* 24 */(standard::CACHE_CONTROL, None),
    /* 25 */(standard::CONTENT_DISPOSITION, None),
    /* 26 */(standard::CONTENT_ENCODING, None),
    /* 27 */(standard::CONTENT_LANGUAGE, None),
    /* 28 */(standard::CONTENT_LENGTH, None),
    /* 29 */(standard::CONTENT_LOCATION, None),
    /* 30 */(standard::CONTENT_RANGE, None),
    /* 31 */(standard::CONTENT_TYPE, None),
    /* 32 */(standard::COOKIE, None),
    /* 33 */(standard::DATE, None),
    /* 34 */(standard::ETAG, None),
    /* 35 */(standard::EXPECT, None),
    /* 36 */(standard::EXPIRES, None),
    /* 37 */(standard::FROM, None),
    /* 38 */(standard::HOST, None),
    /* 39 */(standard::IF_MATCH, None),
    /* 40 */(standard::IF_MODIFIED_SINCE, None),
    /* 41 */(standard::IF_NONE_MATCH, None),
    /* 42 */(standard::IF_RANGE, None),
    /* 43 */(standard::IF_UNMODIFIED_SINCE, None),
    /* 44 */(standard::LAST_MODIFIED, None),
    /* 45 */(HeaderName::from_static(b"link"), None),
    /* 46 */(standard::LOCATION, None),
    /* 47 */(standard::MAX_FORWARDS, None),
    /* 48 */(standard::PROXY_AUTHENTICATE, None),
    /* 49 */(standard::PROXY_AUTHORIZATION, None),
    /* 50 */(standard::RANGE, None),
    /* 51 */(standard::REFERER, None),
    /* 52 */(standard::REFRESH, None),
    /* 54 */(standard::RETRY_AFTER, None),
    /* 56 */(standard::SERVER, None),
    /* 58 */(standard::SET_COOKIE, None),
    /* 60 */(standard::STRICT_TRANSPORT_SECURITY, None),
    /* 62 */(standard::TRANSFER_ENCODING, None),
    /* 64 */(standard::USER_AGENT, None),
    /* 66 */(standard::VARY, None),
    /* 68 */(standard::VIA, None),
    /* 70 */(standard::WWW_AUTHENTICATE, None),
];

#[test]
fn test_hpack_appendix_c1_2() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Bytes::copy_from_slice(&[
        0b00011111u8,
        0b10011010,
        0b00001010,
    ]);
    let prefix = bytes[0];
    parse_int!(U5, prefix, &mut bytes);
    Ok(())
}
