use tcio::{ByteStr, bytes::Bytes};

use super::{Uri, error::UriError, simd, uri};

/// Request Target.
#[derive(Debug, PartialEq)]
pub enum Target {
    // /// `/users/all?page=4&filter=available`
    // Origin(Path),
    // /// `http://example.com/users/all?page=4&filter=available`
    // Absolute {
    //     scheme: Scheme,
    //     authority: Authority,
    //     path: Path,
    // },
    // /// `example.com:443`
    // Authority(Authority),
    /// `*`
    Asterisk,
}

macro_rules! u16 {
    ($val:expr, $max:expr) => {
        match $val {
            val if val > $max => return Err(UriError::TooLong),
            val => val as u16,
        }
    };
    ($val:expr) => {
        match u16::try_from($val) {
            Ok(ok) => ok,
            Err(_) => return Err(UriError::TooLong),
        }
    };
}

impl Uri {
    /// `*`.
    pub const fn asterisk() -> Self {
        Self {
            value: ByteStr::from_static("*"),
            scheme: 0,
            authority: 0,
            path: 1,
            query: 1,
        }
    }

    /// `/`.
    pub const fn root() -> Self {
        Self {
            value: ByteStr::from_static("/"),
            scheme: 0,
            authority: 0,
            path: 1,
            query: 1,
        }
    }

    /// Construct an empty [`Uri`].
    pub const fn empty() -> Self {
        Self {
            value: ByteStr::new(),
            scheme: 0,
            authority: 0,
            path: 0,
            query: 0,
        }
    }

    /// Copy and try parse uri from `str`.
    #[inline]
    pub fn try_copy_from(value: &[u8]) -> Result<Self, UriError> {
        parse(Bytes::copy_from_slice(value))
    }
}

/// Parse full uri.
///
/// - uri cannot empty
/// - scheme can be empty, using the '/' prefix, resulting in path only uri
/// - cannot starts with authority, so `example.com:80/users/all` will treat `example.com` as
///   scheme
/// - fragment will be trimmed
pub fn parse(mut bytes: Bytes) -> Result<Uri, UriError> {
    let len = u16!(bytes.len());
    let Some(&prefix) = bytes.first() else {
        return Err(UriError::Incomplete)
    };

    if len == 1 {
        return match prefix {
            b'*' => Ok(Uri::asterisk()),
            b'/' => Ok(Uri::root()),
            _ => Err(UriError::Char)
        }
    }

    let mut cursor = bytes.cursor_mut();
    let mut scheme = uri::SCHEME_NONE;
    let mut authority = uri::AUTH_NONE;

    if prefix != b'/' {
        simd::match_scheme!(cursor else {
            return Err(UriError::Incomplete)
        });

        scheme = u16!(cursor.steps(), uri::MAX_SCHEME as usize);

        match cursor.next() {
            Some(b':') => {},
            Some(_) => return Err(UriError::Char),
            None => return Err(UriError::Incomplete),
        }

        match cursor.peek_chunk() {
            Some(b"//") => {
                // authority
                cursor.advance(2);

                simd::match_authority!(cursor);

                authority = u16!(cursor.steps(), uri::MAX_AUTH as usize);

                match cursor.peek() {
                    Some(b'/' | b'?' | b'#') => {},
                    Some(_) => return Err(UriError::Char),
                    None => return Ok(Uri {
                        // SAFETY: `match_*` also guarantee valid ASCII
                        value: unsafe { ByteStr::from_utf8_unchecked(bytes) },
                        scheme,
                        authority,
                        path: len,
                        query: len,
                    })
                }
            },
            Some(_) => authority = uri::AUTH_NONE,
            None => return Ok(Uri {
                // SAFETY: `match_*` also guarantee valid ASCII
                value: unsafe { ByteStr::from_utf8_unchecked(bytes) },
                scheme,
                authority: len,
                path: len,
                query: len,
            })
        };
    }

    let path = u16!(cursor.steps());

    simd::match_path!(cursor);

    let query = match cursor.next() {
        Some(b'?') => {
            let query = u16!(cursor.steps());

            simd::match_query!(cursor);

            match cursor.peek() {
                Some(b'#') => cursor.truncate_buf(),
                Some(_) => return Err(UriError::Char),
                None => {},
            }

            query
        },
        Some(b'#') => {
            cursor.step_back(1);
            cursor.truncate_buf();
            len
        },
        Some(_) => return Err(UriError::Char),
        None => len,
    };

    Ok(Uri {
        // SAFETY: `match_*` postcondition guarantee valid ASCII
        value: unsafe { ByteStr::from_utf8_unchecked(bytes) },
        scheme,
        authority,
        path,
        query,
    })
}

