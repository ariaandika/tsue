use tcio::ByteStr;

// ===== HeaderName =====

/// HTTP Header name.
pub struct HeaderName {
    repr: Repr,
}

enum Repr {
    Standard(StandardHeader),
    Bytes(ByteStr),
}

/// Precomputed known header name.
struct StandardHeader {
    name: &'static str,
    hash: u16,
}

impl HeaderName {
    /// Used in iterator.
    pub(crate) const PLACEHOLDER: Self = Self {
        repr: Repr::Standard(StandardHeader {
            name: "",
            hash: 0,
        })
    };

    /// Create new [`HeaderName`].
    pub fn new(name: impl Into<ByteStr>) -> Self {
        Self { repr: Repr::Bytes(name.into()) }
    }

    /// May calculate hash
    pub(crate) fn hash(&self) -> u16 {
        match &self.repr {
            Repr::Standard(s) => s.hash,
            Repr::Bytes(b) => fnv_hash(b.as_bytes()),
        }
    }

    /// Extracts a string slice of the header name.
    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.repr {
            Repr::Standard(s) => s.name,
            Repr::Bytes(s) => s.as_str(),
        }
    }
}

// ===== Hash =====

#[inline]
const fn fnv_hash(bytes: &[u8]) -> u16 {
    const INITIAL_STATE: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = INITIAL_STATE;
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(PRIME);
        i += 1;
    }

    hash as _
}

// ===== Ref Traits =====

/// The contrete type used in header map lookup operation.
pub(crate) struct HeaderNameRef<'a> {
    pub(crate) name: &'a str,
    pub(crate) hash: u16,
}

/// A type that can be used for [`HeaderMap`] operation.
///
/// [`HeaderMap`]: super::HeaderMap
#[allow(private_bounds)]
pub trait AsHeaderName: SealedRef { }
pub(crate) trait SealedRef: Sized {
    fn hash(&self) -> u16;

    fn as_str(&self) -> &str;

    /// May calculate hash
    fn to_header_ref(&self) -> HeaderNameRef {
        HeaderNameRef {
            name: self.as_str(),
            hash: self.hash(),
        }
    }
}

impl<K: AsHeaderName> AsHeaderName for &K { }
impl<S: SealedRef> SealedRef for &S {
    fn hash(&self) -> u16 {
        S::hash(self)
    }

    fn as_str(&self) -> &str {
        S::as_str(self)
    }
}

impl AsHeaderName for &str { }
impl SealedRef for &str {
    fn hash(&self) -> u16 {
        fnv_hash(self.as_bytes())
    }

    fn as_str(&self) -> &str {
        self
    }
}

impl AsHeaderName for HeaderName { }
impl SealedRef for HeaderName {
    fn hash(&self) -> u16 {
        match &self.repr {
            Repr::Standard(s) => s.hash,
            Repr::Bytes(s) => fnv_hash(s.as_bytes()),
        }
    }

    fn as_str(&self) -> &str {
        HeaderName::as_str(self)
    }
}

// ===== Owned Traits =====

/// A type that can be used for name consuming [`HeaderMap`] operation.
///
/// [`HeaderMap`]: super::HeaderMap
#[allow(private_bounds)]
pub trait IntoHeaderName: Sealed {}
pub(crate) trait Sealed: Sized {
    fn into_header_name(self) -> HeaderName;
}

impl IntoHeaderName for ByteStr {}
impl Sealed for ByteStr {
    fn into_header_name(self) -> HeaderName {
        HeaderName {
            repr: Repr::Bytes(self),
        }
    }
}

// for static data use provided constants, not static str
impl IntoHeaderName for &str {}
impl Sealed for &str {
    fn into_header_name(self) -> HeaderName {
        HeaderName {
            repr: Repr::Bytes(ByteStr::copy_from_str(self)),
        }
    }
}

impl IntoHeaderName for HeaderName {}
impl Sealed for HeaderName {
    fn into_header_name(self) -> HeaderName {
        self
    }
}

// ===== Debug =====

impl std::fmt::Debug for HeaderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("HeaderName");
        match &self.repr {
            Repr::Standard(s) => f.field("name", &s.name),
            Repr::Bytes(b) => f.field("name", &b),
        }.finish()
    }
}

// ===== Constants =====

// https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers

