use tcio::ByteStr;

const NONE: u16 = u16::MAX;

/// HTTP path and query.
#[derive(Debug, Clone)]
pub struct Path {
    value: ByteStr,
    query: u16,
}

impl Path {
    /// Create new [`Path`].
    pub(crate) fn new(bytes: ByteStr) -> Self {
        let query = bytes
            .find('?')
            .and_then(|e| e.try_into().ok())
            .unwrap_or(u16::MAX);
        Self {
            value: bytes,
            query,
        }
    }

    /// Returns the uri path.
    pub fn path(&self) -> &str {
        let path = if self.query == NONE {
            &self.value[..]
        } else {
            &self.value[..self.query as usize]
        };

        if path.is_empty() {
            "/"
        } else {
            path
        }
    }

    /// Returns the uri query.
    pub fn query(&self) -> Option<&str> {
        if self.query == NONE {
            None
        } else {
            Some(&self.value[self.query as usize + 1..])
        }
    }

    /// Returns uri as string.
    pub fn as_str(&self) -> &str {
        let value = self.value.as_str();
        if value.is_empty() {
            "/"
        } else {
            value
        }
    }
}

