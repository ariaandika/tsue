
/// Header name/value parsing error.
pub struct HeaderError {
    kind: Kind,
}

#[derive(Debug)]
enum Kind {
    Empty,
    TooLong,
    InvalidHeaderName,
    InvalidHeaderValue,
    Duplicate,
    // /// Only when constructing HeaderValue with utf8 promise.
    // NonUtf8HeaderValue,
}

impl HeaderError {
    pub(crate) const fn invalid_name() -> Self {
        Self {
            kind: Kind::InvalidHeaderName,
        }
    }

    pub(crate) const fn invalid_value() -> Self {
        Self {
            kind: Kind::InvalidHeaderValue,
        }
    }

    pub(crate) const fn invalid_len(len: usize) -> Self {
        Self {
            kind: match len {
                0 => Kind::Empty,
                _ => Kind::TooLong,
            },
        }
    }

    pub(crate) const fn msg(&self) -> &'static str {
        match self.kind {
            Kind::Empty => "header cannot be empty",
            Kind::TooLong => "header too long",
            Kind::InvalidHeaderName => "invalid header name",
            Kind::InvalidHeaderValue => "invalid header value",
            Kind::Duplicate => "invalid duplicate header",
        }
    }

    pub(crate) const fn panic_const(&self) -> ! {
        panic!("{}", self.msg())
    }
}

impl std::error::Error for HeaderError {}

impl std::fmt::Debug for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HeaderError").field(&self.kind).finish()
    }
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.msg().fmt(f)
    }
}