standard_header! {
    // ===== Authentication =====

    /// Defines the authentication method that should be used to access a resource.
    pub const WWW_AUTHENTICATE: HeaderName = "www-authenticate";
    /// Contains the credentials to authenticate a user-agent with a server.
    pub const AUTHORIZATION: HeaderName = "authorization";
    /// Defines the authentication method that should be used to access a resource behind a proxy server.
    pub const PROXY_AUTHENTICATE: HeaderName = "proxy-authenticate";
    /// Contains the credentials to authenticate a user agent with a proxy server.
    pub const PROXY_AUTHORIZATION: HeaderName = "proxy-authorization";

    // ===== Caching =====

    /// The time, in seconds, that the object has been in a proxy cache.
    pub const AGE: HeaderName = "age";
    /// Directives for caching mechanisms in both requests and responses.
    pub const CACHE_CONTROL: HeaderName = "cache-control";
    /// Clears browsing data (e.g., cookies, storage, cache) associated with the requesting website.
    pub const CLEAR_SITE_DATA: HeaderName = "clear-site-data";
    /// The date/time after which the response is considered stale.
    pub const EXPIRES: HeaderName = "expires";

    // ===== Conditionals =====

    /// The last modification date of the resource, used to compare several versions of the same
    /// resource. It is less accurate than ETag, but easier to calculate in some environments.
    /// Conditional requests using If-Modified-Since and If-Unmodified-Since use this value to
    /// change the behavior of the request.
    pub const LAST_MODIFIED: HeaderName = "last-modified";

    /// A unique string identifying the version of the resource. Conditional requests using
    /// If-Match and If-None-Match use this value to change the behavior of the request.
    pub const ETAG: HeaderName = "etag";

    /// Makes the request conditional, and applies the method only if the stored resource matches
    /// one of the given ETags.
    pub const IF_MATCH: HeaderName = "if-match";

    /// Makes the request conditional, and applies the method only if the stored resource doesn't
    /// match any of the given ETags. This is used to update caches (for safe requests), or to
    /// prevent uploading a new resource when one already exists.
    pub const IF_NONE_MATCH: HeaderName = "if-none-match";

    /// Makes the request conditional, and expects the resource to be transmitted only if it has
    /// been modified after the given date. This is used to transmit data only when the cache is
    /// out of date.
    pub const IF_MODIFIED_SINCE: HeaderName = "if-modified-since";

    /// Makes the request conditional, and expects the resource to be transmitted only if it has
    /// not been modified after the given date. This ensures the coherence of a new fragment of a
    /// specific range with previous ones, or to implement an optimistic concurrency control system
    /// when modifying existing documents.
    pub const IF_UNMODIFIED_SINCE: HeaderName = "if-unmodified-since";

    /// Determines how to match request headers to decide whether a cached response can be used
    /// rather than requesting a fresh one from the origin server.
    pub const VARY: HeaderName = "vary";

    // ===== Connection management =====

    /// Controls whether the network connection stays open after the current transaction finishes.
    pub const CONNECTION: HeaderName = "connection";

    /// Controls how long a persistent connection should stay open.
    pub const KEEP_ALIVE: HeaderName = "keep-alive";

    /// ===== Content negotiation =====
    /// more details on [mdn]<https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Content_negotiation>

    /// Informs the server about the types of data that can be sent back.
    pub const ACCEPT: HeaderName = "accept";

    /// The encoding algorithm, usually a compression algorithm, that can be used on the resource
    /// sent back.
    pub const ACCEPT_ENCODING: HeaderName = "accept-encoding";

    /// Informs the server about the human language the server is expected to send back. This is a
    /// hint and is not necessarily under the full control of the user: the server should always
    /// pay attention not to override an explicit user choice (like selecting a language from a
    /// dropdown).
    pub const ACCEPT_LANGUAGE: HeaderName = "accept-language";

    /// A request content negotiation response header that advertises which media type the server
    /// is able to understand in a PATCH request.
    pub const ACCEPT_PATCH: HeaderName = "accept-patch";

    /// A request content negotiation response header that advertises which media type the server
    /// is able to understand in a POST request.
    pub const ACCEPT_POST: HeaderName = "accept-post";
}

// ===== Macros =====

macro_rules! standard_header {
    (
        $(
            $(#[$doc:meta])*
            pub const $id:ident: $t:ty = $name:literal;
        )*
    ) => {
        pub mod standards {
            pub use {$(super::$id),*};
        }
        $(
            $(#[$doc])*
            pub const $id: $t = HeaderName {
                repr: Repr::Standard(StandardHeader { name: $name, hash: fnv_hash($name.as_bytes()) })
            };
        )*
    };
}

use standard_header;

