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
            Repr::Bytes(b) => fnv_hash(b.as_bytes()),
        }
    }

    /// Extracts a string slice of the header name.
    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.repr {
            Repr::Standard(s) => s.name,
            Repr::Bytes(s) => s.as_str(),
        }
    }
}

// ===== Hash =====

#[inline]
const fn fnv_hash(bytes: &[u8]) -> u16 {
    const INITIAL_STATE: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = INITIAL_STATE;
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(PRIME);
        i += 1;
    }

    hash as _
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
        fnv_hash(self.as_bytes())
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
            Repr::Bytes(s) => fnv_hash(s.as_bytes()),
        }
    }

    fn as_str(&self) -> &str {
        HeaderName::as_str(self)
    }
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

