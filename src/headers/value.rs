use bytes::Bytes;
use std::{
    mem::take,
    str::{FromStr, from_utf8},
};
use tcio::ByteStr;

// ===== HeaderValue =====

/// HTTP Header Value.
#[derive(Clone)]
pub struct HeaderValue {
    repr: Repr,
}

#[derive(Clone)]
enum Repr {
    Bytes(Bytes),
    Str(ByteStr),
}

impl HeaderValue {
    /// used in iterator.
    pub(crate) const PLACEHOLDER: Self = Self {
        repr: Repr::Bytes(Bytes::new()),
    };

    /// Parse [`HeaderValue`] from slice.
    #[inline]
    pub fn try_from_slice(value: impl Into<Bytes>) -> Result<Self, InvalidHeaderValue> {
        let bytes: Bytes = value.into();
        match parse_from_slice(&bytes) {
            Ok(()) => Ok(Self {
                repr: Repr::Bytes(bytes),
            }),
            Err(err) => Err(err),
        }
    }

    /// Parse [`HeaderValue`] from string.
    ///
    /// This will cache the result and make [`to_str`] and [`as_str`] infallible.
    ///
    /// [`to_str`]: HeaderValue::to_str
    /// [`as_str`]: HeaderValue::as_str
    #[inline]
    pub fn try_from_string(value: impl Into<ByteStr>) -> Result<HeaderValue, InvalidHeaderValue> {
        let value: ByteStr = value.into();
        match parse_from_slice(value.as_bytes()) {
            Ok(()) => Ok(Self {
                repr: Repr::Str(value),
            }),
            Err(err) => Err(err),
        }
    }

    /// Parse [`HeaderValue`] by copying from slice.
    #[inline]
    pub fn try_copy_from_slice(value: &[u8]) -> Result<HeaderValue, InvalidHeaderValue> {
        match parse_from_slice(value) {
            Ok(()) => Ok(Self {
                repr: Repr::Bytes(Bytes::copy_from_slice(value)),
            }),
            Err(err) => Err(err),
        }
    }

    /// Parse [`HeaderValue`] by copying from str.
    ///
    /// This will cache the result and make [`to_str`] and [`as_str`] infallible.
    ///
    /// [`to_str`]: HeaderValue::to_str
    /// [`as_str`]: HeaderValue::as_str
    #[inline]
    pub fn try_copy_from_string(value: &str) -> Result<HeaderValue, InvalidHeaderValue> {
        match parse_from_slice(value.as_bytes()) {
            Ok(()) => Ok(Self {
                repr: Repr::Str(ByteStr::copy_from_str(value)),
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
    pub fn from_string(value: impl Into<ByteStr>) -> HeaderValue {
        Self::try_from_string(value).expect("called `HeaderValue::from_string` with invalid bytes")
    }

    /// Returns value as slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.repr {
            Repr::Bytes(b) => b,
            Repr::Str(s) => s.as_bytes(),
        }
    }

    /// Parse value as [`str`].
    ///
    /// # Panics
    ///
    /// Panic if header value is not a valid utf8.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.try_as_str()
            .expect("cannot convert header value as utf8 string")
    }

    /// Try to parse value as [`str`].
    #[inline]
    pub fn try_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        match &self.repr {
            Repr::Bytes(b) => from_utf8(b),
            Repr::Str(s) => Ok(s),
        }
    }

    /// Parse value as [`str`] and cache the result.
    ///
    /// # Panics
    ///
    /// Panic if header value is not a valid utf8.
    #[inline]
    pub fn to_str(&mut self) -> &str {
        self.try_to_str()
            .expect("cannot convert header value as utf8 string")
    }

    /// Try to parse value as [`str`] and cache the result.
    #[inline]
    pub fn try_to_str(&mut self) -> Result<&str, std::str::Utf8Error> {
        match self.repr {
            Repr::Bytes(ref mut b) => {
                let s = ByteStr::from_utf8(take(b))?;
                self.repr = Repr::Str(s);
                self.try_as_str()
            }
            Repr::Str(ref s) => Ok(s.as_str()),
        }
    }
}

impl FromStr for HeaderValue {
    type Err = InvalidHeaderValue;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_copy_from_string(s)
    }
}

// ===== Parsing =====

const fn parse_from_slice(value: &[u8]) -> Result<(), InvalidHeaderValue> {
    let ptr = value.as_ptr();
    let len = value.len();
    let mut i = 0;
    while i < len {
        unsafe {
            // SAFETY: i < value.len()
            let b = *ptr.add(i);
            if b >= b' ' && b != 127 || b == b'\t' {
            } else {
                return Err(InvalidHeaderValue { });
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

// ===== Error =====

/// An error that can occur when parsing header value.
#[non_exhaustive]
pub struct InvalidHeaderValue {

}

impl std::error::Error for InvalidHeaderValue { }

impl std::fmt::Display for InvalidHeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("header contains invalid bytes")
    }
}

impl std::fmt::Debug for InvalidHeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvalidHeaderValue").finish()
    }
}
