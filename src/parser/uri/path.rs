use tcio::bytes::{ByteStr, Bytes};

use super::{error::UriError, simd};

#[derive(Debug, Clone)]
pub struct Path {
    value: ByteStr,
    query: u16,
}

impl Path {
    /// `/`
    pub(crate) const fn slash() -> Path {
        Self {
            value: ByteStr::from_static("/"),
            query: 1,
        }
    }

    /// `*`
    pub(crate) const fn asterisk() -> Path {
        Self {
            value: ByteStr::from_static("*"),
            query: 1,
        }
    }

    /// Construct a [`Path`] from [`Bytes`].
    ///
    /// Input is validated for valid path characters.
    ///
    /// Fragment is trimmed.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `value` contains invalid character.
    #[inline]
    pub fn try_from_bytes(value: Bytes) -> Result<Self, UriError> {
        match value.as_slice() {
            [] => Err(UriError::Incomplete),
            [b'*'] => Ok(Self::asterisk()),
            [b'/'] => Ok(Path::slash()),
            _ => parse(value)
        }
    }

    #[inline]
    pub const fn path(&self) -> &str {
        match self.query {
            0 => "/",
            q => self.value.as_str().split_at(q as usize).0,
        }
    }

    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.value.as_str()
    }

    #[inline]
    pub const fn query(&self) -> Option<&str> {
        match self
            .value
            .as_str()
            .split_at_checked((self.query + 1) as usize)
        {
            Some((_, q)) => {
                if q.is_empty() {
                    None
                } else {
                    Some(q)
                }
            }
            None => None,
        }
    }
}

/// Does not check for common cases or empty string.
pub(crate) fn parse(mut bytes: Bytes) -> Result<Path, UriError> {
    let mut cursor = bytes.cursor_mut();

    simd::match_path!(cursor);

    let (query, path) = match cursor.peek() {
        None => (bytes.len(), bytes),
        Some(b'?') => {
            let query = cursor.steps();

            simd::match_fragment!(cursor);

            if !matches!(cursor.peek(), Some(b'#') | None) {
                return Err(UriError::Char);
            }
            cursor.truncate_buf();

            (query, bytes)
        }
        Some(b'#') => {
            cursor.truncate_buf();
            (bytes.len(), bytes)
        }
        Some(_) => return Err(UriError::Char),
    };

    let Ok(query) = query.try_into() else {
        return Err(UriError::TooLong);
    };

    Ok(Path {
        // SAFETY: `simd::match_*!` check for valid ASCII
        value: unsafe { ByteStr::from_utf8_unchecked(path) },
        query,
    })
}

// ===== std Traits =====

impl PartialEq for Path {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
