use std::num::NonZeroU16;

/// HTTP Status Code.
///
/// Status code is a three-digit integer code in a response that describes the result of the
/// request and the semantics of the response, including whether the request was successful and
/// what content is enclosed (if any).
///
/// This API supports status codes defined in [RFC9110] and [RFC6585], [451 (Unavailable For Legal
/// Reasons)][RFC7725], and [103 (Early Hints)][RFC8297]
///
/// [RFC9110]: <https://www.rfc-editor.org/rfc/rfc9110#name-status-codes>
/// [RFC6585]: <https://www.rfc-editor.org/rfc/rfc6585>
/// [RFC7725]: <https://www.rfc-editor.org/rfc/rfc7725>
/// [RFC8297]: <https://www.rfc-editor.org/rfc/rfc8297>
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(NonZeroU16);

impl Default for StatusCode {
    #[inline]
    fn default() -> Self {
        Self::OK
    }
}

impl StatusCode {
    /// Returns status code value as `u16`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tsue::http::StatusCode;
    ///
    /// let status = StatusCode::OK;
    /// assert_eq!(status.as_u16(), 200);
    /// ```
    #[inline]
    pub const fn as_u16(&self) -> u16 {
        self.0.get()
    }

    const fn string(&self) -> (usize, usize) {
        unsafe {
            let index = status_to_index(self.0.get()) as usize;

            // SAFETY: valid status will always result in bounds index
            let end = (*TABLE.as_ptr().add(index)).1 as usize;

            // SAFETY: lowest status (100) will not resulting in index 0
            // there always previous index
            let offset = (*TABLE.as_ptr().add(index - 1)).1 as usize;

            (offset, end)
        }
    }

    /// Returns status code value as str.
    ///
    /// # Examples
    ///
    /// ```
    /// use tsue::http::StatusCode;
    ///
    /// let status = StatusCode::OK;
    /// assert_eq!(status.code_str(), "200");
    /// ```
    #[inline]
    pub const fn code_str(&self) -> &'static str {
        let (offset, _) = self.string();
        unsafe {
            str::from_utf8_unchecked(std::slice::from_raw_parts(REASONS.as_ptr().add(offset), 3))
        }
    }

    /// Returns status code reason as str.
    ///
    /// # Examples
    ///
    /// ```
    /// use tsue::http::StatusCode;
    ///
    /// let status = StatusCode::OK;
    /// assert_eq!(status.reason(), "OK");
    /// ```
    #[inline]
    pub const fn reason(&self) -> &'static str {
        let (offset, end) = self.string();
        unsafe {
            str::from_utf8_unchecked(std::slice::from_raw_parts(
                REASONS.as_ptr().add(offset + 4),
                end - offset - 4,
            ))
        }
    }

    /// Returns status code and message as string.
    ///
    /// # Examples
    ///
    /// ```
    /// use tsue::http::StatusCode;
    ///
    /// let status = StatusCode::OK;
    /// assert_eq!(status.as_str(), "200 OK");
    /// ```
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        let (offset, end) = self.string();
        unsafe {
            str::from_utf8_unchecked(std::slice::from_raw_parts(
                REASONS.as_ptr().add(offset),
                end - offset,
            ))
        }
    }

    /// Returns `true` is status code class is informational.
    ///
    /// Informational means the request was received, continuing process.
    #[inline]
    pub const fn is_informational(&self) -> bool {
        self.0.get() / 100 == 1
    }

    /// Returns `true` is status code class is successful.
    ///
    /// Successful means the request was successfully received, understood, and accepted.
    #[inline]
    pub const fn is_successful(&self) -> bool {
        self.0.get() / 100 == 2
    }

    /// Returns `true` is status code class is redirection.
    ///
    /// Redirection means further action needs to be taken in order to complete the request.
    #[inline]
    pub const fn is_redirection(&self) -> bool {
        self.0.get() / 100 == 3
    }

    /// Returns `true` is status code class is client error.
    ///
    /// Client error means the request contains bad syntax or cannot be fulfilled.
    #[inline]
    pub const fn is_client_error(&self) -> bool {
        self.0.get() / 100 == 4
    }

    /// Returns `true` is status code class is server error.
    ///
    /// Server error means the server failed to fulfill an apparently valid request
    #[inline]
    pub const fn is_server_error(&self) -> bool {
        self.0.get() / 100 == 5
    }
}

