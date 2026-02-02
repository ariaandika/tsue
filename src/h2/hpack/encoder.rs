use tcio::bytes::{BufMut, BytesMut};

use crate::h2::hpack::table::{Table, STATIC_HEADER};
use crate::h2::hpack::huffman;
use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue, standard};
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
const LITERAL_NOINDEX: u8 = 0b0000_0000;
const LITERAL_NEINDEX: u8 = 0b0001_0000;
const LITERAL_NINDEX_INT: u8 = U4;

/// 0bx1xx_xxxx = literal with indexed
/// 0bx0xx_xxxx = literal without/never indexed
const LITERAL_IS_INDEXED_MASK: u8 = 0b0100_0000;

/// 0bxxx0_xxxx = literal without indexed
/// 0bxxx1_xxxx = literal never indexed
const LITERAL_NINDEX_SHIFT: u8 = 4;

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
                // GET  => 2 (0 + 2),
                // POST => 3 (1 + 2),
                write_buffer.put_u8(INDEXED | (matches!(method, Method::POST) as u8 + 2));
            }
            _ => {
                // SAFETY: `Method::as_str` is statically valid ASCII
                let val = unsafe { HeaderValue::unvalidated_static(method.as_str().as_bytes()) };
                self.encode_header(standard::PSEUDO_METHOD, val.clone(), write_buffer)
            },
        }
    }

    // pub fn encode_path(&mut self, path: &[u8], write_buffer: &mut BytesMut) {
    //     match path {
    //         b"/" | b"/index.html" => {
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
            self.encode_header(standard::PSEUDO_STATUS, val.clone(), write_buffer);
        }
    }

    /// Encode headers in header map.
    ///
    /// Note that this method skips check for hpack static header with value, use other
    /// corresponding method instead.
    pub fn encode_map(&mut self, map: &HeaderMap, write_buffer: &mut BytesMut) {
        for field in map.fields().iter().filter_map(|e|e.as_ref()) {
            self.encode_dynamic(field, write_buffer);
        }
    }

    /// Encode a single header.
    ///
    /// Note that this method skips check for hpack static header with value, use other
    /// corresponding method instead.
    pub fn encode_header(&mut self, name: HeaderName, val: HeaderValue, write_buffer: &mut BytesMut) {
        self.encode_dynamic(&HeaderField::new(name, val), write_buffer);
    }

    fn encode_dynamic(&mut self, field: &HeaderField, write_buffer: &mut BytesMut) {
        let name = field.name();
        let value = field.value();
        let static_index = name.hpack_static().map(std::num::NonZero::get).unwrap_or(0) as usize;

        let is_sensitive = field.is_sensitive();
        let is_large = field_size(name, value) * 4 > self.table.max_size() * 3;

        if is_sensitive | is_large {
            // if header is sensitive, use literal never indexed
            let repr = (is_sensitive as u8) << LITERAL_NINDEX_SHIFT;
            encode_int!(LITERAL_NINDEX_INT, write_buffer, static_index, | repr);

        } else {
            // TODO: optimize hpack dynamic table lookup
            if let Some(i) = self.table.fields().iter().position(|(n,_)|n == name) {
                // header is indexed in hpack dynamic table,
                // `+ 1` because HPACK is 1-indexed
                write_buffer.put_u8((i + STATIC_HEADER.len() + 1) as u8 | INDEXED);
                return;
            }

            self.table.insert(name.clone(), value.clone());
            encode_int!(LITERAL_INDEXED_INT, write_buffer, static_index, | LITERAL_INDEXED);
        }

        if static_index == 0 {
            encode_string(name.as_str().as_bytes(), write_buffer);
        }
        // value always literal
        encode_string(value.as_bytes(), write_buffer);
    }
}

macro_rules! encode_int {
    (
        $int:ident, $buffer:expr, $value:expr $(, | $mask:expr)?
    ) => {{
        const MAX: u8 = $int >> 1;

        let value = $value;
        if value < MAX as usize {
            $buffer.put_u8((value as u8) $(| $mask)?);
        } else {
            $buffer.put_u8($int $(| $mask)?);
            continue_encode_int(value - $int as usize, $buffer);
        }
    }};
}

use {encode_int};

// fn one_encode_int<const INT: u8, const MAX: u8>(value: usize, mask: u8, buffer: &mut BytesMut) {
//     if value < MAX as usize {
//         buffer.put_u8((value as u8) | mask);
//     } else {
//         buffer.put_u8(INT | mask);
//         continue_encode_int(value - INT as usize, buffer);
//     };
// }

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

fn field_size(name: &HeaderName, val: &HeaderValue) -> usize {
    name.as_str().len() + val.as_bytes().len() + 32
}

// ===== Test =====

#[test]
fn test_hpack_encode_int() {
    let mut buffer = BytesMut::new();
    encode_int!(U5, &mut buffer, 1337usize);
    assert_eq!(
        buffer.as_slice(),
        &[0b0001_1111, 0b1001_1010, 0b0000_1010,][..]
    );
}

#[test]
fn test_hpack_encode_int2() {
    let mut buffer = BytesMut::new();
    encode_int!(U5, &mut buffer, 31usize);
    assert_eq!(
        buffer.as_slice(),
        &[0b0001_1111, 0b0000_0000][..]
    );
}
