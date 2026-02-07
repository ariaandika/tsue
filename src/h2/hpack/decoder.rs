use tcio::bytes::{Buf, Bytes, BytesMut};

use crate::h2::hpack::error::HpackError;
use crate::h2::hpack::repr;
use crate::h2::hpack::table::Table;
use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue};

use HpackError as E;

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
    /// Note that this method returns `Err` if it found pseudo header.
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

        while let Some(prefix) = block.try_get_u8() {
            let field = self.decode_inner(prefix, &mut block, write_buffer)?;
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
        let Some(&prefix) = bytes.first() else {
            return Err(E::Incomplete);
        };
        if repr::is_size_update(prefix) {
            return Err(E::InvalidSizeUpdate);
        }
        bytes.advance(1);

        self.decode_inner(prefix, bytes, write_buffer)
    }

    fn decode_inner(
        &mut self,
        prefix: u8,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<HeaderField, HpackError> {
        if let Some(index) = repr::decode_indexed(prefix, bytes)? {
            let field = self.table.get(index).ok_or(E::NotFound)?;
            if field.name().is_pseudo_header() {
                return Err(E::InvalidPseudoHeader);
            }
            return Ok(field.clone());
        }

        let (is_indexed, index) = repr::decode_literal(prefix, bytes)?;
        let (name, hash) = match index.checked_sub(1) {
            Some(index) => {
                let name = self.table.get_name(index).ok_or(E::NotFound)?;
                if name.is_pseudo_header() {
                    return Err(E::InvalidPseudoHeader);
                }
                (name.clone(), name.hash())
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
