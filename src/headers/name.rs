use tcio::bytes::ByteStr;

// ===== HeaderName =====

/// HTTP Header name.
#[derive(Clone)]
pub struct HeaderName {
    repr: Repr,
}

#[derive(Clone)]
enum Repr {
    Standard(StandardHeader),
    Bytes(ByteStr),
}

/// Precomputed known header name.
#[derive(Clone)]
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
            Repr::Bytes(b) => fnv_hash_to_lowercase(b.as_bytes()),
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
const fn fnv_hash_to_lowercase(bytes: &[u8]) -> u16 {
    const INITIAL_STATE: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = INITIAL_STATE;
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i].to_ascii_lowercase() as u64;
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
    fn to_header_ref(&self) -> HeaderNameRef<'_> {
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
        fnv_hash_to_lowercase(self.as_bytes())
    }

    fn as_str(&self) -> &str {
        self
    }
}

impl AsHeaderName for HeaderName { }
impl SealedRef for HeaderName {
    fn hash(&self) -> u16 {
        self.hash()
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

// ===== Traits =====

impl std::fmt::Display for HeaderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        str::fmt(self.as_str(), f)
    }
}

impl std::fmt::Debug for HeaderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("HeaderName");
        match &self.repr {
            Repr::Standard(s) => f.field("name", &s.name),
            Repr::Bytes(b) => f.field("name", &b),
        }.finish()
    }
}

impl std::hash::Hash for HeaderName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self.repr {
            Repr::Standard(s) => s.hash.hash(state),
            Repr::Bytes(b) => b.hash(state),
        }
    }
}

// ===== Standard Headers =====

// https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers

