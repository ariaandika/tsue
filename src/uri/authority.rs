use tcio::bytes::Bytes;

use crate::uri::path;
use crate::uri::{UriError, matches};

/// URI Authority.
///
/// The [authority] component of a URI.
///
/// [authority]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2>
///
/// ```not_rust
/// foo://username@example.com:8042/over/there?name=ferret
///       \_______________________/
///                   |
///               authority
/// ```
///
/// Authority contains a host, and optional userinfo and port.
///
/// ```not_rust
/// username@example.com:8042
/// \______/ \_________/ \__/
///    |          |       |
/// userinfo    host     port
/// ```
///
/// # Example
///
/// To create `Authority` use one of the `Authority::from_*` method:
///
/// ```
/// use tsue::uri::Authority;
/// let authority = Authority::from_bytes("username@example.com:8042").unwrap();
/// assert_eq!(authority.userinfo(), Some("username"));
/// assert_eq!(authority.host(), "example.com");
/// assert_eq!(authority.port(), Some(8042));
/// ```
#[derive(Clone)]
pub struct Authority {
    /// is valid ASCII
    value: Bytes,
}

impl Authority {
    /// Validate authority from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid authority.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_authority(bytes) {
            Some([]) => Self {
                value: Bytes::from_static(bytes),
            },
            _ => UriError::InvalidAuthority.panic_const()
        }
    }

    /// Validate authority from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid authority.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        match validate_authority(value.as_slice()) {
            Some([]) => Ok(Self { value }),
            _ => Err(UriError::InvalidAuthority)
        }
    }

    /// Validate authority by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid authority.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match validate_authority(bytes.as_ref()) {
            Some([]) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
            }),
            _ => Err(UriError::InvalidAuthority),
        }
    }
}

impl Authority {
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
        match matches::split_at_sign(self.value.as_slice()) {
            Some((userinfo, _)) => unsafe {
                Some(str::from_utf8_unchecked(userinfo))
            },
            None => None,
        }
    }

    /// Returns the authority host.
    ///
    /// ```not_rust
    /// user:pass@example.com:8042
    ///           \_________/
    ///                |
    ///              host
    /// ```
    #[inline]
    pub const fn host(&self) -> &str {
        let host_port = match matches::split_at_sign(self.value.as_slice()) {
            Some((_, suffix)) => suffix,
            None => self.value.as_slice(),
        };
        let host = match matches::split_port(host_port) {
            Some((prefix, _)) => prefix,
            None => host_port,
        };
        unsafe { str::from_utf8_unchecked(host) }
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
        let host_port = match matches::split_at_sign(self.value.as_slice()) {
            Some((_, suffix)) => suffix,
            None => self.value.as_slice(),
        };
        match matches::split_port(host_port) {
            Some((_, port)) => Some(matches::atou(port)),
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

// ===== Host =====

/// URI Host.
///
/// The [host] component of a URI.
///
/// [host]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2.2>
///
/// Host can be a domain name or an ip address.
///
/// ```not_rust
/// foo://username@example.com:8042/over/there?name=ferret
///                \_________/
///                     |
///                   host
/// ```
///
/// # Example
///
/// To create `Host` use one of the `Host::from_*` method:
///
/// ```
/// use tsue::uri::Host;
/// let host = Host::from_bytes("example.com").unwrap();
/// assert_eq!(host.as_str(), "example.com");
/// ```
#[derive(Clone)]
pub struct Host {
    /// is valid ASCII
    value: Bytes,
}

impl Host {
    /// Validate host from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid host.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_host(bytes) {
            Some([]) => Self { value: Bytes::from_static(bytes) },
            _ => UriError::InvalidAuthority.panic_const(),
        }
    }

    /// Validate host from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid host.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        match validate_host(value.as_slice()) {
            Some([]) => Ok(Self { value }),
            _ => Err(UriError::InvalidAuthority),
        }
    }

    /// Validate host by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid host.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match validate_host(bytes.as_ref()) {
            Some([]) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
            }),
            _ => Err(UriError::InvalidAuthority),
        }
    }
}

