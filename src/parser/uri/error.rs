// ===== Error =====

pub enum InvalidUri {
    /// Bytes ends before all components parsed.
    Incomplete,
    /// Bytes length is too large.
    TooLong,
    /// Invalid character found.
    Char,
    /// Contains non-ASCII byte.
    NonAscii,
}

impl std::error::Error for InvalidUri { }

impl std::fmt::Display for InvalidUri {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use InvalidUri::*;
        f.write_str("invalid uri: ")?;
        match self {
            TooLong => f.write_str("data length is too large"),
            Incomplete => f.write_str("data is incomplete"),
            Char => write!(f, "invalid character"),
            NonAscii => write!(f, "found non-ASCII byte"),
        }
    }
}

impl std::fmt::Debug for InvalidUri {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\"{self}\"")
    }
}
