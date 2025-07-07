/// HTTP [Method][rfc].
///
/// [rfc]: <https://datatracker.ietf.org/doc/html/rfc9110#name-methods>
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Method(Inner);

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
enum Inner {
    #[default]
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

impl Method {
    forward! {
        /// The `GET` method requests a representation of the specified resource.
        pub const GET: Get = b"GET";
        /// The `HEAD` method asks for a response identical to a GET request, but without a
        /// response body.
        pub const HEAD: Head = b"HEAD";
        /// The `POST` method submits an entity to the specified resource, often causing a change
        /// in state or side effects on the server.
        pub const POST: Post = b"POST";
        /// The `PUT` method replaces all current representations of the target resource with the
        /// request content.
        pub const PUT: Put = b"PUT";
        /// The `DELETE` method deletes the specified resource.
        pub const DELETE: Delete = b"DELETE";
        /// The `CONNECT` method establishes a tunnel to the server identified by the target
        /// resource.
        pub const CONNECT: Connect = b"CONNECT";
        /// The `OPTIONS` method describes the communication options for the target resource.
        pub const OPTIONS: Options = b"OPTIONS";
        /// The `TRACE` method performs a message loop-back test along the path to the target
        /// resource.
        pub const TRACE: Trace = b"TRACE";
        /// The `PATCH` method applies partial modifications to a resource.
        pub const PATCH: Patch = b"PATCH";
    }
}

impl std::fmt::Debug for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        str::fmt(self.as_str(), f)
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.as_str())
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
        #[inline]
        pub const fn from_bytes(src: &[u8]) -> Option<Method> {
            match src {
                $(
                    $val => Some(Self::$name),
                )*
                _ => None,
            }
        }
        /// Returns string representation.
        #[inline]
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

