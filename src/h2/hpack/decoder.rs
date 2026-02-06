use tcio::bytes::{Bytes, BytesMut};

use crate::h2::hpack::error::HpackError;
use crate::h2::hpack::repr;
use crate::h2::hpack::table::{STATIC_HEADER, Table, get_static_header_value};
use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue};

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

    /// Decode header block.
    ///
    /// Note that this method does not accept `INDEXED` representation with pseudo headers.
    pub fn decode_block(
        &mut self,
        mut block: Bytes,
        maps: &mut HeaderMap,
        write_buffer: &mut BytesMut,
    ) -> Result<(), HpackError> {
        // Dynamic table size update MUST occur at the beginning of the first header block
        // following the change to the dynamic table size.
        if let Some(size) = repr::decode_size_update(&mut block)? {
            self.table.update_size(size);
        }

        while !block.is_empty() {
            let field = self.decode_inner(&mut block, write_buffer)?;
            maps.try_append_field(field)?;
        }
        Ok(())
    }

    /// Decode single header field.
    ///
    /// Note that this method does not accept `SIZE_UPDATE` or `INDEXED` representation with pseudo
    /// header.
    pub fn decode(
        &mut self,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<HeaderField, HpackError> {
        use HpackError as E;

        let Some(&prefix) = bytes.first() else {
            return Err(E::Incomplete);
        };

        if repr::size_update::is(prefix) {
            return Err(E::InvalidSizeUpdate);
        }

        self.decode_inner(bytes, write_buffer)
    }

    fn decode_inner(
        &mut self,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<HeaderField, HpackError> {
        use HpackError as E;

        if let Some(index) = repr::decode_indexed(bytes)? {
            return match STATIC_HEADER.get(index) {
                Some(name) => {
                    let val = get_static_header_value(index).ok_or(E::NotFound)?;
                    return Ok(HeaderField::new(name.clone(), val));
                }
                _ => self
                    .table
                    .fields()
                    .get(index - STATIC_HEADER.len())
                    .cloned()
                    .ok_or(E::NotFound)
            }
        }

        let (is_indexed, index) = repr::decode_literal(bytes)?;

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
            None => {
                let string = repr::decode_string(bytes, write_buffer)?;
                HeaderName::from_internal_lowercase(string)?
            },
        };
        let value = HeaderValue::from_bytes(repr::decode_string(bytes, write_buffer)?)?;
        let field = HeaderField::with_hash(name, value, hash);

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
    ) -> Result<HeaderField, HpackError> {
        self.decode(bytes, write_buffer)
    }
}
