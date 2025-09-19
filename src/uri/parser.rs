use super::{Authority, Path, Scheme, HttpUri, UriError, matches, Bytes};

const SCHEME_HTTP: bool = false;
const SCHEME_HTTPS: bool = true;

impl Scheme {
    #[inline]
    pub const fn from_static(string: &'static str) -> Self {
        Self::from_shared(Bytes::from_static(string.as_bytes()))
    }

    #[inline]
    pub const fn from_shared(value: Bytes) -> Self {
        match validate_scheme(value.as_slice()) {
            Ok(()) => Self { value },
            Err(err) => err.panic_const(),
        }
    }

    #[inline]
    pub fn try_from(value: impl Into<Bytes>) -> Result<Self, UriError> {
        let value = value.into();
        match validate_scheme(value.as_slice()) {
            Ok(()) => Ok(Self { value }),
            Err(err) => Err(err),
        }
    }
}

impl Authority {
    /// Parse authority from static str.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid authority.
    #[inline]
    pub const fn from_static(string: &'static str) -> Self {
        Self::from_shared(Bytes::from_static(string.as_bytes()))
    }

    /// Parse authority from [`Bytes`].
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid authority.
    #[inline]
    pub const fn from_shared(value: Bytes) -> Self {
        match validate_authority(value.as_slice()) {
            Ok(()) => Self { value },
            Err(err) => err.panic_const(),
        }
    }

    /// Parse authority from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid authority.
    #[inline]
    pub fn try_from(value: impl Into<Bytes>) -> Result<Self, UriError> {
        let value = value.into();
        match validate_authority(value.as_slice()) {
            Ok(()) => Ok(Self { value }),
            Err(err) => Err(err),
        }
    }
}

impl Path {
    #[inline]
    pub const fn asterisk() -> Self {
        Self {
            value: Bytes::from_static(b"*"),
            query: 1,
        }
    }

    #[inline]
    pub const fn empty() -> Self {
        Self {
            value: Bytes::new(),
            query: 0,
        }
    }

    #[inline]
    pub const fn from_static(value: &'static str) -> Self {
        match validate_path(value.as_bytes()) {
            Ok((query, f)) => Self {
                value: Bytes::from_static(unsafe { std::slice::from_raw_parts(value.as_ptr(), f) }),
                query,
            },
            Err(err) => err.panic_const(),
        }
    }

    #[inline]
    pub fn try_from(value: impl Into<Bytes>) -> Result<Self, UriError> {
        Self::try_from_shared(value.into())
    }

    fn try_from_shared(mut value: Bytes) -> Result<Self, UriError> {
        let (query, f) = validate_path(value.as_slice())?;
        value.truncate(f);
        Ok(Self { value, query })
    }
}

impl HttpUri {
    pub fn parse_http(value: Bytes) -> Result<Self, UriError> {
        parse_http(value)
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
    if let Some((mut userinfo, host)) = matches::split_at_sign!(bytes) {
        let Some((b'@', host)) = host.split_first() else {
            return Err(UriError::Char);
        };

        bytes = host;

        while let [byte, rest @ ..] = userinfo {
            if matches::is_userinfo(*byte) {
                userinfo = rest
            } else {
                return Err(UriError::Char);
            }
        }
    }

    // host
    if let Some((mut host, port)) = matches::split_col!(bytes) {
        let Some((b':', port)) = port.split_first() else {
            return Err(UriError::Char);
        };

        bytes = port;

        match host {
            [b'[', ip @ .., b']'] => {
                if let [b'v' | b'V', lead, rest @ ..] = ip {
                    if !lead.is_ascii_hexdigit() || rest.is_empty() {
                        return Err(UriError::Char)
                    }

                    let mut ip = rest;

                    while let [byte, rest @ ..] = ip {
                        if byte.is_ascii_hexdigit() {
                            ip = rest;
                        } else if *byte == b'.' {
                            ip = rest;
                            break
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
            }
            [] => {}
            _ => {
                while let [byte, rest @ ..] = host {
                    if matches::is_regname(*byte) {
                        host = rest
                    } else {
                        return Err(UriError::Char);
                    }
                }
            }
        }
    }

    // port
    if !bytes.is_empty() {
        if bytes.len() > 5 {
            // add specific error ?
            return Err(UriError::Char);
        }
        while let [byte, rest @ ..] = bytes {
            if !byte.is_ascii_digit() {
                return Err(UriError::Char);
            } else {
                bytes = rest;
            }
        }
    }

    Ok(())
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
        Some(ok) => value.slice_ref(ok),
        None => std::mem::take(&mut value),
    };
    let authority = Authority::try_from(authority)?;

    let path = Path::try_from_shared(value)?;

    Ok(HttpUri {
        is_https,
        authority,
        path,
    })
}

