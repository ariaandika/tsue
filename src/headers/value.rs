use std::str::FromStr;
use tcio::bytes::{ByteStr, Bytes};

use super::error::HeaderError;

/// HTTP Header Value.
#[derive(Clone)]
pub struct HeaderValue {
    bytes: Bytes,
    is_str: bool,
}

impl HeaderValue {
    /// used in iterator.
    pub(crate) fn placeholder() -> Self {
        Self {
            bytes: Bytes::new(),
            is_str: false,
        }
    }

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
                is_str: false,
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
        let value = name.into();
        match validate_header_value(value.as_slice()) {
            Ok(()) => Ok(Self {
                bytes: value,
                is_str: false,
            }),
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
                is_str: false,
            }),
            Err(err) => Err(err),
        }
    }

    /// Parse [`HeaderValue`] from [`ByteStr`].
    ///
    /// This will cache the result and make [`to_str`] and [`as_str`] infallible.
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
            Ok(mut ok) => {
                ok.is_str = true;
                ok
            },
            Err(err) => err.panic_const(),
        }
    }

    /// Returns value as slice.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    /// Parse value as [`str`].
    ///
    /// # Panics
    ///
    /// Panic if header value is not a valid utf8.
    #[inline]
    pub const fn as_str(&self) -> &str {
        match self.try_as_str() {
            Ok(ok) => ok,
            Err(_) => panic!("cannot convert header value as utf8 string"),
        }
    }

    /// Try to parse value as [`str`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if header value is not a valid utf8.
    #[inline]
    pub const fn try_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        match self.is_str {
            true => unsafe { Ok(str::from_utf8_unchecked(self.bytes.as_slice())) },
            false => str::from_utf8(self.bytes.as_slice()),
        }
    }

    /// Parse value as [`str`] and cache the result.
    ///
    /// # Panics
    ///
    /// Panic if header value is not a valid utf8.
    #[inline]
    pub const fn to_str(&mut self) -> &str {
        match self.try_to_str() {
            Ok(ok) => ok,
            Err(_) => panic!("cannot convert header value as utf8 string"),
        }
    }

    /// Try to parse value as [`str`] and cache the result.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if header value is not a valid utf8.
    #[inline]
    pub const fn try_to_str(&mut self) -> Result<&str, std::str::Utf8Error> {
        if !self.is_str {
            if let Err(err) = str::from_utf8(self.bytes.as_slice()) {
                return Err(err);
            };
            self.is_str = true;
        }
        unsafe { Ok(str::from_utf8_unchecked(self.bytes.as_slice())) }
    }
}

impl FromStr for HeaderValue {
    type Err = HeaderError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ok = Self::from_slice(s)?;
        ok.is_str = true;
        Ok(ok)
    }
}

// ===== Parsing =====

const fn validate_header_value(value: &[u8]) -> Result<(), HeaderError> {
    let ptr = value.as_ptr();
    let len = value.len();
    let mut i = 0;
    while i < len {
        unsafe {
            // SAFETY: i < value.len()
            let b = *ptr.add(i);
            if !(b >= b' ' && b != 127 || b == b'\t') {
                return Err(HeaderError::invalid_value());
            }
            // SAFETY: i < value.len()
            i = i.unchecked_add(1);
        }
    }
    Ok(())
}

// ===== Traits =====

impl std::fmt::Debug for HeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"",tcio::fmt::lossy(&self.as_bytes()))
    }
}
