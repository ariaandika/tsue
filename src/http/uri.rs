//! Uniform Resource Identifier.
use std::num::NonZeroU16;
use tcio::ByteStr;

/// Uniform Resource Identifier.
///
/// A Uniform Resource Identifier ([URI](https://datatracker.ietf.org/doc/html/rfc3986)) provides a
/// simple and extensible means for identifying a resource.
///
/// The generic URI syntax consists of a hierarchical sequence of components referred to as the scheme,
/// authority, path, and query.
///
/// ```not_rust
/// URI         = scheme ":" hier-part [ "?" query ] [ "#" fragment ]
///
/// hier-part   = "//" authority path-abempty
///             / path-absolute
///             / path-rootless
///             / path-empty
/// ```
///
/// The [`Uri`] is slightly differs from the standards, where scheme is optional and there is no
/// fragment. Path is required, though it may be empty (no characters).  When authority is present, the
/// path must either be empty or begin with a slash (`/`) character.  When authority is not present,
/// the path cannot begin with two slash characters (`//`).
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
#[derive(Debug)]
pub struct Uri {
    value: ByteStr,
    // scheme end, point to `:`
    scheme: u16,
    // auth end
    authority: Option<NonZeroU16>,
    // path start
    path: u16,
    // query start, point to either `?` or end of bytes
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
    use InvalidUri::*;

    let mut buf = value.as_bytes();

    let scheme = parse_scheme(buf)?;

    if buf.len() == (scheme + 1) as usize {
        let path = value.len() as _;
        return Ok(Uri {
            value,
            scheme,
            authority: None,
            path,
            query: path,
        })
    }

    buf = &buf[(scheme + 1) as usize..];

    let Some(delim) = buf.first_chunk::<2>() else {
        return Err(Incomplete)
    };

    const COL_DELIM: u16 = "://".len() as _;

    match delim {
        b"//" => {
            let auth_len = parse_authority(&buf[2..])?;
            let auth_end = if auth_len == 0 {
                None
            } else {
                Some(scheme + COL_DELIM + auth_len)
            };

            let path_len = parse_path(&buf[(2 + auth_len) as usize..])?;
            let path = scheme + COL_DELIM + auth_len;
            let query = path + path_len;

            Ok(Uri {
                value,
                scheme,
                // SAFETY: COL_DELIM is non zero which parts of addition
                authority: auth_end.map(|ok|unsafe { NonZeroU16::new_unchecked(ok) }),
                path,
                query,
            })
        }
        _ => {
            let path_len = parse_path(buf)?;
            let path = scheme + 1;
            let query = path + path_len;

            Ok(Uri {
                value,
                scheme,
                authority: None,
                path,
                query,
            })
        },
    }
}

/// ```not_rust
/// scheme      = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
/// ```
///
/// terminated by `:`
///
/// [source](https://datatracker.ietf.org/doc/html/rfc3986#section-3.1)
fn parse_scheme(buf: &[u8]) -> Result<u16, InvalidUri> {
    use InvalidUri::*;

    let Some(lead) = buf.first() else {
        return Err(Incomplete);
    };

    match lead {
        b'+' | b'-' | b'.' => {}
        e if e.is_ascii_alphanumeric() => {}
        ch => return Err(Char(*ch as char)),
    };

    let mut n = 1;

    loop {
        let Some(byte) = buf.get(n) else {
            return Err(Incomplete);
        };

        match byte {
            b'+' | b'-' | b'.' => {}
            b':' => break,
            e if e.is_ascii_alphanumeric() => {}
            ch => return Err(Char(*ch as char)),
        }

        n += 1;
    }

    n.try_into().map_err(|_| TooLong)
}

/// ```not_rust
/// authority   = [ userinfo "@" ] host [ ":" port ]
/// ```
///
/// terminated by `/`, `?`, `#`, or by the end
///
/// [source](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2)
fn parse_authority(buf: &[u8]) -> Result<u16, InvalidUri> {
    use InvalidUri::*;

    let mut n = 0;

    loop {
        match buf.get(n) {
            Some(b'/' | b'?') => break,
            Some(b'#') => return Err(Fragment),
            Some(_) => n += 1,
            None => break,
        }
    }

    n.try_into().map_err(|_| TooLong)
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
fn parse_path(buf: &[u8]) -> Result<u16, InvalidUri> {
    let mut n = 0;

    loop {
        match buf.get(n) {
            Some(b'?') => break,
            Some(b'#') => return Err(InvalidUri::Fragment),
            Some(_) => n += 1,
            None => break,
        }
    }

    match n.try_into() {
        Ok(ok) => Ok(ok),
        Err(_) => Err(InvalidUri::TooLong),
    }
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
    /// Fragment are not allowed.
    Fragment,
}

impl std::error::Error for InvalidUri { }

impl std::fmt::Display for InvalidUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use InvalidUri::*;
        f.write_str("invalid uri: ")?;
        match self {
            TooLong => todo!(),
            Incomplete => todo!(),
            Char(ch) => write!(f, "unexpected character `{ch}`"),
            Fragment => todo!(),
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
    fn test_parse_failures() {
        assert!(Uri::try_copy_from("://example.com").is_err());
        assert!(Uri::try_copy_from("ht%tp://example.com").is_err());
        assert!(Uri::try_copy_from("").is_err());
    }
}

