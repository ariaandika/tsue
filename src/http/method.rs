/// HTTP Method.
///
/// [httpwg](https://httpwg.org/specs/rfc9110.html#method.definitions)
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
        /// The `GET` method requests transfer of a current selected representation for the target
        /// resource.
        ///
        /// [httpwg](https://httpwg.org/specs/rfc9110.html#GET)
        pub const GET: Get = b"GET";
        /// The `HEAD` method is identical to `GET` except that the server MUST NOT send content in
        /// the response.
        ///
        /// [httpwg](https://httpwg.org/specs/rfc9110.html#HEAD)
        pub const HEAD: Head = b"HEAD";
        /// The `POST` method requests that the target resource process the representation enclosed
        /// in the request according to the resource's own specific semantics.
        ///
        /// [httpwg](https://httpwg.org/specs/rfc9110.html#POST)
        pub const POST: Post = b"POST";
        /// The `PUT` method requests that the state of the target resource be created or replaced
        /// with the state defined by the representation enclosed in the request message content.
        ///
        /// [httpwg](https://httpwg.org/specs/rfc9110.html#PUT)
        pub const PUT: Put = b"PUT";
        /// The `DELETE` method requests that the origin server remove the association between the
        /// target resource and its current functionality.
        ///
        /// [httpwg](https://httpwg.org/specs/rfc9110.html#DELETE)
        pub const DELETE: Delete = b"DELETE";
        /// The `CONNECT` method requests that the recipient establish a tunnel to the destination
        /// origin server identified by the request target and, if successful, thereafter restrict
        /// its behavior to blind forwarding of data, in both directions, until the tunnel is
        /// closed.
        ///
        /// [httpwg](https://httpwg.org/specs/rfc9110.html#CONNECT)
        pub const CONNECT: Connect = b"CONNECT";
        /// The `OPTIONS` method requests information about the communication options available for
        /// the target resource, at either the origin server or an intervening intermediary.
        ///
        /// [httpwg](https://httpwg.org/specs/rfc9110.html#OPTIONS)
        pub const OPTIONS: Options = b"OPTIONS";
        /// The `TRACE` method requests a remote, application-level loop-back of the request
        /// message.
        ///
        /// [httpwg](https://httpwg.org/specs/rfc9110.html#TRACE)
        pub const TRACE: Trace = b"TRACE";
        /// The `PATCH` method requests that a set of changes described in the request entity be
        /// applied to the resource identified by the Request-URI.
        ///
        /// [httpwg](https://httpwg.org/specs/rfc5789.html#patch)
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
