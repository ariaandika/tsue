use tcio::bytes::Bytes;

use crate::uri::{UriError, authority};

/// HTTP `Host` header or `:authority` pseudo-header.
///
/// `Host` contains [host] and optional [port] component from URI.
///
/// Note that this is different from the URI [`uri::Host`] struct.
///
/// [`uri::Host`]: crate::uri::Host
/// [host]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2.2>
/// [port]: <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2.3>
///
/// # Example
///
/// To create `Host` use one of the `Host::from_*` method:
///
/// ```
/// use tsue::http::Host;
/// let host = Host::from_bytes("example.com:80").unwrap();
/// assert_eq!(host.as_str(), "example.com:80");
/// assert_eq!(host.host(), "example.com");
/// assert_eq!(host.port(), Some("80"));
/// ```
#[derive(Clone)]
pub struct Host {
    /// is valid ASCII
    value: Bytes,
    port: u32,
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
            Ok(port) => Self {
                value: Bytes::from_static(bytes),
                port,
            },
            Err(err) => err.panic_const(),
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
            Ok(port) => Ok(Self { value, port }),
            Err(err) => Err(err),
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
            Ok(port) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
                port,
            }),
            Err(err) => Err(err),
        }
    }
}

impl Host {
    /// Returns the host and port as string.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }

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
        if self.port as usize != self.value.len() {
            // SAFETY: precondition `value` is valid ASCII
            unsafe {
                let offset = (self.port + 1) as usize;
                Some(str_from_parts!(
                    self.value.as_ptr().add(offset),
                    self.value.len() - offset
                ))
            }
        } else {
            None
        }
    }
}

// ===== std traits =====

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

macro_rules! str_from_parts {
    ($d:expr, $l:expr) => {
        str::from_utf8_unchecked(std::slice::from_raw_parts($d, $l))
    };
}

use str_from_parts;

/// ```not_rust
/// http-host   = host [ ":" port ]
/// host        = IP-literal / IPv4address / reg-name
/// ```
const fn validate_host(bytes: &[u8]) -> Result<u32, UriError> {
    if bytes.len() > u16::MAX as usize {
        return Err(UriError::ExcessiveBytes);
    }
    let Some(rest) = authority::validate_host(bytes) else {
        return Err(UriError::InvalidHost);
    };
    let Some((delim, mut port)) = rest.split_first() else {
        return Ok(bytes.len() as u32);
    };
    if *delim != b':' {
        return Err(UriError::InvalidHost);
    }
    loop {
        let Some((digit, rest)) = port.split_first() else {
            return unsafe {
                Ok((delim as *const u8).offset_from_unsigned(bytes.as_ptr()) as u32)
            }
        };
        if !digit.is_ascii_digit() {
            return Err(UriError::InvalidPort);
        }
        port = rest;
    }
}
