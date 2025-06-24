use std::{fmt, str::FromStr};

/// HTTP Method.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Method(Inner);

// https://tools.ietf.org/html/rfc7231#section-4
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
enum Inner {
    Options,
    #[default]
    Get,
    Head,
    Trace,
    Connect,
    Post,
    Put,
    Delete,
    Patch,
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        str::fmt(self.as_str(), f)
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Method {
    forward! {
        /// OPTIONS
        pub const OPTIONS: Options = b"OPTIONS";
        /// GET
        pub const GET: Get = b"GET";
        /// HEAD
        pub const HEAD: Head = b"HEAD";
        /// TRACE
        pub const TRACE: Trace = b"TRACE";
        /// CONNECT
        pub const CONNECT: Connect = b"CONNECT";
        /// POST
        pub const POST: Post = b"POST";
        /// PUT
        pub const PUT: Put = b"PUT";
        /// DELETE
        pub const DELETE: Delete = b"DELETE";
        /// PATCH
        pub const PATCH: Patch = b"PATCH";
    }
}

// ===== Error =====

/// An error when trying to parse [`Method`] from a string.
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
    ($($(#[$doc:meta])* pub const $name:ident: $variant:ident = $val:literal;)*) => {
        $(
            $(#[$doc])*
            pub const $name: Method = Method(Inner::$variant);
        )*

        /// Create [`Method`] from bytes.
        pub const fn from_bytes(src: &[u8]) -> Option<Method> {
            match src {
                $(
                    $val => Some(Self::$name),
                )*
                _ => None,
            }
        }
        /// Returns string representation.
        pub const fn as_str(&self) -> &'static str {
            match self.0 {
                $(
                    Inner::$variant => stringify!($name),
                )*
            }
        }
    };
}

use forward;