standard_header! {
    /// HTTP Standard Headers
    mod standard;

    // ===== Authentication =====

    /// Defines the authentication method that should be used to access a resource.
    pub const WWW_AUTHENTICATE: HeaderName = "www-authenticate";

    /// Contains the credentials to authenticate a user-agent with a server.
    pub const AUTHORIZATION: HeaderName = "authorization";

    /// Defines the authentication method that should be used to access a resource behind a proxy
    /// server.
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

    // ===== Content negotiation =====
    // more details on [mdn]<https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Content_negotiation>

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

    // ===== Controls =====

    /// Indicates expectations that need to be fulfilled by the server to properly handle the
    /// request.
    pub const EXPECT: HeaderName = "expect";

    /// When using TRACE, indicates the maximum number of hops the request can do before being
    /// reflected to the sender.
    pub const MAX_FORWARDS: HeaderName = "max-forwards";

    // ===== Cookies =====

    /// Contains stored HTTP cookies previously sent by the server with the Set-Cookie header.
    pub const COOKIE: HeaderName = "cookie";

    /// Send cookies from the server to the user-agent.
    pub const SET_COOKIE: HeaderName = "set-cookie";

    // ===== CORS =====
    // more details on [mdn]<https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS>

    /// Indicates whether the response to the request can be exposed when the credentials flag is
    /// true.
    pub const ACCESS_CONTROL_ALLOW_CREDENTIALS: HeaderName = "access-control-allow-credentials";

    /// Used in response to a preflight request to indicate which HTTP headers can be used when
    /// making the actual request.
    pub const ACCESS_CONTROL_ALLOW_HEADERS: HeaderName = "access-control-allow-headers";

    /// Specifies the methods allowed when accessing the resource in response to a preflight
    /// request.
    pub const ACCESS_CONTROL_ALLOW_METHODS: HeaderName = "access-control-allow-methods";

    /// Indicates whether the response can be shared.
    pub const ACCESS_CONTROL_ALLOW_ORIGIN: HeaderName = "access-control-allow-origin";

    /// Indicates which headers can be exposed as part of the response by listing their names.
    pub const ACCESS_CONTROL_EXPOSE_HEADERS: HeaderName = "access-control-expose-headers";

    /// Indicates how long the results of a preflight request can be cached.
    pub const ACCESS_CONTROL_MAX_AGE: HeaderName = "access-control-max-age";

    /// Used when issuing a preflight request to let the server know which HTTP headers will be
    /// used when the actual request is made.
    pub const ACCESS_CONTROL_REQUEST_HEADERS: HeaderName = "access-control-request-headers";

    /// Used when issuing a preflight request to let the server know which HTTP method will be used
    /// when the actual request is made.
    pub const ACCESS_CONTROL_REQUEST_METHOD: HeaderName = "access-control-request-method";

    /// Indicates where a fetch originates from.
    pub const ORIGIN: HeaderName = "origin";

    /// Specifies origins that are allowed to see values of attributes retrieved via features of
    /// the Resource Timing API, which would otherwise be reported as zero due to cross-origin
    /// restrictions.
    pub const TIMING_ALLOW_ORIGIN: HeaderName = "timing-allow-origin";

    // ===== Downloads =====

    /// Indicates if the resource transmitted should be displayed inline (default behavior without
    /// the header), or if it should be handled like a download and the browser should present a
    /// "Save As" dialog.
    pub const CONTENT_DISPOSITION: HeaderName = "content-disposition";

    // ===== Message body information =====

    /// The size of the resource, in decimal number of bytes.
    pub const CONTENT_LENGTH: HeaderName = "content-length";

    /// Indicates the media type of the resource.
    pub const CONTENT_TYPE: HeaderName = "content-type";

    /// Used to specify the compression algorithm.
    pub const CONTENT_ENCODING: HeaderName = "content-encoding";

    /// Describes the human language(s) intended for the audience, so that it allows a user to
    /// differentiate according to the users' own preferred language.
    pub const CONTENT_LANGUAGE: HeaderName = "content-language";

    /// Indicates an alternate location for the returned data.
    pub const CONTENT_LOCATION: HeaderName = "content-location";

    // ===== Preferences =====

    /// Indicates preferences for specific server behaviors during request processing. For example,
    /// it can request minimal response content (return=minimal) or asynchronous processing
    /// (respond-async). The server processes the request normally if the header is unsupported.
    pub const PREFER: HeaderName = "prefer";

    /// Informs the client which preferences specified in the Prefer header were applied by the
    /// server. It is a response-only header providing transparency about preference handling.
    pub const PREFERENCE_APPLIED: HeaderName = "preference-applied";

    // ===== Proxies =====

    /// Contains information from the client-facing side of proxy servers that is altered or lost
    /// when a proxy is involved in the path of the request.
    pub const FORWARDED: HeaderName = "forwarded";

    /// Added by proxies, both forward and reverse proxies, and can appear in the request headers
    /// and the response headers.
    pub const VIA: HeaderName = "via";

    // ===== Range requests =====

    /// Indicates if the server supports range requests, and if so in which unit the range can be
    /// expressed.
    pub const ACCEPT_RANGES: HeaderName = "accept-ranges";

    /// Indicates the part of a document that the server should return.
    pub const RANGE: HeaderName = "range";

    /// Creates a conditional range request that is only fulfilled if the given etag or date
    /// matches the remote resource. Used to prevent downloading two ranges from incompatible
    /// version of the resource.
    pub const IF_RANGE: HeaderName = "if-range";

    /// Indicates where in a full body message a partial message belongs.
    pub const CONTENT_RANGE: HeaderName = "content-range";

    // ===== Redirects =====

    /// Indicates the URL to redirect a page to.
    pub const LOCATION: HeaderName = "location";

    /// Directs the browser to reload the page or redirect to another. Takes the same value as the
    /// meta element with http-equiv="refresh".
    pub const REFRESH: HeaderName = "refresh";

    // ===== Request context =====

    /// Contains an Internet email address for a human user who controls the requesting user agent.
    pub const FROM: HeaderName = "from";

    /// Specifies the domain name of the server (for virtual hosting), and (optionally) the TCP
    /// port number on which the server is listening.
    pub const HOST: HeaderName = "host";

    /// The address of the previous web page from which a link to the currently requested page was
    /// followed.
    pub const REFERER: HeaderName = "referer";

    /// Governs which referrer information sent in the Referer header should be included with
    /// requests made.
    pub const REFERRER_POLICY: HeaderName = "referrer-policy";

    /// Contains a characteristic string that allows the network protocol peers to identify the
    /// application type, operating system, software vendor or software version of the requesting
    /// software user agent.
    pub const USER_AGENT: HeaderName = "user-agent";

    // ===== Response context =====

    /// Lists the set of HTTP request methods supported by a resource.
    pub const ALLOW: HeaderName = "allow";

    /// Contains information about the software used by the origin server to handle the request.
    pub const SERVER: HeaderName = "server";

    // ===== Security =====

    /// Allows a server to declare an embedder policy for a given document.
    pub const CROSS_ORIGIN_EMBEDDER_POLICY: HeaderName = "cross-origin-embedder-policy";

    /// Prevents other domains from opening/controlling a window.
    pub const CROSS_ORIGIN_OPENER_POLICY: HeaderName = "cross-origin-opener-policy";

    /// Prevents other domains from reading the response of the resources to which this header is
    /// applied. See also CORP explainer article.
    pub const CROSS_ORIGIN_RESOURCE_POLICY: HeaderName = "cross-origin-resource-policy";

    /// Controls resources the user agent is allowed to load for a given page.
    pub const CONTENT_SECURITY_POLICY: HeaderName = "content-security-policy";

    /// Allows web developers to experiment with policies by monitoring, but not enforcing, their
    /// effects. These violation reports consist of JSON documents sent via an HTTP POST request to
    /// the specified URI.
    pub const CONTENT_SECURITY_POLICY_REPORT_ONLY: HeaderName = "content-security-policy-report-only";

    /// Provides a mechanism to allow and deny the use of browser features in a website's own
    /// frame, and in `<iframe>`s that it embeds.
    pub const PERMISSIONS_POLICY: HeaderName = "permissions-policy";

    /// Force communication using HTTPS instead of HTTP.
    pub const STRICT_TRANSPORT_SECURITY: HeaderName = "strict-transport-security";

    /// Sends a signal to the server expressing the client's preference for an encrypted and
    /// authenticated response, and that it can successfully handle the upgrade-insecure-requests
    /// directive.
    pub const UPGRADE_INSECURE_REQUESTS: HeaderName = "upgrade-insecure-requests";

    /// Disables MIME sniffing and forces browser to use the type given in Content-Type.
    pub const X_CONTENT_TYPE_OPTIONS: HeaderName = "x-content-type-options";

    /// Indicates whether a browser should be allowed to render a page in a `<frame>`, `<iframe>`,
    /// `<embed>` or `<object>`.
    pub const X_FRAME_OPTIONS: HeaderName = "x-frame-options";

    /// A cross-domain policy file may grant clients, such as Adobe Acrobat or Apache Flex (among
    /// others), permission to handle data across domains that would otherwise be restricted due to
    /// the Same-Origin Policy. The X-Permitted-Cross-Domain-Policies header overrides such policy
    /// files so that clients still block unwanted requests.
    pub const X_PERMITTED_CROSS_DOMAIN_POLICIES: HeaderName = "x-permitted-cross-domain-policies";

    /// May be set by hosting environments or other frameworks and contains information about them
    /// while not providing any usefulness to the application or its visitors. Unset this header to
    /// avoid exposing potential vulnerabilities.
    pub const X_POWERED_BY: HeaderName = "x-powered-by";

    /// Enables cross-site scripting filtering.
    pub const X_XSS_PROTECTION: HeaderName = "x-xss-protection";

    // ===== Fetch metadata request headers =====

    /// Indicates the relationship between a request initiator's origin and its target's origin. It
    /// is a Structured Header whose value is a token with possible values cross-site, same-origin,
    /// same-site, and none.
    pub const SEC_FETCH_SITE: HeaderName = "sec-fetch-site";

    /// Indicates the request's mode to a server. It is a Structured Header whose value is a token
    /// with possible values cors, navigate, no-cors, same-origin, and websocket.
    pub const SEC_FETCH_MODE: HeaderName = "sec-fetch-mode";

    /// Indicates whether or not a navigation request was triggered by user activation. It is a
    /// Structured Header whose value is a boolean so possible values are ?0 for false and ?1 for
    /// true.
    pub const SEC_FETCH_USER: HeaderName = "sec-fetch-user";

    /// Indicates the request's destination. It is a Structured Header whose value is a token with
    /// possible values audio, audioworklet, document, embed, empty, font, image, manifest, object,
    /// paintworklet, report, script, serviceworker, sharedworker, style, track, video, worker, and
    /// xslt.
    pub const SEC_FETCH_DEST: HeaderName = "sec-fetch-dest";

    // ===== Transfer coding =====

    /// Specifies the form of encoding used to safely transfer the resource to the user.
    pub const TRANSFER_ENCODING: HeaderName = "transfer-encoding";

    /// Specifies the transfer encodings the user agent is willing to accept.
    pub const TE: HeaderName = "te";

    /// Allows the sender to include additional fields at the end of chunked message.
    pub const TRAILER: HeaderName = "trailer";

    // ===== WebSockets =====

    /// Response header that indicates that the server is willing to upgrade to a WebSocket
    /// connection.
    pub const SEC_WEBSOCKET_ACCEPT: HeaderName = "sec-websocket-accept";

    /// In requests, this header indicates the WebSocket extensions supported by the client in
    /// preferred order. In responses, it indicates the extension selected by the server from the
    /// client's preferences.
    pub const SEC_WEBSOCKET_EXTENSIONS: HeaderName = "sec-websocket-extensions";

    /// Request header containing a key that verifies that the client explicitly intends to open a
    /// WebSocket.
    pub const SEC_WEBSOCKET_KEY: HeaderName = "sec-websocket-key";

    /// In requests, this header indicates the sub-protocols supported by the client in preferred
    /// order. In responses, it indicates the sub-protocol selected by the server from the client's
    /// preferences.
    pub const SEC_WEBSOCKET_PROTOCOL: HeaderName = "sec-websocket-protocol";

    /// In requests, this header indicates the version of the WebSocket protocol used by the
    /// client. In responses, it is sent only if the requested protocol version is not supported by
    /// the server, and lists the versions that the server supports.
    pub const SEC_WEBSOCKET_VERSION: HeaderName = "sec-websocket-version";

    // ===== Other =====

    /// Contains the date and time at which the message was originated.
    pub const DATE: HeaderName = "date";

    /// Indicates how long the user agent should wait before making a follow-up request.
    pub const RETRY_AFTER: HeaderName = "retry-after";

    /// Communicates one or more metrics and descriptions for the given request-response cycle.
    pub const SERVER_TIMING: HeaderName = "server-timing";

    /// Included in fetches for a service worker's script resource. This header helps
    /// administrators log service worker script requests for monitoring purposes.
    pub const SERVICE_WORKER: HeaderName = "service-worker";

    /// Used to remove the path restriction by including this header in the response of the Service
    /// Worker script.
    pub const SERVICE_WORKER_ALLOWED: HeaderName = "service-worker-allowed";

    /// Links to a source map so that debuggers can step through original source code instead of
    /// generated or transformed code.
    pub const SOURCEMAP: HeaderName = "sourcemap";

    /// This HTTP/1.1 (only) header can be used to upgrade an already established client/server
    /// connection to a different protocol (over the same transport protocol). For example, it can
    /// be used by a client to upgrade a connection from HTTP 1.1 to HTTP 2.0, or an HTTP or HTTPS
    /// connection into a WebSocket.
    pub const UPGRADE: HeaderName = "upgrade";
}

// ===== Macros =====

macro_rules! standard_header {
    (@CORE
        $(
            $(#[$doc:meta])*
            pub const $id:ident: $t:ty = $name:literal;
        )*
    ) => {
        $(
            $(#[$doc])*
            pub const $id: $t = HeaderName {
                repr: Repr::Standard(StandardHeader { name: $name, hash: fnv_hash_to_lowercase($name.as_bytes()) })
            };
        )*
    };

    (
        $(#[$mod_doc:meta])*
        mod $mod_name:ident;

        $($tt:tt)*
    ) => {
        $(#[$mod_doc])*
        pub mod $mod_name {
            use super::*;
            standard_header!(@CORE $($tt)*);
        }

    };
}

use standard_header;

