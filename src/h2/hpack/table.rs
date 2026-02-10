use std::collections::VecDeque;

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
    /// Returns `Err` if index out of bounds or have no value.
    pub(crate) fn get(&mut self, index: usize) -> Option<&HeaderField> {
        if matches!(index, 0 | 14) {
            // static header without value
            return None;
        }
        if index < STATIC_HEADER_FIELDS.len() {
            return Some(&STATIC_HEADER_FIELDS[index]);
        }
        if index < STATIC_HEADER.len() {
            // static header without value
            return None;
        }
        self.fields.get(index - STATIC_HEADER.len())
    }

    /// Get header name by index.
    ///
    /// # Errors
    ///
    /// Returns `Err` if index not found.
    ///
    /// Returns `Err` if index is referencing pseudo header.
    pub(crate) fn get_name(&mut self, index: usize) -> Option<&HeaderName> {
        match STATIC_HEADER.get(index) {
            Some(name) => Some(name),
            None => self.fields.get(index - STATIC_HEADER.len()).map(HeaderField::name),
        }
    }

    pub(crate) fn insert(&mut self, field: HeaderField) -> std::borrow::Cow<'_, HeaderField> {
        let size = field.hpack_size();

        // It is not an error to attempt to add an entry that is larger than the maximum size; an
        // attempt to add an entry larger than the maximum size causes the table to be emptied of
        // all existing entries and results in an empty table.
        if self.max_size < size {
            self.fields.clear();
            return std::borrow::Cow::Owned(field);
        }

        while self.max_size - self.size < size {
            self.evict_entry();
        }

        self.fields.push_front(field);
        self.size += size;
        let Some(field) = self.fields.front() else {
            // SAFETY: we just `push_front`
            // `push_mut` is unstable
            unsafe { std::hint::unreachable_unchecked() }
        };

        debug_assert!(self.size <= self.max_size);
        std::borrow::Cow::Borrowed(field)
    }

    fn evict_entry(&mut self) -> Option<HeaderField> {
        let field = self.fields.pop_back()?;
        self.size -= field.hpack_size();
        Some(field)
    }
}

#[cfg(test)]
impl Table {
    pub(super) fn size(&self) -> usize {
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

macro_rules! header_fields {
    ($($name:ident, $val:expr),* $(,)?) => {
        [$(HeaderField::new(standard::$name, HeaderValue::from_static($val))),*]
    };
}

static STATIC_HEADER_FIELDS: [HeaderField; 16] = header_fields![
    /* 0*/PSEUDO_AUTHORITY, b"_", // PLACEHOLDER
    /* 1*/PSEUDO_METHOD, b"GET",
    /* 2*/PSEUDO_METHOD, b"POST",
    /* 3*/PSEUDO_PATH, b"/",
    /* 4*/PSEUDO_PATH, b"/index.html",
    /* 5*/PSEUDO_SCHEME, b"http",
    /* 6*/PSEUDO_SCHEME, b"https",
    /* 7*/PSEUDO_STATUS, b"200",
    /* 8*/PSEUDO_STATUS, b"204",
    /* 9*/PSEUDO_STATUS, b"206",
    /*10*/PSEUDO_STATUS, b"304",
    /*11*/PSEUDO_STATUS, b"400",
    /*12*/PSEUDO_STATUS, b"404",
    /*13*/PSEUDO_STATUS, b"500",
    /*14*/ACCEPT_CHARSET, b"_", // PLACEHOLDER
    /*15*/ACCEPT_ENCODING, b"gzip, deflate",
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
}

#[test]
fn test_is_pseudo_header() {
    for name in &STATIC_HEADER[..14] {
        assert!(name.is_pseudo_header());
    }
    for name in &STATIC_HEADER[14..] {
        assert!(!name.is_pseudo_header());
    }
}

