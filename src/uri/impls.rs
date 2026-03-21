use std::slice::from_raw_parts;

use super::{Authority, Host, Path, Scheme, Uri};

impl Scheme {
    /// Extracts a string slice containing the scheme.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }

    /// Checks that two scheme are an ASCII case-insensitive match.
    #[inline]
    pub const fn eq_ignore_ascii_case(&self, scheme: &str) -> bool {
        // Although schemes are case-insensitive, the canonical form is lowercase and documents
        // that specify schemes must do so with lowercase letters.
        self.as_str().eq_ignore_ascii_case(scheme)
    }
}

impl Authority {
    const fn split_userinfo(&self) -> Option<(&[u8], &[u8])> {
        split_at_sign(self.value.as_slice())
    }

    const fn split_port(&self) -> Option<(&[u8], &[u8])> {
        let host = match self.split_userinfo() {
            Some((_, host)) => host,
            None => self.value.as_slice(),
        };
        split_port(host)
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
        let hostname = match split_port(host) {
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
            Some((_, port)) => Some(atou(port)),
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
        split_port(self.value.as_slice())
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
            Some((_, port)) => Some(atou(port)),
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

delegate_fmt!(Scheme, Authority, Host, Path);

// ===== FromStr =====

macro_rules! impl_from_str {
    ($($ty:ty),*) => {$(
        impl std::str::FromStr for $ty {
            type Err = super::UriError;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::from_slice(s.as_bytes())
            }
        }
    )*};
}

impl_from_str!(Scheme, Authority, Host, Path, Uri);

// ===== Util =====

const fn atou(mut bytes: &[u8]) -> u16 {
    let mut o = 0;
    while let [lead, rest @ ..] = bytes {
        o *= 10;
        o += lead.wrapping_sub(b'0') as u16;
        bytes = rest;
    }
    o
}

/// Split '@'.
pub const fn split_at_sign(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    const BLOCK: usize = size_of::<usize>();
    const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
    const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
    const AT: usize = usize::from_ne_bytes([b'@'; BLOCK]);

    let mut state: &[u8] = bytes;

    while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);

        // '@'
        let is_at = (block ^ AT).wrapping_sub(LSB);

        let result = is_at & MSB;
        if result != 0 {
            let nth = (result.trailing_zeros() / 8) as usize;
            unsafe {
                let nth_ptr = state.as_ptr().add(nth);
                let end_ptr = bytes.as_ptr().add(bytes.len());

                let start = bytes.as_ptr();
                let start_len = nth_ptr.offset_from_unsigned(start);

                // skip the '@'
                let end = nth_ptr.add(1);
                let end_len = end_ptr.offset_from_unsigned(end);

                return Some((
                    from_raw_parts(start, start_len),
                    from_raw_parts(end, end_len),
                ))
            };
        }

        state = rest;
    }

    while let [byte, rest @ ..] = state {
        if *byte == b'@' {
            let start = bytes.as_ptr();
            let lead = unsafe {
                from_raw_parts(start,state.as_ptr().offset_from_unsigned(start))
            };
            return Some((lead,rest));
        }

        state = rest;
    }

    None
}

#[test]
fn test_split_at_sign() {
    assert!(split_at_sign(b"example.com").is_none());

    let (left, right) = split_at_sign(b"user:passwd@example.com").unwrap();
    assert_eq!(left, b"user:passwd");
    assert_eq!(right, b"example.com");

    let (left, right) = split_at_sign(b"a@b").unwrap();
    assert_eq!(left, b"a");
    assert_eq!(right, b"b");

    let (left, right) = split_at_sign(b"user:passwd@b").unwrap();
    assert_eq!(left, b"user:passwd");
    assert_eq!(right, b"b");
}

/// Split ':'.
const fn split_port(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut state: &[u8] = bytes;

    while let [lead @ .., byte] = state {
        if !byte.is_ascii_digit() {
            if *byte == b':' {
                unsafe {
                    let mid_ptr = bytes.as_ptr().add(state.len());
                    let len = bytes.len() - state.len();
                    return Some((lead, from_raw_parts(mid_ptr, len)));
                };
            } else {
                return None;
            }
        }
        state = lead;
    }

    None
}

#[test]
fn test_split_port() {
    assert!(split_port(b"example.com").is_none());
    assert!(split_port(b"[a2f::1]").is_none());

    let (left, right) = split_port(b"example.com:443").unwrap();
    assert_eq!(left, b"example.com");
    assert_eq!(right, b"443");

    let (left, right) = split_port(b"[a2f::1]:443").unwrap();
    assert_eq!(left, b"[a2f::1]");
    assert_eq!(right, b"443");
}
