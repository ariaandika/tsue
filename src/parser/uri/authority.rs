use tcio::bytes::{ByteStr, Bytes};

use super::{simd, UriError};

#[derive(Debug, Clone)]
pub struct Authority {
    /// Preconditions:
    /// - if there is a ':', bytes after it is less than 5 valid digit
    value: Option<ByteStr>,
}

impl Authority {
    pub(crate) const fn none() -> Authority {
        Self {
            value: None,
        }
    }

    /// # Safety
    ///
    /// `host` must be valid ASCII
    pub(crate) unsafe fn new_unchecked(host: Bytes, port: u16) -> Self {
        Self {
            // SAFETY: ensured by caller
            value: Some(unsafe { ByteStr::from_utf8_unchecked(host) }),
        }
    }

    /// Construct an [`Authority`] from [`Bytes`].
    ///
    /// Input is validated for valid authority characters.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `value` contains invalid character.
    pub fn try_from_bytes(bytes: Bytes) -> Result<Self, UriError> {
        let mut cursor = bytes.cursor();
        let col = simd::validate_authority!(cursor);

        match cursor.peek() {
            // TODO: dedicated userinfo error
            Some(b'@') => return Err(UriError::Char),
            Some(_) => return Err(UriError::Char),
            None => {},
        }

        if let Some(col) = col {
            let mut cursor = bytes.cursor();
            cursor.advance(col);

            if cursor.remaining() > 5 {
                return Err(UriError::TooLong);
            }

            while let Some(byte) = cursor.next() {
                if !simd::is_digit(byte) {
                    return Err(UriError::Char)
                }
            }
        }

        Ok(Self {
            value: Some(unsafe { ByteStr::from_utf8_unchecked(bytes) }),
        })
    }

    // #[inline]
    // pub const fn host(&self) -> Option<&str> {
    //     match &self.value {
    //         Some(host) => Some(host.as_str()),
    //         None => None,
    //     }
    // }

    #[inline]
    pub const fn port(&self) -> Option<u16> {
        match self.value.as_ref() {
            Some(_value) => {
                // value.eq_ignore_ascii_case(other)
                // let mut cursor = Cursor::from_end(value.as_str().as_bytes());
                // simd::match_port_rev!(cursor else {
                //     return None
                // });
                // let mut port = 0;
                // while let Some(digit) = cursor.next() {
                //     port = (port * 10) + (digit - 48) as u16;
                // }
                // Some(port)
                todo!()
            },
            None => None,
        }
    }
}

impl PartialEq for Authority {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
