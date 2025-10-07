use super::{Authority, Bytes, HttpUri, Path, Scheme, Uri, UriError, matches};

impl Scheme {
    /// Parse scheme from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid scheme.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_scheme(bytes) {
            Ok(()) => Self { value: Bytes::from_static(bytes) },
            Err(err) => err.panic_const(),
        }
    }

    /// Parse scheme from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid scheme.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        match validate_scheme(value.as_slice()) {
            Ok(()) => Ok(Self { value }),
            Err(err) => Err(err),
        }
    }

    /// Parse scheme by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid scheme.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match validate_scheme(bytes.as_ref()) {
            Ok(()) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
            }),
            Err(err) => Err(err),
        }
    }
}

impl Authority {
    /// Parse authority from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid authority.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_authority(bytes) {
            Ok(()) => Self { value: Bytes::from_static(bytes) },
            Err(err) => err.panic_const(),
        }
    }

    /// Parse authority from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid authority.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        match validate_authority(value.as_slice()) {
            Ok(()) => Ok(Self { value }),
            Err(err) => Err(err),
        }
    }

    /// Parse authority by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid authority.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match validate_authority(bytes.as_ref()) {
            Ok(()) => Ok(Self { value: Bytes::copy_from_slice(bytes.as_ref()) }),
            Err(err) => Err(err),
        }
    }
}

impl Path {
    /// Parse path from static bytes.
    ///
    /// Path fragment is trimmed.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid path.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_path(bytes) {
            Ok((query, f)) => Self {
                value: Bytes::from_static(unsafe { std::slice::from_raw_parts(bytes.as_ptr(), f) }),
                query,
            },
            Err(err) => err.panic_const(),
        }
    }

    /// Parse path from [`Bytes`].
    ///
    /// Path fragment is trimmed.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid path.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let mut bytes = bytes.into();
        let (query, f) = validate_path(bytes.as_slice())?;
        bytes.truncate(f);
        Ok(Self {
            value: bytes,
            query,
        })
    }

    /// Parse path by copying from slice of bytes.
    ///
    /// Path fragment is trimmed.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid path.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        let (query, f) = validate_path(bytes.as_ref())?;
        let mut bytes = Bytes::copy_from_slice(bytes.as_ref());
        bytes.truncate(f);
        Ok(Self {
            value: bytes,
            query,
        })
    }
}

impl Uri {
    /// Creates [`Uri`] from [`Scheme`], optionally [`Authority`], and [`Path`].
    #[inline]
    pub const fn from_parts(scheme: Scheme, authority: Option<Authority>, path: Path) -> Self {
        Self {
            scheme,
            authority,
            path,
        }
    }

    /// Parse URI from [`Bytes`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use tsue::uri::Uri;
    /// let http = Uri::from_bytes("http://example.com/users/all").unwrap();
    /// assert_eq!(http.host(), Some("example.com"));
    /// assert_eq!(http.path(), "/users/all");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid URI.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        parse_uri(bytes.into())
    }

    /// Parse URI by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid URI.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        parse_uri(Bytes::copy_from_slice(bytes.as_ref()))
    }
}

impl HttpUri {
    /// Creates [`HttpUri`] from https flag, [`Authority`], and [`Path`].
    #[inline]
    pub const fn from_parts(is_https: bool, authority: Authority, path: Path) -> Self {
        Self {
            is_https,
            authority,
            path,
        }
    }

    /// Parse HTTP URI from [`Bytes`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tsue::uri::HttpUri;
    /// let http = HttpUri::from_bytes("http://example.com/users/all").unwrap();
    /// assert_eq!(http.host(), "example.com");
    /// assert_eq!(http.path(), "/users/all");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid HTTP URI.
    #[inline]
    pub fn from_bytes(bytes: impl Into<Bytes>) -> Result<Self, UriError> {
        parse_http(bytes.into())
    }

    /// Parse HTTP URI by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid HTTP URI.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        parse_http(Bytes::copy_from_slice(bytes.as_ref()))
    }
}

// ===== Logic =====

const fn validate_scheme(mut bytes: &[u8]) -> Result<(), UriError> {
    if bytes.is_empty() {
        return Err(UriError::InvalidScheme);
    }
    while let [byte, rest @ ..] = bytes {
        if matches::is_scheme(*byte) {
            bytes = rest
        } else {
            return Err(UriError::InvalidScheme)
        }
    }
    Ok(())
}

