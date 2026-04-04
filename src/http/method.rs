/// HTTP Method.
///
/// This API implements methods defined in [RFC9110] and the [PATCH] method.
///
/// Arbitrary method is not supported.
///
/// [RFC9110]: <https://www.rfc-editor.org/rfc/rfc9110#name-method-definitions>
/// [PATCH]: <https://www.rfc-editor.org/rfc/rfc5789>
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Method(Inner);

props! {
    /// The [GET] method requests transfer of a current [selected representation][sr] for the
    /// [target resource][tr].
    ///
    /// [GET]: <https://www.rfc-editor.org/rfc/rfc9110#name-get>
    /// [sr]: <https://www.rfc-editor.org/rfc/rfc9110#selected.representation>
    /// [tr]: <https://www.rfc-editor.org/rfc/rfc9110#target.resource>
    pub const GET = Get { safe, idempotent };
    /// The [HEAD] method is identical to GET except that the server MUST NOT send content in the
    /// response. HEAD
    ///
    /// [HEAD]: <https://www.rfc-editor.org/rfc/rfc9110#name-head>
    pub const HEAD = Head { safe, idempotent };
    /// The [POST] method requests that the [target resource][tr] process the representation
    /// enclosed in the request according to the resource's own specific semantics.
    ///
    /// [POST]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-post>
    /// [tr]: <https://www.rfc-editor.org/rfc/rfc9110#target.resource>
    pub const POST = Post { };
    /// The [PUT] method requests that the state of the [target resource][tr] be created or
    /// replaced with the state defined by the representation enclosed in the request message
    /// content.
    ///
    /// [PUT]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-put>
    /// [tr]: <https://www.rfc-editor.org/rfc/rfc9110#target.resource>
    pub const PUT = Put { idempotent };
    /// The [DELETE] method requests that the origin server remove the association between the
    /// [target resource][tr] and its current functionality.
    ///
    /// [DELETE]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-delete>
    /// [tr]: <https://www.rfc-editor.org/rfc/rfc9110#target.resource>
    pub const DELETE = Delete { idempotent };
    /// The [CONNECT] method requests that the recipient establish a tunnel to the destination
    /// origin server identified by the request target and, if successful, thereafter restrict its
    /// behavior to blind forwarding of data, in both directions, until the tunnel is closed.
    ///
    /// [CONNECT]: <https://www.rfc-editor.org/rfc/rfc9110#name-connect>
    pub const CONNECT = Connect { };
    /// The [OPTIONS] method requests information about the communication options available for the
    /// target resource, at either the origin server or an intervening intermediary.
    ///
    /// [OPTIONS]: <https://www.rfc-editor.org/rfc/rfc9110#name-options>
    pub const OPTIONS = Options { safe, idempotent };
    /// The [TRACE] method requests a remote, application-level loop-back of the request message.
    ///
    /// [TRACE]: <https://www.rfc-editor.org/rfc/rfc9110#name-trace>
    pub const TRACE = Trace { safe, idempotent };
    /// The [PATCH] method requests that a set of changes described in the request entity be
    /// applied to the resource identified by the Request-URI.
    ///
    /// [PATCH]: <https://www.rfc-editor.org/rfc/rfc5789#section-2>
    pub const PATCH = Patch { };
}

// ===== std traits =====

impl std::fmt::Debug for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ===== Macros =====

macro_rules! props {
    (
        $(
           $(#[$doc:meta])*
           $vis:vis const $konst:ident = $method:ident { $($prop:ident),* };
        )*
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        enum Inner {
            $($method),*
        }

        impl Default for Method {
            #[inline]
            fn default() -> Self {
                Self::GET
            }
        }

        impl Method {
            $(
               $(#[$doc])*
               $vis const $konst: Self = Self(Inner::$method);
            )*

            /// Creates `Method` from bytes.
            ///
            /// This method only accept ASCII uppercase alphabetic.
            #[inline]
            pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::http::error::UnknownMethod> {
                // there is no `bytestify` to allow placing it in pattern matching
                $(const $konst: &[u8] = stringify!($konst).as_bytes();)*
                match bytes {
                    $($konst => Ok(Self::$konst),)*
                    _ => Err(<_>::default())
                }
            }

            /// Returns string representation of the method.
            #[inline]
            pub const fn as_str(&self) -> &'static str {
                match &self.0 {
                    $(
                        Inner::$method => stringify!($konst),
                    )*
                }
            }

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
                match &self.0 {
                    $(
                        Inner::$method => safe!($($prop),*),
                    )*
                }
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
                match &self.0 {
                    $(
                        Inner::$method => idempotent!($($prop),*),
                    )*
                }
            }
        }

        impl std::str::FromStr for Method {
            type Err = crate::http::error::UnknownMethod;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(stringify!($konst) => Ok(Self::$konst),)*
                    _ => Err(<_>::default())
                }
            }
        }
    };
}

macro_rules! safe {
    (safe, $($tt:ident),*) => { true };
    (safe) => { true };
    ($tt:ident, $($t2:ident)*) => { safe!($($t2)*) };
    ($tt:ident) => { false };
    () => { false };
}

macro_rules! idempotent {
    (idempotent, $($tt:ident),*) => { true };
    (idempotent) => { true };
    ($tt:ident, $($t2:ident)*) => { idempotent!($($t2)*) };
    ($tt:ident) => { false };
    () => { false };
}

use {idempotent, props, safe};
