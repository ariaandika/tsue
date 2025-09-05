use tcio::bytes::Bytes;

use super::{simd, UriError};

#[derive(Clone)]
pub struct Path {
    /// is valid ASCII
    value: Bytes,
    query: u16,
}

impl Path {
    pub fn from_shared(mut value: Bytes) -> Self {
        let query = simd::match_query! {
            value;
            |val, cursor| match val {
                b'?' => {
                    let query = cursor.steps();
                    simd::match_fragment! {
                        cursor;
                        |val| match val {
                            b'#' => cursor.truncate_buf(),
                            _ => UriError::Char.panic_const()
                        };
                    }
                    query
                },
                b'#' => {
                    cursor.truncate_buf();
                    value.len()
                }
                _ => UriError::Char.panic_const()
            };
            else {
                value.len()
            }
        };
        Self {
            value,
            query: query as u16,
        }
    }

    /// Returns the path as `str`, e.g: `/over/there`.
    #[inline]
    pub fn path(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(&self.value[..self.query as usize]) }
    }

    /// Returns the query as `str`, e.g: `name=joe&query=4`.
    #[inline]
    pub fn query(&self) -> Option<&str> {
        if self.query as usize == self.value.len() {
            None
        } else {
            // SAFETY: precondition `value` is valid ASCII
            unsafe { Some(str::from_utf8_unchecked(&self.value[self.query as usize + 1..])) }
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
