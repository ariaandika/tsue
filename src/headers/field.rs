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
    value: HeaderValue,
    next: Option<Box<FieldExtra>>,
    len: NonZeroSize,
}

#[derive(Clone)]
struct FieldExtra {
    value: HeaderValue,
    next: Option<Box<FieldExtra>>,
}

impl HeaderField {
    pub(crate) const fn new(hash: Size, name: HeaderName, value: HeaderValue) -> Self {
        Self {
            hash,
            name,
            value,
            next: None,
            len: unsafe { NonZeroSize::new_unchecked(1) },
        }
    }

    /// Returns cached hash.
    #[inline]
    pub(crate) const fn get_hashed(&self) -> &Size {
        &self.hash
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
    pub fn push(&mut self, value: HeaderValue) {
        let new_len = self.len.checked_add(1).unwrap();

        match self.next.as_mut() {
            Some(next) => next.push(value),
            None => self.next = FieldExtra::new_option_box(value),
        }

        self.len = new_len;
    }

    /// Consume [`HeaderField`] into [`HeaderName`] and [`HeaderValue`].
    ///
    /// Extra header value will be dropped.
    #[inline]
    pub fn into_parts(mut self) -> (HeaderName, HeaderValue) {
        (
            replace(&mut self.name, HeaderName::placeholder()),
            replace(&mut self.value, HeaderValue::placeholder()),
        )
    }
}

impl FieldExtra {
    fn new_option_box(value: HeaderValue) -> Option<Box<FieldExtra>> {
        Some(Box::new(Self {
            value,
            next: None
        }))
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
pub struct GetAll<'a> {
    first: Option<&'a HeaderField>,
    next: Option<&'a Box<FieldExtra>>,
}

impl<'a> GetAll<'a> {
    pub(crate) const fn new(field: &'a HeaderField) -> Self {
        Self {
            first: Some(field),
            next: field.next.as_ref(),
        }
    }

    pub(crate) const fn empty() -> Self {
        Self {
            first: None,
            next: None,
        }
    }

    /// Returns `true` if there is still remaining value.
    #[inline]
    pub const fn has_remaining(&self) -> bool {
        self.first.is_some() || self.next.is_some()
    }
}

impl<'a> Iterator for GetAll<'a> {
    type Item = &'a HeaderValue;

    fn next(&mut self) -> Option<Self::Item> {
        match self.first.take() {
            Some(e) => Some(e.value()),
            None => {
                let extra = self.next?;
                self.next = extra.next.as_ref();
                Some(&extra.value)
            }
        }
    }
}

impl std::fmt::Debug for GetAll<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_list()
            .entries(Self {
                first: self.first,
                next: self.next,
            })
            .finish()
    }
}
