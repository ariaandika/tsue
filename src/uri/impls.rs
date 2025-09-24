use std::slice::from_raw_parts;

use super::{Authority, Path, Scheme, Uri, HttpUri, matches};

impl Scheme {
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

impl Authority {
    const fn split_at(&self) -> Option<(&[u8], &[u8])> {
        matches::split_at_sign(self.value.as_slice())
    }

    const fn split_col(&self) -> Option<(&[u8], &[u8])> {
        let host = match self.split_at() {
            Some((_, host)) => host,
            None => self.value.as_slice(),
        };
        matches::split_port(host)
    }

    /// Returns the authority host.
    #[inline]
    pub const fn host(&self) -> &str {
        match self.split_at() {
            Some((_, host)) => unsafe { str::from_utf8_unchecked(host) },
            None => self.as_str(),
        }
    }

    /// Returns the authority hostname.
    #[inline]
    pub const fn hostname(&self) -> &str {
        let hostname = match self.split_col() {
            Some((ok, _)) => ok,
            None => self.value.as_slice(),
        };
        unsafe { str::from_utf8_unchecked(hostname) }
    }

    /// Returns the authority port.
    #[inline]
    pub const fn port(&self) -> Option<u16> {
        match self.split_col() {
            // with port validation in constructor, should we do unsafe calculation ?
            Some((_, port)) => match tcio::atou(port) {
                Some(ok) => Some(ok as u16),
                None => None,
            }
            None => None,
        }
    }

    /// Returns the authority userinfo.
    #[inline]
    pub const fn userinfo(&self) -> Option<&str> {
        match self.split_at() {
            Some((userinfo, _)) => unsafe {
                Some(str::from_utf8_unchecked(userinfo))
            },
            None => None,
        }
    }

    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

impl Path {
    /// Returns the path as `str`, e.g: `/over/there`.
    #[inline]
    pub const fn path(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII, `query` is less than or equal to `value`
        // length
        unsafe {
            str::from_utf8_unchecked(from_raw_parts(self.value.as_ptr(), self.query as usize))
        }
    }

    /// Returns the query as `str`, e.g: `name=joe&query=4`.
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        if self.query as usize == self.value.len() {
            None
        } else {
            // SAFETY: precondition `value` is valid ASCII
            unsafe {
                let query = self.query as usize;
                Some(str::from_utf8_unchecked(from_raw_parts(
                    self.value.as_ptr().add(query.unchecked_add(1)),
                    self.value.len().unchecked_sub(query),
                )))
            }
        }
    }

    /// Returns the path and query as `str`, e.g: `/over/there?name=joe&query=4`.
    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.as_str()
    }

    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

impl Uri {
    #[inline]
    pub const fn from_parts(scheme: Scheme, authority: Option<Authority>, path: Path) -> Self {
        Self {
            scheme,
            authority,
            path,
        }
    }

    #[inline]
    pub const fn scheme(&self) -> &str {
        self.scheme.as_str()
    }

    #[inline]
    pub const fn as_scheme(&self) -> &Scheme {
        &self.scheme
    }

    #[inline]
    pub const fn authority(&self) -> Option<&str> {
        match &self.authority {
            Some(auth) => Some(auth.as_str()),
            None => None,
        }
    }

    #[inline]
    pub const fn as_authority(&self) -> Option<&Authority> {
        self.authority.as_ref()
    }

    #[inline]
    pub const fn path(&self) -> &str {
        self.path.path()
    }

    #[inline]
    pub const fn query(&self) -> Option<&str> {
        self.path.query()
    }

    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.path.as_str()
    }
}

impl HttpUri {
    #[inline]
    pub const fn from_parts(is_https: bool, authority: Authority, path: Path) -> Self {
        Self {
            is_https,
            authority,
            path,
        }
    }

    #[inline]
    pub const fn is_https(&self) -> bool {
        self.is_https
    }

    #[inline]
    pub const fn authority(&self) -> &str {
        self.authority.as_str()
    }

    #[inline]
    pub const fn as_authority(&self) -> &Authority {
        &self.authority
    }

    #[inline]
    pub const fn host(&self) -> &str {
        self.authority.host()
    }

    #[inline]
    pub const fn path(&self) -> &str {
        self.path.path()
    }

    #[inline]
    pub const fn query(&self) -> Option<&str> {
        self.path.query()
    }

    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.path.as_str()
    }

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
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.as_str().fmt(f)
                }
            }

            impl std::fmt::Display for $ty {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.as_str().fmt(f)
                }
            }
        )*
    };
    () => {}
}

delegate_fmt! {
    Scheme,
    Authority,
    Path
}
