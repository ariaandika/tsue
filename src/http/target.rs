use tcio::bytes::Bytes;

use crate::uri::{UriError, matches};

/// HTTP Request Target.
///
/// `Target` contains [path] and optional [query] component from URI.
///
/// `Target` is retrieved from `HTTP/1.1` request target in request line, or `:path` pseudo-header
/// in `HTTP/2.0`.
///
/// [path]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.3>
/// [query]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.4>
#[derive(Clone)]
pub struct Target {
    /// is valid ASCII
    value: Bytes,
    query: u32,
}

impl Default for Target {
    #[inline]
    fn default() -> Self {
        Self::root()
    }
}

impl Target {
    /// Returns request target with value `/`.
    #[inline]
    pub const fn root() -> Self {
        Self {
            value: Bytes::from_static(b"/"),
            query: 1,
        }
    }

    /// Validate request target from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid target.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_path(bytes) {
            Ok(query) => Self {
                value: Bytes::from_static(bytes),
                query,
            },
            Err(err) => err.panic_const(),
        }
    }

    /// Validate request target from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid target.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        match validate_path(value.as_slice()) {
            Ok(query) => Ok(Self { value, query }),
            Err(err) => Err(err),
        }
    }

    /// Validate request by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid target.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match validate_path(bytes.as_ref()) {
            Ok(query) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
                query,
            }),
            Err(err) => Err(err),
        }
    }

    /// Returns the path component.
    #[inline]
    pub const fn path(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII, `query` is less than or equal to `value`
        // length
        unsafe { str_from_parts!(self.value.as_ptr(), self.query as usize) }
    }

    /// Returns the query component.
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        let offset = (self.query + 1) as usize;
        if offset < self.value.len() {
            // SAFETY: precondition `value` is valid ASCII
            unsafe {
                Some(str_from_parts!(
                    self.value.as_ptr().add(offset),
                    self.value.len() - offset
                ))
            }
        } else {
            None
        }
    }

    /// Extracts a string slice containing the request target.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

// ===== std validation =====

impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

// ===== Validation =====

macro_rules! str_from_parts {
    ($d:expr, $l:expr) => {
        str::from_utf8_unchecked(std::slice::from_raw_parts($d, $l))
    };
}

use str_from_parts;

matches::ascii_lookup_table! {
    /// `pchar            = unreserved / pct-encoded / sub-delims / ":" / "@"`
    /// `pchar-and-slash  = pchar / "/"`
    const fn is_pchar_and_slash(byte: u8) -> bool {
        matches::unreserved(byte)
        || matches::pct_encoded(byte)
        || matches::sub_delims(byte)
        || matches!(byte, b':' | b'@')
        || matches!(byte, b'/')
    }
}

matches::ascii_lookup_table! {
    /// `query = *( pchar / "/" / "?" )`
    const fn is_query(byte: u8) -> bool {
        is_pchar_and_slash(byte)
        || matches!(byte, b'/' | b'?')
    }
}

const fn validate_path(mut bytes: &[u8]) -> Result<u32, UriError> {
    match match_path(&mut bytes) {
        Ok(query) => if bytes.is_empty() {
            Ok(query)
        } else {
            Err(UriError::InvalidPath)
        },
        Err(err) => Err(err),
    }
}

/// ```not_rust
/// origin-form     = absolute-path [ "?" query ]
/// absolute-path   = 1*( "/" segment )
/// segment         = *pchar
/// ```
pub(crate) const fn match_path(bytes: &mut &[u8]) -> Result<u32, UriError> {
    if bytes.len() > u32::MAX as usize {
        return Err(UriError::ExcessiveBytes);
    }

    let base = bytes.as_ptr();

    let Some((prefix, state)) = bytes.split_first() else {
        return Err(UriError::InvalidPath);
    };

    if *prefix != b'/' {
        return Err(UriError::InvalidPath);
    }

    if state.is_empty() {
        return Ok(1);
    }
    *bytes = state;

    while let [byte, rest @ ..] = bytes {
        if !is_pchar_and_slash(*byte) {
            break;
        }
        *bytes = rest;
    }

    let Some((delim, rest)) = bytes.split_first() else {
        return unsafe {
            Ok(bytes.as_ptr().offset_from_unsigned(base) as u32)
        }
    };

    let query = unsafe { bytes.as_ptr().offset_from_unsigned(base) };
    if *delim != b'?' {
        return Err(UriError::InvalidPath);
    }
    *bytes = rest;

    loop {
        let [byte, rest @ ..] = bytes else {
            return Ok(query as u32);
        };
        if !is_query(*byte) {
            return Err(UriError::InvalidPath);
        }
        *bytes = rest;
    }
}

#[test]
fn test_path() {
    let target = Target::from_slice(b"/users/all").unwrap();
    assert_eq!(target.as_str(), "/users/all");
    assert_eq!(target.path(), "/users/all");
    assert_eq!(target.query(), None);

    let target = Target::from_slice(b"/users/all?").unwrap();
    assert_eq!(target.as_str(), "/users/all?");
    assert_eq!(target.path(), "/users/all");
    assert_eq!(target.query(), None);

    let target = Target::from_slice(b"/users/all?page=420").unwrap();
    assert_eq!(target.as_str(), "/users/all?page=420");
    assert_eq!(target.path(), "/users/all");
    assert_eq!(target.query(), Some("page=420"));
}
