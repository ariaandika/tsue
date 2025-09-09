use std::slice::from_raw_parts;
use tcio::bytes::Bytes;

use super::{UriError, simd};

#[derive(Clone)]
pub struct Path {
    /// is valid ASCII
    value: Bytes,
    query: u16,
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

    /// Returns the path as `str`, e.g: `/over/there`.
    #[inline]
    pub const fn path(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII, `query` is less than or equal to `value`
        // length
        unsafe {
            str::from_utf8_unchecked(from_raw_parts(self.value.as_ptr(), self.query as usize))
        }
    }

    /// Returns the query as `str`, e.g: `name=joe&query=4`.
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        if self.query as usize == self.value.len() {
            None
        } else {
            // SAFETY: precondition `value` is valid ASCII
            // unsafe { Some(str::from_utf8_unchecked(&self.value[self.query as usize + 1..])) }
            unsafe {
                let query = self.query as usize;
                Some(str::from_utf8_unchecked(from_raw_parts(
                    self.value.as_ptr().add(query.unchecked_add(1)),
                    self.value.len().unchecked_sub(query),
                )))
            }
        }
    }

    /// Returns the path and query as `str`, e.g: `/over/there?name=joe&query=4`.
    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.as_str()
    }

    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

// ===== Formatting =====

impl std::fmt::Debug for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
