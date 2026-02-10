use tcio::bytes::{Buf, Bytes, BytesMut};

use crate::h2::hpack::error::HpackError;
use crate::h2::hpack::repr;
use crate::h2::hpack::table::Table;
use crate::headers::{HeaderField, HeaderName, HeaderValue};

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

    #[inline]
    pub fn decode_size_update(&mut self, bytes: &mut Bytes) -> Result<(), HpackError> {
        if let Some(size) = repr::decode_size_update(bytes)? {
            self.table.update_size(size);
        }
        Ok(())
    }

    /// Decode single header field.
    ///
    /// Note that this method does not accept `SIZE_UPDATE` or `INDEXED` representation with pseudo
    /// header.
    #[inline]
    pub fn decode<'a>(
        &'a mut self,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<std::borrow::Cow<'a, HeaderField>, HpackError> {
        let Some(prefix) = bytes.try_get_u8() else {
            return Err(E::Incomplete);
        };
        self.decode_inner(prefix, bytes, write_buffer)
    }

    fn decode_inner<'a>(
        &'a mut self,
        prefix: u8,
        bytes: &mut Bytes,
        write_buffer: &mut BytesMut,
    ) -> Result<std::borrow::Cow<'a, HeaderField>, HpackError> {
        if let Some(index) = repr::decode_indexed(prefix, bytes)? {
            let field = self.table.get(index).ok_or(E::NotFound)?;
            return Ok(std::borrow::Cow::Borrowed(field));
        }

        if repr::is_size_update(prefix) {
            return Err(E::InvalidSizeUpdate);
        }

        let (is_indexed, index) = repr::decode_literal(prefix, bytes)?;
        let (name, hash) = match index.checked_sub(1) {
            Some(index) => {
                let name = self.table.get_name(index).ok_or(E::NotFound)?;
                (name.clone(), name.hash())
            }
            None => {
                let string = repr::decode_string(bytes, write_buffer)?;
                HeaderName::from_internal_lowercase(string)?
            },
        };
        let value = HeaderValue::from_bytes(repr::decode_string(bytes, write_buffer)?)?;
        let field = HeaderField::with_hash(name, value, hash);

        let field = if is_indexed {
            self.table.insert(field)
        } else {
            std::borrow::Cow::Owned(field)
        };

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
        self.decode(bytes, write_buffer).map(std::borrow::Cow::into_owned)
    }
}
