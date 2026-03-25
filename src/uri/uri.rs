use tcio::bytes::{Buf, Bytes};

use crate::uri::scheme::{self, Scheme};
use crate::uri::{Authority, Path, UriError, matches};

/// URI Generic Syntax.
///
/// This API follows the [RFC3986].
///
/// [RFC3986]: <https://www.rfc-editor.org/rfc/rfc3986.html>
///
/// # Example
///
/// The following are two example URIs and their component parts:
///
/// ```not_rust
///   foo://example.com:8042/over/there?name=ferret
///   \_/   \______________/\_________/ \_________/
///    |           |            |            |
/// scheme     authority       path        query
///    |   _____________________|__
///   / \ /                        \
///   urn:example:animal:ferret:nose
/// ```
///
/// ```
/// use tsue::uri::Uri;
///
/// let uri = Uri::from_bytes("foo://example.com:8042/over/there?name=ferret").unwrap();
/// assert_eq!(uri.scheme(), "foo");
/// assert_eq!(uri.authority(), Some("example.com:8042"));
/// assert_eq!(uri.path(), "/over/there");
/// assert_eq!(uri.query(), Some("name=ferret"));
///
/// let urn = Uri::from_bytes("urn:example:animal:ferret:nose").unwrap();
/// assert_eq!(urn.scheme(), "urn");
/// assert_eq!(urn.authority(), None);
/// assert_eq!(urn.path(), "example:animal:ferret:nose");
/// ```
#[derive(Debug, Clone)]
pub struct Uri {
    scheme: Scheme,
    authority: Option<Authority>,
    path: Path,
}

impl Uri {
    /// Creates [`Uri`] from [`Scheme`], optionally [`Authority`], and [`Path`].
    #[inline]
    pub const fn from_parts(scheme: Scheme, authority: Option<Authority>, path: Path) -> Self {
        // TODO: check that path should be `path-abempty` ?
        // > [RFC3986#section-3] When authority is present, the path must either be empty or begin
        // > with a slash ("/") character

        Self {
            scheme,
            authority,
            path,
        }
    }

    /// Parse URI from [`Bytes`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use tsue::uri::Uri;
    /// let http = Uri::from_bytes("http://example.com/users/all").unwrap();
    /// assert_eq!(http.host(), Some("example.com"));
    /// assert_eq!(http.path(), "/users/all");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid URI.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        parse_uri(bytes.into())
    }

    /// Parse URI by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid URI.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        parse_uri(Bytes::copy_from_slice(bytes.as_ref()))
    }
}

impl Uri {
    /// Returns the scheme component.
    ///
    /// ```not_rust
    ///   foo://example.com:8042/over/there?name=ferret
    ///   \_/
    ///    |
    /// scheme
    ///    |
    ///   / \
    ///   urn:example:animal:ferret:nose
    /// ```
    #[inline]
    pub const fn scheme(&self) -> &str {
        self.scheme.as_str()
    }

    /// Returns the scheme component as [`Scheme`].
    #[inline]
    pub const fn as_scheme(&self) -> &Scheme {
        &self.scheme
    }

    /// Returns the authority component if exists.
    ///
    /// If returned [`Some`], the string will not be empty.
    ///
    /// ```not_rust
    /// foo://example.com:8042/over/there?name=ferret
    ///       \______________/
    ///              |
    ///          authority
    /// ```
    #[inline]
    pub const fn authority(&self) -> Option<&str> {
        match &self.authority {
            Some(auth) => Some(auth.as_str()),
            None => None,
        }
    }

    /// Returns the authority component as [`Authority`] if exists.
    #[inline]
    pub const fn as_authority(&self) -> Option<&Authority> {
        self.authority.as_ref()
    }

    /// Returns the host component if exists.
    ///
    /// ```not_rust
    /// foo://example.com:8042/over/there?name=ferret
    ///       \______________/
    ///              |
    ///            host
    /// ```
    #[inline]
    pub const fn host(&self) -> Option<&str> {
        match &self.authority {
            Some(auth) => Some(auth.host()),
            None => None,
        }
    }

