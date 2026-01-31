use tcio::bytes::{BufMut, BytesMut};

use crate::h2::hpack::table::{Table, STATIC_HEADER};
use crate::h2::hpack::huffman;
use crate::headers::{HeaderMap, HeaderName, HeaderValue, standard};
use crate::http::{Method, StatusCode};

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
pub struct Encoder {
    table: Table,
}

impl Encoder {
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

    // ===== Encode =====

    pub fn encode_method(&mut self, method: Method, write_buffer: &mut BytesMut) {
        match method {
            Method::GET | Method::POST => {
                // `[0 | 1] + 2` because HPACK is 1-indexed
                write_buffer.put_u8(INDEXED | (matches!(method, Method::POST) as u8 + 2));
            }
            _ => {
                // SAFETY: `Method::as_str` is statically valid ASCII
                let val = unsafe { HeaderValue::unvalidated_static(method.as_str().as_bytes()) };
                self.encode_dynamic(&standard::PSEUDO_METHOD, &val, write_buffer)
            },
        }
    }

    // pub fn encode_path(&mut self, path: &[u8], write_buffer: &mut BytesMut) {
    //     match path {
    //         b"/" | b"/index.html" => {
    //             // `[0 | 1] + 4` because HPACK is 1-indexed
    //             write_buffer.put_u8(INDEXED | (matches!(path, b"/index.html") as u8 + 4));
    //         },
    //         _ => {
    //             // SAFETY: `Method::as_str` is statically valid ASCII
    //             let val = unsafe { HeaderValue::unvalidated_static(path) };
    //             self.encode_dynamic(&standard::PSEUDO_METHOD, &val, write_buffer)
    //         },
    //     }
    // }

    pub fn encode_status(&mut self, status: StatusCode, write_buffer: &mut BytesMut) {
        let idx = match status.status() {
            200 => 7,
            204 => 8,
            206 => 9,
            304 => 10,
            400 => 11,
            404 => 12,
            500 => 13,
            _ => 0,
        };
        if idx != 0 {
            write_buffer.put_u8(INDEXED | idx);
        } else {
            // SAFETY: `Status::status_str` is statically valid ASCII
            let val = unsafe { HeaderValue::unvalidated_static(status.status_str().as_bytes()) };
            self.encode_dynamic(&standard::PSEUDO_STATUS, &val, write_buffer);
        }
    }

    /// Encode headers in header map.
    ///
    /// Note that this does not check for static hpack header name and value, use corresponding method
    /// instead.
    pub fn encode_map(&mut self, map: &HeaderMap, write_buffer: &mut BytesMut) {
        for (name, val) in map {
            self.encode_dynamic(name, val, write_buffer);
        }
    }

    pub fn encode_dynamic(&mut self, name: &HeaderName, val: &HeaderValue, write_buffer: &mut BytesMut) {
        match name.hpack_idx() {
            Some(idx) => {
                // the highest index is 61,
                // this allows for single byte int encoding
                debug_assert!(idx.get() < LITERAL_INDEXED_INT);

                let encoded = (idx.get() - 1) | LITERAL_INDEXED;
                write_buffer.put_u8(encoded);
            }
            None => {
                write_buffer.put_u8(LITERAL_INDEXED); // len 0 to denote literal name
                encode_string(name.as_str().as_bytes(), write_buffer);
            }
        }
        encode_string(val.as_bytes(), write_buffer);
    }
}

macro_rules! encode_int {
    (
        $int:ident, $buffer:expr, $value:expr $(, | $mask:expr)?
    ) => {
        const MAX: u8 = $int >> 1;

        let value = $value;
        if value < MAX as usize {
            $buffer.put_u8((value as u8) $(| $mask)?);
        } else {
            $buffer.put_u8($int $(| $mask)?);
            continue_encode_int(value - $int as usize, $buffer);
        };
    };
}

use {encode_int};

fn continue_encode_int(mut value: usize, bytes: &mut BytesMut) {
    while value > 127 {
        bytes.put_u8(value as u8 | MSB);
        value >>= 7;
    }
    bytes.put_u8(value as u8);
}

fn encode_string(string: &[u8], write_buffer: &mut BytesMut) {
    //   0   1   2   3   4   5   6   7
    // +---+---+---+---+---+---+---+---+
    // | H |    String Length (7+)     |
    // +---+---------------------------+
    // |  String Data (Length octets)  |
    // +-------------------------------+
    encode_int!(U7, write_buffer, string.len(), | IS_HUFFMAN);
    huffman::encode(string, write_buffer);
}

// ===== Test =====

#[test]
fn test_hpack_encode_int() -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = BytesMut::new();
    encode_int!(U5, &mut buffer, 1337usize);
    assert_eq!(
        buffer.as_slice(),
        &[0b0001_1111, 0b1001_1010, 0b0000_1010,][..]
    );
    Ok(())
}

#[test]
fn test_hpack_encode_int2() -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = BytesMut::new();
    encode_int!(U5, &mut buffer, 31usize);
    assert_eq!(
        buffer.as_slice(),
        &[0b0001_1111, 0b0000_0000][..]
    );
    Ok(())
}
