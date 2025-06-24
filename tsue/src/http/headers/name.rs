use std::hash::Hasher;
use tcio::ByteStr;

// ===== HeaderName =====

/// HTTP Header name.
pub struct HeaderName {
    repr: Repr,
}

enum Repr {
    Standard(StandardHeader),
    Bytes(ByteStr),
}

/// Precomputed known header name.
struct StandardHeader {
    name: &'static str,
    hash: u16,
}

impl HeaderName {
    /// Used in iterator.
    pub(crate) const PLACEHOLDER: Self = Self {
        repr: Repr::Standard(StandardHeader {
            name: "",
            hash: 0,
        })
    };

    /// Create new [`HeaderName`].
    pub fn new(name: impl Into<ByteStr>) -> Self {
        Self { repr: Repr::Bytes(name.into()) }
    }

    pub(crate) fn hash(&self) -> u16 {
        match &self.repr {
            Repr::Standard(s) => s.hash,
            Repr::Bytes(b) => hash_str(b),
        }
    }

    /// Extracts a string slice of the header name.
    pub fn as_str(&self) -> &str {
        match &self.repr {
            Repr::Standard(s) => s.name,
            Repr::Bytes(s) => s.as_str(),
        }
    }
}

// ===== Ref Traits =====

/// A type that can be used for [`HeaderMap`] operation.
///
/// [`HeaderMap`]: super::HeaderMap
#[allow(private_bounds)]
pub trait AsHeaderName: SealedRef { }

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

impl<K: AsHeaderName> AsHeaderName for &K { }
impl<S: SealedRef> SealedRef for &S {
    fn hash(&self) -> u16 {
        S::hash(self)
    }

    fn as_str(&self) -> &str {
        S::as_str(self)
    }
}

impl AsHeaderName for &str { }
impl SealedRef for &str {
    fn hash(&self) -> u16 {
        hash_str(self)
    }

    fn as_str(&self) -> &str {
        self
    }
}

impl AsHeaderName for HeaderName { }
impl SealedRef for HeaderName {
    fn hash(&self) -> u16 {
        match &self.repr {
            Repr::Standard(s) => s.hash,
            Repr::Bytes(s) => hash_str(s),
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

/// The contrete type used in header map operation.
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

// ===== Owned Traits =====

/// A type that can be used for name consuming [`HeaderMap`] operation.
///
/// [`HeaderMap`]: super::HeaderMap
#[allow(private_bounds)]
pub trait IntoHeaderName: Sealed {}

pub(crate) trait Sealed: Sized {
    fn into_header_name(self) -> HeaderName;
}

impl IntoHeaderName for ByteStr {}
impl Sealed for ByteStr {
    fn into_header_name(self) -> HeaderName {
        HeaderName {
            repr: Repr::Bytes(self),
        }
    }
}

// for static data use provided constants, not static str
impl IntoHeaderName for &str {}
impl Sealed for &str {
    fn into_header_name(self) -> HeaderName {
        HeaderName {
            repr: Repr::Bytes(ByteStr::copy_from_str(self)),
        }
    }
}

impl IntoHeaderName for HeaderName {}
impl Sealed for HeaderName {
    fn into_header_name(self) -> HeaderName {
        self
    }
}

// ===== Debug =====

impl std::fmt::Debug for HeaderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("HeaderName");
        match &self.repr {
            Repr::Standard(s) => f.field("name", &s.name),
            Repr::Bytes(b) => f.field("name", &b),
        }.finish()
    }
}

