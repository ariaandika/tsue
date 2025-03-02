use bytes::Bytes;
use std::{ops::Deref, str::Utf8Error};

/// str based on [`Bytes`]
#[derive(Clone, Default)]
pub struct ByteStr(Bytes);

impl ByteStr {
    pub const fn new() -> Self {
        Self(Bytes::new())
    }

    pub const fn from_static(s: &'static [u8]) -> ByteStr {
        Self(Bytes::from_static(s))
    }

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

impl PartialEq<&str> for ByteStr {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == other.as_bytes()
    }
}

impl std::fmt::Display for ByteStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        str::fmt(&*self, f)
    }
}

impl std::fmt::Debug for ByteStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        str::fmt(&*self, f)
    }
}

