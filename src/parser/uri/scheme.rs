use tcio::bytes::{ByteStr, Bytes};

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
    ByteStr(ByteStr),
}

impl Scheme {
    #[inline]
    pub const fn none() -> Scheme {
        Self {
            repr: Repr::Static(NONE),
        }
    }

    #[inline]
    pub const fn http() -> Scheme {
        Self {
            repr: Repr::Static(HTTP),
        }
    }

    #[inline]
    pub const fn https() -> Scheme {
        Self {
            repr: Repr::Static(HTTPS),
        }
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
                repr: Repr::ByteStr(unsafe { ByteStr::from_utf8_unchecked(scheme) }),
            },
        }
    }

    #[inline]
    pub const fn as_str(&self) -> Option<&str> {
        match &self.repr {
            Repr::Static(NONE) => None,
            Repr::Static(HTTP) => Some("http"),
            Repr::Static(HTTPS) => Some("https"),
            Repr::ByteStr(s) => Some(s.as_str()),
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

