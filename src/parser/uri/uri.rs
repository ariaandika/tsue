use super::Uri;

pub(crate) const MAX_SCHEME: u16    = 0b0111_1111_1111_1111;
pub(crate) const SCHEME_HTTP: u16  = 0b1000_0000_0000_0001;
pub(crate) const SCHEME_HTTPS: u16 = 0b1000_0000_0000_0010;

pub(crate) const MAX_AUTH: u16   = 0b0111_1111_1111_1111;
pub(crate) const AUTH_NONE: u16  = 0b1000_0000_0000_0000;

impl Uri {
    /// Returns the scheme as `str`, e.g: `http`.
    ///
    /// Scheme can be empty for abempty path.
    #[inline]
    pub fn scheme(&self) -> &str {
        match self.scheme {
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

// ===== Formatting =====

impl std::fmt::Display for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.scheme {
            self::SCHEME_HTTP => f.write_str("http:")?,
            self::SCHEME_HTTPS => f.write_str("https:")?,
            _ => {},
        }
        f.write_str(&self.value)
    }
}
