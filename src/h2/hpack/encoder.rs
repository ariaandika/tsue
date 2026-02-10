use tcio::bytes::{BufMut, BytesMut};

use crate::h2::hpack::repr;
use crate::h2::hpack::table::{STATIC_HEADER, Table};
use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue, standard};
use crate::http::{Method, StatusCode};

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
        if let Method::GET | Method::POST = method {
            // GET  => 2 (0 + 2),
            // POST => 3 (1 + 2),
            write_buffer.put_u8(128 | ((method == Method::POST) as u8 + 2));
            return;
        }
        // SAFETY: `Method::as_str` is statically valid ASCII
        let val = unsafe { HeaderValue::unvalidated_static(method.as_str().as_bytes()) };
        self.encode_header(standard::PSEUDO_METHOD, val, write_buffer)
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
            200 => 8,
            204 => 9,
            206 => 10,
            304 => 11,
            400 => 12,
            404 => 13,
            500 => 14,
            _ => 0,
        };
        if idx != 0 {
            write_buffer.put_u8(128 | idx);
            return;
        }
        // SAFETY: `Status::status_str` is statically valid ASCII
        let val = unsafe { HeaderValue::unvalidated_static(status.status_str().as_bytes()) };
        self.encode_header(standard::PSEUDO_STATUS, val, write_buffer);
    }

    /// Encode headers in header map.
    ///
    /// Note that this method skips check for hpack static header with value, use other
    /// corresponding method instead.
    pub fn encode_map(&mut self, map: &HeaderMap, write_buffer: &mut BytesMut) {
        for field in map.fields().iter().filter_map(Option::as_ref) {
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
        let is_large = field.hpack_size() * 4 > self.table.max_size() * 3;

        let (max, repr) = if is_sensitive | is_large {
            // if header is sensitive, use literal never indexed
            let repr = (is_sensitive as u8) << repr::LITERAL_NINDEX_SHIFT;
            (15, repr)

        } else {
            // TODO: optimize hpack dynamic table lookup
            if let Some(i) = self.table.fields().iter().position(|f|f.name() == name) {
                // header is indexed in hpack dynamic table,
                // `+ 1` because HPACK is 1-indexed
                repr::encode_int(127, 128, i + STATIC_HEADER.len() + 1, write_buffer);
                return;
            }

            self.table.insert(field.clone());
            (63, 64)
        };

        if static_index == 0 {
            write_buffer.put_u8(repr);
            repr::encode_string(name.as_str().as_bytes(), write_buffer);
        } else {
            repr::encode_int(max, repr, static_index, write_buffer);
        }

        // value always literal
        repr::encode_string(value.as_bytes(), write_buffer);
    }
}
