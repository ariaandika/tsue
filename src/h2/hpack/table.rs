use std::collections::VecDeque;

use crate::h2::hpack::error::HpackError;
use crate::headers::{HeaderField, HeaderName, HeaderValue, standard};

/// HPACK Table.
#[derive(Debug)]
pub struct Table {
    fields: VecDeque<HeaderField>,
    size: usize,
    max_size: usize,
}

impl Default for Table {
    #[inline]
    fn default() -> Self {
        Self {
            fields: VecDeque::new(),
            size: 0,
            max_size: 4096,
        }
    }
}

impl Table {
    #[inline]
    pub const fn new(max_size: usize) -> Table {
        Self {
            fields: VecDeque::new(),
            size: 0,
            max_size,
        }
    }

    #[inline]
    pub fn with_capacity(max_size: usize, capacity: usize) -> Table {
        Self {
            fields: VecDeque::with_capacity(capacity),
            size: 0,
            max_size,
        }
    }

    pub(crate) fn fields(&self) -> &VecDeque<HeaderField> {
        &self.fields
    }

    pub(crate) fn max_size(&self) -> usize {
        self.max_size
    }

    pub(crate) fn update_size(&mut self, max_size: usize) {
        self.max_size = max_size;
        while self.max_size < self.size {
            self.evict_entry();
        }
    }

    /// Get header field by index.
    ///
    /// # Errors
    ///
    /// Returns `Err` if index not found.
    ///
    /// Returns `Err` if index is referencing pseudo header.
    pub(crate) fn get(&mut self, index: usize) -> Result<HeaderField, HpackError> {
        use HpackError as E;

        if index < 15 {
            return Err(E::InvalidPseudoHeader);
        }
        if index == 15 {
            return Ok(HeaderField::new(
                STATIC_HEADER[index].clone(),
                STATIC_HEADER_VALUES[index].clone(),
            ));
        }
        let Some(index) = index.checked_sub(STATIC_HEADER.len()) else {
            // static header without value
            return Err(E::NotFound);
        };
        Ok(self.fields.get(index).ok_or(HpackError::NotFound)?.clone())
    }

    /// Get header name by index.
    ///
    /// # Errors
    ///
    /// Returns `Err` if index not found.
    ///
    /// Returns `Err` if index is referencing pseudo header.
    pub(crate) fn get_name(&mut self, index: usize) -> Result<HeaderName, HpackError> {
        if index <= LAST_PSEUDO_HEADER_INDEX {
            return Err(HpackError::InvalidPseudoHeader);
        }
        match STATIC_HEADER.get(index) {
            Some(name) => Ok(name.clone()),
            None => Ok(self
                .fields
                .get(index - STATIC_HEADER.len())
                .ok_or(HpackError::NotFound)?
                .name()
                .clone()),
        }
    }

    pub(crate) fn insert(&mut self, field: HeaderField) {
        let size = field.hpack_size();

        // It is not an error to attempt to add an entry that is larger than the maximum size; an
        // attempt to add an entry larger than the maximum size causes the table to be emptied of
        // all existing entries and results in an empty table.
        if self.max_size < size {
            self.fields.clear();
            return;
        }

        while self.max_size - self.size < size {
            self.evict_entry();
        }

        self.fields.push_front(field);
        self.size += size;

        debug_assert!(self.size <= self.max_size);
    }

    fn evict_entry(&mut self) -> Option<HeaderField> {
        let field = self.fields.pop_back()?;
        self.size -= field.hpack_size();
        Some(field)
    }
}

#[cfg(test)]
impl Table {
    pub(crate) fn size(&self) -> usize {
        self.size
    }
}

