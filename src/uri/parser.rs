use super::{Authority, Path, Scheme, HttpUri, UriError, matches, Bytes};

impl Scheme {
    /// Parse scheme from static slice.
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
    pub fn parse_from<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        match validate_scheme(value.as_slice()) {
            Ok(()) => Ok(Self { value }),
            Err(err) => Err(err),
        }
    }

    /// Parse scheme by copying from slice reference.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid scheme.
    #[inline]
    pub fn parse<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match validate_scheme(bytes.as_ref()) {
            Ok(()) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
            }),
            Err(err) => Err(err),
        }
    }
}

impl Authority {
    /// Parse authority from static slice.
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
    pub fn parse_from<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        match validate_authority(value.as_slice()) {
            Ok(()) => Ok(Self { value }),
            Err(err) => Err(err),
        }
    }

    /// Parse authority by copying from slice reference.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid authority.
    #[inline]
    pub fn parse<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match validate_authority(bytes.as_ref()) {
            Ok(()) => Ok(Self { value: Bytes::copy_from_slice(bytes.as_ref()) }),
            Err(err) => Err(err),
        }
    }
}

impl Path {
    /// Parse path from static slice.
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
    pub fn parse_from<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let mut bytes = bytes.into();
        let (query, f) = validate_path(bytes.as_slice())?;
        bytes.truncate(f);
        Ok(Self {
            value: bytes,
            query,
        })
    }

    /// Parse path by copying from slice reference.
    ///
    /// Path fragment is trimmed.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid path.
    #[inline]
    pub fn parse<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        let (query, f) = validate_path(bytes.as_ref())?;
        let mut bytes = Bytes::copy_from_slice(bytes.as_ref());
        bytes.truncate(f);
        Ok(Self {
            value: bytes,
            query,
        })
    }
}

impl HttpUri {
    // Parse HTTP URI from [`Bytes`].
    //
    // # Examples
    //
    // ```
    // # use tsue::uri::HttpUri;
    // # use tcio::bytes::Bytes;
    // let bytes = Bytes::from_static(b"http://example.com/users/all");
    // let http = HttpUri::parse_from(bytes).unwrap();
    // assert_eq!(http.host(), "example.com");
    // assert_eq!(http.path(), "/users/all");
    // ```
    #[inline]
    pub fn parse_from(bytes: impl Into<Bytes>) -> Result<Self, UriError> {
        parse_http(bytes.into())
    }

    /// Parse HTTP URI by copying from slice.
    ///
    /// If the input is owned [`Bytes`], consider using [`HttpUri::parse_from`].
    #[inline]
    pub fn parse<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        parse_http(Bytes::copy_from_slice(bytes.as_ref()))
    }
}

// ===== Logic =====

const fn validate_scheme(mut bytes: &[u8]) -> Result<(), UriError> {
    if bytes.is_empty() {
        return Err(UriError::Incomplete);
    }
    while let [byte, rest @ ..] = bytes {
        if matches::is_scheme(*byte) {
            bytes = rest
        } else {
            return Err(UriError::Char)
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
                return Err(UriError::Char);
            }
        }
    }

    // port
    if let Some((host, mut port)) = matches::split_port(bytes) {
        bytes = host;

        // port
        if port.len() > 5 {
            // add specific error ?
            return Err(UriError::Char);
        }
        while let [byte, rest @ ..] = port {
            if !byte.is_ascii_digit() {
                return Err(UriError::Char);
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
                return Err(UriError::Char);
            }
        }

        Ok(())
    } else if let [b'[', ip @ .., b']'] = bytes {
        if let [b'v' | b'V', lead, rest @ ..] = ip {
            if !lead.is_ascii_hexdigit() || rest.is_empty() {
                return Err(UriError::Char);
            }

            let mut ip = rest;

            while let [byte, rest @ ..] = ip {
                if byte.is_ascii_hexdigit() {
                    ip = rest;
                } else if *byte == b'.' {
                    ip = rest;
                    break;
                } else {
                    return Err(UriError::Char);
                }
            }

            while let [byte, rest @ ..] = ip {
                if matches::is_ipvfuture(*byte) {
                    ip = rest;
                } else {
                    return Err(UriError::Char);
                }
            }
        } else {
            // TODO: validate ipv6
            let mut ip = ip;
            while let [byte, rest @ ..] = ip {
                if matches::is_ipv6(*byte) {
                    ip = rest;
                } else {
                    return Err(UriError::Char);
                }
            }
        }

        Ok(())
    } else {
        Err(UriError::Char)
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
            return Err(UriError::Char);
        }
    }

    while let [byte, rest @ ..] = bytes {
        if matches::is_query(*byte) {
            bytes = rest;
        } else if *byte == b'#' {
            frag = frag - rest.len() - 1;
            break;
        } else {
            return Err(UriError::Char);
        }
    }

    Ok((query, frag))
}

fn parse_http(mut value: Bytes) -> Result<HttpUri, UriError> {
    const SCHEME_HTTP: bool = false;
    const SCHEME_HTTPS: bool = true;

    let (is_https, bytes) = match value.as_slice().split_first_chunk::<5>() {
        Some((b"http:", rest)) => {
            let Some((b"//", rest)) = rest.split_first_chunk() else {
                return Err(UriError::Char)
            };
            (SCHEME_HTTP, rest)
        },
        Some((b"https", rest)) => {
            let Some((b"://", rest)) = rest.split_first_chunk() else {
                return Err(UriError::Char)
            };
            (SCHEME_HTTPS, rest)
        },
        _ => return Err(UriError::Char),
    };

    let authority = match matches::find_path_delim!(bytes) {
        Some(ok) => {
            value.slice_ref(ok)
        },
        None => std::mem::take(&mut value),
    };
    let authority = Authority::parse_from(authority)?;

    let path = Path::parse(value)?;

    Ok(HttpUri {
        is_https,
        authority,
        path,
    })
}

