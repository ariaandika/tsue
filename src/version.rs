use std::fmt;

/// HTTP Version.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Version(Inner);

impl Version {
    /// [`HTTP/0.9`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Evolution_of_HTTP#http0.9_%E2%80%93_the_one-line_protocol)
    pub const HTTP_09: Version = Version(Inner::Http09);

    /// [`HTTP/1.0`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Evolution_of_HTTP#http1.0_%E2%80%93_building_extensibility)
    pub const HTTP_10: Version = Version(Inner::Http10);

    /// [`HTTP/1.1`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Evolution_of_HTTP#http1.1_%E2%80%93_the_standardized_protocol)
    pub const HTTP_11: Version = Version(Inner::Http11);

    /// [`HTTP/2.0`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Evolution_of_HTTP#http2_%E2%80%93_a_protocol_for_greater_performance)
    pub const HTTP_2: Version = Version(Inner::H2);

    /// [`HTTP/3.0`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Evolution_of_HTTP#http3_-_http_over_quic)
    pub const HTTP_3: Version = Version(Inner::H3);
}

#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash)]
enum Inner {
    Http09,
    Http10,
    Http11,
    H2,
    H3,
}

impl Default for Version {
    #[inline]
    fn default() -> Version {
        Version::HTTP_11
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Inner::*;

        f.write_str(match self.0 {
            Http09 => "HTTP/0.9",
            Http10 => "HTTP/1.0",
            Http11 => "HTTP/1.1",
            H2 => "HTTP/2.0",
            H3 => "HTTP/3.0",
        })
    }
}

