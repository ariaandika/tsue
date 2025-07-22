use std::mem::replace;

use super::{HeaderName, HeaderValue};

type Size = u16;

/// Header Entry.
#[derive(Clone)]
pub struct Entry {
    hash: Size,
    name: HeaderName,
    value: HeaderValue,
    next: *mut EntryExtra,
    extra_len: Size,
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("name", &self.name)
            .field("values", &GetAll::new(self))
            .finish()
    }
}

// SAFETY: EntryExtra pointer is exclusively owned by Entry
unsafe impl Send for Entry { }

// SAFETY: EntryExtra pointer is exclusively owned by Entry
unsafe impl Sync for Entry { }

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
    pub const fn get_hashed(&self) -> &Size {
        &self.hash
    }

    /// Returns reference to [`HeaderName`].
    pub const fn name(&self) -> &HeaderName {
        &self.name
    }

    /// Returns reference to [`HeaderValue`].
    pub const fn value(&self) -> &HeaderValue {
        &self.value
    }

    /// Returns duplicate header name length.
    pub const fn extra_len(&self) -> u16 {
        self.extra_len
    }

    /// Push value with duplicate header name.
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
    pub fn into_parts(mut self) -> (HeaderName, HeaderValue) {
        (
            replace(&mut self.name, HeaderName::PLACEHOLDER),
            replace(&mut self.value, HeaderValue::PLACEHOLDER),
        )
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        let mut next = self.next;
        loop {
            let now = next;
            if now.is_null() {
                break;
            }
            // SAFETY: null checked
            let now = unsafe { Box::from_raw(now) };
            next = now.next;
            drop(now);
        }
    }
}

// ===== Iterator =====

/// Iterator returned from [`HeaderMap::get_all`][super::HeaderMap::get_all].
pub struct GetAll<'a> {
    first: Option<&'a Entry>,
    next: *const EntryExtra,
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

impl<'a> GetAll<'a> {
    pub(crate) fn new(entry: &'a Entry) -> Self {
        Self {
            first: Some(entry),
            next: entry.next,
        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            first: None,
            next: std::ptr::null(),
        }
    }

    /// Returns `true` if there is still remaining value.
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
