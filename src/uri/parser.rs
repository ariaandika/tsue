use tcio::bytes::Bytes;

use super::{Authority, Path, Scheme, UriError, simd};

impl Scheme {
    #[inline]
    pub const fn from_static(string: &'static str) -> Self {
        Self::from_shared(Bytes::from_static(string.as_bytes()))
    }

    pub const fn from_shared(value: Bytes) -> Self {
        simd::validate_scheme! {
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
        simd::validate_scheme! {
            value;
            else {
                return Err(UriError::Char)
            }
        };
        Ok(Self { value })
    }
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
}

impl Path {
    #[inline]
    pub const fn asterisk() -> Self {
        Self {
            value: Bytes::from_static(b"*"),
            query: 1,
        }
    }

    #[inline]
    pub const fn empty() -> Self {
        Self {
            value: Bytes::new(),
            query: 0,
        }
    }

    #[inline]
    pub fn try_from(value: impl Into<Bytes>) -> Result<Self, UriError> {
        Self::try_from_shared(value.into())
    }

    fn try_from_shared(mut value: Bytes) -> Result<Self, UriError> {
        let query = simd::match_query! {
            value;
            |val, cursor| match val {
                b'?' => {
                    let query = cursor.steps();
                    simd::match_fragment! {
                        cursor;
                        |val| match val {
                            b'#' => cursor.truncate_buf(),
                            _ => return Err(UriError::Char)
                        };
                    }
                    query
                },
                b'#' => {
                    cursor.truncate_buf();
                    value.len()
                }
                _ => return Err(UriError::Char)
            };
            else {
                value.len()
            }
        };
        Ok(Self {
            value,
            query: query as u16,
        })
    }
}
