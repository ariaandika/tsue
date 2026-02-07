use std::num::{NonZeroU8};
use tcio::bytes::{Buf, Bytes, BytesMut};

use crate::h2::hpack::{error::HpackError, huffman};

use HpackError as E;

/// 0bx1xx_xxxx = literal with indexed
/// 0bx0xx_xxxx = literal without/never indexed
const LITERAL_IS_INDEXED_MASK: u8 = 0b0100_0000;

// /// Header field name representation.
// #[derive(Debug, Clone, Copy)]
// pub struct Index {
//     index: usize,
//     /// is current representation is `INDEXED`
//     is_indexed: bool,
//     /// is current representation is `LITERAL_INDEXED`
//     is_with_indexing: bool,
// }
//
// impl Index {
//     pub fn decode_chunk(bytes: &mut Bytes) -> Result<Self, HpackError> {
//         use HpackError as E;
//
//         let Some(&prefix) = bytes.first() else {
//             return Err(E::Incomplete);
//         };
//         if prefix & 32 == 32 {
//             return Err(E::InvalidSizeUpdate);
//         }
//         let is_indexed = prefix & 128 == 128;
//         let is_with_indexing = prefix & 64 == 64;
//         let max = if is_indexed {
//             127
//         } else if is_with_indexing {
//             63
//         } else {
//             15
//         };
//         let index = prefix & max;
//         if is_indexed && index == 0 {
//             return Err(E::ZeroIndex);
//         }
//         let index = if index != max {
//             index as usize
//         } else {
//             index as usize + continue_decode_int(bytes)?
//         };
//         Ok(Self {
//             index,
//             is_indexed,
//             is_with_indexing,
//         })
//     }
//
//     /// Returns `Some(index)` if current index use `INDEXED` representation.
//     ///
//     /// The returned index is 0 based.
//     pub fn as_indexed(&self) -> Option<usize> {
//         if self.is_indexed {
//             // This cannot overflow because it is checked that if `self.is_indexed` index cannot be
//             // zero.
//             Some(self.index - 1)
//         } else {
//             None
//         }
//     }
//
//     /// Returns `Some(index)` if current index is non-zero.
//     ///
//     /// If returns `Some`, the index is 0 based.
//     pub fn index(&self) -> Option<usize> {
//         self.index.checked_sub(1)
//     }
//
//     /// Return `true` if current index use `LITERAL_INDEXED` representation.
//     pub fn is_with_indexing(&self) -> bool {
//         self.is_with_indexing
//     }
// }

/// Returns `Some(size_update)` if given bytes is a header block with `SIZE_UPDATE` 
pub fn decode_size_update(bytes: &mut Bytes) -> Result<Option<usize>, HpackError> {
    let Some(&prefix) = bytes.first() else {
        return Ok(None);
    };
    if !size_update::is(prefix) {
        return Ok(None);
    }
    bytes.advance(1);
    let int = prefix & size_update::INT;
    if int != size_update::INT {
        Ok(Some(int as usize))
    } else {
        Ok(Some(int as usize + continue_decode_int(bytes)?))
    }
}

/// Returns `Some(index)` if given bytes is a header block with `INDEXED` representation.
///
/// The `index` is already 0 based.
pub fn decode_indexed(prefix: u8, bytes: &mut Bytes) -> Result<Option<usize>, HpackError> {
    if prefix & 128 == 0 {
        return Ok(None);
    }
    let index = (prefix & 127).checked_sub(1).ok_or(E::ZeroIndex)?;
    if index != 127 {
        Ok(Some(index as usize))
    } else {
        Ok(Some(index as usize + continue_decode_int(bytes)?))
    }
}

/// Returns `Some((is_with_indexing, index))` if given bytes is a header block with `LITERAL_*`
/// representation.
///
/// The returned index can be zero, which denote a string literal.
///
/// This function should be used as a fallback after checking for `INDEXED` or `SIZE_UPDATE`
/// representation.
///
/// # Panics
///
/// Panics in debug mode if the header reresentation is `INDEXED` or `SIZE_UPDATE`.
pub fn decode_literal(prefix: u8, bytes: &mut Bytes) -> Result<(bool, usize), HpackError> {
    debug_assert!(
        prefix & 128 == 0 || prefix & 32 == 0,
        "cannot be INDEXED or SIZE_UPDATE"
    );
    // is "literal" should use incremental indexing
    let is_with_indexing = prefix & 64 == 64;
    let max = if is_with_indexing { 63 } else { 15 };
    let int = prefix & max;
    if int != max {
        Ok((is_with_indexing, int as usize))
    } else {
        Ok((is_with_indexing, int as usize + continue_decode_int(bytes)?))
    }
}

