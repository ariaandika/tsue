//! Uniform Resource Identifier.
use std::num::NonZeroU16;
use tcio::{slice::Cursor, ByteStr};

/// Uniform Resource Identifier.
///
/// A Uniform Resource Identifier ([URI]) provides a simple and extensible means for identifying a
/// resource.
///
/// The generic URI syntax consists of a hierarchical sequence of components referred to as the
/// scheme, authority, path, and query.
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
/// [URI]: <https://datatracker.ietf.org/doc/html/rfc7230#section-2.7>
//
//
// Internally:
//
// ```
//   foo://example.com:8042/over/there?name=ferret
//     _/          ________/    _____/\_____
//    /           /            /            \
// scheme     authority       path        query
//
//   foo:/over/there
//     _/\___       \_____
//    /      \            \
// scheme   path        query
// ```
#[derive(Debug)]
pub struct Uri {
    value: ByteStr,
    scheme: u16,
    authority: Option<NonZeroU16>,
    path: u16,
    query: u16,
}

impl Uri {
    #[inline]
    pub fn try_copy_from(value: &str) -> Result<Self, InvalidUri> {
        parse_uri(ByteStr::copy_from_str(value))
    }

    #[inline]
    pub fn scheme_str(&self) -> &str {
        &self.value[..self.scheme as usize]
    }

    #[inline]
    pub fn authority_str(&self) -> Option<&str> {
        match self.authority {
            Some(ok) => Some(&self.value[(self.scheme + 3) as usize..ok.get() as usize]),
            None => None,
        }
    }

    #[inline]
    pub fn path(&self) -> &str {
        &self.value[self.path as usize..self.query as usize]
    }

    #[inline]
    pub fn query(&self) -> Option<&str> {
        if self.query as usize == self.value.len() {
            None
        } else {
            Some(&self.value[(self.query + 1) as usize..])
        }
    }

    #[inline]
    pub fn path_and_query(&self) -> &str {
        &self.value[self.path as usize..]
    }
}

// ===== Parser =====

/// ```not_rust
/// URI         = scheme ":" hier-part [ "?" query ]
///
/// hier-part   = "//" authority path-abempty
///             / path-absolute
///             / path-rootless
///             / path-empty
/// ```
///
/// NOTE: no uri fragment, will be striped by `parse_path`
///
/// [source](https://datatracker.ietf.org/doc/html/rfc3986#section-3)
fn parse_uri(value: ByteStr) -> Result<Uri, InvalidUri> {
    let mut bufm = value.as_bytes();

    // ===== Scheme =====

    let scheme = parse_scheme(&mut bufm)?;

    debug_assert!(!bufm.is_empty(), "`parse_scheme` success with no `:` found");

    if bufm.len() == 1 {
        let path = value.len() as _;
        return Ok(Uri {
            value,
            scheme,
            authority: None,
            path,
            query: path,
        })
    }

    // SAFETY: `bufm` is not empty, thus `bufm.len >= 1`
    bufm = unsafe { bufm.get_unchecked(1..) };

    // ===== Authority =====

    let auth_len = parse_authority(&mut bufm)?;
    let authority = if auth_len <= 2 {
        None
    } else {
        let auth_end = scheme + 1 + auth_len;
        // SAFETY: addition with constant non zero value
        Some(unsafe { NonZeroU16::new_unchecked(auth_end) })
    };

    // ===== Path =====

    let path_len = parse_path(&mut bufm)?;
    let path = match authority {
        Some(ok) => ok.get(),
        None => scheme + auth_len + 1,
    };
    let query = path + path_len;

    Ok(Uri {
        value,
        scheme,
        authority,
        path,
        query,
    })
}

/// ```not_rust
/// scheme      = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
/// ```
///
/// terminated by `:`
///
/// [source](https://datatracker.ietf.org/doc/html/rfc3986#section-3.1)
fn parse_scheme(buf: &mut &[u8]) -> Result<u16, InvalidUri> {
    use InvalidUri::*;

    let len = buf.len();
    let mut cursor = Cursor::new(buf);

    match cursor.pop_front() {
        Some(lead) => match lead {
            b'+' | b'-' | b'.' => {}
            e if e.is_ascii_alphanumeric() => {}
            ch => return Err(Char(ch as char)),
        },
        None => return Err(Incomplete),
    }

    loop {
        match cursor.pop_front() {
            Some(lead) => match lead {
                b':' => break,
                b'+' | b'-' | b'.' => {}
                e if e.is_ascii_alphanumeric() => {}
                ch => return Err(Char(ch as char)),
            },
            None => return Err(Incomplete),
        }
    }

    // SAFETY: loop run at least once, which call `.pop_front()`
    unsafe { cursor.step_back(1) };

    *buf = cursor.as_bytes();
    (len - cursor.remaining())
        .try_into()
        .map_err(|_| TooLong)
}

