use std::num::NonZeroU16;

/// HTTP [Status Code][rfc].
///
/// [rfc]: <https://datatracker.ietf.org/doc/html/rfc9110#name-status-codes>
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(NonZeroU16);

impl Default for StatusCode {
    #[inline]
    fn default() -> Self {
        Self::OK
    }
}

macro_rules! status_code_v3 {
    (
        $(
            $(#[$doc:meta])*
            $int:literal $id:ident $msg:literal;
        )*
    ) => {
        impl StatusCode {
            /// Returns status code value, e.g: `200`.
            #[inline]
            pub const fn status(&self) -> u16 {
                self.0.get()
            }

            /// Returns status code and message as string slice, e.g: `"200 OK"`.
            #[inline]
            pub const fn as_str(&self) -> &'static str {
                match self.0.get() {
                    $(
                        $int => concat!(stringify!($int)," ",$msg),
                    )*
                    // SAFETY: StatusCode value is privately constructed and immutable
                    _ => unsafe { std::hint::unreachable_unchecked() },
                }
            }

            /// Returns status code as str, e.g: `"200"`.
            #[inline]
            pub const fn status_str(&self) -> &'static str {
                match self.0.get() {
                    $(
                        $int => stringify!($int),
                    )*
                    // SAFETY: StatusCode value is privately constructed and immutable
                    _ => unsafe { std::hint::unreachable_unchecked() },
                }
            }

            /// Returns status message, e.g: `"OK"`.
            #[inline]
            pub const fn message(&self) -> &'static str {
                match self.0.get() {
                    $(
                        $int => $msg,
                    )*
                    // SAFETY: StatusCode value is privately constructed and immutable
                    _ => unsafe { std::hint::unreachable_unchecked() },
                }
            }
        }

        impl StatusCode {
            $(
                $(#[$doc])*
                pub const $id: Self = Self(NonZeroU16::new($int).unwrap());
            )*
        }
    };
}

status_code_v3! {
    /// `101`, This code is sent in response to an `Upgrade` request header from the client and indicates
    /// the protocol the server is switching to.
    101 SWITCHING_PROTOCOL "Switching Protocols";
    /// `200`. The request succeeded.
    200 OK "OK";
    /// `201`. The request succeeded, and a new resource was created as a result.
    201 CREATED "Created";
    /// `204`. There is no content to send for this request, but the headers are useful.
    204 NO_CONTENT "No Content";
    /// `302`. This response code means that the URI of requested resource has been changed temporarily.
    302 FOUND "Found";
    /// `303`. The server sent this response to direct the client to get the requested resource at another
    /// URI with a GET request.
    303 SEE_OTHER "See Other";
    /// `304`. This is used for caching purposes. It tells the client that the response has not been
    /// modified, so the client can continue to use the same cached version of the response.
    304 NOT_MODIFIED "Not Modified";
    /// `307`. The server sends this response to direct the client to get the requested resource at
    /// another URI with the same method that was used in the prior request.
    307 TEMPORARY_REDIRECT "Temporary Redirect";
    /// `400`. The server cannot or will not process the request due to something that is perceived to be
    /// a client error.
    400 BAD_REQUEST "Bad Request";
    /// `401`. Although the HTTP standard specifies "unauthorized", semantically this response means
    /// "unauthenticated".
    401 UNAUTHORIZED "Unauthorized";
    /// `403`. The client's identity is known to the server, but client does not have access rights
    /// to the content.
    403 FORBIDDEN "Forbidden";
    /// `404`. The server cannot find the requested resource.
    404 NOT_FOUND "Not Found";
    /// `405`. The request method is known by the server but is not supported by the target resource.
    405 METHOD_NOT_ALLOWED "Method Not Allowed";
    /// `406`. This response is sent when the web server, after performing server-driven content
    /// negotiation, doesn't find any content that conforms to the criteria given by the user
    /// agent.
    406 NOT_ACCEPTABLE "Not Acceptable";
    /// `408`. This response is sent on an idle connection by some servers, even without any previous
    /// request by the client. It means that the server would like to shut down this unused
    /// connection.
    408 REQUEST_TIMEOUT "Request Timeout";
    /// `411`. Server rejected the request because the `Content-Length` header field is not defined and the
    /// server requires it.
    411 LENGTH_REQUIRED "Length Required";
    /// `412`. In conditional requests, the client has indicated preconditions in its headers which the
    /// server does not meet.
    412 PRECONDITION_FAILED "Precondition Failed";
    /// `413`. The request body is larger than limits defined by server. The server might close the
    /// connection or return an `Retry-After` header field.
    413 CONTENT_TOO_LARGE "Content Too Large";
    /// `414`. The URI requested by the client is longer than the server is willing to interpret.
    414 URI_TOO_LONG "URI Too Long";
    /// `415`. The media format of the requested data is not supported by the server, so the server is
    /// rejecting the request.
    415 UNSUPPORTED_MEDIA_TYPE "Unsupported Media Type";
    /// `416`. The ranges specified by the `Range` header field in the request cannot be fulfilled. It's
    /// possible that the range is outside the size of the target resource's data.
    416 RANGE_NOT_SATISFIABLE "Range Not Satisfiable";
    /// `417`. This response code means the expectation indicated by the `Expect` request header field
    /// cannot be met by the server.
    417 EXPECTATION_FAILED "Expectation Failed";
    /// `418`. The server refuses the attempt to brew coffee with a teapot.
    418 IM_A_TEAPOT "I'm a teapot";
    /// `429`. The user has sent too many requests in a given amount of time ([rate limiting][1]).
    ///
    /// [1]: <https://developer.mozilla.org/en-US/docs/Glossary/Rate_limit>
    429 TOO_MANY_REQUESTS "Too Many Requests";
    /// `431`. The server is unwilling to process the request because its header fields are too large. The
    /// request may be resubmitted after reducing the size of the request header fields.
    431 REQUEST_HEADER_FIELDS_TOO_LARGE "Request Header Fields Too Large";
    /// `500`. The server has encountered a situation it does not know how to handle. This error is
    /// generic, indicating that the server cannot find a more appropriate 5XX status code to
    /// respond with.
    500 INTERNAL_SERVER_ERROR "Internal Server Error";
    /// `501`. The request method is not supported by the server and cannot be handled. The only methods
    /// that servers are required to support (and therefore that must not return this code) are GET
    /// and HEAD.
    501 NOT_IMPLEMENTED "Not Implemented";
    /// `502`. This error response means that the server, while working as a gateway to get a response
    /// needed to handle the request, got an invalid response.
    502 BAD_GATEWAY "Bad Gateway";
    /// `503`. The server is not ready to handle the request.
    ///
    /// Common causes are a server that is down for maintenance or that is overloaded. Note that
    /// together with this response, a user-friendly page explaining the problem should be sent.
    503 SERVICE_UNAVAILABLE "Service Unavailable";
    /// `504`. This error response is given when the server is acting as a gateway and cannot get a
    /// response in time.
    504 GATEWAY_TIMEOUT "Gateway Timeout";
    /// `505`. The HTTP version used in the request is not supported by the server.
    505 HTTP_VERSION_NOT_SUPPORTED "HTTP Version Not Supported";
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Debug for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("StatusCode").field(&self.as_str()).finish()
    }
}

