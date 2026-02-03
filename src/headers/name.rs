use tcio::bytes::{Bytes, BytesMut};

use crate::headers::matches;
use crate::headers::error::HeaderError;

/// HTTP Header name.
///
/// # Case Normalization
///
/// Input is normalized to lowercase at construction time. [`from_static`][HeaderName::from_static]
/// will panic at compile time when name contains uppercase character.
///
/// Normalization requires copying the bytes. If the input is known to not contains uppercase
/// character, use [`from_bytes_lowercase`][HeaderName::from_bytes_lowercase] that does not incur
/// copy but returns error instead.
//
// HeaderName is optimized towards predefined standard headers
//
// Optimized operations is:
// - validation, must contains valid bytes
// - hashing, for header map lookup
//
// predefined headers skip validation and returns precomputed hash
// while arbitrary headers must pass validation and compute hash on demand
#[derive(Clone)]
pub struct HeaderName {
    repr: Repr,
}

#[derive(Clone)]
enum Repr {
    Static(&'static Static),
    /// is valid ASCII
    Arbitrary(Bytes),
}

struct Static {
    string: &'static str,
    hash: u32,
    hpack_idx: Option<std::num::NonZeroU8>
}

impl HeaderName {
    /// Parse header name from static bytes.
    ///
    /// The input must not contains ASCII uppercase characters.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid header name or contains ASCII uppercase characters.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_header_name_lowercase(bytes) {
            Ok(()) => Self {
                repr: Repr::Arbitrary(Bytes::from_static(bytes)),
            },
            Err(err) => err.panic_const(),
        }
    }

    /// Parse header name from [`Bytes`].
    ///
    /// The input must not contains ASCII uppercase characters.
    ///
    /// For more flexible API use [`HeaderName::from_slice`].
    ///
    /// # Errors
    ///
    /// Returns error if the input is not a valid header name or contains ASCII uppercase
    /// characters.
    #[inline]
    pub fn from_bytes_lowercase<B: Into<Bytes>>(name: B) -> Result<Self, HeaderError> {
        let name = name.into();
        match validate_header_name_lowercase(name.as_slice()) {
            Ok(()) => Ok(Self {
                repr: Repr::Arbitrary(name),
            }),
            Err(err) => Err(err),
        }
    }

    pub(crate) fn from_internal(name: BytesMut) -> Result<(Self, u32), HeaderError> {
        if matches!(name.len(), 1..=MAX_HEADER_NAME_LEN) {
            internal_to_header_name(name)
        } else {
            Err(HeaderError::invalid_len(name.len()))
        }
    }

    /// Parse header name by copying from slice of bytes.
    ///
    /// Input name is normalized to lowercase.
    ///
    /// # Errors
    ///
    /// Returns error if the input is not a valid header name.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(name: A) -> Result<Self, HeaderError> {
        let bytes = name.as_ref();
        if matches!(bytes.len(), 1..=MAX_HEADER_NAME_LEN) {
            copy_to_header_name(bytes)
        } else {
            Err(HeaderError::invalid_len(bytes.len()))
        }
    }

    /// Extracts a string slice of the header name.
    ///
    /// The returned string will always in ASCII lowercase.
    #[inline]
    pub const fn as_str(&self) -> &str {
        match &self.repr {
            Repr::Static(s) => s.string,
            Repr::Arbitrary(bytes) => unsafe { str::from_utf8_unchecked(bytes.as_slice()) },
        }
    }

    /// Checks that two header name are an ASCII case-insensitive match.
    ///
    /// Header names are case-insensitive.
    #[inline]
    pub const fn eq_ignore_ascii_case(&self, name: &str) -> bool {
        self.as_str().eq_ignore_ascii_case(name)
    }

    pub(crate) const fn hash(&self) -> u32 {
        match &self.repr {
            Repr::Static(s) => s.hash,
            Repr::Arbitrary(bytes) => matches::hash_32(bytes.as_slice()),
        }
    }

    pub(crate) const fn validate_lowercase(s: &[u8]) {
        if let Err(err) = validate_header_name_lowercase(s) {
            err.panic_const();
        }
    }

    /// Returns hpack static header index if any.
    ///
    /// Note that this value only available in constant headers.
    pub(crate) const fn hpack_static(&self) -> Option<std::num::NonZero<u8>> {
        match &self.repr {
            Repr::Static(s) => s.hpack_idx,
            Repr::Arbitrary(_) => None,
        }
    }
}

// ===== Parser =====