/// ```not_rust
/// authority   = [ userinfo "@" ] host [ ":" port ]
/// ```
///
/// terminated by `/`, `?`, `#`, or by the end
///
/// [source](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2)
fn parse_authority(buf: &mut &[u8]) -> Result<u16, InvalidUri> {
    let len = buf.len();
    let mut cursor = Cursor::new(buf);

    match cursor.first_chunk::<2>() {
        Some(b"//") => {
            // SAFETY: checked by `.first_chunk::<2>()`
            unsafe { cursor.advance(2) }
        },
        _ => return Ok(0),
    };

    loop {
        match cursor.first() {
            Some(b'/' | b'?' | b'#') => break,
            Some(_) => {
                // SAFETY: checked by `.first()`
                unsafe { cursor.advance(1) };
            },
            None => break,
        }
    }

    *buf = cursor.as_bytes();
    (len - cursor.remaining())
        .try_into()
        .map_err(|_| InvalidUri::TooLong)
}

/// NOTE: does not check for leading slashes
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
/// segment       = *pchar
/// segment-nz    = 1*pchar
/// segment-nz-nc = 1*( unreserved / pct-encoded / sub-delims / "@" )
///                 ; non-zero-length segment without any colon ":"
///
/// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
/// ```
///
/// If a URI contains an authority component, then the path component must either be empty or begin
/// with a slash (`/`) character.
///
/// If a URI does not contain an authority component, then the path cannot begin with two slash
/// characters (`//`).
///
/// [source](https://datatracker.ietf.org/doc/html/rfc3986#section-3.3)
fn parse_path(buf: &mut &[u8]) -> Result<u16, InvalidUri> {
    let len = buf.len();
    let mut cursor = Cursor::new(buf);

    loop {
        match cursor.first() {
            Some(b'?' | b'#') => break,
            Some(_) => {
                // SAFETY: checked by `.first()`
                unsafe { cursor.advance(1) };
            }
            None => break,
        }
    }

    *buf = cursor.as_bytes();
    (len - cursor.remaining())
        .try_into()
        .map_err(|_| InvalidUri::TooLong)
}

// ===== Error =====

#[derive(Debug)]
pub enum InvalidUri {
    /// Bytes ends before all components parsed.
    Incomplete,
    /// Bytes length is too large.
    TooLong,
    /// Invalid character found.
    Char(char),
}

impl std::error::Error for InvalidUri { }

impl std::fmt::Display for InvalidUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use InvalidUri::*;
        f.write_str("invalid uri: ")?;
        match self {
            TooLong => f.write_str("data length is too large"),
            Incomplete => f.write_str("data is incomplete"),
            Char(ch) => write!(f, "unexpected character `{ch}`"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! assert_uri {
        (
            $rw:expr;
            $schema:expr, $auth:expr, $path:expr, $q:expr;
        ) => {
            let ok = Uri::try_copy_from($rw).unwrap();
            assert_eq!(ok.scheme_str(), $schema);
            assert_eq!(ok.authority_str(), $auth);
            assert_eq!(ok.path(), $path);
            assert_eq!(ok.query(), $q);
        };
    }

    #[test]
    fn test_parse_uri() {
        assert_uri! {
            "http://localhost:3000/users";
            "http", Some("localhost:3000"), "/users", None;
        }

        assert_uri! {
            "https://example.com/search?q=rust&lang=en";
            "https", Some("example.com"), "/search", Some("q=rust&lang=en");
        }

        assert_uri! {
            "postgresql://user@localhost";
            "postgresql", Some("user@localhost"), "", None;
        }

        assert_uri! {
            "mailto:";
            "mailto", None, "", None;
        }

        assert_uri! {
            "http://[2001:db8::1]:8080/path";
            "http", Some("[2001:db8::1]:8080"), "/path", None;
        }

        assert_uri! {
            "file:///etc/hosts";
            "file", None, "/etc/hosts", None;
        }

        assert_uri! {
            "https://example.com/foo%20bar?name=John%20Doe";
            "https", Some("example.com"), "/foo%20bar", Some("name=John%20Doe");
        }

        assert_uri! {
            "foo:/bar";
            "foo", None, "/bar", None;
        }

        assert_uri! {
            "https://example.com?";
            "https", Some("example.com"), "", Some("");
        }
    }

    #[test]
    fn test_parse_uri_err() {
        assert!(Uri::try_copy_from("://example.com").is_err());
        assert!(Uri::try_copy_from("ht%tp://example.com").is_err());
        assert!(Uri::try_copy_from("").is_err());
    }
}

