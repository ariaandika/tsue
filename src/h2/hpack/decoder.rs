use tcio::bytes::{Buf, Bytes, BytesMut};

use crate::h2::hpack::huffman;
use crate::h2::hpack::table::{STATIC_HEADER, Table, get_static_header_value};
use crate::headers::{self, HeaderField, HeaderMap, HeaderName, HeaderValue};

const MSB: u8 = 0b1000_0000;
const U7: u8 = 0b0111_1111;
const U6: u8 = 0b0011_1111;
const U5: u8 = 0b0001_1111;
const U4: u8 = 0b0000_1111;
const IS_HUFFMAN: u8 = MSB;

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

#[derive(Debug, Default)]
pub struct Decoder {
    table: Table
}

impl Decoder {
    #[inline]
    pub const fn new(max_size: usize) -> Self {
        Self {
            table: Table::new(max_size),
        }
    }

    #[inline]
    pub fn with_capacity(max_size: usize, capacity: usize) -> Self {
        Self {
            table: Table::with_capacity(max_size, capacity),
        }
    }

    // ===== Decode =====

    pub fn decode_block(
        &mut self,
        mut block: Bytes,
        maps: &mut HeaderMap,
        write_buffer: &mut BytesMut,
    ) -> Result<(), DecodeError> {
        let Some(&prefix) = block.first() else {
            return Ok(());
        };
        // Dynamic table size update MUST occur at the beginning of the first header block
        // following the change to the dynamic table size.
        if prefix & SIZE_UPDATE_MASK == SIZE_UPDATE {
            block.advance(1);
            let max_size = decode_int::<SIZE_UPDATE_INT>(prefix, &mut block)?;
            self.table.update_size(max_size);
        }

        while !block.is_empty() {
            let field = self.decode(&mut block, write_buffer)?;
            maps.try_append_field(field)?;
        }
        Ok(())
    }

    fn decode(
        &mut self,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<HeaderField, DecodeError> {
        use DecodeError as E;

        let prefix = bytes.try_get_u8().ok_or(E::Incomplete)?;

        // decoding

        if prefix & INDEXED == INDEXED {
            let index = decode_int::<INDEXED_INT>(prefix, bytes)?
                .checked_sub(1)
                .ok_or(E::ZeroIndex)?;
            if let Some(name) = STATIC_HEADER.get(index) {
                let val = get_static_header_value(index).ok_or(E::NotFound)?;
                return Ok(HeaderField::new(name.clone(), val));
            }
            return self
                .table
                .fields()
                .get(index - STATIC_HEADER.len())
                .cloned()
                .ok_or(E::NotFound);
        }

        let index = if prefix & LITERAL_INDEXED == LITERAL_INDEXED {
            decode_int::<LITERAL_INDEXED_INT>(prefix, bytes)?
        } else if prefix & SIZE_UPDATE_MASK == 0 {
            // Literal without/never indexed
            decode_int::<LITERAL_NINDEX_INT>(prefix, bytes)?
        } else {
            return Err(E::InvalidSizeUpdate);
        };

        // processing

        let (name, hash) = match index.checked_sub(1) {
            Some(index) => {
                // HPACK is 1 indexed
                match STATIC_HEADER.get(index) {
                    Some(name) => (name.clone(), name.hash()),
                    None => {
                        let field = self
                            .table
                            .fields()
                            .get(index - STATIC_HEADER.len())
                            .ok_or(E::NotFound)?;
                        (field.name().clone(), field.cached_hash())
                    }
                }
            }
            None => HeaderName::from_internal_lowercase(decode_string(bytes, write_buffer)?)?,
        };
        let value = HeaderValue::from_bytes(decode_string(bytes, write_buffer)?)?;
        let field = HeaderField::with_hash(name, value, hash);

        let is_indexed = prefix & LITERAL_IS_INDEXED_MASK == LITERAL_IS_INDEXED_MASK;
        if is_indexed {
            self.table.insert(field.clone());
        }

        Ok(field)
    }
}

#[cfg(test)]
impl Decoder {
    pub(crate) fn fields(&self) -> &std::collections::VecDeque<HeaderField> {
        self.table.fields()
    }

    pub(crate) fn size(&self) -> usize {
        self.table.size()
    }

    pub(crate) fn decode_test(
        &mut self,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<HeaderField, DecodeError> {
        self.decode(bytes, write_buffer)
    }
}

fn decode_int<const INT: u8>(prefix: u8, bytes: &mut Bytes) -> Result<usize, DecodeError> {
    let int = prefix & INT;
    if int != INT {
        Ok(int as usize)
    } else {
        Ok((int as usize) + continue_decode_int(bytes)?)
    }
}

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

    let len = decode_int::<U7>(prefix, bytes)?;
    let Some(value) = bytes.get(..len) else {
        return Err(DecodeError::Incomplete);
    };

    if prefix & IS_HUFFMAN == IS_HUFFMAN {
        huffman::decode(value, write_buffer)?;
        let value = write_buffer.split().freeze();
        bytes.advance(len);
        Ok(value)
    } else {
        bytes.try_split_to(len).ok_or(DecodeError::Incomplete)
    }
}

// ===== Error =====

/// HPACK Decoding Error.
#[derive(Debug)]
pub enum DecodeError {
    /// Bytes given is insufficient.
    Incomplete,
    /// Headers is too large.
    TooLarge,
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
            Self::TooLarge => f.write_str("header too large"),
            Self::UnknownRepr => f.write_str("unknown header block representation"),
            Self::ZeroIndex => f.write_str("index cannot be 0"),
            Self::NotFound => f.write_str("field with given index not found"),
            Self::Huffman => f.write_str("huffman coding error"),
            Self::InvalidHeader => f.write_str("invalid header"),
            Self::InvalidSizeUpdate => f.write_str("invalid size update"),
        }
    }
}

impl From<headers::error::HeaderError> for DecodeError {
    fn from(_: headers::error::HeaderError) -> Self {
        Self::InvalidHeader
    }
}

impl From<huffman::HuffmanError> for DecodeError {
    fn from(_: huffman::HuffmanError) -> Self {
        Self::Huffman
    }
}

impl From<headers::error::TryReserveError> for DecodeError {
    fn from(_: headers::error::TryReserveError) -> Self {
        Self::TooLarge
    }
}

// ===== Test =====

#[test]
fn test_hpack_decode_int() {
    let mut bytes = Bytes::copy_from_slice(&[
        0b0001_1111,
        0b1001_1010,
        0b0000_1010,
    ]);
    let prefix = bytes.get_u8();
    let int = decode_int::<U5>(prefix, &mut bytes).unwrap();
    assert!(bytes.is_empty());
    assert_eq!(int, 1337);
}

#[test]
fn test_hpack_decode_int2() {
    let mut bytes = Bytes::copy_from_slice(&[
        0b0001_1111,
        0b0000_0000,
    ]);
    let prefix = bytes.get_u8();
    let int = decode_int::<U5>(prefix, &mut bytes).unwrap();
    assert!(bytes.is_empty());
    assert_eq!(int, 31);
}
