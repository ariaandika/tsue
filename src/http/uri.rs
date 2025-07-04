//! Uniform Resource Identifier.
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
// Internally:
//
// ```
//   foo://example.com:8042/over/there?name=ferret
//     _/          ________|___       \_____
//    /           /            \            \
// scheme     authority       path        query
//
//   foo:/over/there
//     _/\___       \_____
//    /      \            \
// scheme   path        query
//
//   /over/there
//   \___       \_____
//       \            \
//      path        query
//
//   example.com
//      ________|______
//     /          \    \
// authority    path  query
// ```
#[derive(Debug)]
pub struct Uri {
    value: ByteStr,
    scheme: u16,
    authority: u16,
    path: u16,
    query: u16,
}

const MAX_SCHEME: u16   = 0b0111_1111_1111_1111;
const SCHEME_NONE: u16  = 0b1000_0000_0000_0000;
const SCHEME_HTTP: u16  = 0b1000_0000_0000_0001;
const SCHEME_HTTPS: u16 = 0b1000_0000_0000_0010;

const MAX_AUTH: u16   = 0b0111_1111_1111_1111;
const AUTH_NONE: u16  = 0b1000_0000_0000_0000;

impl Uri {
    #[inline]
    pub fn try_copy_from(value: &str) -> Result<Self, InvalidUri> {
        parse_uri(ByteStr::copy_from_str(value))
    }

    #[inline]
    pub fn scheme_str(&self) -> Option<&str> {
        match self.scheme {
            self::SCHEME_NONE => None,
            self::SCHEME_HTTP => Some("http"),
            self::SCHEME_HTTPS => Some("https"),
            _ => Some(&self.value[..self.scheme as usize]),
        }
    }

