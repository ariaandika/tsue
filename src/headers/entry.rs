use std::mem::replace;

use super::{HeaderName, HeaderValue};

type Size = u16;

/// Header Entry.
///
/// Contains [`HeaderName`] and multiple [`HeaderValue`].
pub struct Entry {
    hash: Size,
    name: HeaderName,
    value: HeaderValue,
    next: *mut EntryExtra,
    extra_len: Size,
}

// SAFETY: EntryExtra pointer is exclusively owned by Entry
unsafe impl Send for Entry {}

// SAFETY: EntryExtra pointer is exclusively owned by Entry
unsafe impl Sync for Entry {}

struct EntryExtra {
    value: HeaderValue,
    next: *mut EntryExtra,
}

impl Entry {
    pub(crate) fn new(hash: Size, name: HeaderName, value: HeaderValue) -> Self {
        Self {
            hash,
            name,
            value,
            next: std::ptr::null_mut(),
            extra_len: 0,
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
        reason = "Entry always have at least 1 value"
    )]
    pub const fn len(&self) -> usize {
        self.extra_len as usize + 1
    }

    #[inline]
    pub(crate) const fn extra_len(&self) -> u16 {
        self.extra_len
    }

    /// Returns an iterator over [`HeaderValue`].
    #[inline]
    pub const fn iter(&self) -> GetAll<'_> {
        GetAll::new(self)
    }

    /// Push header value.
    pub fn push(&mut self, value: HeaderValue) {
        let new = Box::into_raw(Box::new(EntryExtra {
            value,
            next: std::ptr::null_mut(),
        }));

        if self.next.is_null() {
            self.next = new;
            self.extra_len += 1;
            return;
        }

        let mut next = self.next;

        loop {
            // SAFETY: null checked above and below
            let now = unsafe { &mut *next };

            if now.next.is_null() {
                now.next = new;
                self.extra_len += 1;
                return;
            } else {
                next = now.next;
            }
        }
    }

    /// Consume [`Entry`] into [`HeaderName`] and [`HeaderValue`].
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

impl Clone for Entry {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash,
            name: self.name.clone(),
            value: self.value.clone(),
            next: if self.next.is_null() {
                self.next
            } else {
                // SAFETY: null checked
                Box::into_raw(Box::new(EntryExtra::clone(unsafe { &*self.next })))
            },
            extra_len: self.extra_len,
        }
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        if !self.next.is_null() {
            // SAFETY: null checked
            drop(unsafe { Box::from_raw(self.next) });
        }
    }
}

impl Clone for EntryExtra {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            next: {
                if self.next.is_null() {
                    self.next
                } else {
                    // SAFETY: null checked
                    Box::into_raw(Box::new(EntryExtra::clone(unsafe { &*self.next })))
                }
            },
        }
    }
}

impl Drop for EntryExtra {
    fn drop(&mut self) {
        if !self.next.is_null() {
            // SAFETY: null checked
            drop(unsafe { Box::from_raw(self.next) });
        }
    }
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("name", &self.name)
            .field("values", &GetAll::new(self))
            .finish()
    }
}

// ===== Iterator =====

impl<'a> IntoIterator for &'a Entry {
    type Item = &'a HeaderValue;

    type IntoIter = GetAll<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        GetAll::new(self)
    }
}

/// Iterator returned from [`HeaderMap::get_all`][super::HeaderMap::get_all].
pub struct GetAll<'a> {
    first: Option<&'a Entry>,
    next: *const EntryExtra,
}

impl<'a> GetAll<'a> {
    pub(crate) const fn new(entry: &'a Entry) -> Self {
        Self {
            first: Some(entry),
            next: entry.next,
        }
    }

    pub(crate) const fn empty() -> Self {
        Self {
            first: None,
            next: std::ptr::null(),
        }
    }

    /// Returns `true` if there is still remaining value.
    #[inline]
    pub const fn has_remaining(&self) -> bool {
        self.first.is_some() || !self.next.is_null()
    }
}

impl<'a> Iterator for GetAll<'a> {
    type Item = &'a HeaderValue;

    fn next(&mut self) -> Option<Self::Item> {
        match self.first.take() {
            Some(e) => Some(e.value()),
            None => {
                if self.next.is_null() {
                    return None;
                }

                // SAFETY: null checked
                let extra = unsafe { &*self.next };
                self.next = extra.next;
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
