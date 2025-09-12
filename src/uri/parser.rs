use tcio::bytes::Cursor;

use super::{Authority, Path, Scheme, UriError, matches, Bytes};

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
    pub fn try_from(value: impl Into<Bytes>) -> Result<Self, UriError> {
        Self::try_from_shared(value.into())
    }

    fn try_from_shared(mut value: Bytes) -> Result<Self, UriError> {
        let query = matches::match_query! {
            value;
            |val, cursor| match val {
                b'?' => {
                    let query = cursor.steps();
                    matches::match_fragment! {
                        cursor;
                        |val| match val {
                            b'#' => cursor.truncate_buf(),
                            _ => return Err(UriError::Char)
                        };
                    }
                    query
                },
                b'#' => {
                    cursor.truncate_buf();
                    value.len()
                }
                _ => return Err(UriError::Char)
            };
            else {
                value.len()
            }
        };
        Ok(Self {
            value,
            query: query as u16,
        })
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

const fn validate_authority(bytes: &[u8]) -> Result<(), UriError> {
    if bytes.is_empty() {
        return Ok(())
    }

    let mut cursor = Cursor::new(bytes);

    // userinfo
    matches::find_at! {
        Some(cursor) => '_foo: {
            let mut userinfo = Cursor::new(cursor.advanced_slice().split_last().unwrap().1);
            while let Some(byte) = userinfo.next() {
                if !matches::is_userinfo(byte) {
                    return Err(UriError::Char)
                }
            }
        },
        None => {},
    }

    {
        // port
        let mut port = Cursor::new(cursor.as_slice());
        matches::find_col! {
            match {
                Some(port) => '_foo: {
                    cursor = Cursor::new(port.advanced_slice());

                    if !matches!(port.next(), Some(b':')){
                        return Err(UriError::Char)
                    }

                    if port.remaining() > 5 {
                        // add specific error ?
                        return Err(UriError::Char)
                    }
                    while let Some(byte) = port.next() {
                        if !byte.is_ascii_digit() {
                            return Err(UriError::Char)
                        }
                    }
                },
                None => cursor = Cursor::new(port.original()),
            }
        };
    }

    // host
    match cursor.peek() {
        Some(b'[') => {
            let [_, ip @ .., b']'] = cursor.as_slice() else {
                return Err(UriError::Char)
            };
            if let [b'v', _ip] = ip {
                todo!()// ip-future
            } else {
                todo!()// ipv6
            }
        },
        Some(_) => while let Some(byte) = cursor.next() {
            if !matches::is_regname(byte) {
                return Err(UriError::Char)
            }
        },
        None => { },
    }

    Ok(())
}
