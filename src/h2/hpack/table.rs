use std::collections::VecDeque;

use crate::headers::{HeaderName, HeaderValue, standard};

/// HPACK Table.
#[derive(Debug)]
pub struct Table {
    fields: VecDeque<(HeaderName, HeaderValue)>,
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

    pub(crate) fn fields(&self) -> &VecDeque<(HeaderName, HeaderValue)> {
        &self.fields
    }

    pub(crate) fn update_size(&mut self, max_size: usize) {
        self.max_size = max_size;
        while self.max_size < self.size {
            self.evict_entry();
        }
    }

    pub(crate) fn insert(&mut self, name: HeaderName, val: HeaderValue) {
        let size = field_size(&name, &val);

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

        self.fields.push_front((name, val));
        self.size += size;

        debug_assert!(self.size <= self.max_size);
    }

    fn evict_entry(&mut self) -> Option<(HeaderName, HeaderValue)> {
        let (name, val) = self.fields.pop_back()?;
        let size = field_size(&name, &val);
        self.size -= size;
        Some((name, val))
    }
}

#[cfg(test)]
impl Table {
    pub(crate) fn size(&self) -> usize {
        self.size
    }
}

fn field_size(name: &HeaderName, val: &HeaderValue) -> usize {
    name.as_str().len() + val.as_bytes().len() + 32
}

pub(crate) static STATIC_HEADER: [(HeaderName, Option<HeaderValue>); 61] = [
    (standard::PSEUDO_AUTHORITY, None),
    (standard::PSEUDO_METHOD, Some(HeaderValue::from_static(b"GET"))),
    (standard::PSEUDO_METHOD, Some(HeaderValue::from_static(b"POST"))),
    (standard::PSEUDO_PATH, Some(HeaderValue::from_static(b"/"))),
    (standard::PSEUDO_PATH, Some(HeaderValue::from_static(b"/index.html"))),
    (standard::PSEUDO_SCHEME, Some(HeaderValue::from_static(b"http"))),
    (standard::PSEUDO_SCHEME, Some(HeaderValue::from_static(b"https"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"200"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"204"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"206"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"304"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"400"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"404"))),
    (standard::PSEUDO_STATUS, Some(HeaderValue::from_static(b"500"))),
    (standard::ACCEPT_CHARSET, None),
    (standard::ACCEPT_ENCODING, Some(HeaderValue::from_static(b"gzip, deflate"))),
    (standard::ACCEPT_LANGUAGE, None),
    (standard::ACCEPT_RANGES, None),
    (standard::ACCEPT, None),
    (standard::ACCESS_CONTROL_ALLOW_ORIGIN, None),
    (standard::AGE, None),
    (standard::ALLOW, None),
    (standard::AUTHORIZATION, None),
    (standard::CACHE_CONTROL, None),
    (standard::CONTENT_DISPOSITION, None),
    (standard::CONTENT_ENCODING, None),
    (standard::CONTENT_LANGUAGE, None),
    (standard::CONTENT_LENGTH, None),
    (standard::CONTENT_LOCATION, None),
    (standard::CONTENT_RANGE, None),
    (standard::CONTENT_TYPE, None),
    (standard::COOKIE, None),
    (standard::DATE, None),
    (standard::ETAG, None),
    (standard::EXPECT, None),
    (standard::EXPIRES, None),
    (standard::FROM, None),
    (standard::HOST, None),
    (standard::IF_MATCH, None),
    (standard::IF_MODIFIED_SINCE, None),
    (standard::IF_NONE_MATCH, None),
    (standard::IF_RANGE, None),
    (standard::IF_UNMODIFIED_SINCE, None),
    (standard::LAST_MODIFIED, None),
    (standard::LINK, None),
    (standard::LOCATION, None),
    (standard::MAX_FORWARDS, None),
    (standard::PROXY_AUTHENTICATE, None),
    (standard::PROXY_AUTHORIZATION, None),
    (standard::RANGE, None),
    (standard::REFERER, None),
    (standard::REFRESH, None),
    (standard::RETRY_AFTER, None),
    (standard::SERVER, None),
    (standard::SET_COOKIE, None),
    (standard::STRICT_TRANSPORT_SECURITY, None),
    (standard::TRANSFER_ENCODING, None),
    (standard::USER_AGENT, None),
    (standard::VARY, None),
    (standard::VIA, None),
    (standard::WWW_AUTHENTICATE, None),
];

#[test]
fn test_hpack_static_idx() {
    // +---+---+---+---+---+---+---+---+
    // | 0 | 1 |      Index (6+)       |
    // +---+---+-----------------------+
    const LITERAL_INDEXED_INT: u8 = 0b0011_1111;

    for (name, _) in &STATIC_HEADER {
        let idx = name.hpack_idx().unwrap();

        // this allows for single byte int encoding
        assert!(idx.get() < LITERAL_INDEXED_INT);

        let (name2, _) = &STATIC_HEADER[(idx.get() - 1) as usize];
        assert_eq!(name, name2);
    }
}