pub fn decode_string(bytes: &mut Bytes, write_buffer: &mut BytesMut) -> Result<Bytes, HpackError> {
    let Some(prefix) = bytes.try_get_u8() else {
        return Err(E::Incomplete);
    };
    let len = prefix & string::INT;
    let len = if len != string::INT {
        len as usize
    } else {
        len as usize + continue_decode_int(bytes)?
    };
    if string::is_huffman(prefix) {
        let value = bytes.get(..len).ok_or(E::Incomplete)?;
        huffman::decode(value, write_buffer)?;
        let value = write_buffer.split().freeze();
        bytes.advance(len);
        Ok(value)
    } else {
        bytes.try_split_to(len).ok_or(E::Incomplete)
    }
}

/// # N-bit Prefix Integer
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | ? | ? | ? |       Value       |
/// +---+---------------------------+
/// |  String Data (Length octets)  |
/// +-------------------------------+
/// N = 5
/// ```
///
/// # Multi Bytes Integer
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | ? | ? | ? | 1   1   1   1   1 |
/// +---+---+---+-------------------+
/// | 1 |    Value-(2^N-1) LSB      |
/// +---+---------------------------+
///                ...
/// +---+---------------------------+
/// | 0 |    Value-(2^N-1) MSB      |
/// +---+---------------------------+
/// ```
pub mod int {
    // Integers are used to represent name indexes, header field indexes, or string lengths.
    pub const MAX: usize = crate::headers::HeaderValue::MAX_LENGTH;

    pub const CONTINUE: u8 = 0b1000_0000;

    const INT: u8 = CONTINUE - 1;

    pub const fn decode_continue(bits: u8) -> (u8, bool) {
        (bits & INT, bits & CONTINUE == CONTINUE)
    }
}

/// # String
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | H |    String Length (7+)     |
/// +---+---------------------------+
/// |  String Data (Length octets)  |
/// +-------------------------------+
/// ```
pub mod string {
    pub const HUFFMAN_FLAG: u8 = 0b1000_0000;
    pub const INT: u8 = HUFFMAN_FLAG - 1;

    pub fn is_huffman(prefix: u8) -> bool {
        prefix & HUFFMAN_FLAG == HUFFMAN_FLAG
    }
}

/// # Indexed
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | 1 |        Index (7+)         |
/// +---+---------------------------+
/// ```
pub mod indexed {
    pub const BITS: u8 = 0b1000_0000;
    pub const INT: u8 = BITS - 1;

    pub const fn is(prefix: u8) -> bool {
        prefix & BITS == BITS
    }
}

/// # Literal Indexed
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | 0 | 1 |      Index (6+)       |
/// +---+---+-----------------------+
/// ```
pub mod literal_indexed {
    pub const BITS: u8 = 0b0100_0000;
    pub const MASK: u8 = 0b1100_0000;
    pub const INT: u8 = BITS - 1;

    pub const fn is(prefix: u8) -> bool {
        prefix & MASK == BITS
    }
}

/// # Size Update
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | 0 | 0 | 1 |   Max size (5+)   |
/// +---+---------------------------+
/// ```
pub mod size_update {
    pub const BITS: u8 = 0b0010_0000;
    pub const MASK: u8 = 0b1110_0000;
    pub const INT: u8 = BITS - 1;

    pub const fn is(prefix: u8) -> bool {
        prefix & MASK == BITS
    }
}

/// # Literal Without Indexing
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | 0 | 0 | 0 | 0 |  Index (4+)   |
/// +---+---+-----------------------+
/// ```
///
/// # Literal Never Indexed
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | 0 | 0 | 0 | 1 |  Index (4+)   |
/// +---+---+-----------------------+
/// ```
pub mod literal_nindexed {
    pub const INT: u8 = 0b0000_1111;

    pub const fn is(prefix: u8) -> bool {
        prefix & super::size_update::MASK == 0
    }
}

#[doc(hidden)]
pub(crate) fn continue_decode_int(bytes: &mut Bytes) -> Result<usize, HpackError> {
    let mut shift = 0;
    let mut value = 0;
    loop {
        let bits = bytes.try_get_u8().ok_or(HpackError::Incomplete)?;
        let (int, is_continue) = int::decode_continue(bits);

        value += (int as usize) << shift;
        shift += 7;

        if value > int::MAX {
            return Err(crate::headers::error::HeaderError::TooLong.into());
        }

        if !is_continue {
            break;
        }
    }
    Ok(value)
}
