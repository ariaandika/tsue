use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue, map::Probe};

// ===== Header Values Iterator =====

/// An immutable iterator over the header values with the same header name.
///
/// This iterator is created from [`HeaderMap::get_all`] method.
#[derive(Clone)]
pub struct GetAll<'a> {
    probe: Probe<'a>,
    name: &'a str,
    hash: u32,
}

impl<'a> GetAll<'a> {
    pub(crate) fn new(map: &'a HeaderMap, name: &'a str, hash: u32) -> Self {
        Self {
            probe: Probe::from_hash(map, hash),
            name,
            hash,
        }
    }

    /// Returns `true` if there is still remaining value.
    pub(crate) fn has_remaining(&self) -> bool {
        self.probe.peek().is_some()
    }
}

impl<'a> Iterator for GetAll<'a> {
    type Item = &'a HeaderValue;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let field = self.probe.next()?;
        if field.cached_hash() == self.hash && field.name().as_str() == self.name {
            Some(field.value())
        } else {
            self.next()
        }
    }
}

impl<'a> std::fmt::Debug for GetAll<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

// ===== Header Fields Iterator =====

/// An immutable iterator over the header fields in a [`HeaderMap`].
///
/// Note that the order of the returned headers is arbitrary.
#[derive(Clone)]
pub struct Fields<'a> {
    iter: std::slice::Iter<'a, Option<HeaderField>>,
    remaining: u32,
}

impl<'a> IntoIterator for &'a HeaderMap {
    type Item = <Fields<'a> as Iterator>::Item;

    type IntoIter = Fields<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Fields {
            iter: self.fields().iter(),
            remaining: self.len_size(),
        }
    }
}

impl<'a> Iterator for Fields<'a> {
    type Item = &'a HeaderField;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        match self.iter.next()?.as_ref() {
            Some(field) => {
                self.remaining -= 1;
                Some(field)
            }
            None => self.next(),
        }
    }
}

impl<'a> std::fmt::Debug for Fields<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

// ===== Header Pairs Iterator =====

/// An immutable iterator over the headers as name and value pair in a [`HeaderMap`].
///
/// Note that the order of the returned headers is arbitrary.
#[derive(Clone)]
pub struct Pairs<'a> {
    iter: Fields<'a>
}

impl<'a> Pairs<'a> {
    pub(crate) fn new(map: &'a HeaderMap) -> Self {
        Self { iter: map.into_iter() }
    }
}

impl<'a> Iterator for Pairs<'a> {
    type Item = (&'a HeaderName, &'a HeaderValue);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|f|(f.name(), f.value()))
    }
}

impl<'a> std::fmt::Debug for Pairs<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.clone()).finish()
    }
}
