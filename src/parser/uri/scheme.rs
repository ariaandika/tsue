use tcio::bytes::{ByteStr, Bytes};

use super::{simd, InvalidUri};

#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    repr: Repr,
}

const NONE: u8  = 0b0000_0000;
const HTTP: u8  = 0b0000_0001;
const HTTPS: u8 = 0b0000_0010;

#[derive(Debug, Clone, PartialEq)]
enum Repr {
    Static(u8),
    Str(ByteStr),
}

impl Scheme {
    /// Construct a [`Scheme`] with no value.
    #[inline]
    pub const fn none() -> Scheme {
        Self {
            repr: Repr::Static(NONE),
        }
    }

    /// Construct a [`Scheme`] with value of `HTTP`.
    #[inline]
    pub const fn http() -> Scheme {
        Self {
            repr: Repr::Static(HTTP),
        }
    }

    /// Construct a [`Scheme`] with value of `HTTPS`.
    #[inline]
    pub const fn https() -> Scheme {
        Self {
            repr: Repr::Static(HTTPS),
        }
    }

    pub(crate) fn new_unvalidated(scheme: ByteStr) -> Scheme {
        match scheme.as_bytes() {
            b"http" => Self::http(),
            b"https" => Self::https(),
            _ => Self {
                repr: Repr::Str(scheme),
            },
        }
    }

    /// Construct a [`Scheme`] from [`Bytes`].
    ///
    /// Input is validated for valid scheme characters, that is alphanumeric, `+`, `-`, and `.`.
    ///
    /// # Panics
    ///
    /// Panics if `value` contains invalid character.
    pub const fn from_bytes(value: Bytes) -> Self {
        simd::validate_scheme!(value else {
            panic!("`Scheme::new` contains invalid byte")
        });
        Self {
            // SAFETY: `simd::validate_scheme` checks for valid ASCII
            repr: Repr::Str(unsafe { ByteStr::from_utf8_unchecked(value) }),
        }
    }

    /// Construct a [`Scheme`] from [`Bytes`].
    ///
    /// Input is validated for valid scheme characters, that is alphanumeric, `+`, `-`, and `.`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `value` contains invalid character.
    pub fn try_from_bytes(value: Bytes) -> Result<Self, InvalidUri> {
        simd::validate_scheme!(value else {
            return Err(InvalidUri::Char)
        });
        Ok(Self {
            // SAFETY: `simd::validate_scheme` checks for valid ASCII
            repr: Repr::Str(unsafe { ByteStr::from_utf8_unchecked(value) }),
        })
    }

    /// # Safety
    ///
    /// Scheme must be valid ASCII.
    pub(crate) unsafe fn new_unchecked(scheme: Bytes) -> Scheme {
        match scheme.as_slice() {
            b"http" => Self::http(),
            b"https" => Self::https(),
            _ => Self {
                // SAFETY: ensured by caller
                repr: Repr::Str(unsafe { ByteStr::from_utf8_unchecked(scheme) }),
            },
        }
    }

    #[inline]
    pub const fn as_str(&self) -> Option<&str> {
        match &self.repr {
            Repr::Static(NONE) => None,
            Repr::Static(HTTP) => Some("http"),
            Repr::Static(HTTPS) => Some("https"),
            Repr::Str(s) => Some(s.as_str()),
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

