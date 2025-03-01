use std::{ops::Deref, str::Utf8Error};

use bytes::Bytes;

/// str based on [`Bytes`]
#[derive(Clone, Default)]
pub struct ByteStr(Bytes);

impl ByteStr {
    pub const fn new() -> Self {
        Self(Bytes::new())
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
        f.write_str(&*self)
    }
}

impl std::fmt::Debug for ByteStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ByteString").field(&*self).finish()
    }
}

