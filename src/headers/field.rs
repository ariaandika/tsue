use std::mem::replace;

use super::{HeaderName, HeaderValue};

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

#[derive(Clone)]
struct FieldEntry {
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
    pub(crate) fn eq_hash_and_name(&self, hash: Size, name: &str) -> bool {
        self.hash == hash && self.name.as_str() == name
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
    pub const fn iter(&self) -> GetAll<'_> {
        GetAll::new(self)
    }

    /// Push header value.
    #[inline]
    pub fn push(&mut self, value: HeaderValue) {
        let new_len = self.len.checked_add(1).unwrap();
        self.entry.push(value);
        self.len = new_len;
    }

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
    pub fn into_parts(mut self) -> (HeaderName, HeaderValue) {
        (
            replace(&mut self.name, HeaderName::placeholder()),
            replace(&mut self.entry.value, HeaderValue::placeholder()),
        )
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
}

impl std::fmt::Debug for HeaderField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderField")
            .field("name", &self.name)
            .field("values", &GetAll::new(self))
            .finish()
    }
}

// ===== Iterator =====

impl<'a> IntoIterator for &'a HeaderField {
    type Item = &'a HeaderValue;

    type IntoIter = GetAll<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        GetAll::new(self)
    }
}

/// Iterator returned from [`HeaderMap::get_all`][super::HeaderMap::get_all].
#[derive(Clone)]
pub struct GetAll<'a> {
    entry: Option<&'a FieldEntry>,
}

impl<'a> GetAll<'a> {
    pub(crate) const fn new(field: &'a HeaderField) -> Self {
        Self {
            entry: Some(&field.entry),
        }
    }

    /// Returns `true` if there is still remaining value.
    #[inline]
    pub const fn has_remaining(&self) -> bool {
        self.entry.is_some()
    }
}

impl<'a> Iterator for GetAll<'a> {
    type Item = &'a HeaderValue;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.entry.take() {
            Some(entry) => {
                self.entry = entry.next.as_deref();
                Some(&entry.value)
            }
            None => None,
        }
    }
}

impl<'a> std::fmt::Debug for GetAll<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
