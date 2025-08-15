use tcio::bytes::ByteStr;

#[derive(Debug)]
pub struct Scheme {
    repr: Repr,
}

const HTTP: u8  = 0b0000_0000;
const HTTPS: u8 = 0b0000_0001;

#[derive(Debug)]
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

    #[inline]
    pub fn new(scheme: ByteStr) -> Scheme {
        // TODO: spec validation
        match scheme.as_str() {
            "http" => Self::http(),
            "https" => Self::https(),
            _ => Self {
                repr: Repr::ByteStr(scheme),
            },
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.repr {
            Repr::Static(HTTP) => "http",
            Repr::Static(HTTPS) => "https",
            Repr::ByteStr(s) => s.as_str(),
            Repr::Static(_) => unreachable!(),
        }
    }
}


