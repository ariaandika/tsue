//! cheaply cloneable and sliceable string
use bytes::Bytes;
use std::{ops::Deref, str::Utf8Error};

/// a cheaply cloneable and sliceable string
///
/// internally it uses [`Bytes`], so we get all the benefit of
/// `Bytes` while having utf8 checked
#[derive(Clone, Default)]
pub struct ByteStr(Bytes);

impl ByteStr {
    /// Creates a new empty [`ByteStr`]
    ///
    /// This will not allocate
    pub const fn new() -> Self {
        Self(Bytes::new())
    }

    /// Creates a new [`ByteStr`] from a static str
    ///
    /// The returned [`ByteStr`] will point directly to the static str.
    /// There is no allocating or copying
    pub const fn from_static(s: &'static str) -> ByteStr {
        Self(Bytes::from_static(s.as_bytes()))
    }

    /// Creates a new [`ByteStr`] from a [`Bytes`]
    ///
    /// Input is checked to ensure that the bytes are valid
    /// UTF-8
    pub fn from_bytes(bytes: Bytes) -> Result<ByteStr, Utf8Error> {
        std::str::from_utf8(bytes.as_ref())?;
        Ok(Self(bytes))
    }
}

impl Deref for ByteStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        // SAFETY: checked from the start and immutable
        unsafe { std::str::from_utf8_unchecked(self.0.as_ref()) }
    }
}

impl PartialEq<str> for ByteStr {
    fn eq(&self, other: &str) -> bool {
        self.0.as_ref() == other.as_bytes()
    }
}

impl PartialEq<&str> for ByteStr {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == other.as_bytes()
    }
}

impl std::fmt::Display for ByteStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        str::fmt(self, f)
    }
}

impl std::fmt::Debug for ByteStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        str::fmt(self, f)
    }
}