const MAX_HEADER_NAME_LEN: usize = 1024;  // 1KB

/// token       = 1*tchar
/// field-name  = token
const fn validate_header_name_lowercase(mut bytes: &[u8]) -> Result<(), HeaderError> {
    use HeaderError as E;

    if !matches!(bytes.len(), 1..=MAX_HEADER_NAME_LEN) {
        return Err(E::invalid_len(bytes.len()));
    }

    while let [byte, rest @ ..] = bytes {
        if matches::is_token_lowercase(*byte) {
            bytes = rest;
        } else {
            return Err(E::Invalid)
        }
    }

    Ok(())
}

fn copy_to_header_name(bytes: &[u8]) -> Result<HeaderName, HeaderError> {
    use HeaderError as E;

    let mut name = vec![0; bytes.len()];

    for (output, input) in name.iter_mut().zip(bytes) {
        *output = matches::HEADER_NAME[*input as usize];

        // Any invalid character will have it MSB set
        if *output & 128 == 128 {
            return Err(E::Invalid);
        }
    }

    Ok(HeaderName {
        repr: Repr::Arbitrary(name.into()),
    })
}

fn internal_to_header_name(mut bytes: BytesMut) -> Result<(HeaderName, u32), HeaderError> {
    use HeaderError as E;

    const BASIS: u32 = 0x811C_9DC5;
    const PRIME: u32 = 0x0100_0193;

    let mut hash = BASIS;

    for byte in bytes.as_mut_slice() {
        *byte = matches::HEADER_NAME[*byte as usize];
        hash = PRIME.wrapping_mul(hash ^ *byte as u32);

        // Any invalid character will have it MSB set
        if *byte & 128 == 128 {
            return Err(E::Invalid);
        }
    }

    Ok((
        HeaderName {
            repr: Repr::Arbitrary(bytes.freeze()),
        },
        hash,
    ))
}

// ===== Traits =====

impl std::fmt::Display for HeaderName {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        str::fmt(self.as_str(), f)
    }
}

impl std::fmt::Debug for HeaderName {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HeaderName").field(&self.as_str()).finish()
    }
}

impl std::hash::Hash for HeaderName {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u32(self.hash());
    }
}

impl PartialEq for HeaderName {
    fn eq(&self, other: &Self) -> bool {
        // HeaderName is guaranteed to have ascii lowercase value,
        // therefore it is correct for case-insensitive eq
        self.as_str() == other.as_str()
    }
}

// ===== Standard Headers =====

// https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers

standard_header! {
    /// HTTP Standard Headers
    mod standard;

    // ===== Pseudo Headers =====

    pub(crate) const PSEUDO_AUTHORITY: HeaderName = ":authority", hpack_idx: 1;
    pub(crate) const PSEUDO_METHOD: HeaderName = ":method", hpack_idx: 2;
    pub(crate) const PSEUDO_PATH: HeaderName = ":path", hpack_idx: 4;
    pub(crate) const PSEUDO_SCHEME: HeaderName = ":scheme", hpack_idx: 6;
    pub(crate) const PSEUDO_STATUS: HeaderName = ":status", hpack_idx: 8;

    // ===== Authentication =====

    /// Defines the authentication method that should be used to access a resource.
    pub const WWW_AUTHENTICATE: HeaderName = "www-authenticate", hpack_idx: 61;

    /// Contains the credentials to authenticate a user-agent with a server.
    pub const AUTHORIZATION: HeaderName = "authorization", hpack_idx: 23;

    /// Defines the authentication method that should be used to access a resource behind a proxy
    /// server.
    pub const PROXY_AUTHENTICATE: HeaderName = "proxy-authenticate", hpack_idx: 48;

    /// Contains the credentials to authenticate a user agent with a proxy server.
    pub const PROXY_AUTHORIZATION: HeaderName = "proxy-authorization", hpack_idx: 49;

    // ===== Caching =====

    /// The time, in seconds, that the object has been in a proxy cache.
    pub const AGE: HeaderName = "age", hpack_idx: 21;

    /// Directives for caching mechanisms in both requests and responses.
    pub const CACHE_CONTROL: HeaderName = "cache-control", hpack_idx: 24;

    /// Clears browsing data (e.g., cookies, storage, cache) associated with the requesting website.
    pub const CLEAR_SITE_DATA: HeaderName = "clear-site-data";

    /// The date/time after which the response is considered stale.
    pub const EXPIRES: HeaderName = "expires", hpack_idx: 36;

    // ===== Conditionals =====

    /// The last modification date of the resource, used to compare several versions of the same
    /// resource. It is less accurate than ETag, but easier to calculate in some environments.
    /// Conditional requests using If-Modified-Since and If-Unmodified-Since use this value to
    /// change the behavior of the request.
    pub const LAST_MODIFIED: HeaderName = "last-modified", hpack_idx: 44;

    /// A unique string identifying the version of the resource. Conditional requests using
    /// If-Match and If-None-Match use this value to change the behavior of the request.
    pub const ETAG: HeaderName = "etag", hpack_idx: 34;

    /// Makes the request conditional, and applies the method only if the stored resource matches
    /// one of the given ETags.
    pub const IF_MATCH: HeaderName = "if-match", hpack_idx: 39;

    /// Makes the request conditional, and applies the method only if the stored resource doesn't
    /// match any of the given ETags. This is used to update caches (for safe requests), or to
    /// prevent uploading a new resource when one already exists.
    pub const IF_NONE_MATCH: HeaderName = "if-none-match", hpack_idx: 41;

    /// Makes the request conditional, and expects the resource to be transmitted only if it has
    /// been modified after the given date. This is used to transmit data only when the cache is
    /// out of date.
    pub const IF_MODIFIED_SINCE: HeaderName = "if-modified-since", hpack_idx: 40;

    /// Makes the request conditional, and expects the resource to be transmitted only if it has
    /// not been modified after the given date. This ensures the coherence of a new fragment of a
    /// specific range with previous ones, or to implement an optimistic concurrency control system
    /// when modifying existing documents.
    pub const IF_UNMODIFIED_SINCE: HeaderName = "if-unmodified-since", hpack_idx: 43;

    /// Determines how to match request headers to decide whether a cached response can be used
    /// rather than requesting a fresh one from the origin server.
    pub const VARY: HeaderName = "vary", hpack_idx: 59;

    // ===== Connection management =====

    /// Controls whether the network connection stays open after the current transaction finishes.
    pub const CONNECTION: HeaderName = "connection";

    /// Controls how long a persistent connection should stay open.
    pub const KEEP_ALIVE: HeaderName = "keep-alive";

    // ===== Content negotiation =====
    // more details on [mdn]<https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Content_negotiation>

    /// Informs the server about the types of data that can be sent back.
    pub const ACCEPT: HeaderName = "accept", hpack_idx: 19;

    /// The "Accept-Charset" header field can be sent by a user agent to indicate its preferences
    /// for charsets in textual response content.
    pub const ACCEPT_CHARSET: HeaderName = "accept-charset", hpack_idx: 15;

    /// The encoding algorithm, usually a compression algorithm, that can be used on the resource
    /// sent back.
    pub const ACCEPT_ENCODING: HeaderName = "accept-encoding", hpack_idx: 16;

    /// Informs the server about the human language the server is expected to send back. This is a
    /// hint and is not necessarily under the full control of the user: the server should always
    /// pay attention not to override an explicit user choice (like selecting a language from a
    /// dropdown).
    pub const ACCEPT_LANGUAGE: HeaderName = "accept-language", hpack_idx: 17;

    /// A request content negotiation response header that advertises which media type the server
    /// is able to understand in a PATCH request.
    pub const ACCEPT_PATCH: HeaderName = "accept-patch";

    /// A request content negotiation response header that advertises which media type the server
    /// is able to understand in a POST request.
    pub const ACCEPT_POST: HeaderName = "accept-post";

    // ===== Controls =====

    /// Indicates expectations that need to be fulfilled by the server to properly handle the
    /// request.
    pub const EXPECT: HeaderName = "expect", hpack_idx: 35;

    /// When using TRACE, indicates the maximum number of hops the request can do before being
    /// reflected to the sender.
    pub const MAX_FORWARDS: HeaderName = "max-forwards", hpack_idx: 47;

    // ===== Cookies =====

    /// Contains stored HTTP cookies previously sent by the server with the Set-Cookie header.
    pub const COOKIE: HeaderName = "cookie", hpack_idx: 32;

    /// Send cookies from the server to the user-agent.
    pub const SET_COOKIE: HeaderName = "set-cookie", hpack_idx: 55;

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
    pub const ACCESS_CONTROL_ALLOW_ORIGIN: HeaderName = "access-control-allow-origin", hpack_idx: 20;

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
    pub const CONTENT_DISPOSITION: HeaderName = "content-disposition", hpack_idx: 25;

    // ===== Message body information =====

    /// The size of the resource, in decimal number of bytes.
    pub const CONTENT_LENGTH: HeaderName = "content-length", hpack_idx: 28;

    /// Indicates the media type of the resource.
    pub const CONTENT_TYPE: HeaderName = "content-type", hpack_idx: 31;

    /// Used to specify the compression algorithm.
    pub const CONTENT_ENCODING: HeaderName = "content-encoding", hpack_idx: 26;

    /// Describes the human language(s) intended for the audience, so that it allows a user to
    /// differentiate according to the users' own preferred language.
    pub const CONTENT_LANGUAGE: HeaderName = "content-language", hpack_idx: 27;

    /// Indicates an alternate location for the returned data.
    pub const CONTENT_LOCATION: HeaderName = "content-location", hpack_idx: 29;

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
    pub const VIA: HeaderName = "via", hpack_idx: 60;

    // ===== Range requests =====

    /// Indicates if the server supports range requests, and if so in which unit the range can be
    /// expressed.
    pub const ACCEPT_RANGES: HeaderName = "accept-ranges", hpack_idx: 18;

    /// Indicates the part of a document that the server should return.
    pub const RANGE: HeaderName = "range", hpack_idx: 50;

    /// Creates a conditional range request that is only fulfilled if the given etag or date
    /// matches the remote resource. Used to prevent downloading two ranges from incompatible
    /// version of the resource.
    pub const IF_RANGE: HeaderName = "if-range", hpack_idx: 42;

    /// Indicates where in a full body message a partial message belongs.
    pub const CONTENT_RANGE: HeaderName = "content-range", hpack_idx: 30;

    // ===== Redirects =====

    /// Indicates the URL to redirect a page to.
    pub const LOCATION: HeaderName = "location", hpack_idx: 46;

    /// Directs the browser to reload the page or redirect to another. Takes the same value as the
    /// meta element with http-equiv="refresh".
    pub const REFRESH: HeaderName = "refresh", hpack_idx: 52;

    // ===== Web Linking =====

    /// The HTTP Link header provides a means for serializing one or more links in HTTP headers.
    pub const LINK: HeaderName = "link", hpack_idx: 45;

    // ===== Request context =====

    /// Contains an Internet email address for a human user who controls the requesting user agent.
    pub const FROM: HeaderName = "from", hpack_idx: 37;

    /// Specifies the domain name of the server (for virtual hosting), and (optionally) the TCP
    /// port number on which the server is listening.
    pub const HOST: HeaderName = "host", hpack_idx: 38;

    /// The address of the previous web page from which a link to the currently requested page was
    /// followed.
    pub const REFERER: HeaderName = "referer", hpack_idx: 51;

    /// Governs which referrer information sent in the Referer header should be included with
    /// requests made.
    pub const REFERRER_POLICY: HeaderName = "referrer-policy";

    /// Contains a characteristic string that allows the network protocol peers to identify the
    /// application type, operating system, software vendor or software version of the requesting
    /// software user agent.
    pub const USER_AGENT: HeaderName = "user-agent", hpack_idx: 58;

    // ===== Response context =====

    /// Lists the set of HTTP request methods supported by a resource.
    pub const ALLOW: HeaderName = "allow", hpack_idx: 22;

    /// Contains information about the software used by the origin server to handle the request.
    pub const SERVER: HeaderName = "server", hpack_idx: 54;

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
    pub const STRICT_TRANSPORT_SECURITY: HeaderName = "strict-transport-security", hpack_idx: 56;

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
    pub const TRANSFER_ENCODING: HeaderName = "transfer-encoding", hpack_idx: 57;

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
    pub const DATE: HeaderName = "date", hpack_idx: 33;

    /// Indicates how long the user agent should wait before making a follow-up request.
    pub const RETRY_AFTER: HeaderName = "retry-after", hpack_idx: 53;

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
    (@HPACK_IDX , hpack_idx: $idx:literal) => {std::num::NonZeroU8::new($idx)};
    (@HPACK_IDX) => {None};

    // ===== CORE =====

    (@CORE
        $(
            $(#[$doc:meta])*
            $vis:vis const $id:ident: $t:ty = $name:literal $(, $k:ident:$v:expr)*;
        )*
    ) => {
        $(
            $(#[$doc])*
            $vis const $id: $t = {
                static $id: Static = Static {
                    string: $name,
                    hash: matches::hash_32($name.as_bytes()),
                    hpack_idx: standard_header! { @HPACK_IDX $(, $k:$v)* },
                };

                HeaderName {
                    repr: Repr::Static(&$id)
                }
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
