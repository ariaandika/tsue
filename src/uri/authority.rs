use tcio::bytes::Bytes;

use super::{simd, UriError};

#[derive(Clone)]
pub struct Authority {
    /// is valid ASCII
    value: Bytes,
}

impl Authority {
    #[inline]
    pub const fn from_static(string: &'static str) -> Self {
        Self::from_shared(Bytes::from_static(string.as_bytes()))
    }

    pub const fn from_shared(value: Bytes) -> Self {
        simd::validate_authority! {
            value;
            else {
                UriError::Char.panic_const()
            }
        }
        Self { value }
    }

    #[inline]
    pub fn try_from(value: impl Into<Bytes>) -> Result<Self, UriError> {
        Self::try_from_shared(value.into())
    }

    fn try_from_shared(value: Bytes) -> Result<Self, UriError> {
        simd::validate_authority! {
            value;
            else {
                return Err(UriError::Char)
            }
        };
        Ok(Self { value })
    }

    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

// ===== Formatting =====

impl std::fmt::Debug for Authority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Authority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

