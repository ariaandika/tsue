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
    query: u16,
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

    pub(crate) fn from_bytes(bytes: Bytes) -> Result<Self, UriError> {
        match validate_path(bytes.as_slice()) {
            Ok(query) => Ok(Self {
                value: bytes,
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
        if offset == self.value.len() {
            None
        } else {
            // SAFETY: precondition `value` is valid ASCII
            unsafe {
                let query = self.query as usize;
                Some(str_from_parts!(
                    self.value.as_ptr().add(query + 1),
                    self.value.len() - query - 1
                ))
            }
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
    /// `pchar = unreserved / pct-encoded / sub-delims / ":" / "@"`
    const fn is_pchar(byte: u8) -> bool {
        matches::unreserved(byte)
        || matches::pct_encoded(byte)
        || matches::sub_delims(byte)
        || matches!(byte, b':' | b'@')
    }
}

matches::ascii_lookup_table! {
    /// `query = *( pchar / "/" / "?" )`
    const fn is_query(byte: u8) -> bool {
        is_pchar(byte)
        || matches!(byte, b'/' | b'?')
    }
}

/// ```not_rust
/// origin-form     = absolute-path [ "?" query ]
/// absolute-path   = 1*( "/" segment )
/// segment         = *pchar
/// ```
const fn validate_path(bytes: &[u8]) -> Result<u16, UriError> {
    let Some((prefix, mut state)) = bytes.split_first() else {
        return Err(UriError::InvalidPath);
    };

    if *prefix != b'/' {
        return Err(UriError::InvalidPath);
    }

    if state.is_empty() {
        return Ok(1);
    }

    let mut query = bytes.len();

    while let [byte, rest @ ..] = state {
        state = rest;
        if !is_pchar(*byte) {
            if *byte != b'?' {
                return Err(UriError::InvalidPath);
            }
            query = unsafe { state.as_ptr().offset_from_unsigned(bytes.as_ptr()) };
            break;
        }
    }

    loop {
        let [byte, rest @ ..] = state else {
            return if query <= (u16::MAX as usize) {
                Ok(query as u16)
            } else {
                Err(UriError::ExcessiveBytes)
            };
        };
        if !is_query(*byte) {
            return Err(UriError::InvalidPath);
        }
        state = rest;
    }
}
