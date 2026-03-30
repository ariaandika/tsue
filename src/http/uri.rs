use std::slice::from_raw_parts;
use tcio::bytes::Bytes;

use crate::http::error::UriError;
use crate::http::{authority, target};

/// HTTP URI.
///
/// `HttpUri` represent [URI with `http` or `https` scheme][1].
///
/// Note that `HttpUri` does not allow [userinfo][2] or [fragment][3] component.
///
/// [1]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-identifiers-in-http>
/// [2]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-deprecation-of-userinfo-in->
/// [3]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-https-references-with-fragm>
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
/// ```rust
/// use tsue::uri::HttpUri;
///
/// let uri = HttpUri::from_bytes("https://example.com:80/over/there?name=ferret").unwrap();
/// assert!(uri.is_https());
/// assert_eq!(uri.authority(), "example.com:80");
/// assert_eq!(uri.path(), "/over/there");
/// assert_eq!(uri.query(), Some("name=ferret"));
/// ```
//
// ```not_rust
// http-URI = "http" "://" authority path-abempty [ "?" query ]
// https-URI = "https" "://" authority path-abempty [ "?" query ]
// ```
#[derive(Debug, Clone)]
pub struct HttpUri {
    value: Bytes,
    is_https: bool,
    host_len: u16,
    path: u16,
    path_len: u16,
}

impl HttpUri {
    /// Parse HTTP URI by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid HTTP URI.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match parse_http(bytes) {
            Ok((is_https, host_len, path, path_len)) => Self {
                value: Bytes::from_static(bytes),
                is_https,
                host_len,
                path,
                path_len,
            },
            Err(err) => err.panic_const(),
        }
    }

    /// Parse HTTP URI from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid HTTP URI.
    #[inline]
    pub fn from_bytes(bytes: impl Into<Bytes>) -> Result<Self, UriError> {
        let value = bytes.into();
        match parse_http(value.as_slice()) {
            Ok((is_https, host_len, path, path_len)) => Ok(Self {
                value,
                is_https,
                host_len,
                path,
                path_len,
            }),
            Err(err) => Err(err),
        }
    }

    /// Parse HTTP URI by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid HTTP URI.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match parse_http(bytes.as_ref()) {
            Ok((is_https, host_len, path, path_len)) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
                is_https,
                host_len,
                path,
                path_len,
            }),
            Err(err) => Err(err),
        }
    }
}

const SCHEME_OFF: u16 = b"http://".len() as u16;

impl HttpUri {
    /// Returns `true` if the scheme is `http`.
    #[inline]
    pub const fn is_http(&self) -> bool {
        !self.is_https
    }

    /// Returns `true` if the scheme is `https`.
    #[inline]
    pub const fn is_https(&self) -> bool {
        self.is_https
    }

    const fn host_start(&self) -> *const u8 {
        unsafe { self.value.as_ptr().add(SCHEME_OFF as usize + self.is_https as usize) }
    }

    /// Returns the authority component.
    #[inline]
    pub const fn authority(&self) -> &str {
        unsafe {
            let len = self.path - self.is_https as u16 - SCHEME_OFF;
            str_from!(self.host_start(), len as usize)
        }
    }

    /// Returns the host component.
    #[inline]
    pub const fn host(&self) -> &str {
        unsafe { str_from!(self.host_start(), self.host_len as usize) }
    }

    /// Returns the port component.
    #[inline]
    pub const fn port(&self) -> Option<&str> {
        let offset = SCHEME_OFF + self.is_https as u16 + self.host_len;
        if offset < self.path {
            unsafe {
                Some(str_from!(
                    self.value.as_ptr().add(offset as usize),
                    (self.path - offset) as usize
                ))
            }
        } else {
            None
        }
    }

    /// Returns the path component.
    #[inline]
    pub const fn path(&self) -> &str {
        unsafe {
            str_from!(
                self.value.as_ptr().add(self.path as usize),
                self.path_len as usize
            )
        }
    }

    /// Returns the query component if exists.
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        let offset = (self.path + self.path_len + 1) as usize;
        if offset < self.value.len() {
            unsafe {
                Some(str_from!(
                    self.value.as_ptr().add(offset),
                    self.value.len() - offset
                ))
            }
        } else {
            None
        }
    }
}

// ===== std traits =====

macro_rules! str_from {
    ($data:expr, $len:expr $(,)?) => {
        str::from_utf8_unchecked(from_raw_parts($data, $len))
    };
}

use str_from;

/// Returns:
///
/// ```not_rust
/// is_https: bool,
/// host_len: u16,
/// path: u16,
/// path_len: u16,
/// ```
///
/// ```not_rust
/// http-URI = "http" "://" authority path-abempty [ "?" query ]
/// https-URI = "https" "://" authority path-abempty [ "?" query ]
/// ```
const fn parse_http(bytes: &[u8]) -> Result<(bool, u16, u16, u16), UriError> {
    let Some((scheme, mut state)) = bytes.split_first_chunk() else {
        return Err(UriError::InvalidScheme);
    };
    let is_https = if let b"http://" = scheme {
        false
    } else if let b"https:/" = scheme {
        let Some((b'/', rest)) = state.split_first() else {
            return Err(UriError::InvalidScheme);
        };
        state = rest;
        true
    } else {
        return Err(UriError::InvalidScheme);
    };

    let host_len = match authority::match_authority(&mut state) {
        Ok(ok) => ok as u16,
        Err(err) => return Err(err),
    };

    // > A sender MUST NOT generate an "http" URI with an empty host identifier.
    if host_len == 0 {
        return Err(UriError::InvalidAuthority);
    }

    let Some(delim) = state.first() else {
        return Ok((is_https, host_len, bytes.len() as u16, 0));
    };
    if *delim != b'/' {
        return Err(UriError::InvalidAuthority);
    }
    let path_len = match target::match_path(&mut state) {
        Ok(ok) => ok as u16,
        Err(err) => return Err(err),
    };
    let path = unsafe { (delim as *const u8).offset_from_unsigned(bytes.as_ptr()) as u16 };
    Ok((is_https, host_len, path, path_len))
}

#[test]
fn test_http_uri() {
    let http = HttpUri::from_slice("http://example.com/users/all?page=440").unwrap();
    assert!(http.is_http());
    assert_eq!(http.authority(), "example.com");
    assert_eq!(http.host(), "example.com");
    assert_eq!(http.port(), None);
    assert_eq!(http.path(), "/users/all");
    assert_eq!(http.query(), Some("page=440"));

    // authority required

    assert!(HttpUri::from_slice("http:/users/all?page=440").is_err());
}
