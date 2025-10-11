
/// Header name/value parsing error.
pub struct HeaderError {
    error: &'static str,
}

impl HeaderError {
    pub(crate) const fn new_name() -> HeaderError {
        Self {
            error: "invalid header name",
        }
    }

    pub(crate) const fn panic_const(&self) -> ! {
        panic!("{}", self.error)
    }
}

impl std::error::Error for HeaderError {}

impl std::fmt::Debug for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error)
    }
}