const fn status_to_index(status: u16) -> u16 {
    // 100 to 300 status have at most 9 elements each, divide the first `9 * 3` of the table
    ((status / 100) * 9)
    // but 400 status have 32 elements, so for 500 status shift forward the index more to make
    // space for excess 400 status
    + ((status / 500) * 23)
    + (status % 100)
    // 500 status have more than 9 elements, but its fine because there is no 600 status and still
    // in bounds of the table
}

macro_rules! status_code_v4 {
    (
        $(
            $(#[$doc:meta])*
            $int:literal $id:ident $msg:literal;
        )*
    ) => {
        impl StatusCode {
            $(
                $(#[$doc])*
                pub const $id: Self = Self(NonZeroU16::new($int).unwrap());
            )*
        }

        static REASONS: &[u8] = concat!($(concat!(stringify!($int)," ",$msg)),*).as_bytes();

        /// `(status, reason_len)`
        static TABLE: [(u16,u16); 80] = {
            let values = [$(($int,$msg)),*];
            let mut table = [(0,0); 80];
            table[0] = (0,0);
            let mut table_i = 1;
            let mut values_i = 0;

            while table_i < table.len() {
                let (status, reason) = values[values_i];
                let idx = status_to_index(status);

                if table_i != idx as usize {
                    // the padding filled with closest previous value
                    table[table_i] = table[table_i - 1];
                } else {
                    let reason_len = table[table_i - 1].1 + reason.len() as u16 + 4;
                    table[table_i] = (status, reason_len);
                    values_i += 1;
                }

                table_i += 1;
            }
            table
        };

        #[cfg(test)]
        static TEST_STATUS: [(StatusCode, &str); 48] = [
            $((StatusCode(unsafe { NonZeroU16::new_unchecked($int) }), $msg)),*
        ];
    }
}

status_code_v4! {
    /// The 100 (Continue) status code indicates that the initial part of a request has been
    /// received and has not yet been rejected by the server. The server intends to send a final
    /// response after the request has been fully received and acted upon.
    100 CONTINUE "Continue";
    /// The 101 (Switching Protocols) status code indicates that the server understands and is
    /// willing to comply with the client's request, via the Upgrade header field, for a change in
    /// the application protocol being used on this connection.
    101 SWITCHING_PROTOCOL "Switching Protocols";
    /// The 103 (Early Hints) informational status code indicates to the client that the server is
    /// likely to send a final response with the header fields included in the informational
    /// response.
    103 EARLY_HINTS "Early Hints";
    /// The 200 (OK) status code indicates that the request has succeeded.
    200 OK "OK";
    /// The 201 (Created) status code indicates that the request has been fulfilled and has
    /// resulted in one or more new resources being created.
    201 CREATED "Created";
    /// The 202 (Accepted) status code indicates that the request has been accepted for processing,
    /// but the processing has not been completed.
    202 ACCEPTED "Accepted";
    /// The 203 (Non-Authoritative Information) status code indicates that the request was
    /// successful but the enclosed content has been modified from that of the origin server's 200
    /// (OK) response by a transforming proxy.
    203 NON_AUTHORATIVE_INFORMATION "Non-Authoritative Information";
    /// The 204 (No Content) status code indicates that the server has successfully fulfilled the
    /// request and that there is no additional content to send in the response content.
    204 NO_CONTENT "No Content";
    /// The 205 (Reset Content) status code indicates that the server has fulfilled the request and
    /// desires that the user agent reset the "document view", which caused the request to be sent,
    /// to its original state as received from the origin server.
    205 RESET_CONTENT "Reset Content";
    /// The 206 (Partial Content) status code indicates that the server is successfully fulfilling
    /// a range request for the target resource by transferring one or more parts of the selected
    /// representation.
    206 PARTIAL_CONTENT "Partial Content";
    /// The 300 (Multiple Choices) status code indicates that the target resource has more than one
    /// representation, each with its own more specific identifier, and information about the
    /// alternatives is being provided so that the user (or user agent) can select a preferred
    /// representation by redirecting its request to one or more of those identifiers.
    300 MULTIPLE_CHOICES "Multiple Choices";
    /// The 301 (Moved Permanently) status code indicates that the target resource has been
    /// assigned a new permanent URI and any future references to this resource ought to use one of
    /// the enclosed URIs.
    301 MOVED_PERMANENTLY "Moved Permanently";
    /// The 302 (Found) status code indicates that the target resource resides temporarily under a
    /// different URI.
    302 FOUND "Found";
    /// The 303 (See Other) status code indicates that the server is redirecting the user agent to
    /// a different resource, as indicated by a URI in the Location header field, which is intended
    /// to provide an indirect response to the original request
    303 SEE_OTHER "See Other";
    /// The 304 (Not Modified) status code indicates that a conditional GET or HEAD request has
    /// been received and would have resulted in a 200 (OK) response if it were not for the fact
    /// that the condition evaluated to false.
    304 NOT_MODIFIED "Not Modified";
    /// The 307 (Temporary Redirect) status code indicates that the target resource resides
    /// temporarily under a different URI and the user agent MUST NOT change the request method if
    /// it performs an automatic redirection to that URI.
    307 TEMPORARY_REDIRECT "Temporary Redirect";
    /// The 308 (Permanent Redirect) status code indicates that the target resource has been
    /// assigned a new permanent URI and any future references to this resource ought to use one of
    /// the enclosed URIs.
    308 PERMANENT_REDIRECT "Permanent Redirect";
    /// The 400 (Bad Request) status code indicates that the server cannot or will not process the
    /// request due to something that is perceived to be a client error (e.g., malformed request
    /// syntax, invalid request message framing, or deceptive request routing).
    400 BAD_REQUEST "Bad Request";
    /// The 401 (Unauthorized) status code indicates that the request has not been applied because
    /// it lacks valid authentication credentials for the target resource.
    401 UNAUTHORIZED "Unauthorized";
    // The 402 (Payment Required) status code is reserved for future use.
    /// The 403 (Forbidden) status code indicates that the server understood the request but
    /// refuses to fulfill it.
    403 FORBIDDEN "Forbidden";
    /// The 404 (Not Found) status code indicates that the origin server did not find a current
    /// representation for the target resource or is not willing to disclose that one exists.
    404 NOT_FOUND "Not Found";
    /// The 405 (Method Not Allowed) status code indicates that the method received in the
    /// request-line is known by the origin server but not supported by the target resource.
    405 METHOD_NOT_ALLOWED "Method Not Allowed";
    /// The 406 (Not Acceptable) status code indicates that the target resource does not have a
    /// current representation that would be acceptable to the user agent, according to the
    /// proactive negotiation header fields received in the request, and the server is unwilling to
    /// supply a default representation.
    406 NOT_ACCEPTABLE "Not Acceptable";
    /// The 407 (Proxy Authentication Required) status code is similar to 401 (Unauthorized), but
    /// it indicates that the client needs to authenticate itself in order to use a proxy for this
    /// request.
    407 PROXY_AUTHENTICATION_REQUIRED "Proxy Authentication Required";
    /// The 408 (Request Timeout) status code indicates that the server did not receive a complete
    /// request message within the time that it was prepared to wait.
    408 REQUEST_TIMEOUT "Request Timeout";
    /// The 409 (Conflict) status code indicates that the request could not be completed due to a
    /// conflict with the current state of the target resource.
    409 CONFLICT "Conflict";
    /// The 410 (Gone) status code indicates that access to the target resource is no longer
    /// available at the origin server and that this condition is likely to be permanent.
    410 GONE "Gone";
    /// The 411 (Length Required) status code indicates that the server refuses to accept the
    /// request without a defined Content-Length.
    411 LENGTH_REQUIRED "Length Required";
    /// The 412 (Precondition Failed) status code indicates that one or more conditions given in
    /// the request header fields evaluated to false when tested on the server.
    412 PRECONDITION_FAILED "Precondition Failed";
    /// The 413 (Content Too Large) status code indicates that the server is refusing to process a
    /// request because the request content is larger than the server is willing or able to
    /// process.
    413 CONTENT_TOO_LARGE "Content Too Large";
    /// The 414 (URI Too Long) status code indicates that the server is refusing to service the
    /// request because the target URI is longer than the server is willing to interpret.
    414 URI_TOO_LONG "URI Too Long";
    /// The 415 (Unsupported Media Type) status code indicates that the origin server is refusing
    /// to service the request because the content is in a format not supported by this method on
    /// the target resource.
    415 UNSUPPORTED_MEDIA_TYPE "Unsupported Media Type";
    /// The 416 (Range Not Satisfiable) status code indicates that the set of ranges in the
    /// request's Range header field has been rejected either because none of the requested ranges
    /// are satisfiable or because the client has requested an excessive number of small or
    /// overlapping ranges (a potential denial of service attack).
    416 RANGE_NOT_SATISFIABLE "Range Not Satisfiable";
    /// The 417 (Expectation Failed) status code indicates that the expectation given in the
    /// request's Expect header field could not be met by at least one of the inbound servers.
    417 EXPECTATION_FAILED "Expectation Failed";
    /// The 418 (I'm a teapot) status code indicates that the server refuses the attempt to brew
    /// coffee with a teapot.
    418 IM_A_TEAPOT "I'm a teapot";
    /// The 421 (Misdirected Request) status code indicates that the request was directed at a
    /// server that is unable or unwilling to produce an authoritative response for the target URI.
    421 MISDIRECTED_REQUEST "Misdirected Request";
    /// The 422 (Unprocessable Content) status code indicates that the server understands the
    /// content type of the request content (hence a 415 (Unsupported Media Type) status code is
    /// inappropriate), and the syntax of the request content is correct, but it was unable to
    /// process the contained instructions.
    422 UNPROCESSABLE_CONTENT "Unprocessable Content";
    /// The 426 (Upgrade Required) status code indicates that the server refuses to perform the
    /// request using the current protocol but might be willing to do so after the client upgrades
    /// to a different protocol.
    426 UPGRADE_REQUIRED "Upgrade Required";
    /// The 428 (Precondition Required) status code indicates that the origin server requires the
    /// request to be conditional.
    428 PRECONDITION_REQUIRED "Precondition Required";
    /// The 429 (Too Many Requests) status code indicates that the user has sent too many requests
    /// in a given amount of time ("rate limiting").
    429 TOO_MANY_REQUESTS "Too Many Requests";
    /// The 431 (Request Header Fields Too Large) status code indicates that the server is
    /// unwilling to process the request because its header fields are too large.
    431 REQUEST_HEADER_FIELDS_TOO_LARGE "Request Header Fields Too Large";
    /// The 500 (Internal Server Error) status code indicates that the server encountered an
    /// unexpected condition that prevented it from fulfilling the request.
    500 INTERNAL_SERVER_ERROR "Internal Server Error";
    /// The 501 (Not Implemented) status code indicates that the server does not support the
    /// functionality required to fulfill the request.
    501 NOT_IMPLEMENTED "Not Implemented";
    /// The 502 (Bad Gateway) status code indicates that the server, while acting as a gateway or
    /// proxy, received an invalid response from an inbound server it accessed while attempting to
    /// fulfill the request.
    502 BAD_GATEWAY "Bad Gateway";
    /// The 503 (Service Unavailable) status code indicates that the server is currently unable to
    /// handle the request due to a temporary overload or scheduled maintenance, which will likely
    /// be alleviated after some delay.
    503 SERVICE_UNAVAILABLE "Service Unavailable";
    /// The 504 (Gateway Timeout) status code indicates that the server, while acting as a gateway
    /// or proxy, did not receive a timely response from an upstream server it needed to access in
    /// order to complete the request.
    504 GATEWAY_TIMEOUT "Gateway Timeout";
    /// The 505 (HTTP Version Not Supported) status code indicates that the server does not
    /// support, or refuses to support, the major version of HTTP that was used in the request
    /// message.
    505 HTTP_VERSION_NOT_SUPPORTED "HTTP Version Not Supported";
    /// The 511 (Network Authentication Required) status code indicates that the client needs to
    /// authenticate to gain network access.
    511 NETWORK_AUTHENTICATION_REQUIRED "Network Authentication Required";
}

// ===== std traits =====

impl std::fmt::Debug for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("StatusCode").field(&self.as_str()).finish()
    }
}

// ===== tests =====

#[test]
fn test_status_code() {
    assert_eq!(StatusCode::SWITCHING_PROTOCOL.reason(), "Switching Protocols");
    assert_eq!(StatusCode::SWITCHING_PROTOCOL.as_str(), "101 Switching Protocols");

    for (status, expected_reason) in TEST_STATUS {
        assert_eq!(status.reason(), expected_reason);
        assert_eq!(status.code_str(), status.0.to_string());
        assert_eq!(status.as_str(), format!("{} {expected_reason}", status.0));
    }
}

