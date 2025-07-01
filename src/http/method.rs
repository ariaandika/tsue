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
        /// The `OPTIONS` method describes the communication options for the target resource.
        pub const OPTIONS: Options = b"OPTIONS";
        /// The `GET` method requests a representation of the specified resource. Requests using GET
        /// should only retrieve data and should not contain a request content.
        pub const GET: Get = b"GET";
        /// The `HEAD` method asks for a response identical to a GET request, but without a response
        /// body.
        pub const HEAD: Head = b"HEAD";
        /// The `TRACE` method performs a message loop-back test along the path to the target
        /// resource.
        pub const TRACE: Trace = b"TRACE";
        /// The `CONNECT` method establishes a tunnel to the server identified by the target
        /// resource.
        pub const CONNECT: Connect = b"CONNECT";
        /// The `POST` method submits an entity to the specified resource, often causing a change in
        /// state or side effects on the server.
        pub const POST: Post = b"POST";
        /// The `PUT` method replaces all current representations of the target resource with the
        /// request content.
        pub const PUT: Put = b"PUT";
        /// The `DELETE` method deletes the specified resource.
        pub const DELETE: Delete = b"DELETE";
        /// The `PATCH` method applies partial modifications to a resource.
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

impl std::error::Error for UnknownMethod { }

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

