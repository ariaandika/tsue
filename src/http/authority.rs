use tcio::bytes::Bytes;

use crate::http::error::UriError;
use crate::matches;

/// HTTP Authority.
///
/// In `HTTP/1.1`, this is the value of the `Host` header.
///
/// In `HTTP/2.0`, this is the value of the `:authority` pseudo-header.
///
/// `Authority` contains [host] and optional [port] component.
///
/// [host]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2.2>
/// [port]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2.3>
///
/// # Example
///
/// To create `Authority` use one of the `Authority::from_*` method:
///
/// ```
/// use tsue::http::Authority;
/// let auth = Authority::from_bytes("example.com:80").unwrap();
/// assert_eq!(auth.as_str(), "example.com:80");
/// assert_eq!(auth.host(), "example.com");
/// assert_eq!(auth.port(), Some("80"));
/// ```
#[derive(Clone)]
pub struct Authority {
    /// `uri-host [ ":" port ] ; Section 4`
    value: Bytes,
    port: u32,
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
            Ok(port) => Self {
                value: Bytes::from_static(bytes),
                port,
            },
            Err(err) => err.panic_const(),
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
            Ok(port) => Ok(Self { value, port }),
            Err(err) => Err(err),
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
            Ok(port) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
                port,
            }),
            Err(err) => Err(err),
        }
    }
}

impl Authority {
    /// Returns the host component.
    ///
    /// ```not_rust
    /// example.com:8042
    /// \_________/
    ///      |
    ///    host
    /// ```
    #[inline]
    pub const fn host(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str_from_parts!(self.value.as_ptr(), self.port as usize) }
    }

    /// Returns the port component if exists.
    ///
    /// ```not_rust
    /// example.com:8042
    ///             \__/
    ///              |
    ///             port
    /// ```
    #[inline]
    pub const fn port(&self) -> Option<&str> {
        let offset = (self.port + 1) as usize;
        if offset < self.value.len() {
            // SAFETY: precondition `value` is valid ASCII
            unsafe {
                Some(str_from_parts!(
                    self.value.as_ptr().add(offset),
                    self.value.len() - offset
                ))
            }
        } else {
            None
        }
    }

    /// Returns the authority as string.
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

// ===== Validation =====

macro_rules! str_from_parts {
    ($d:expr, $l:expr) => {
        str::from_utf8_unchecked(std::slice::from_raw_parts($d, $l))
    };
}

use str_from_parts;

const fn validate_authority(mut bytes: &[u8]) -> Result<u32, UriError> {
    match match_authority(&mut bytes) {
        Ok(port) => {
            if bytes.is_empty() {
                Ok(port)
            } else {
                Err(UriError::InvalidAuthority)
            }
        }
        Err(err) => Err(err),
    }
}

/// A sender MUST NOT generate an "http" URI with an empty host identifier
///
/// ```not_rust
/// Host          = uri-host [ ":" port ] ; Section 4
/// uri-host      = IP-literal / IPv4address / reg-name
/// IP-literal    = "[" ( IPv6address / IPvFuture  ) "]"
/// IPv4address   = dec-octet "." dec-octet "." dec-octet "." dec-octet
/// IPvFuture     = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
/// reg-name      = *( unreserved / pct-encoded / sub-delims )
///
/// dec-octet     = DIGIT                 ; 0-9
///               / %x31-39 DIGIT         ; 10-99
///               / "1" 2DIGIT            ; 100-199
///               / "2" %x30-34 DIGIT     ; 200-249
///               / "25" %x30-35          ; 250-255
/// IPv6address   =                            6( h16 ":" ) ls32
///               /                       "::" 5( h16 ":" ) ls32
///               / [               h16 ] "::" 4( h16 ":" ) ls32
///               / [ *1( h16 ":" ) h16 ] "::" 3( h16 ":" ) ls32
///               / [ *2( h16 ":" ) h16 ] "::" 2( h16 ":" ) ls32
///               / [ *3( h16 ":" ) h16 ] "::"    h16 ":"   ls32
///               / [ *4( h16 ":" ) h16 ] "::"              ls32
///               / [ *5( h16 ":" ) h16 ] "::"              h16
///               / [ *6( h16 ":" ) h16 ] "::"
/// h16           = 1*4HEXDIG
/// ls32          = ( h16 ":" h16 ) / IPv4address
/// ```
pub(crate) const fn match_authority(bytes: &mut &[u8]) -> Result<u32, UriError> {
    if bytes.len() > u32::MAX as usize {
        return Err(UriError::ExcessiveBytes);
    }

    let base = bytes.as_ptr();

    // ===== host =====
    let Some((prefix, state)) = bytes.split_first() else {
        // empty host
        return Err(UriError::InvalidAuthority);
    };
    if *prefix != b'[' {
        loop {
            let [byte, rest @ ..] = bytes else {
                break;
            };
            if !is_regname(*byte) {
                break;
            }
            *bytes = rest;
        }
    } else {
        *bytes = state;
        loop {
            let [byte, rest @ ..] = bytes else {
                // unclosed ip-literal bracket
                return Err(UriError::InvalidAuthority);
            };
            *bytes = rest;
            if !(is_regname(*byte) | (*byte == b':')) {
                if *byte != b']' {
                    return Err(UriError::InvalidAuthority);
                }
                break;
            }
        }
    }
    let [delim, port @ ..] = bytes else {
        return unsafe {
            Ok(bytes.as_ptr().offset_from_unsigned(base) as u32)
        };
    };

    // ===== port =====

    if *delim != b':' {
        return unsafe {
            Ok((delim as *const u8).offset_from_unsigned(base) as u32)
        };
    }
    *bytes = port;
    loop {
        let Some((digit, rest)) = bytes.split_first() else {
            break;
        };
        if !digit.is_ascii_digit() {
            break;
        }
        *bytes = rest;
    }

    unsafe {
        Ok((delim as *const u8).offset_from_unsigned(base) as u32)
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

#[test]
pub fn test_authority() {
    let auth = Authority::from_slice("example.com").unwrap();
    assert_eq!(auth.as_str(), "example.com");
    assert_eq!(auth.host(), "example.com");
    assert_eq!(auth.port(), None);

    let auth = Authority::from_slice("example.com:443").unwrap();
    assert_eq!(auth.as_str(), "example.com:443");
    assert_eq!(auth.host(), "example.com");
    assert_eq!(auth.port(), Some("443"));

    // note that currently the exact syntax of ip address are not validated

    let auth = Authority::from_slice("[a2f::1]").unwrap();
    assert_eq!(auth.as_str(), "[a2f::1]",);
    assert_eq!(auth.host(), "[a2f::1]",);
    assert_eq!(auth.port(), None);

    let auth = Authority::from_slice("[a2f::1]:443").unwrap();
    assert_eq!(auth.as_str(), "[a2f::1]:443");
    assert_eq!(auth.host(), "[a2f::1]");
    assert_eq!(auth.port(), Some("443"));
}
