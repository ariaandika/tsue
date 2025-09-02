use tcio::{ByteStr, bytes::Bytes};

use super::{Uri, error::UriError, simd, uri};

macro_rules! u16 {
    ($val:expr, $max:expr) => {
        match $val {
            val if val > $max => return Err(UriError::TooLong),
            val => val as u16,
        }
    };
    ($val:expr) => {{
        let val = $val;
        if val > u16::MAX as usize {
            return Err(UriError::TooLong)
        } else {
            val as u16
        }
    }};
}

impl Uri {
    /// `*`.
    #[inline]
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
    #[inline]
    pub const fn http_root() -> Self {
        Self {
            value: ByteStr::from_static("/"),
            scheme: uri::SCHEME_HTTP,
            authority: 0,
            path: 1,
            query: 1,
        }
    }

    #[inline]
    pub fn try_from_shared(value: Bytes) -> Result<Self, UriError> {
        parse(value)
    }

    /// Copy and try parse uri from `str`.
    #[inline]
    pub fn try_copy_from(value: &[u8]) -> Result<Self, UriError> {
        parse(Bytes::copy_from_slice(value))
    }
}

/// Parse full uri.
///
/// - cannot be empty
/// - scheme is required
/// - fragment will be trimmed
fn parse(mut bytes: Bytes) -> Result<Uri, UriError> {
    let len = u16!(bytes.len());

    let mut cursor = bytes.cursor_mut();

    simd::match_scheme!(cursor else {
        return Err(UriError::Incomplete)
    });

    let scheme = u16!(cursor.steps(), uri::MAX_SCHEME as usize);

    match cursor.next() {
        Some(b':') => {},
        Some(_) => return Err(UriError::Char),
        None => return Err(UriError::Incomplete),
    }

    let authority;

    match cursor.peek_chunk() {
        Some(b"//") => {
            // authority
            cursor.advance(2);

            if cursor.peek() == Some(b'/') {
                authority = uri::AUTH_NONE

            } else {
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
    }

    let path = u16!(cursor.steps());

    simd::match_path!(cursor);

    let query = match cursor.peek() {
        Some(b'?') => {
            let query = u16!(cursor.steps());
            cursor.advance(1);

            simd::match_query!(cursor);

            match cursor.peek() {
                Some(b'#') => cursor.truncate_buf(),
                Some(_) => return Err(UriError::Char),
                None => {},
            }

            query
        },
        Some(b'#') => {
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

// ===== Static Parsing =====

impl Uri {
    /// Construct [`Uri`] from static string.
    ///
    /// # Panics
    ///
    /// Panics if [`Uri::try_from_shared`] returns [`Err`].
    #[inline]
    pub const fn from_static(mut value: &'static str) -> Self {
        match parse_const(value.as_bytes()) {
            Ok(idx) => {
                let UriIndex { fragment, scheme, authority, path, query } = idx;
                if fragment != MAX_FRAG {
                    value = value.split_at(fragment as usize).0;
                }
                Self { value: ByteStr::from_static(value), scheme, authority, path, query  }
            },
            Err(err) => err.panic_const(),
        }
    }
}

#[derive(Debug)]
pub struct UriIndex {
    fragment: u16,
    scheme: u16,
    authority: u16,
    path: u16,
    query: u16,
}

const MAX_FRAG: u16 = u16::MAX;

pub const fn parse_const(bytes: &[u8]) -> Result<UriIndex, UriError> {
    use self::UriIndex as Uri;

    let len = u16!(bytes.len());

    let mut cursor = tcio::bytes::Cursor::new(bytes);
    let mut fragment = MAX_FRAG;

    simd::match_scheme!(cursor else {
        return Err(UriError::Incomplete)
    });

    let scheme = u16!(cursor.steps(), uri::MAX_SCHEME as usize);

    match cursor.next() {
        Some(b':') => {},
        Some(_) => return Err(UriError::Char),
        None => return Err(UriError::Incomplete),
    }

    let authority;

    match cursor.peek_chunk() {
        Some(b"//") => {
            // authority
            cursor.advance(2);

            if let Some(b'/') = cursor.peek() {
                authority = uri::AUTH_NONE

            } else {
                simd::match_authority!(cursor);

                authority = u16!(cursor.steps(), uri::MAX_AUTH as usize);

                match cursor.peek() {
                    Some(b'/' | b'?' | b'#') => {},
                    Some(_) => return Err(UriError::Char),
                    None => return Ok(Uri {
                        fragment,
                        scheme,
                        authority,
                        path: len,
                        query: len,
                    })
                }
            }
        },
        Some(_) => authority = uri::AUTH_NONE,
        None => return Ok(Uri {
            fragment,
            scheme,
            authority: len,
            path: len,
            query: len,
        })
    };

    let path = u16!(cursor.steps());

    simd::match_path!(cursor);

    let query = match cursor.next() {
        Some(b'?') => {
            let query = u16!(cursor.steps());

            simd::match_query!(cursor);

            match cursor.peek() {
                Some(b'#') => fragment = u16!(cursor.steps(), MAX_FRAG as usize),
                Some(_) => return Err(UriError::Char),
                None => {},
            }

            query
        },
        Some(b'#') => {
            fragment = u16!(cursor.steps(), MAX_FRAG as usize);
            len
        },
        Some(_) => return Err(UriError::Char),
        None => len,
    };

    Ok(Uri {
        fragment,
        scheme,
        authority,
        path,
        query,
    })
}
