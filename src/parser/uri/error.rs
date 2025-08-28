
pub enum UriError {
    /// Bytes ends before all components parsed.
    Incomplete,
    /// Bytes length is too large.
    TooLong,
    /// Invalid character found.
    Char,
}

impl std::error::Error for UriError { }

impl std::fmt::Display for UriError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use UriError::*;
        f.write_str("invalid uri, ")?;
        match self {
            TooLong => f.write_str("data length is too large"),
            Incomplete => f.write_str("data is incomplete"),
            Char => f.write_str("invalid character"),
        }
    }
}

impl std::fmt::Debug for UriError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\"{self}\"")
    }
}
