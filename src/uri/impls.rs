use std::slice::from_raw_parts;

use super::{Authority, Host, HttpScheme, HttpUri, Path, Scheme, Uri, matches};

impl Scheme {
    /// Extracts a string slice containing the scheme.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

impl HttpScheme {
    /// HTTP Scheme.
    pub const HTTP: Self = Self(false);
    /// HTTPS Scheme.
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
    pub const fn as_str(&self) -> &str {
        const HTTPS: &str = "https";
        unsafe { str::from_utf8_unchecked(from_raw_parts(HTTPS.as_ptr(), 4 + self.0 as usize)) }
    }
}

impl Authority {
    const fn split_userinfo(&self) -> Option<(&[u8], &[u8])> {
        matches::split_at_sign(self.value.as_slice())
    }

    const fn split_port(&self) -> Option<(&[u8], &[u8])> {
        let host = match self.split_userinfo() {
            Some((_, host)) => host,
            None => self.value.as_slice(),
        };
        matches::split_port(host)
    }

    /// Returns the authority host.
    ///
    /// ```not_rust
    /// user:pass@example.com:8042
    ///           \______________/
    ///                  |
    ///                 host
    /// ```
    #[inline]
    pub const fn host(&self) -> &str {
        match self.split_userinfo() {
            Some((_, host)) => unsafe { str::from_utf8_unchecked(host) },
            None => self.as_str(),
        }
    }

    /// Returns the authority hostname.
    ///
    /// ```not_rust
    /// user:pass@example.com:8042
    ///           \_________/
    ///                |
    ///             hostname
    /// ```
    #[inline]
    pub const fn hostname(&self) -> &str {
        let host = match self.split_userinfo() {
            Some((_, host)) => host,
            None => self.value.as_slice(),
        };
        let hostname = match matches::split_port(host) {
            Some((hostname, _)) => hostname,
            None => host,
        };
        unsafe { str::from_utf8_unchecked(hostname) }
    }

    /// Returns the authority port.
    ///
    /// ```not_rust
    /// user:pass@example.com:8042
    ///                       \__/
    ///                        |
    ///                       port
    /// ```
    #[inline]
    pub const fn port(&self) -> Option<u16> {
        match self.split_port() {
            // with port validation in constructor, should we do unsafe calculation ?
            Some((_, port)) => match tcio::atou(port) {
                Some(ok) => Some(ok as u16),
                None => None,
            }
            None => None,
        }
    }

    /// Returns the authority userinfo.
    ///
    /// ```not_rust
    /// user:pass@example.com:8042
    /// \_______/
    ///     |
    ///  userinfo
    /// ```
    #[inline]
    pub const fn userinfo(&self) -> Option<&str> {
        match self.split_userinfo() {
            Some((userinfo, _)) => unsafe {
                Some(str::from_utf8_unchecked(userinfo))
            },
            None => None,
        }
    }

    /// Extracts a string slice containing the authority.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

impl Host {
    const fn split_port(&self) -> Option<(&[u8], &[u8])> {
        matches::split_port(self.value.as_slice())
    }

    /// Returns the entire host.
    #[inline]
    pub const fn host(&self) -> &str {
        self.as_str()
    }

    /// Returns the authority hostname.
    ///
    /// ```not_rust
    /// example.com:8042
    /// \_________/
    ///      |
    ///   hostname
    /// ```
    #[inline]
    pub const fn hostname(&self) -> &str {
        let hostname = match self.split_port() {
            Some((hostname, _)) => hostname,
            None => self.value.as_slice(),
        };
        unsafe { str::from_utf8_unchecked(hostname) }
    }

    /// Returns the authority port.
    ///
    /// ```not_rust
    /// example.com:8042
    ///             \__/
    ///              |
    ///             port
    /// ```
    #[inline]
    pub const fn port(&self) -> Option<u16> {
        match self.split_port() {
            // with port validation in constructor, should we do unsafe calculation ?
            Some((_, port)) => match tcio::atou(port) {
                Some(ok) => Some(ok as u16),
                None => None,
            }
            None => None,
        }
    }

    /// Extracts a string slice containing the host.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
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
        unsafe {
            str::from_utf8_unchecked(from_raw_parts(self.value.as_ptr(), self.query as usize))
        }
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
                Some(str::from_utf8_unchecked(from_raw_parts(
                    self.value.as_ptr().add(query + 1),
                    self.value.len() - query - 1,
                )))
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
    /// ```not_rust
    /// foo://example.com:8042/over/there?name=ferret
    ///       \______________/
    ///              |
    ///          authority
    /// ```
    ///
    /// If returned [`Some`], the string will not be empty.
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

impl HttpUri {
    /// Returns `true` if the scheme is HTTPS.
    #[inline]
    pub const fn is_https(&self) -> bool {
        self.is_https
    }

    /// Returns the authority component.
    ///
    /// ```not_rust
    /// http://example.com:8042/over/there?name=ferret
    ///        \______________/
    ///               |
    ///           authority
    /// ```
    #[inline]
    pub const fn authority(&self) -> &str {
        self.authority.as_str()
    }

    /// Returns the authority component as [`Authority`].
    #[inline]
    pub const fn as_authority(&self) -> &Authority {
        &self.authority
    }

    /// Returns the host component.
    ///
    /// ```not_rust
    /// http://example.com:8042/over/there?name=ferret
    ///        \______________/
    ///               |
    ///           authority
    /// ```
    #[inline]
    pub const fn host(&self) -> &str {
        self.authority.host()
    }

    /// Returns the path component.
    ///
    /// ```not_rust
    /// http://example.com:8042/over/there?name=ferret
    ///        \______________/
    ///               |
    ///           authority
    /// ```
    #[inline]
    pub const fn path(&self) -> &str {
        self.path.path()
    }

    /// Returns the query component if exists.
    ///
    /// ```not_rust
    /// http://example.com:8042/over/there?name=ferret
    ///                                    \_________/
    ///                                         |
    ///                                       query
    /// ```
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        self.path.query()
    }

    /// Returns the path and query component.
    ///
    /// ```not_rust
    /// http://example.com:8042/over/there?name=ferret
    ///                        \_____________________/
    ///                                   |
    ///                             path and query
    /// ```
    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.path.as_str()
    }

    /// Consume `HttpUri` into each components.
    #[inline]
    pub fn into_parts(self) -> (bool, Authority, Path) {
        (self.is_https, self.authority, self.path)
    }
}

// ===== Formatting =====

macro_rules! delegate_fmt {
    ($($ty:ty),*) => {
        $(
            impl std::fmt::Debug for $ty {
                #[inline]
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.as_str().fmt(f)
                }
            }

            impl std::fmt::Display for $ty {
                #[inline]
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.as_str().fmt(f)
                }
            }
        )*
    };
    () => {}
}

delegate_fmt!(Scheme, HttpScheme, Authority, Host, Path);
