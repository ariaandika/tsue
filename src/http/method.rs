/// HTTP Method.
///
/// This API follows the [RFC9110] and the PATCH method from [RFC5789].
///
/// Arbitrary method is not supported.
///
/// [RFC5789]: https://www.rfc-editor.org/rfc/rfc5789
/// [RFC9110]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-methods>
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Method(u8);

struct Props {
    safe: bool,
    idem: bool,
    value: &'static [u8],
}

props! {
    static PROPS: [9];

    /// The [GET] method requests transfer of a current [selected representation][sr] for the
    /// [target resource][tr].
    ///
    /// [GET]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-get>
    /// [sr]: <https://www.rfc-editor.org/rfc/rfc9110.html#selected.representation>
    /// [tr]: <https://www.rfc-editor.org/rfc/rfc9110.html#target.resource>
    pub const GET = (0, b"GET", safe, idempotent);
    /// The [HEAD] method is identical to GET except that the server MUST NOT send content in the
    /// response. HEAD
    ///
    /// [HEAD]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-head>
    pub const HEAD = (1, b"HEAD", safe, idempotent);
    /// The [POST] method requests that the [target resource][tr] process the representation
    /// enclosed in the request according to the resource's own specific semantics.
    ///
    /// [POST]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-post>
    /// [tr]: <https://www.rfc-editor.org/rfc/rfc9110.html#target.resource>
    pub const POST = (2, b"POST", , );
    /// The [PUT] method requests that the state of the [target resource][tr] be created or
    /// replaced with the state defined by the representation enclosed in the request message
    /// content.
    ///
    /// [PUT]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-put>
    /// [tr]: <https://www.rfc-editor.org/rfc/rfc9110.html#target.resource>
    pub const PUT = (3, b"PUT", , idempotent);
    /// The [DELETE] method requests that the origin server remove the association between the
    /// [target resource][tr] and its current functionality.
    ///
    /// [DELETE]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-delete>
    /// [tr]: <https://www.rfc-editor.org/rfc/rfc9110.html#target.resource>
    pub const DELETE = (4, b"DELETE", , idempotent);
    /// The [CONNECT] method requests that the recipient establish a tunnel to the destination
    /// origin server identified by the request target and, if successful, thereafter restrict its
    /// behavior to blind forwarding of data, in both directions, until the tunnel is closed.
    ///
    /// [CONNECT]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-connect>
    pub const CONNECT = (5, b"CONNECT", , );
    /// The [OPTIONS] method requests information about the communication options available for the
    /// target resource, at either the origin server or an intervening intermediary.
    ///
    /// [OPTIONS]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-options>
    pub const OPTIONS = (6, b"OPTIONS", safe, idempotent);
    /// The [TRACE] method requests a remote, application-level loop-back of the request message.
    ///
    /// [TRACE]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-trace>
    pub const TRACE = (7, b"TRACE", safe, idempotent);
    /// The [PATCH] method requests that a set of changes described in the request entity be
    /// applied to the resource identified by the Request-URI.
    ///
    /// [PATCH]: <https://www.rfc-editor.org/rfc/rfc5789#section-2>
    pub const PATCH = (8, b"PATCH", , );
}

impl Method {
    /// Returns `true` if method is considered ["safe"].
    ///
    /// Request methods are considered "safe" if their defined semantics are essentially read-only;
    /// i.e., the client does not request, and does not expect, any state change on the origin
    /// server as a result of applying a safe method to a target resource.
    ///
    /// Of the request methods defined by this specification, the GET, HEAD, OPTIONS, and TRACE
    /// methods are defined to be safe.
    ///
    /// ["safe"]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-safe-methods>
    #[inline]
    pub const fn is_safe(&self) -> bool {
        PROPS[self.0 as usize].safe
    }

    /// Returns `true` if method is considered ["idempotent"].
    ///
    /// A request method is considered "idempotent" if the intended effect on the server of
    /// multiple identical requests with that method is the same as the effect for a single such
    /// request.
    ///
    /// Of the request methods defined by this specification, PUT, DELETE, and safe request
    /// methods are idempotent.
    ///
    /// ["idempotent"]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-idempotent-methods>
    #[inline]
    pub const fn is_idempoten(&self) -> bool {
        PROPS[self.0 as usize].idem
    }

    /// Returns string representation of the method.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        unsafe { str::from_utf8_unchecked(PROPS[self.0 as usize].value) }
    }
}

impl std::str::FromStr for Method {
    type Err = UnknownMethod;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bytes(s.as_bytes()).ok_or(UnknownMethod)
    }
}

impl std::fmt::Debug for Method {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        str::fmt(self.as_str(), f)
    }
}

impl std::fmt::Display for Method {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        str::fmt(self.as_str(), f)
    }
}

// ===== Error =====

pub struct UnknownMethod;

impl std::error::Error for UnknownMethod { }

impl std::fmt::Debug for UnknownMethod {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unknown method")
    }
}

impl std::fmt::Display for UnknownMethod {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unknown method")
    }
}

// ===== Macros =====

macro_rules! props {
    (
        static $props:ident: [$len:literal];
        $(
           $(#[$doc:meta])*
           pub const $name:ident = ($idx:literal, $val:literal, $($safe:ident)?, $($idem:ident)?);
        )*
    ) => {
        impl Method {
            $(
               $(#[$doc])*
               pub const $name: Self = Self($idx);
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
        }

        static $props: [Props; $len] = [
            $(
                Props { value: $val, safe: prop!($($safe)?), idem: prop!($($safe)?) },
            )*
        ];
    };
}

macro_rules! prop {
    (safe) => { true };
    (idem) => { true };
    () => { false };
}

use {props, prop};
