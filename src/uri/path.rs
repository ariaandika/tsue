use tcio::bytes::Bytes;

use crate::uri::{UriError, matches};

/// URI Path.
///
/// The [path] component of a URI.
///
/// [path]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.3>
///
/// ```not_rust
///   foo://example.com:8042/over/there?name=ferret
///                         \_________/
///                             |
///                            path
///        _____________________|__
///       /                        \
///   urn:example:animal:ferret:nose
/// ```
///
/// # Example
///
/// To create `Path` use one of the `Path::from_*` method:
///
/// ```
/// use tsue::uri::Path;
/// let path = Path::from_bytes("/over/there").unwrap();
/// assert_eq!(path.as_str(), "/over/there");
/// ```
#[derive(Clone)]
pub struct Path {
    /// is valid ASCII
    value: Bytes,
    query: u16,
}

impl Path {
    // pub(crate) const MAX_LEN: u16 = 8 * 1024;

    /// Validate path from static bytes.
    ///
    /// Path fragment is trimmed.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid path.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_path(bytes) {
            Ok((query, slice)) => Self {
                value: Bytes::from_static(slice),
                query,
            },
            Err(err) => err.panic_const(),
        }
    }

    /// Validate path from [`Bytes`].
    ///
    /// Path fragment is trimmed.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid path.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let mut bytes = bytes.into();
        let (query, slice) = validate_path(bytes.as_slice())?;
        bytes.truncate(slice.len());
        Ok(Self {
            value: bytes,
            query,
        })
    }

    /// Validate path by copying from slice of bytes.
    ///
    /// Path fragment is trimmed.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid path.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        let (query, slice) = validate_path(bytes.as_ref())?;
        let value = Bytes::copy_from_slice(slice);
        Ok(Self { value, query })
    }
}

impl Path {
    /// Returns the path component.
    ///
    /// ```not_rust
    /// /over/there?name=ferret
    /// \_________/
    ///      |
    ///    path
    /// ```
    #[inline]
    pub const fn path(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII, `query` is less than or equal to `value`
        // length
        unsafe { str_from_parts!(self.value.as_ptr(), self.query as usize) }
    }

    /// Returns the query component.
    ///
    /// ```not_rust
    /// /over/there?name=ferret
    ///             \_________/
    ///                  |
    ///                query
    /// ```
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        if self.query as usize == self.value.len() {
            None
        } else {
            // SAFETY: precondition `value` is valid ASCII
            unsafe {
                let query = self.query as usize;
                Some(str_from_parts!(
                    self.value.as_ptr().add(query + 1),
                    self.value.len() - query - 1
                ))
            }
        }
    }

    /// Returns the entire path and query.
    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.as_str()
    }

    /// Extracts a string slice containing the path and query.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

// ===== std validation =====

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

// ===== Validation =====

macro_rules! str_from_parts {
    ($d:expr, $l:expr) => {
        str::from_utf8_unchecked(std::slice::from_raw_parts($d, $l))
    };
}

use str_from_parts;

pub const fn is_path_delim(byte: u8) -> bool {
    matches!(byte, b'/' | b'?' | b'#')
}

matches::ascii_lookup_table! {
    /// `pchar = unreserved / pct-encoded / sub-delims / ":" / "@"`
    const fn is_pchar(byte: u8) -> bool {
        matches::unreserved(byte)
        || matches::pct_encoded(byte)
        || matches::sub_delims(byte)
        || matches!(byte, b':' | b'@')
    }
}

matches::ascii_lookup_table! {
    /// `query = *( pchar / "/" / "?" )`
    const fn is_query(byte: u8) -> bool {
        is_pchar(byte)
        || matches!(byte, b'/' | b'?')
    }
}

/// This allows for query component.
///
/// ```not_rust
/// path          = path-abempty    ; begins with "/" or is empty
///               / path-absolute   ; begins with "/" but not "//"
///               / path-noscheme   ; begins with a non-colon segment
///               / path-rootless   ; begins with a segment
///               / path-empty      ; zero characters
///
/// path-abempty  = *( "/" segment )
/// path-absolute = "/" [ segment-nz *( "/" segment ) ]
/// path-noscheme = segment-nz-nc *( "/" segment )
/// path-rootless = segment-nz *( "/" segment )
/// path-empty    = 0<pchar>
///
/// segment       = *pchar
/// segment-nz    = 1*pchar
/// segment-nz-nc = 1*( unreserved / pct-encoded / sub-delims / "@" )
///               ; non-zero-length segment without any colon ":"
///
/// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
/// ```
const fn validate_path(mut bytes: &[u8]) -> Result<(u16, &[u8]), UriError> {
    if bytes.is_empty() {
        return Ok((0, &[]));
    }

    let mut query = bytes.len() as u16;
    let mut frag = bytes.len();

    while let [byte, rest @ ..] = bytes {
        if !is_pchar(*byte) {
            if *byte == b'?' {
                bytes = rest;
                query = query - rest.len() as u16 - 1;
                break;
            } else if *byte == b'#' {
                frag = frag - rest.len() - 1;
                query = frag as u16;
                bytes = &[];
                break;
            } else {
                return Err(UriError::InvalidPath);
            }
        }
        bytes = rest;
    }

    while let [byte, rest @ ..] = bytes {
        if !is_query(*byte) {
            if *byte != b'#' {
                return Err(UriError::InvalidPath);
            }
            frag = frag - rest.len() - 1;
            break;
        }
        bytes = rest;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes.as_ptr(), frag) };

    Ok((query, slice))
}
