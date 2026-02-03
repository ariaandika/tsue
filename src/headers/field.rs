use crate::headers::{HeaderName, HeaderValue};

type Size = u32;

/// Header Field.
///
/// Contains [`HeaderName`] and multiple [`HeaderValue`].
#[derive(Clone)]
pub struct HeaderField {
    hash: Size,
    name: HeaderName,
    value: HeaderValue,
    is_sensitive: bool,
}

impl HeaderField {
    pub(crate) const fn new(name: HeaderName, value: HeaderValue) -> Self {
        Self {
            hash: name.hash(),
            name,
            value,
            is_sensitive: false,
        }
    }

    pub(crate) const fn with_hash(name: HeaderName, value: HeaderValue, hash: u32) -> Self {
        Self {
            hash,
            name,
            value,
            is_sensitive: false,
        }
    }

    /// name must be in lowercase
    pub(crate) fn cached_hash(&self) -> Size {
        self.hash
    }

    pub(crate) fn hpack_size(&self) -> usize {
        self.name.as_str().len() + self.value.as_bytes().len() + 32
    }

    /// Returns reference to [`HeaderName`].
    #[inline]
    pub const fn name(&self) -> &HeaderName {
        &self.name
    }

    /// Returns reference to [`HeaderValue`].
    #[inline]
    pub const fn value(&self) -> &HeaderValue {
        &self.value
    }

    /// Returns `true` if header is marked as sensitive.
    #[inline]
    pub const fn is_sensitive(&self) -> bool {
        self.is_sensitive
    }

    /// Consume [`HeaderField`] into [`HeaderName`] and [`HeaderValue`].
    ///
    /// Extra header value will be dropped.
    #[inline]
    pub fn into_parts(self) -> (HeaderName, HeaderValue) {
        (self.name, self.value)
    }
}

impl std::fmt::Debug for HeaderField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderField")
            .field("name", &self.name)
            .field("value", &self.value)
            .field("is_sensitive", &self.is_sensitive)
            .finish()
    }
}
