use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Method(Inner);

// https://tools.ietf.org/html/rfc7231#section-4.1W
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
enum Inner {
    Options,
    #[default]
    Get,
    Post,
    Put,
    Delete,
    Head,
    Trace,
    Connect,
    Patch,
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Method {
    forward! {
        pub const OPTIONS: Options = b"OPTIONS";
        pub const GET: Get = b"GET";
        pub const POST: Post = b"POST";
        pub const PUT: Put = b"PUT";
        pub const DELETE: Delete = b"DELETE";
        pub const HEAD: Head = b"HEAD";
        pub const TRACE: Trace = b"TRACE";
        pub const CONNECT: Connect = b"CONNECT";
        pub const PATCH: Patch = b"PATCH";
    }
}

#[derive(Debug)]
pub struct UnknownMethod;

impl FromStr for Method {
    type Err = UnknownMethod;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bytes(s.as_bytes()).ok_or(UnknownMethod)
    }
}

impl fmt::Display for UnknownMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown method")
    }
}

// ===== Macros =====

macro_rules! forward {
    ($(pub const $name:ident: $variant:ident = $val:literal;)*) => {
        $(
            #[doc = stringify!($name)]
            pub const $name: Method = Method(Inner::$variant);
        )*
        /// Create [`Method`] from bytes.
        pub fn from_bytes(src: &[u8]) -> Option<Method> {
            match src {
                $(
                    $val => Some(Self::$name),
                )*
                _ => None,
            }
        }
        /// Returns string representation.
        pub fn as_str(&self) -> &'static str {
            match self.0 {
                $(
                    Inner::$variant => stringify!($name),
                )*
            }
        }
    };
}

use forward;

