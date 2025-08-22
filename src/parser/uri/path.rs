use tcio::bytes::ByteStr;

use super::{error::InvalidUri, simd};

#[derive(Debug)]
pub struct Path {
    bytes: ByteStr,
    query: u16,
}

impl Path {
    /// `/`
    pub(crate) const fn slash() -> Path {
        Self {
            bytes: ByteStr::from_static("/"),
            query: 1,
        }
    }

    #[inline]
    pub fn parse(string: ByteStr) -> Result<Self, InvalidUri> {
        parse(string)
    }

    #[inline]
    pub const fn path(&self) -> &str {
        match self.query {
            0 => "/",
            q => self.bytes.as_str().split_at(q as usize).0,
        }
    }

    #[inline]
    pub const fn query(&self) -> Option<&str> {
        match self
            .bytes
            .as_str()
            .split_at_checked((self.query + 1) as usize)
        {
            Some((_, q)) if q.is_empty() => None,
            Some((_, query)) => Some(query),
            None => None,
        }
    }
}

/// Does not check for common cases or empty string.
pub(crate) fn parse(string: ByteStr) -> Result<Path, InvalidUri> {
    let mut bytes = string.into_bytes();
    let mut cursor = bytes.cursor_mut();

    simd::match_path(&mut cursor);

    let (query, path) = match cursor.peek() {
        None => (bytes.len(), bytes),
        Some(b'?') => {
            let steps = cursor.steps();

            simd::match_fragment(&mut cursor);

            if !matches!(cursor.peek(), Some(b'#') | None) {
                return Err(InvalidUri::Char);
            }
            cursor.truncate_buf();

            (steps, bytes)
        }
        Some(b'#') => {
            cursor.truncate_buf();
            (bytes.len(), bytes)
        }
        Some(_) => return Err(InvalidUri::Char),
    };

    let Ok(query) = query.try_into() else {
        return Err(InvalidUri::TooLong);
    };

    Ok(Path {
        // SAFETY: input is valid ASCII
        bytes: unsafe { ByteStr::from_utf8_unchecked(path) },
        query,
    })
}
