use super::Path;

impl Path {
    /// Returns the path as `str`, e.g: `/over/there`.
    #[inline]
    pub fn path(&self) -> &str {
        &self.value[..self.query as usize]
    }

    /// Returns the query as `str`, e.g: `name=joe&query=4`.
    #[inline]
    pub fn query(&self) -> Option<&str> {
        if self.query as usize == self.value.len() {
            None
        } else {
            Some(&self.value[self.query as usize + 1..])
        }
    }

    /// Returns the path and query as `str`, e.g: `/over/there?name=joe&query=4`.
    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.value.as_str()
    }

    /// Returns the str representation.
    #[inline]
    pub const fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

// ===== Formatting =====

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}