pub(crate) static STATIC_HEADER: [HeaderName; 61] = [
    standard::PSEUDO_AUTHORITY,
    standard::PSEUDO_METHOD,
    standard::PSEUDO_METHOD,
    standard::PSEUDO_PATH,
    standard::PSEUDO_PATH,
    standard::PSEUDO_SCHEME,
    standard::PSEUDO_SCHEME,
    standard::PSEUDO_STATUS,
    standard::PSEUDO_STATUS,
    standard::PSEUDO_STATUS,
    standard::PSEUDO_STATUS,
    standard::PSEUDO_STATUS,
    standard::PSEUDO_STATUS,
    standard::PSEUDO_STATUS,
    standard::ACCEPT_CHARSET,
    standard::ACCEPT_ENCODING,
    standard::ACCEPT_LANGUAGE,
    standard::ACCEPT_RANGES,
    standard::ACCEPT,
    standard::ACCESS_CONTROL_ALLOW_ORIGIN,
    standard::AGE,
    standard::ALLOW,
    standard::AUTHORIZATION,
    standard::CACHE_CONTROL,
    standard::CONTENT_DISPOSITION,
    standard::CONTENT_ENCODING,
    standard::CONTENT_LANGUAGE,
    standard::CONTENT_LENGTH,
    standard::CONTENT_LOCATION,
    standard::CONTENT_RANGE,
    standard::CONTENT_TYPE,
    standard::COOKIE,
    standard::DATE,
    standard::ETAG,
    standard::EXPECT,
    standard::EXPIRES,
    standard::FROM,
    standard::HOST,
    standard::IF_MATCH,
    standard::IF_MODIFIED_SINCE,
    standard::IF_NONE_MATCH,
    standard::IF_RANGE,
    standard::IF_UNMODIFIED_SINCE,
    standard::LAST_MODIFIED,
    standard::LINK,
    standard::LOCATION,
    standard::MAX_FORWARDS,
    standard::PROXY_AUTHENTICATE,
    standard::PROXY_AUTHORIZATION,
    standard::RANGE,
    standard::REFERER,
    standard::REFRESH,
    standard::RETRY_AFTER,
    standard::SERVER,
    standard::SET_COOKIE,
    standard::STRICT_TRANSPORT_SECURITY,
    standard::TRANSFER_ENCODING,
    standard::USER_AGENT,
    standard::VARY,
    standard::VIA,
    standard::WWW_AUTHENTICATE,
];

const LAST_PSEUDO_HEADER_INDEX: usize = 13;

static STATIC_HEADER_VALUES: [HeaderValue; 16] = [
    HeaderValue::from_static(b"_"), // PLACEHOLDER
    HeaderValue::from_static(b"GET"),
    HeaderValue::from_static(b"POST"),
    HeaderValue::from_static(b"/"),
    HeaderValue::from_static(b"/index.html"),
    HeaderValue::from_static(b"http"),
    HeaderValue::from_static(b"https"),
    HeaderValue::from_static(b"200"),
    HeaderValue::from_static(b"204"),
    HeaderValue::from_static(b"206"),
    HeaderValue::from_static(b"304"),
    HeaderValue::from_static(b"400"),
    HeaderValue::from_static(b"404"),
    HeaderValue::from_static(b"500"),
    HeaderValue::from_static(b"_"), // PLACEHOLDER
    HeaderValue::from_static(b"gzip, deflate"),
];

#[test]
fn test_hpack_static_idx() {
    // +---+---+---+---+---+---+---+---+
    // | 0 | 1 |      Index (6+)       |
    // +---+---+-----------------------+
    const LITERAL_INDEXED_INT: u8 = 0b0011_1111;

    for name in &STATIC_HEADER {
        let idx = name.hpack_static().unwrap();

        assert!(idx.get() < LITERAL_INDEXED_INT);

        let name2 = &STATIC_HEADER[(idx.get() - 1) as usize];
        assert_eq!(name, name2);
    }

    assert_eq!(STATIC_HEADER[LAST_PSEUDO_HEADER_INDEX], standard::PSEUDO_STATUS);
    assert_eq!(STATIC_HEADER[LAST_PSEUDO_HEADER_INDEX + 1], standard::ACCEPT_CHARSET);
}

