use std::str::FromStr;
use tcio::bytes::{ByteStr, Bytes};

use crate::headers::matches;
use crate::headers::error::HeaderError;

/// HTTP Header Value.
///
/// This API does not support non-ASCII value.
#[derive(Clone)]
pub struct HeaderValue {
    /// is ASCII
    bytes: Bytes,
}

impl HeaderValue {
    /// Parse header value from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid header value.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_header_value(bytes) {
            Ok(()) => Self {
                bytes: Bytes::from_static(bytes),
            },
            Err(err) => err.panic_const(),
        }
    }

    /// Parse header value from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns error if the input is not a valid header value.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(name: B) -> Result<Self, HeaderError> {
        let bytes = name.into();
        match validate_header_value(bytes.as_slice()) {
            Ok(()) => Ok(Self { bytes }),
            Err(err) => Err(err),
        }
    }

    /// Parse header value by coyping from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns error if the input is not a valid header value.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(name: A) -> Result<Self, HeaderError> {
        match validate_header_value(name.as_ref()) {
            Ok(()) => Ok(Self {
                bytes: Bytes::copy_from_slice(name.as_ref()),
            }),
            Err(err) => Err(err),
        }
    }

    /// Parse [`HeaderValue`] from string.
    ///
    /// # Panics
    ///
    /// This function will panic if header contains invalid character.
    ///
    /// [`to_str`]: HeaderValue::to_str
    /// [`as_str`]: HeaderValue::as_str
    #[inline]
    pub fn from_string<S: Into<ByteStr>>(value: S) -> HeaderValue {
        match Self::from_bytes(ByteStr::into_bytes(value.into())) {
            Ok(value) => value,
            Err(err) => err.panic_const(),
        }
    }

    /// Returns header value as a byte slice.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    /// Returns header value as `str`.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // `bytes` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.bytes.as_slice()) }
    }
}

// ===== Parsing =====

const MAX_HEADER_VALUE_LEN: usize = 1 << 13;  // 8KB

const fn validate_header_value(mut bytes: &[u8]) -> Result<(), HeaderError> {
    use HeaderError as E;
    match bytes {
        // no leading SP / HTAB
        | [b' ' | b'\t', ..]
        // no trailing SP / HTAB
        | [.., b' ' | b'\t'] => {
            return Err(E::Invalid);
        },
        _ => {}
    }
    // too long
    if bytes.len() > MAX_HEADER_VALUE_LEN {
        return Err(E::TooLong);
    }
    let mut error = false;
    while let [byte, rest @ ..] = bytes {
        error |= !matches::is_header_value(*byte);
        bytes = rest;
    }
    if !error { Ok(()) } else { Err(E::Invalid) }
}

// ===== Traits =====

impl std::fmt::Debug for HeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HeaderValue").field(&self.as_str()).finish()
    }
}

impl FromStr for HeaderValue {
    type Err = HeaderError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s)
    }
}

impl PartialEq for HeaderValue {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl PartialEq<[u8]> for HeaderValue {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.bytes.as_slice() == other
    }
}

impl PartialEq<str> for HeaderValue {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.bytes.as_slice() == other.as_bytes()
    }
}

impl PartialEq<String> for HeaderValue {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.bytes.as_slice() == other.as_bytes()
    }
}

impl From<HeaderValue> for Bytes {
    #[inline]
    fn from(value: HeaderValue) -> Self {
        value.bytes
    }
}
