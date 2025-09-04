use tcio::bytes::Bytes;

#[derive(Clone)]
pub struct Scheme {
    /// is valid ASCII
    value: Bytes,
}

impl Scheme {
    pub fn from_shared(value: Bytes) -> Self {
        todo!()
    }

    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

// ===== Formatting =====

impl std::fmt::Debug for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

