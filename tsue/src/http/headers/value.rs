use bytes::Bytes;
use std::{mem::take, str::{from_utf8, FromStr}};

use crate::common::ByteStr;

/// HTTP Header Value.
pub struct HeaderValue {
    repr: Repr,
}

enum Repr {
    Bytes(Bytes),
    Str(ByteStr),
}

macro_rules! valid {
    ($b:tt) => {
        if $b >= 32 && $b != 127 || $b == b'\t' {
        } else {
            return Err(ERROR);
        }
    };
}

impl HeaderValue {
    pub(crate) const PLACEHOLDER: Self = Self {
        repr: Repr::Bytes(Bytes::new()),
    };

    /// Parse [`HeaderValue`] from [`Bytes`].
    pub fn try_from_slice(value: impl Into<Bytes>) -> Result<Self, InvalidHeaderValue> {
        let bytes: Bytes = value.into();
        for &b in &bytes {
            valid!(b)
        }
        Ok(Self {
            repr: Repr::Bytes(bytes),
        })
    }

    /// Parse [`HeaderValue`] from [`ByteStr`].
    pub fn try_from_string(value: impl Into<ByteStr>) -> Result<HeaderValue, InvalidHeaderValue> {
        let value = value.into();
        for &b in value.as_bytes() {
            valid!(b)
        }
        Ok(Self {
            repr: Repr::Str(value),
        })
    }

    /// Parse [`HeaderValue`] by copying from slice.
    pub fn try_copy_from_slice(value: &[u8]) -> Result<HeaderValue, InvalidHeaderValue> {
        Self::try_from_slice(Bytes::copy_from_slice(value))
    }

    /// Parse [`HeaderValue`] by copying from str.
    pub fn try_copy_from_string(value: &str) -> Result<HeaderValue, InvalidHeaderValue> {
        Self::try_from_string(ByteStr::copy_from_str(value))
    }

    /// Parse [`HeaderValue`] from [`ByteStr`].
    ///
    /// # Panics
    ///
    /// This function will panic if header contains invalid character.
    pub fn from_string(value: impl Into<ByteStr>) -> HeaderValue {
        Self::try_from_string(value).expect("failed to parse header")
    }

    /// Returns value as slice.
    pub fn as_bytes(&self) -> &[u8] {
        match &self.repr {
            Repr::Bytes(b) => b,
            Repr::Str(s) => s.as_bytes(),
        }
    }

    /// Try to parse value as [`str`].
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        match &self.repr {
            Repr::Bytes(b) => from_utf8(b),
            Repr::Str(s) => Ok(s),
        }
    }

    /// Try to parse value as [`str`] and cache the result.
    pub fn to_str(&mut self) -> Result<&str, std::str::Utf8Error> {
        match self.repr {
            Repr::Bytes(ref mut b) => {
                let s = ByteStr::from_utf8(take(b))?;
                self.repr = Repr::Str(s);
                self.as_str()
            }
            Repr::Str(ref s) => Ok(s.as_str()),
        }
    }

    /// Parse `"; "` separated value as [`Iterator`].
    pub fn as_sequence(&self) -> Sequence {
        Sequence {
            value: self.as_str().ok().map(|e| e.split("; ")),
        }
    }
}

impl FromStr for HeaderValue {
    type Err = InvalidHeaderValue;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_copy_from_string(s)
    }
}

// ===== Debug =====

impl std::fmt::Debug for HeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderValue").finish()
    }
}

// ===== Error =====

pub struct InvalidHeaderValue {
    _p: ()
}

const ERROR: InvalidHeaderValue = InvalidHeaderValue {
    _p: ()
};

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

// ===== Sequence =====

/// Parse `"; "` separated value as [`Iterator`].
///
/// This struct is returned from [`as_sequence`][HeaderValue::as_sequence].
#[derive(Debug)]
pub struct Sequence<'a> {
    value: Option<std::str::Split<'a,&'static str>>,
}

impl<'a> Iterator for Sequence<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.value.as_mut()?.next()
    }
}

