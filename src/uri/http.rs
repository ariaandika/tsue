use std::slice::from_raw_parts;
use tcio::bytes::Bytes;

use crate::uri::{Host, Path};

/// HTTP/HTTPS Scheme.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct HttpScheme(bool);

impl HttpScheme {
    /// "http" scheme.
    pub const HTTP: Self = Self(false);
    /// "https" scheme.
    pub const HTTPS: Self = Self(true);

    /// Returns `true` if this is an HTTP scheme.
    #[inline]
    pub const fn is_http(&self) -> bool {
        !self.0
    }

    /// Returns `true` if this is an HTTPS scheme.
    #[inline]
    pub const fn is_https(&self) -> bool {
        self.0
    }

    /// Extracts a string slice containing http scheme.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        const HTTPS: &str = "https";
        unsafe { str::from_utf8_unchecked(from_raw_parts(HTTPS.as_ptr(), 4 + self.0 as usize)) }
    }
}

/// HTTP URI.
///
/// # Example
///
/// The following is an example HTTP URI and their component parts:
///
/// ```not_rust
///  https://example.com:80/over/there?name=ferret
///  \___/   \____________/\_________/ \_________/
///    |           |          |            |
///  scheme    authority     path        query
/// ```
///
/// [`HttpUri`] used to represent HTTP scheme URI.
///
/// ```rust
/// use tsue::uri::HttpUri;
///
/// let uri = HttpUri::from_bytes("https://example.com:80/over/there?name=ferret").unwrap();
/// assert!(uri.is_https());
/// assert_eq!(uri.host(), "example.com:80");
/// assert_eq!(uri.path(), "/over/there");
/// assert_eq!(uri.query(), Some("name=ferret"));
/// ```
#[derive(Debug, Clone)]
pub struct HttpUri {
    // scheme: HttpScheme,
    host: Host,
    /// is valid ASCII
    path: Bytes,
    /// `[ scheme @ bool | query @ .. ]`
    /// `query <= path.len()`
    query: u16,
}

const QUERY_MASK: u16 = u16::MAX >> 1;
const SCHEME_MASK: u16 = !(u16::MAX >> 1);

impl HttpUri {
    /// Creates [`HttpUri`] from [`HttpScheme`], [`Host`], and [`Path`].
    #[inline]
    pub fn from_parts(scheme: HttpScheme, host: Host, path: Path) -> Self {
        let Path { value: path, query } = path;
        const { assert!(Path::MAX_LEN < QUERY_MASK) };
        let query = ((scheme.0 as u16) << 15) | (query & QUERY_MASK);
        Self { host, path, query }
    }

    /// Returns `true` if the scheme is HTTP.
    #[inline]
    pub const fn is_http(&self) -> bool {
        self.query & SCHEME_MASK == 0
    }

    /// Returns `true` if the scheme is HTTPS.
    #[inline]
    pub const fn is_https(&self) -> bool {
        self.query & SCHEME_MASK == SCHEME_MASK
    }

    /// Returns the host component.
    #[inline]
    pub const fn host(&self) -> &str {
        self.host.as_str()
    }

    /// Returns the host component as [`Host`].
    #[inline]
    pub const fn as_host(&self) -> &Host {
        &self.host
    }

    /// Returns the path component.
    #[inline]
    pub const fn path(&self) -> &str {
        // SAFETY: precondition
        // - `path` is valid ASCII,
        // - `query` is less than or equal to `value` length
        unsafe { str_from!(self.path.as_ptr(), (self.query & QUERY_MASK) as usize) }
    }

    /// Returns the query component if exists.
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        let query = self.query & QUERY_MASK;
        if query == QUERY_MASK {
            None
        } else {
            // SAFETY: precondition
            // - `path` is valid ASCII,
            // - `query <= path.len()`
            unsafe {
                Some(str_from!(
                    self.path.as_ptr().add((query + 1) as usize),
                    self.path.len() - query as usize - 1,
                ))
            }
        }
    }

    /// Returns the path and query component.
    #[inline]
    pub const fn path_and_query(&self) -> &str {
        // SAFETY: precondition `path` is valid ASCII,
        unsafe { str_from!(self.path.as_ptr(), self.path.len()) }
    }

    /// Consume `HttpUri` into each components.
    #[inline]
    pub fn into_parts(self) -> (HttpScheme, Host, Path) {
        (
            HttpScheme(self.is_https()),
            self.host,
            Path {
                value: self.path,
                query: self.query & QUERY_MASK,
            },
        )
    }

    /// Returns the HTTP port or its default value if it does not exists.
    #[inline]
    pub const fn port(&self) -> u16 {
        match self.host.port() {
            Some(port) => port,
            None => {
                if self.is_http() {
                    80
                } else {
                    443
                }
            }
        }
    }
}

// ===== std traits =====

impl From<HttpUri> for HttpScheme {
    #[inline]
    fn from(value: HttpUri) -> Self {
        HttpScheme(value.is_https())
    }
}

impl From<HttpUri> for Host {
    #[inline]
    fn from(value: HttpUri) -> Self {
        value.host
    }
}

impl From<HttpUri> for Path {
    #[inline]
    fn from(value: HttpUri) -> Self {
        Self {
            value: value.path,
            query: value.query & QUERY_MASK,
        }
    }
}

impl std::fmt::Debug for HttpScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

macro_rules! str_from {
    ($data:expr, $len:expr $(,)?) => {
        str::from_utf8_unchecked(from_raw_parts($data, $len))
    };
}

use {str_from};
