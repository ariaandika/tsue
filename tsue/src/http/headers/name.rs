use std::hash::Hasher;

use crate::common::ByteStr;

pub struct HeaderName {
    repr: Repr,
}

enum Repr {
    Standard(StandardHeader),
    Custom(ByteStr),
}

struct StandardHeader {
    name: &'static str,
    hash: u16,
}

pub(crate) static PLACEHOLDER: HeaderName = HeaderName {
    repr: Repr::Standard(StandardHeader {
        name: "",
        hash: 0,
    })
};

impl HeaderName {
    pub(crate) const PLACEHOLDER: Self = Self {
        repr: Repr::Standard(StandardHeader {
            name: "",
            hash: 0,
        })
    };

    /// Create new [`HeaderName`].
    pub fn new(name: impl Into<ByteStr>) -> Self {
        Self { repr: Repr::Custom(name.into()) }
    }

    pub(crate) fn hash(&self) -> u16 {
        match &self.repr {
            Repr::Standard(s) => s.hash,
            Repr::Custom(b) => hash_str(b),
        }
    }

    /// Extracts a string slice containing the entire [`HeaderName`].
    pub fn as_str(&self) -> &str {
        match &self.repr {
            Repr::Standard(s) => s.name,
            Repr::Custom(s) => s.as_str(),
        }
    }
}

// ===== Ref Traits =====

pub(crate) struct HeaderNameRef<'a> {
    name: &'a str,
    hash: u16,
}

impl<'a> HeaderNameRef<'a> {
    pub(crate) fn as_str(&self) -> &'a str {
        self.name
    }

    pub(crate) fn hash(&self) -> u16 {
        self.hash
    }
}

pub(crate) trait SealedRef: Sized {
    fn hash(&self) -> u16;

    fn as_str(&self) -> &str;

    fn to_header_ref(&self) -> HeaderNameRef {
        HeaderNameRef {
            name: self.as_str(),
            hash: self.hash(),
        }
    }
}

impl<S: SealedRef> SealedRef for &S {
    fn hash(&self) -> u16 {
        S::hash(self)
    }

    fn as_str(&self) -> &str {
        S::as_str(self)
    }
}

impl SealedRef for &str {
    fn hash(&self) -> u16 {
        hash_str(self)
    }

    fn as_str(&self) -> &str {
        self
    }
}

impl SealedRef for HeaderName {
    fn hash(&self) -> u16 {
        match &self.repr {
            Repr::Standard(s) => s.hash,
            Repr::Custom(s) => hash_str(s),
        }
    }

    fn as_str(&self) -> &str {
        HeaderName::as_str(self)
    }
}

fn hash_str(s: &str) -> u16 {
    let mut hasher = fnv::FnvHasher::with_key(199);
    hasher.write(s.as_bytes());
    hasher.finish() as _
}

#[allow(private_bounds)]
pub trait AsHeaderName: SealedRef { }
impl AsHeaderName for HeaderName { }
impl AsHeaderName for &str { }
impl<K: AsHeaderName> AsHeaderName for &K { }

// ===== Owned Traits =====

pub(crate) trait Sealed: Sized {
    fn into_header_name(self) -> HeaderName;
}

impl Sealed for ByteStr {
    fn into_header_name(self) -> HeaderName {
        HeaderName {
            repr: Repr::Custom(self),
        }
    }
}

impl Sealed for &'static str {
    fn into_header_name(self) -> HeaderName {
        HeaderName {
            repr: Repr::Custom(ByteStr::from_static(self)),
        }
    }
}

impl Sealed for HeaderName {
    fn into_header_name(self) -> HeaderName {
        self
    }
}

#[allow(private_bounds)]
pub trait IntoHeaderName: Sealed {}
impl IntoHeaderName for HeaderName {}
impl IntoHeaderName for ByteStr {}
impl IntoHeaderName for &'static str {}

// ===== Debug =====

impl std::fmt::Debug for HeaderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderName").finish()
    }
}

