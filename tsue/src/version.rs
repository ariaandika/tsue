use std::fmt;

/// HTTP Version.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Version(Inner);

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


