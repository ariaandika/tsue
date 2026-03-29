/// HTTP/HTTPS Scheme.
#[derive(Copy, Clone, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Scheme(bool);

impl Scheme {
    /// `http` scheme.
    pub const HTTP: Self = Self(false);
    /// `https` scheme.
    pub const HTTPS: Self = Self(true);

    /// Returns `true` if this is an HTTP scheme.
    #[inline]
    pub const fn is_http(&self) -> bool {
        !self.0
    }

    /// Returns `true` if this is an HTTPS scheme.
    #[inline]
    pub const fn is_https(&self) -> bool {
        self.0
    }

    /// Extracts a string slice containing http scheme.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        const HTTPS: &str = "https";
        unsafe {
            str::from_utf8_unchecked(std::slice::from_raw_parts(
                HTTPS.as_ptr(),
                4 + self.0 as usize,
            ))
        }
    }
}

// ===== std traits =====

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
