/// HTTP Version.
///
/// [httpwg](https://httpwg.org/specs/rfc9112.html#http.version)
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Version(Inner);

#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash)]
enum Inner {
    Http09,
    Http10,
    Http11,
    H2,
    H3,
}

impl Version {
    /// `HTTP/0.9`
    pub const HTTP_09: Version = Version(Inner::Http09);

    /// `HTTP/1.0`
    pub const HTTP_10: Version = Version(Inner::Http10);

    /// `HTTP/1.1`
    pub const HTTP_11: Version = Version(Inner::Http11);

    /// `HTTP/2.0`
    pub const HTTP_2: Version = Version(Inner::H2);

    /// `HTTP/3.0`
    pub const HTTP_3: Version = Version(Inner::H3);

    /// Returns string representation of HTTP version, e.g: `HTTP/1.1`
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self.0 {
            Inner::Http09 => "HTTP/0.9",
            Inner::Http10 => "HTTP/1.0",
            Inner::Http11 => "HTTP/1.1",
            Inner::H2 => "HTTP/2.0",
            Inner::H3 => "HTTP/3.0",
        }
    }
}

impl Default for Version {
    #[inline]
    fn default() -> Version {
        Version::HTTP_11
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\"{self}\"")
    }
}
