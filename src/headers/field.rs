use crate::headers::{HeaderName, HeaderValue};
use crate::headers::iter::GetAll;

type Size = u32;
type NonZeroSize = std::num::NonZeroU32;

/// Header Field.
///
/// Contains [`HeaderName`] and multiple [`HeaderValue`].
#[derive(Clone)]
pub struct HeaderField {
    hash: Size,
    name: HeaderName,
    entry: FieldEntry,
    len: NonZeroSize,
}

/// A linked list node of `HeaderValue`.
#[derive(Clone)]
pub(crate) struct FieldEntry {
    value: HeaderValue,
    next: Option<Box<FieldEntry>>,
}

impl HeaderField {
    pub(crate) const fn new(name: HeaderName, value: HeaderValue) -> Self {
        Self {
            hash: name.hash(),
            name,
            entry: FieldEntry::new(value),
            len: unsafe { NonZeroSize::new_unchecked(1) },
        }
    }

    /// name must be in lowercase
    pub(crate) fn cached_hash(&self) -> Size {
        self.hash
    }

    /// Returns reference to [`HeaderName`].
    #[inline]
    pub const fn name(&self) -> &HeaderName {
        &self.name
    }

    /// Returns reference to [`HeaderValue`].
    #[inline]
    pub const fn value(&self) -> &HeaderValue {
        &self.entry.value
    }

    /// Returns the number of [`HeaderValue`].
    ///
    /// This function will returns at least `1`.
    #[inline]
    #[allow(
        clippy::len_without_is_empty,
        reason = "Field always have at least 1 value"
    )]
    pub const fn len(&self) -> usize {
        self.len.get() as _
    }

    /// Returns an iterator over [`HeaderValue`].
    #[inline]
    pub fn iter(&self) -> GetAll<'_> {
        self.into_iter()
    }

    /// Push header value.
    ///
    /// # Panics
    ///
    /// Panics if the new length exceeds `u32::MAX`.
    #[inline]
    pub fn push(&mut self, value: HeaderValue) {
        let new_len = self.len.checked_add(1).unwrap();
        self.entry.push(value);
        self.len = new_len;
    }

    /// Combine all headers from two [`HeaderField`].
    ///
    /// # Panics
    ///
    /// Panics if the new length exceeds `u32::MAX`.
    pub fn merge(&mut self, other: Self) {
        let mut entry = other.entry;
        loop {
            self.push(entry.value);
            match entry.next {
                Some(next) => entry = *next,
                None => break,
            }
        }
    }

    /// Consume [`HeaderField`] into [`HeaderName`] and [`HeaderValue`].
    ///
    /// Extra header value will be dropped.
    #[inline]
    pub fn into_parts(self) -> (HeaderName, HeaderValue) {
        (self.name, self.entry.value)
    }

    pub(crate) const fn entry(&self) -> &FieldEntry {
        &self.entry
    }
}

impl FieldEntry {
    const fn new(value: HeaderValue) -> FieldEntry {
        Self { value, next: None }
    }

    fn new_option_box(value: HeaderValue) -> Option<Box<FieldEntry>> {
        Some(Box::new(Self { value, next: None }))
    }

    fn push(&mut self, value: HeaderValue) {
        match self.next.as_mut() {
            Some(next) => next.push(value),
            None => self.next = Self::new_option_box(value),
        }
    }

    pub(crate) const fn value(&self) -> &HeaderValue {
        &self.value
    }

    pub(crate) fn next_entry(&self) -> Option<&FieldEntry> {
        self.next.as_deref()
    }
}

impl std::fmt::Debug for HeaderField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderField")
            .field("name", &self.name)
            .field("values", &self.iter())
            .finish()
    }
}
