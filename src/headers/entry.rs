use std::mem::replace;

use super::{HeaderName, HeaderValue};

type Size = u16;

/// Header Entry.
#[derive(Debug, Clone)]
pub struct Entry {
    hash: Size,
    name: HeaderName,
    value: HeaderValue,
    next: *mut EntryExtra,
    extra_len: Size,
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
    pub fn new(hash: Size, name: HeaderName, value: HeaderValue) -> Self {
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

    pub fn name(&self) -> &HeaderName {
        &self.name
    }

    pub fn value(&self) -> &HeaderValue {
        &self.value
    }

    pub fn extra_len(&self) -> u16 {
        self.extra_len
    }

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
            // SAFETY: null chekced
            let now = unsafe { Box::from_raw(now) };
            next = now.next;
            drop(now);
        }
    }
}

// ===== Iterator =====

/// Iterator returned from [`HeaderMap::get_all`][super::HeaderMap::get_all].
#[derive(Debug)]
pub struct GetAll<'a> {
    entry: Option<&'a Entry>,
    next: *const EntryExtra,
}

impl<'a> GetAll<'a> {
    pub(crate) fn new(entry: &'a Entry) -> Self {
        Self {
            next: entry.next,
            entry: Some(entry),
        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            entry: None,
            next: std::ptr::null(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entry.is_none() && self.next.is_null()
    }
}

impl<'a> Iterator for GetAll<'a> {
    type Item = &'a HeaderValue;

    fn next(&mut self) -> Option<Self::Item> {
        match self.entry.take() {
            Some(e) => Some(e.value()),
            None => {
                if self.next.is_null() {
                    return None;
                }

                let extra = unsafe { &*self.next };
                self.next = extra.next;
                Some(&extra.value)
            }
        }
    }
}
