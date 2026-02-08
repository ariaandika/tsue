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

    /// Returns a fallible iterator to decode header block.
    pub fn decode_block<'a>(
        &'a mut self,
        block: Bytes,
        write_buffer: &'a mut BytesMut,
    ) -> DecodeBlock<'a> {
        DecodeBlock {
            decoder: self,
            block,
            write_buffer,
            can_size_update: true,
        }
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

#[derive(Debug)]
pub struct DecodeBlock<'a> {
    decoder: &'a mut Decoder,
    block: Bytes,
    write_buffer: &'a mut BytesMut,
    can_size_update: bool,
}

impl<'a> DecodeBlock<'a> {
    /// Decode the next header field.
    ///
    /// Returns `None` when the header block is exhausted.
    pub fn next_field(&mut self) -> Result<Option<HeaderField>, HpackError> {
        let Self {
            decoder,
            block,
            write_buffer,
            can_size_update,
        } = self;

        // Dynamic table size update MUST occur at the beginning of the first header block
        // following the change to the dynamic table size.
        if *can_size_update && let Some(size) = repr::decode_size_update(block)? {
            decoder.table.update_size(size);
        }
        *can_size_update = false;

        let Some(prefix) = block.try_get_u8() else {
            return Ok(None);
        };
        decoder.decode_inner(prefix, block, write_buffer).map(Some)
    }
}

impl<'a> Iterator for DecodeBlock<'a> {
    type Item = Result<HeaderField, HpackError>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_field().transpose()
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
