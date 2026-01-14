use crate::headers::field::FieldEntry;
use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue};

// ===== Header Values Iterator =====

/// An immutable iterator over the header values with the same header name.
///
/// This iterator is created from [`HeaderMap::get_all`] or [`HeaderField::iter`] method.
#[derive(Clone)]
pub struct GetAll<'a> {
    entry: Option<&'a FieldEntry>,
}

impl<'a> IntoIterator for &'a HeaderField {
    type Item = &'a HeaderValue;

    type IntoIter = GetAll<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        GetAll {
            entry: Some(self.entry()),
        }
    }
}

impl<'a> GetAll<'a> {
    pub(crate) const fn empty() -> Self {
        Self { entry: None }
    }

    /// Returns `Some` if there is only single value for given name in the map.
    #[inline]
    pub fn as_single(self) -> Option<&'a HeaderValue> {
        let current = self.entry?;
        if current.next_entry().is_some() {
            None
        } else {
            Some(current.value())
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
        let next = self.entry.take()?;
        self.entry = next.next_entry();
        Some(next.value())
    }
}

impl<'a> std::fmt::Debug for GetAll<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

// ===== Header Fields Iterator =====

/// An immutable iterator over the headers in a [`HeaderMap`].
///
/// Note that the order of the returned headers is arbitrary.
#[derive(Clone)]
pub struct Iter<'a> {
    iter: std::slice::Iter<'a, Option<HeaderField>>,
    current: Option<(&'a HeaderName, GetAll<'a>)>,
}

impl<'a> IntoIterator for &'a HeaderMap {
    type Item = <Iter<'a> as Iterator>::Item;

    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let mut iter = self.fields().iter();
        Iter {
            current: iter.find_map(|e| e.as_ref().map(|e| (e.name(), e.iter()))),
            iter,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a HeaderName, &'a HeaderValue);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((name, values)) = &mut self.current
                && let Some(value) = values.next()
            {
                return Some((name, value));
            }

            let field = self.iter.find_map(|e| e.as_ref())?;
            self.current = Some((field.name(), field.iter()));
        }
    }
}

impl<'a> std::fmt::Debug for Iter<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
