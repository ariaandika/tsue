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
///
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
///
/// # Indexed
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | 1 |        Index (7+)         |
/// +---+---------------------------+
/// ```
///
/// # Literal Indexed
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | 0 | 1 |      Index (6+)       |
/// +---+---+-----------------------+
/// ```
///
/// # Size Update
///
/// ```not_rust
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | 0 | 0 | 1 |   Max size (5+)   |
/// +---+---------------------------+
/// ```
///
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
use tcio::bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::h2::hpack::error::HpackError;
use crate::h2::hpack::huffman;

use HpackError as E;

pub fn is_size_update(prefix: u8) -> bool {
    prefix & 0b1110_0000 == 32
}

// ===== Decode =====

/// Returns `Some(size_update)` if given bytes contains a header field with `SIZE_UPDATE`
/// representation.
pub fn decode_size_update(bytes: &mut Bytes) -> Result<Option<usize>, HpackError> {
    let Some(&prefix) = bytes.first() else {
        return Ok(None);
    };
    if !is_size_update(prefix) {
        return Ok(None);
    }
    bytes.advance(1);
    let int = prefix & 31;
    if int != 31 {
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
        prefix & 128 == 0 || is_size_update(prefix),
        "cannot be INDEXED or SIZE_UPDATE"
    );
    // is "literal" use incremental indexing
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
    let len = prefix & 127;
    let len = if len != 127 {
        len as usize
    } else {
        len as usize + continue_decode_int(bytes)?
    };
    if prefix & 128 == 128 {
        let value = bytes.get(..len).ok_or(E::Incomplete)?;
        huffman::decode(value, write_buffer)?;
        let value = write_buffer.split().freeze();
        bytes.advance(len);
        Ok(value)
    } else {
        bytes.try_split_to(len).ok_or(E::Incomplete)
    }
}

fn continue_decode_int(bytes: &mut Bytes) -> Result<usize, HpackError> {
    // Integers are used to represent name indexes, header field indexes, or string lengths.
    const MAX: usize = crate::headers::HeaderValue::MAX_LENGTH;

    let mut shift = 0;
    let mut value = 0;
    loop {
        let bits = bytes.try_get_u8().ok_or(E::Incomplete)?;
        let int = bits & 127;

        value += (int as usize) << shift;
        shift += 7;

        if value > MAX {
            return Err(crate::headers::error::HeaderError::TooLong.into());
        }
        if bits & 128 == 0 {
            break;
        }
    }
    Ok(value)
}

// ===== Encode =====

/// 0bxxx0_xxxx = literal without indexed
/// 0bxxx1_xxxx = literal never indexed
pub const LITERAL_NINDEX_SHIFT: u8 = 4;

pub fn encode_int(max: u8, repr: u8, value: usize, write_buffer: &mut BytesMut) {
    write_buffer.put_u8((value as u8 & max) | repr);
    let Some(mut value) = value.checked_sub(max as usize) else {
        return
    };
    while value > 127 {
        write_buffer.put_u8(value as u8 | 128);
        value >>= 7;
    }
    write_buffer.put_u8(value as u8);
}

pub fn encode_string(string: &[u8], write_buffer: &mut BytesMut) {
    encode_int(127, 128, string.len(), write_buffer);
    huffman::encode(string, write_buffer);
}