impl Host {
    /// Extracts a string slice containing the host.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

// ===== std traits =====

impl std::fmt::Debug for Authority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Authority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

// ===== Validation =====

/// `authority = [ userinfo "@" ] host [ ":" port ]`
const fn validate_authority(bytes: &[u8]) -> Option<&[u8]> {
    let [prefix, ..] = bytes else {
        return Some(bytes);
    };

    let mut state = bytes;

    if *prefix == b'[' {
        let Some(rest) = validate_ip_literal(bytes) else {
            return None;
        };
        state = rest;
    } else {
        // fast path for empty authority in hier-part
        if path::is_path_delim(*prefix) {
            return Some(bytes);
        }

        // userinfo or host
        //
        // userinfo    = *( unreserved / pct-encoded / sub-delims / ":" )
        // IPv4address = dec-octet "." dec-octet "." dec-octet "." dec-octet
        // reg-name    = *( unreserved / pct-encoded / sub-delims )
        let mut delim = loop {
            let [byte, rest @ ..] = state else {
                // host only
                return Some(state);
            };
            state = rest;
            if !is_regname(*byte) {
                break *byte;
            }
        };

        let mut is_port_ok = true;

        while delim == b':' {
            // userinfo or port delimiter
            loop {
                let [byte, rest @ ..] = state else {
                    // without userinfo, with port
                    return if is_port_ok {
                        Some(state)
                    } else {
                        None
                    }
                };

                is_port_ok &= byte.is_ascii_digit();
                state = rest;

                if !is_regname(*byte) {
                    delim = *byte;
                    break;
                }
            }

            // port delimiter can only appear once
            is_port_ok = false;
        }

        if delim != b'@' {
            // host only, followed by other component
            return Some(state)
        }

        if let [prefix, ..] = state && *prefix == b'[' {
            match validate_ip_literal(state) {
                Some(rest) => state = rest,
                None => return None,
            }
        } else {
            loop {
                let [byte, rest @ ..] = state else {
                    // with userinfo, without port
                    return Some(state);
                };
                if !is_regname(*byte) {
                    break;
                }
                state = rest;
            }
        }
    }

    let Some((delim, mut state)) = state.split_first() else {
        return Some(state);
    };

    if *delim != b':' {
        // with userinfo, without port, followed by other component
        return Some(state)
    }

    loop {
        let [digit, rest @ ..] = state else {
            // with userinfo and port
            return Some(state);
        };
        if !digit.is_ascii_digit() {
            // with userinfo and port, followed by other component
            return Some(state);
        }
        state = rest;
    }
}

matches::ascii_lookup_table! {
    /// `reg-name = *( unreserved / pct-encoded / sub-delims )`
    const fn is_regname(byte: u8) -> bool {
        matches::unreserved(byte)
        || matches::pct_encoded(byte)
        || matches::sub_delims(byte)
    }
}

/// ```not_rust
/// host          = IP-literal / IPv4address / reg-name
/// IP-literal    = "[" ( IPv6address / IPvFuture  ) "]"
/// IPv4address   = dec-octet "." dec-octet "." dec-octet "." dec-octet
/// reg-name      = *( unreserved / pct-encoded / sub-delims )
///
/// dec-octet     = DIGIT                 ; 0-9
///               / %x31-39 DIGIT         ; 10-99
///               / "1" 2DIGIT            ; 100-199
///               / "2" %x30-34 DIGIT     ; 200-249
///               / "25" %x30-35          ; 250-255
/// ```
const fn validate_host(bytes: &[u8]) -> Option<&[u8]> {
    let Some((prefix, mut state)) = bytes.split_first() else {
        return Some(bytes);
    };
    if *prefix == b'[' {
        return validate_ip_literal(bytes)
    }
    loop {
        let [byte, rest @ ..] = state else {
            return Some(state);
        };
        if !is_regname(*byte) {
            return Some(state);
        }
        state = rest;
    }
}

matches::ascii_lookup_table! {
    /// `hex / ":" / "."`
    const fn is_ipv6(byte: u8) -> bool {
        byte.is_ascii_hexdigit()
        || matches!(byte, b':' | b'.')
    }
}

matches::ascii_lookup_table! {
    /// `IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )`
    const fn is_ipvfuture(byte: u8) -> bool {
        matches::unreserved(byte)
        || matches::sub_delims(byte)
        || matches!(byte, b':')
    }
}

/// ```not_rust
/// IP-literal    = "[" ( IPv6address / IPvFuture  ) "]"
/// IPv6address   =                            6( h16 ":" ) ls32
///               /                       "::" 5( h16 ":" ) ls32
///               / [               h16 ] "::" 4( h16 ":" ) ls32
///               / [ *1( h16 ":" ) h16 ] "::" 3( h16 ":" ) ls32
///               / [ *2( h16 ":" ) h16 ] "::" 2( h16 ":" ) ls32
///               / [ *3( h16 ":" ) h16 ] "::"    h16 ":"   ls32
///               / [ *4( h16 ":" ) h16 ] "::"              ls32
///               / [ *5( h16 ":" ) h16 ] "::"              h16
///               / [ *6( h16 ":" ) h16 ] "::"
/// IPvFuture     = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
///
/// h16           = 1*4HEXDIG
/// ls32          = ( h16 ":" h16 ) / IPv4address
/// IPv4address   = dec-octet "." dec-octet "." dec-octet "." dec-octet
/// dec-octet     = DIGIT                 ; 0-9
///               / %x31-39 DIGIT         ; 10-99
///               / "1" 2DIGIT            ; 100-199
///               / "2" %x30-34 DIGIT     ; 200-249
///               / "25" %x30-35          ; 250-255
/// ```
const fn validate_ip_literal(bytes: &[u8]) -> Option<&[u8]> {
    let Some((b'[', mut state)) = bytes.split_first() else {
        unreachable!()
    };

    // IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
    let close_delim = if let [b'v', hexdig1, rest @ ..] = state {
        if !hexdig1.is_ascii_hexdigit() || rest.is_empty() {
            return None;
        }
        state = rest;

        while let [byte, rest @ ..] = state {
            state = rest;

            if !byte.is_ascii_hexdigit() {
                if *byte != b'.' {
                    return None;
                }
                break;
            }
        }

        if state.is_empty() {
            return None;
        }

        loop {
            let [byte, rest @ ..] = state else {
                return None;
            };
            state = rest;
            if !is_ipvfuture(*byte) {
                break *byte;
            }
        }
    } else {
        // FEAT: validate ipv6
        loop {
            let [byte, rest @ ..] = state else {
                return None;
            };
            state = rest;
            if !is_ipv6(*byte) {
                break *byte;
            }
        }
    };

    if close_delim == b']' {
        Some(state)
    } else {
        None
    }
}
