use tcio::bytes::Bytes;

use crate::uri::UriError;

/// URI Scheme.
///
/// The scheme component of a URI.
///
/// ```not_rust
///   foo://example.com:8042/over/there?name=ferret
///   \_/
///    |
/// scheme
///    |
///   / \
///   urn:example:animal:ferret:nose
/// ```
///
/// This API follows the [RFC3986].
///
/// [RFC3986]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.1>
///
/// # Example
///
/// To create [`Scheme`] use one of the `Scheme::from_*` method:
///
/// ```
/// use tsue::uri::Scheme;
/// let scheme = Scheme::from_bytes("foo").unwrap();
/// assert_eq!(scheme.as_str(), "foo");
/// ```
#[derive(Clone)]
pub struct Scheme {
    /// is valid ASCII
    value: Bytes,
}

impl Scheme {
    pub(crate) const unsafe fn new_unchecked(value: Bytes) -> Self {
        Self { value }
    }

    /// Validate scheme from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid scheme.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        if validate_scheme(bytes) {
            Self {
                value: Bytes::from_static(bytes),
            }
        } else {
            UriError::InvalidScheme.panic_const();
        }
    }

    /// Validate scheme from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid scheme.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        if validate_scheme(value.as_slice()) {
            Ok(Self { value })
        } else {
            Err(UriError::InvalidScheme)
        }
    }

    /// Validate scheme by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid scheme.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        if validate_scheme(bytes.as_ref()) {
            Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
            })
        } else {
            Err(UriError::InvalidScheme)
        }
    }
}

impl Scheme {
    /// Extracts a string slice containing the scheme.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }

    /// Checks that two scheme are an ASCII case-insensitive match.
    #[inline]
    pub const fn eq_ignore_ascii_case(&self, scheme: &str) -> bool {
        // Although schemes are case-insensitive, the canonical form is lowercase and documents
        // that specify schemes must do so with lowercase letters.
        self.as_str().eq_ignore_ascii_case(scheme)
    }
}

// ===== std traits =====

impl std::fmt::Debug for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

// ===== Validation =====

crate::matches::ascii_lookup_table! {
    /// `scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`
    pub const fn is_scheme(byte: u8) -> bool {
        byte.is_ascii_alphanumeric()
        || matches!(byte, b'+' | b'-' | b'.')
    }
}

/// `scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`
const fn validate_scheme(mut bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    loop {
        let [byte, rest @ ..] = bytes else {
            return true;
        };
        if !is_scheme(*byte) {
            return false;
        }
        bytes = rest
    }
}