const fn validate_authority(mut bytes: &[u8]) -> Result<(), UriError> {
    if bytes.is_empty() {
        return Ok(());
    }

    // userinfo
    if let Some((mut userinfo, host)) = matches::split_at_sign(bytes) {
        bytes = host;

        while let [byte, rest @ ..] = userinfo {
            if matches::is_userinfo(*byte) {
                userinfo = rest
            } else {
                return Err(UriError::InvalidAuthority);
            }
        }
    }

    // port
    if let Some((host, mut port)) = matches::split_port(bytes) {
        bytes = host;

        // port
        if port.len() > 5 {
            // add specific error ?
            return Err(UriError::InvalidAuthority);
        }
        while let [byte, rest @ ..] = port {
            if !byte.is_ascii_digit() {
                return Err(UriError::InvalidAuthority);
            } else {
                port = rest;
            }
        }
    }

    if bytes.is_empty() {
        return Ok(());
    }

    if !matches!(bytes.first(), Some(b'[')) {
        while let [byte, rest @ ..] = bytes {
            if matches::is_regname(*byte) {
                bytes = rest
            } else {
                return Err(UriError::InvalidAuthority);
            }
        }

        Ok(())
    } else if let [b'[', ip @ .., b']'] = bytes {
        if let [b'v' | b'V', lead, rest @ ..] = ip {
            if !lead.is_ascii_hexdigit() || rest.is_empty() {
                return Err(UriError::InvalidAuthority);
            }

            let mut ip = rest;

            while let [byte, rest @ ..] = ip {
                if byte.is_ascii_hexdigit() {
                    ip = rest;
                } else if *byte == b'.' {
                    ip = rest;
                    break;
                } else {
                    return Err(UriError::InvalidAuthority);
                }
            }

            while let [byte, rest @ ..] = ip {
                if matches::is_ipvfuture(*byte) {
                    ip = rest;
                } else {
                    return Err(UriError::InvalidAuthority);
                }
            }
        } else {
            // TODO: validate ipv6
            let mut ip = ip;
            while let [byte, rest @ ..] = ip {
                if matches::is_ipv6(*byte) {
                    ip = rest;
                } else {
                    return Err(UriError::InvalidAuthority);
                }
            }
        }

        Ok(())
    } else {
        Err(UriError::InvalidAuthority)
    }
}

const fn validate_path(mut bytes: &[u8]) -> Result<(u16, usize), UriError> {
    if bytes.is_empty() {
        return Ok((0, 0));
    }

    if bytes.len() > u16::MAX as usize {
        return Err(UriError::TooLong);
    }

    let mut query = bytes.len() as u16;
    let mut frag = bytes.len();

    while let [byte, rest @ ..] = bytes {
        if matches::is_path(*byte) {
            bytes = rest;
        } else if *byte == b'?' {
            bytes = rest;
            query = query - rest.len() as u16 - 1;
            break;
        } else if *byte == b'#' {
            frag = frag - rest.len() - 1;
            query = frag as u16;
            bytes = &[];
            break;
        } else {
            return Err(UriError::InvalidPath);
        }
    }

    while let [byte, rest @ ..] = bytes {
        if matches::is_query(*byte) {
            bytes = rest;
        } else if *byte == b'#' {
            frag = frag - rest.len() - 1;
            break;
        } else {
            return Err(UriError::InvalidPath);
        }
    }

    Ok((query, frag))
}

fn parse_uri(mut bytes: Bytes) -> Result<Uri, UriError> {
    let at = matches::match_scheme!(bytes.as_slice(); else {
        return Err(UriError::InvalidScheme)
    });
    let scheme = Scheme::from_bytes(bytes.split_to(at))?;

    bytes.advance(1);

    let authority = if bytes.starts_with(b"//") {
        bytes.advance(2);

        let authority = match matches::find_path_delim!(bytes.as_slice()) {
            Some(at) => bytes.split_to(at),
            None => std::mem::take(&mut bytes),
        };

        if !authority.is_empty() {
            Some(Authority::from_bytes(authority)?)
        } else {
            None
        }
    } else {
        None
    };

    let path = Path::from_bytes(bytes)?;

    Ok(Uri {
        scheme,
        authority,
        path,
    })
}

fn parse_http(mut bytes: Bytes) -> Result<HttpUri, UriError> {
    let is_https = if bytes.starts_with(b"http://") {
        false
    } else if bytes.starts_with(b"https://") {
        true
    } else {
        return Err(UriError::InvalidScheme);
    };

    bytes.advance(5 + 2 + is_https as usize);

    let authority = match matches::find_path_delim!(bytes.as_slice()) {
        Some(at) => bytes.split_to(at),
        None => std::mem::take(&mut bytes),
    };

    // > A sender MUST NOT generate an "http" URI with an empty host identifier.
    if authority.is_empty() {
        return Err(UriError::InvalidAuthority);
    }

    let authority = Authority::from_bytes(authority)?;

    let path = Path::from_slice(bytes)?;

    Ok(HttpUri {
        is_https,
        authority,
        path,
    })
}

