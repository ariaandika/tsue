use crate::common::ByteStr;

const NONE: u16 = u16::MAX;

/// HTTP path and possible query.
#[derive(Debug, Clone)]
pub struct PathAndQuery {
    value: ByteStr,
    query: u16,
}

impl PathAndQuery {
    /// Create new [`PathAndQuery`].
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

    pub fn query(&self) -> Option<&str> {
        if self.query == NONE {
            None
        } else {
            Some(&self.value[self.query as usize + 1..])
        }
    }

    pub fn as_str(&self) -> &str {
        let value = self.value.as_str();
        if value.is_empty() {
            "/"
        } else {
            value
        }
    }
}

