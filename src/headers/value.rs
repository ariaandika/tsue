use std::fmt;
use std::str::FromStr;
use tcio::bytes::{ByteStr, Bytes};

use crate::headers::matches;

/// HTTP Header Value.
#[derive(Clone)]
pub struct HeaderValue {
    bytes: Bytes,
    is_utf8: bool,
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
            Ok(is_ascii) => Self {
                bytes: Bytes::from_static(bytes),
                is_utf8: is_ascii,
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
    pub fn from_bytes<B: Into<Bytes>>(name: B) -> Result<Self, HeaderValueError> {
        let value = name.into();
        match validate_header_value(value.as_slice()) {
            Ok(is_ascii) => Ok(Self {
                bytes: value,
                is_utf8: is_ascii,
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
    pub fn from_slice<A: AsRef<[u8]>>(name: A) -> Result<Self, HeaderValueError> {
        match validate_header_value(name.as_ref()) {
            Ok(is_ascii) => Ok(Self {
                bytes: Bytes::copy_from_slice(name.as_ref()),
                is_utf8: is_ascii,
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
                // validation only detect for valid ASCII not UTF-8,
                // but the input `str` is valid UTF-8 and is a valid ASCII,
                // so it is required to set the flag here
                ok.is_utf8 = true;
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
        match self.is_utf8 {
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
        if !self.is_utf8 {
            if let Err(err) = str::from_utf8(self.bytes.as_slice()) {
                return Err(err);
            };
            self.is_utf8 = true;
        }
        unsafe { Ok(str::from_utf8_unchecked(self.bytes.as_slice())) }
    }
}

impl FromStr for HeaderValue {
    type Err = HeaderValueError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ok = Self::from_slice(s)?;
        // validation only detect for valid ASCII not UTF-8,
        // but the input `str` is valid UTF-8 and is a valid ASCII,
        // so it is required to set the flag here
        ok.is_utf8 = true;
        Ok(ok)
    }
}

// ===== Parsing =====

const MAX_HEADER_VALUE_LEN: usize = 1 << 13;  // 8KB

const fn validate_header_value(mut bytes: &[u8]) -> Result<bool, HeaderValueError> {
    use HeaderValueError as Error;

    match bytes {
        // no leading SP / HTAB
        | [b' ' | b'\t', ..]
        // no trailing SP / HTAB
        | [.., b' ' | b'\t'] => {
            return Err(Error::Invalid);
        },
        _ => {}
    }
    // too long
    if bytes.len() > MAX_HEADER_VALUE_LEN {
        return Err(HeaderValueError::TooLong);
    }
    let mut is_ascii = true;
    while let [byte, rest @ ..] = bytes {
        let (ok, ascii) = matches::is_header_value(*byte);
        if !ok {
            return Err(Error::Invalid)
        }
        is_ascii &= ascii;
        bytes = rest;
    }
    Ok(is_ascii)
}

// ===== Traits =====

impl std::fmt::Debug for HeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"",tcio::fmt::lossy(&self.as_bytes()))
    }
}

impl PartialEq for HeaderValue {
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

// ===== Error =====

/// An error that can occur when parsing [`HeaderValue`].
#[derive(Debug)]
pub enum HeaderValueError {
    /// Header value too long.
    TooLong,
    /// Header value contains invalid character.
    Invalid,
}

impl HeaderValueError {
    pub(crate) const fn message(&self) -> &'static str {
        match self {
            Self::TooLong => "too long",
            Self::Invalid => "invalid value",
        }
    }

    const fn panic_const(self) -> ! {
        panic!("{}",self.message())
    }
}

impl std::error::Error for HeaderValueError { }

impl fmt::Display for HeaderValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}
