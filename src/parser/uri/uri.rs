use super::Uri;

pub(crate) const MAX_SCHEME: u16    = 0b0111_1111_1111_1111;
pub(crate) const SCHEME_NONE: u16   = 0b1000_0000_0000_0000;

const SCHEME_HTTP: u16  = 0b1000_0000_0000_0001;
const SCHEME_HTTPS: u16 = 0b1000_0000_0000_0010;

pub(crate) const MAX_AUTH: u16   = 0b0111_1111_1111_1111;
pub(crate) const AUTH_NONE: u16  = 0b1000_0000_0000_0000;

impl Uri {
    /// Returns the scheme as `str`, e.g: `http`.
    ///
    /// Scheme can be empty for abempty path.
    #[inline]
    pub fn scheme(&self) -> &str {
        match self.scheme {
            self::SCHEME_NONE => "",
            self::SCHEME_HTTP => "http",
            self::SCHEME_HTTPS => "https",
            _ => &self.value[..self.scheme as usize],
        }
    }

    /// Returns the authority as `str`, e.g: `example.com:80`.
    #[inline]
    pub fn authority_str(&self) -> Option<&str> {
        match self.authority {
            self::AUTH_NONE => None,
            _ => {
                let offset = match self.scheme {
                    self::SCHEME_HTTP |
                    self::SCHEME_HTTPS => 0,
                    len => len as usize + "://".len(),
                };
                Some(&self.value[offset..self.authority as usize])
            }
        }
    }

    /// Returns the authority host.
    #[inline]
    pub fn host(&self) -> Option<&str> {
        match self.authority_str() {
            Some(auth) => match auth.find('@') {
                Some(idx) => Some(&auth[idx + 1..]),
                None => Some(auth),
            },
            None => None,
        }
    }

    /// Returns the authority hostname.
    #[inline]
    pub fn hostname(&self) -> Option<&str> {
        match self.host() {
            Some(host) => match host.rfind(':') {
                Some(idx) => Some(&host[..idx]),
                None => Some(host),
            },
            None => None,
        }
    }

    /// Returns the authority port.
    #[inline]
    pub fn port(&self) -> Option<u16> {
        match self.host() {
            Some(host) => match host.rfind(':') {
                Some(col) => host[col + 1..].parse().ok(),
                None => None,
            },
            None => None,
        }
    }

    /// Returns the authority userinfo.
    #[inline]
    pub fn userinfo(&self) -> Option<&str> {
        match self.authority_str() {
            Some(auth) => match auth.find('@') {
                Some(idx) => Some(&auth[..idx]),
                None => None,
            },
            None => None,
        }
    }

    /// Returns the path as `str`, e.g: `/over/there`.
    #[inline]
    pub fn path(&self) -> &str {
        &self.value[self.path as usize..self.query as usize]
    }

    /// Returns the query as `str`, e.g: `name=joe&query=4`.
    #[inline]
    pub fn query(&self) -> Option<&str> {
        if self.query as usize == self.value.len() {
            None
        } else {
            Some(&self.value[self.query as usize + 1..])
        }
    }

    /// Returns the path and query as `str`, e.g: `/over/there?name=joe&query=4`.
    #[inline]
    pub fn path_and_query(&self) -> &str {
        &self.value[self.path as usize..]
    }

    // scheme can be a bitflag value, making the contained str incomplete
    //
    // /// Returns the str representation.
    // #[inline]
    // pub const fn as_str(&self) -> &str {
    //     self.value.as_str()
    // }
}

impl Default for Uri {
    #[inline]
    fn default() -> Self {
        Self::root()
    }
}

// ===== Formatting =====

impl std::fmt::Display for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.scheme {
            self::SCHEME_NONE => {},
            self::SCHEME_HTTP => f.write_str("http:")?,
            self::SCHEME_HTTPS => f.write_str("https:")?,
            _ => {},
        }
        f.write_str(&self.value)
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
            let ok = Uri::try_copy_from($rw.as_bytes()).unwrap();
            assert_eq!(ok.scheme(), $schema);
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
            "https://example.com?";
            "https", Some("example.com"), "", Some("");
        }
    }

    #[test]
    fn test_parse_uri_edge_cases() {
        assert!(Uri::try_copy_from(b"").is_err());

        assert_uri! {
            "*";
            "", None, "*", None;
        }

        assert_uri! {
            "/";
            "", None, "/", None;
        }

        assert_uri! {
            "/over/there?name=ferret#head";
            "", None, "/over/there", Some("name=ferret");
        }

        // assert_uri! {
        //     "d";
        //     "", Some("d"), "", None;
        // }

        // assert_uri! {
        //     "example.com";
        //     None, Some("example.com"), "", None;
        // }

        // assert_uri! {
        //     "example.com/over/there?name=ferret#head";
        //     None, Some("example.com"), "/over/there", Some("name=ferret");
        // }

        // assert_uri! {
        //     "example.com:80/over/there?name=ferret#head";
        //     None, Some("example.com:80"), "/over/there", Some("name=ferret");
        // }

        assert_uri! {
            "foo:";
            "foo", None, "", None;
        }

        assert_uri! {
            "foo:/over/there?name=ferret#head";
            "foo", None, "/over/there", Some("name=ferret");
        }

        assert_uri! {
            "foo://example.com:80/over/there?name=ferret#head";
            "foo", Some("example.com:80"), "/over/there", Some("name=ferret");
        }
    }
}
