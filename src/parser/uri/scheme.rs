use tcio::bytes::{ByteStr, Bytes};

#[derive(Debug, PartialEq)]
pub struct Scheme {
    repr: Repr,
}

const HTTP: u8  = 0b0000_0000;
const HTTPS: u8 = 0b0000_0001;

#[derive(Debug, PartialEq)]
enum Repr {
    Static(u8),
    ByteStr(ByteStr),
}

impl Scheme {
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
    pub const fn as_str(&self) -> &str {
        match &self.repr {
            Repr::Static(HTTP) => "http",
            Repr::Static(HTTPS) => "https",
            Repr::ByteStr(s) => s.as_str(),
            _ => unreachable!(),
        }
    }
}