    /// Returns the hostname component if exists.
    ///
    /// ```not_rust
    /// foo://example.com:8042/over/there?name=ferret
    ///       \_________/
    ///            |
    ///        hostname
    /// ```
    #[inline]
    pub const fn hostname(&self) -> Option<&str> {
        match &self.authority {
            Some(auth) => Some(auth.hostname()),
            None => None,
        }
    }

    /// Returns the userinfo component if exists.
    ///
    /// ```not_rust
    /// foo://user:pass@example.com:8042/over/there?name=ferret
    ///       \_______/
    ///           |
    ///       userinfo
    /// ```
    #[inline]
    pub const fn userinfo(&self) -> Option<&str> {
        match &self.authority {
            Some(auth) => auth.userinfo(),
            None => None,
        }
    }

    /// Returns the path component.
    ///
    /// ```not_rust
    /// foo://example.com:8042/over/there?name=ferret
    ///                       \_________/
    ///                           |
    ///                          path
    ///      _____________________|__
    ///     /                        \
    /// urn:example:animal:ferret:nose
    /// ```
    #[inline]
    pub const fn path(&self) -> &str {
        self.path.path()
    }

    /// Returns the query component if exists.
    ///
    /// ```not_rust
    /// foo://example.com:8042/over/there?name=ferret
    ///                                   \_________/
    ///                                        |
    ///                                      query
    /// ```
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        self.path.query()
    }

    /// Returns the path and query component.
    ///
    /// ```not_rust
    /// foo://example.com:8042/over/there?name=ferret
    ///                       \_____________________/
    ///                                  |
    ///                            path and query
    /// ```
    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.path.as_str()
    }
}

fn parse_uri(mut bytes: Bytes) -> Result<Uri, UriError> {
    let mut state = bytes.as_slice();

    loop {
        let [prefix, rest @ ..] = state else {
            return Err(UriError::InvalidScheme);
        };
        if !scheme::is_scheme(*prefix) {
            if *prefix != b':' {
                return Err(UriError::InvalidScheme);
            }
            break;
        }
        state = rest;
    }
    let scheme = unsafe {
        let len = state.as_ptr().offset_from_unsigned(bytes.as_ptr());
        Scheme::new_unchecked(bytes.split_to_unchecked(len))
    };
    unsafe { bytes.advance_unchecked(1) };

    let authority = if bytes.starts_with(b"//") {
        bytes.advance(2);

        let authority = match matches::find_path_delim(bytes.as_slice()) {
            Some(at) => unsafe { bytes.split_to_unchecked(at) },
            None => std::mem::take(&mut bytes),
        };

        if authority.is_empty() {
            None
        } else {
            Some(Authority::from_bytes(authority)?)
        }
    } else {
        None
    };

    let path = Path::from_bytes(bytes)?;

    Ok(Uri {
        scheme,
        authority,
        path,
    })
}

/*

/// URI         = scheme ":" hier-part [ "?" query ] [ "#" fragment ]
/// hier-part   = "//" authority path-abempty
///             / path-absolute
///             / path-rootless
///             / path-empty
const fn parse_uri2(bytes: &[u8]) -> Result<UriParts, UriError> {
    let mut state = bytes;

    let col = loop {
        let [scheme, rest @ ..] = state else {
            return Err(UriError::InvalidScheme);
        };
        state = rest;
        if !is_scheme(*scheme) {
            if *scheme != b':' {
                return Err(UriError::InvalidScheme);
            }
            break std::ptr::NonNull::from_ref(scheme);
        }
    };

    let auth = if let Some((b"//", rest)) = state.split_first_chunk() {
        let Some(rest) = validate_authority(rest) else {
            return Err(UriError::InvalidAuthority)
        };
        if let Some(delim) = rest.first() && !is_path_delim(*delim) {
            return Err(UriError::InvalidAuthority);
        }
        state = rest;
        state.as_ptr()
    } else {
        std::ptr::null()
    };

    let (query, path) = match validate_path(state) {
        Ok(ok) => ok,
        Err(err) => return Err(err),
    };

    Ok(UriParts {
        col,
        auth,
        path: path.as_ptr(),
        query,
    })
}

struct UriParts {
    col: std::ptr::NonNull<u8>,
    auth: *const u8,
    path: *const u8,
    query: u16,
}

*/