    #[inline]
    pub fn authority_str(&self) -> Option<&str> {
        match self.authority {
            AUTH_NONE => None,
            auth => {
                // `3` here is '://' if there is scheme
                let sc = match self.scheme {
                    self::SCHEME_NONE => 0,
                    self::SCHEME_HTTP => 4 + 3,
                    self::SCHEME_HTTPS => 5 + 3,
                    len => len as usize + 3,
                };
                Some(&self.value[sc..auth as usize])
            },
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

/// NOTE: no uri fragment, will be striped by `trim_fragment`
///
/// [source](https://datatracker.ietf.org/doc/html/rfc3986#section-3)
fn parse_uri(value: ByteStr) -> Result<Uri, InvalidUri> {
    let mut bufm = value.as_bytes();

    // ===== Common cases =====

    if bufm.is_empty() {
        return Err(InvalidUri::Incomplete);
    }

    if bufm.len() == 1 {
        let (a, p, q) = match bufm[0] {
            b'/' | b'*' => (AUTH_NONE, 0, 1),
            _ => (1, 1, 1),
        };
        return Ok(Uri {
            value,
            scheme: SCHEME_NONE,
            authority: a,
            path: p,
            query: q,
        });
    }

    if bufm[0] == b'/' {
        let path_len = parse_path(&mut bufm)?;
        return Ok(Uri {
            value: trim_fragment(value, path_len),
            scheme: SCHEME_NONE,
            authority: AUTH_NONE,
            path: 0,
            query: path_len,
        });
    }

    // ===== Leader =====

    let (is_scheme, leader) = parse_leader(&mut bufm)?;

    // could be early check here in case of authority only
    // if the buffer is already empty

    let (scheme, authority, auth_len) = if is_scheme {
        let auth_len = parse_authority(&mut bufm)?;
        let auth = if auth_len <= 2 {
            AUTH_NONE
        } else {
            leader + 1 + auth_len
        };
        (leader, auth, auth_len)
    } else {
        (SCHEME_NONE, leader, 0)
    };

    // ===== Path =====

    let path_len = parse_path(&mut bufm)?;
    let path = match authority {
        AUTH_NONE => scheme + auth_len + 1,
        _ => authority,
    };
    let query = path + path_len;

    Ok(Uri {
        value: trim_fragment(value, query),
        scheme,
        authority,
        path,
        query,
    })
}

/// find delimiter
///
/// if ':' parse as scheme
///
/// otherwise as authority
///
/// edge cases if authority contains ':', if the next char is a digit, which presumably a port,
/// will be parsed as authority, otherwise, as scheme
///
/// returns (is_scheme, len)
fn parse_leader(buf: &mut &[u8]) -> Result<(bool, u16), InvalidUri> {
    use InvalidUri::*;

    const IS_SCHEME: bool = true;
    const IS_AUTHORITY: bool = false;

    let mut valid_scheme = Ok(());
    let mut cursor = Cursor::new(buf);

    match cursor.pop_front() {
        Some(lead) if lead.is_ascii_alphanumeric() => {},
        Some(lead) => valid_scheme = Err(Char(lead as char)),
        None => return Err(Incomplete),
    }

    loop {
        match cursor.pop_front() {
            Some(lead) => match lead {
                // authority, no trailing colon
                b'/' | b'?' | b'#' => {
                    // SAFETY: called `.pop_front()`
                    unsafe { cursor.step_back(1) };
                    *buf = cursor.as_bytes();
                    return match cursor.step().u16_max(MAX_AUTH) {
                        Ok(ok) => Ok((IS_AUTHORITY, ok)),
                        Err(_) => Err(TooLong)
                    };
                },
                b':' => {
                    // maybe a port
                    let Some(b'0'..=b'9') = cursor.first() else {
                        break;
                    };

                    let mut remain = cursor.as_bytes();
                    let len = cursor.step();
                    let remain_len = parse_authority_partial(&mut remain)?;
                    *buf = remain;
                    return Ok((IS_AUTHORITY, (len + remain_len).u16_max(MAX_AUTH)?));
                },

                b'+' | b'-' | b'.' => {}
                e if e.is_ascii_alphanumeric() => {}
                ch => if valid_scheme.is_ok() {
                    valid_scheme = Err(Char(ch as char));
                },
            },
            // no colon, authority only
            None => {
                *buf = cursor.as_bytes();
                return match cursor.step().u16_max(MAX_AUTH) {
                    Ok(ok) => Ok((IS_AUTHORITY, ok)),
                    Err(_) => Err(TooLong)
                };
            },
        }
    }

    valid_scheme?;

    // note that ':' got eaten here
    *buf = cursor.as_bytes();

    // but the scheme length does not include ':'
    Ok((IS_SCHEME, cursor.step().u16_max(MAX_SCHEME)? - 1))
}

fn parse_authority_partial(buf: &mut &[u8]) -> Result<usize, InvalidUri> {
    let mut cursor = Cursor::new(buf);

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
    Ok(cursor.step())
}

/// ```not_rust
/// authority   = [ userinfo "@" ] host [ ":" port ]
/// ```
///
/// terminated by `/`, `?`, `#`, or by the end
///
/// [source](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2)
fn parse_authority(buf: &mut &[u8]) -> Result<u16, InvalidUri> {
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
    cursor.step().u16()
}

/// NOTE: does not check for leading slashes
///
/// If a URI contains an authority component, then the path component must either be empty or begin
/// with a slash (`/`) character.
///
/// If a URI does not contain an authority component, then the path cannot begin with two slash
/// characters (`//`).
///
/// [source](https://datatracker.ietf.org/doc/html/rfc3986#section-3.3)
fn parse_path(buf: &mut &[u8]) -> Result<u16, InvalidUri> {
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
    cursor.step().u16()
}

fn trim_fragment(value: ByteStr, query: u16) -> ByteStr {
    debug_assert!(matches!(
        value.as_bytes().get(query as usize),
        Some(b'?') | None
    ));

    let Some(qr) = value.as_bytes().get(query as usize..) else {
        return value;
    };

    let Some(hash) = qr.iter().position(|&e|e==b'#') else {
        return value;
    };

    value.slice_ref(&value[..query as usize + hash])
}

// ===== Helper =====

trait TryU16 {
    fn u16(self) -> Result<u16, InvalidUri>;

    fn u16_max(self, max: u16) -> Result<u16, InvalidUri>;
}

impl TryU16 for usize {
    fn u16(self) -> Result<u16, InvalidUri> {
        self.try_into().map_err(|_|InvalidUri::TooLong)
    }

    fn u16_max(self, max: u16) -> Result<u16, InvalidUri> {
        match self.try_into() {
            Ok(ok) if ok <= max => Ok(ok),
            _ => Err(InvalidUri::TooLong),
        }
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

    #[allow(unused, reason = "debugging in test")]
    macro_rules! panic_uri {
        ($e:expr) => {
            panic!("{:?}",Uri::try_copy_from($e))
        };
    }

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
            Some("http"), Some("localhost:3000"), "/users", None;
        }

        assert_uri! {
            "https://example.com/search?q=rust&lang=en";
            Some("https"), Some("example.com"), "/search", Some("q=rust&lang=en");
        }

        assert_uri! {
            "postgresql://user@localhost";
            Some("postgresql"), Some("user@localhost"), "", None;
        }

        assert_uri! {
            "mailto:";
            Some("mailto"), None, "", None;
        }

        assert_uri! {
            "http://[2001:db8::1]:8080/path";
            Some("http"), Some("[2001:db8::1]:8080"), "/path", None;
        }

        assert_uri! {
            "file:///etc/hosts";
            Some("file"), None, "/etc/hosts", None;
        }

        assert_uri! {
            "https://example.com/foo%20bar?name=John%20Doe";
            Some("https"), Some("example.com"), "/foo%20bar", Some("name=John%20Doe");
        }

        assert_uri! {
            "https://example.com?";
            Some("https"), Some("example.com"), "", Some("");
        }
    }

    #[test]
    fn test_parse_uri_edge_cases() {
        assert!(Uri::try_copy_from("").is_err());

        assert_uri! {
            "*";
            None, None, "*", None;
        }

        assert_uri! {
            "/";
            None, None, "/", None;
        }

        assert_uri! {
            "/over/there?name=ferret#head";
            None, None, "/over/there", Some("name=ferret");
        }

        assert_uri! {
            "d";
            None, Some("d"), "", None;
        }

        assert_uri! {
            "example.com";
            None, Some("example.com"), "", None;
        }

        assert_uri! {
            "example.com/over/there?name=ferret#head";
            None, Some("example.com"), "/over/there", Some("name=ferret");
        }

        assert_uri! {
            "example.com:80/over/there?name=ferret#head";
            None, Some("example.com:80"), "/over/there", Some("name=ferret");
        }

        assert_uri! {
            "foo:";
            Some("foo"), None, "", None;
        }

        assert_uri! {
            "foo:/over/there?name=ferret#head";
            Some("foo"), None, "/over/there", Some("name=ferret");
        }

        assert_uri! {
            "foo://example.com:80/over/there?name=ferret#head";
            Some("foo"), Some("example.com:80"), "/over/there", Some("name=ferret");
        }
    }
}

