use std::{
    iter::repeat_with,
    mem::{replace, take},
};

use super::{
    AsHeaderName, HeaderName, HeaderValue,
    entry::{Entry, GetAll},
    iter::Iter,
    name::{HeaderNameRef, IntoHeaderName},
};

type Size = u16;

/// HTTP Headers Multimap.
#[derive(Default)]
pub struct HeaderMap {
    indices: Box<[Option<Size>]>,
    entries: Vec<Entry>,
    extra_len: Size,
    delim: Size,
    is_full: bool,
}

impl HeaderMap {
    /// Create new empty [`HeaderMap`].
    ///
    /// This function does not allocate.
    #[inline]
    pub fn new() -> Self {
        Self {
            // zero sized type does not allocate
            indices: Box::new([]),
            entries: Vec::new(),
            extra_len: 0,
            delim: 0,
            is_full: true,
        }
    }

    /// Create new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// If the `capacity` is `0`, this function does not allocate.
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            return Self::new();
        }
        let new_cap = capacity.next_power_of_two();
        Self {
            indices: Vec::from_iter(repeat_with(<_>::default).take(new_cap)).into_boxed_slice(),
            entries: Vec::with_capacity(new_cap),
            extra_len: 0,
            delim: new_cap as Size * 3 / 4,
            is_full: false,
        }
    }

    /// Returns headers length.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len() + self.extra_len as usize
    }

    /// Returns `true` if headers has no element.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the headers.
    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }

    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    // pub(crate) fn entries_mut(&mut self) -> &mut Vec<Entry> {
    //     &mut self.entries
    // }

    /// Returns `true` if the map contains a header value for the header key.
    pub fn contains_key(&self, name: &HeaderName) -> bool {
        self.get(name).is_some()
    }

    /// Returns a reference to the first header value corresponding to the header name.
    #[inline]
    pub fn get<K: AsHeaderName>(&self, name: K) -> Option<&HeaderValue> {
        self.try_get(name.to_header_ref())
    }

    fn try_get(&self, name: HeaderNameRef) -> Option<&HeaderValue> {
        if self.entries.is_empty() {
            return None;
        }

        let mask = self.indices.len() as Size;
        let hash = name.hash();
        let mut index = hash & (mask - 1);

        loop {
            let entry_index = self.indices[index as usize]?;
            let entry = &self.entries[entry_index as usize];

            if entry.hash() == &hash && entry.name().as_str() == name.as_str() {
                return Some(entry.value());
            }

            // Get Collision
            index = (index + 1) & (mask - 1);
        }
    }

    /// Returns a reference to all header values corresponding to the header name.
    #[inline]
    pub fn get_all<K: AsHeaderName>(&self, name: K) -> GetAll {
        self.try_get_all(name.to_header_ref())
    }

    fn try_get_all(&self, name: HeaderNameRef) -> GetAll {
        if self.entries.is_empty() {
            return GetAll::empty();
        }

        let mask = self.indices.len() as Size;
        let hash = name.hash();
        let mut index = hash & (mask - 1);

        loop {
            let Some(entry_index) = self.indices[index as usize] else {
                return GetAll::empty();
            };
            let entry = &self.entries[entry_index as usize];

            if entry.hash() == &hash && entry.name().as_str() == name.as_str() {
                return GetAll::new(entry);
            }

            // Get Collision
            index = (index + 1) & (mask - 1);
        }
    }

    /// Removes a header from the map, returning the first header value at the key if the key was
    /// previously in the map.
    #[inline]
    pub fn remove<K: AsHeaderName>(&mut self, name: K) -> Option<HeaderValue> {
        self.try_remove(name.to_header_ref())
    }

    fn try_remove(&mut self, name: HeaderNameRef) -> Option<HeaderValue> {
        if self.entries.is_empty() {
            return None;
        }

        let mask = self.indices.len() as Size;
        let hash = name.hash();
        let mut index = hash & (mask - 1);

        loop {
            let entry_index = self.indices[index as usize]? as usize;
            let entry = &self.entries[entry_index];

            if entry.hash() == &hash && entry.name().as_str() == name.as_str() {
                if let Some(last_entry) = self.entries.last() {
                    if last_entry.hash() != entry.hash() {
                        let mut index = last_entry.hash() & (mask - 1);

                        loop {
                            let inner_entry_index = self.indices[index as usize].as_mut().unwrap();
                            let inner_entry = &self.entries[*inner_entry_index as usize];
                            if inner_entry.hash() == last_entry.hash()
                                && inner_entry.name().as_str() == last_entry.name().as_str()
                            {
                                *inner_entry_index = entry_index as Size;
                                break;
                            }
                            index = (index + 1) & (mask - 1);
                        }
                    }
                }
                let entry_index = self.indices[index as usize].take().unwrap();
                let entry = self.entries.swap_remove(entry_index as usize);
                self.extra_len -= entry.extra_len();
                let (_,value) = entry.into_parts();
                return Some(value);
            }

            // Remove Collision
            index = (index + 1) & (mask - 1);
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
    ///
    /// If the map did not have this header key present, [`None`] is returned.
    #[inline]
    pub fn insert<K: IntoHeaderName>(&mut self, name: K, value: HeaderValue) -> Option<HeaderValue> {
        self.try_insert(name.into_header_name(), value, false)
    }

    /// Append a header key and value into the map.
    ///
    /// Unlike [`insert`][HeaderMap::insert], if header key is present, header value is still
    /// appended as extra value.
    #[inline]
    pub fn append<K: IntoHeaderName>(&mut self, name: K, value: HeaderValue) {
        let _result = self.try_insert(name.into_header_name(), value, true);
        debug_assert!(_result.is_none());
    }

    fn try_insert(&mut self, name: HeaderName, value: HeaderValue, append: bool) -> Option<HeaderValue> {
        if self.is_full {
            self.increase_capacity();
        }

        let mask = self.indices.len() as Size;
        let hash = name.hash();
        let mut index = hash & (mask - 1);

        let result = loop {
            match &mut self.indices[index as usize] {
                // No collision
                index @ None => {
                    let entry_index = self.entries.len();
                    *index = Some(entry_index as _);
                    self.entries.push(Entry::new(hash, name, value));
                    break None
                },

                Some(entry_index) => {
                    let entry = &mut self.entries[*entry_index as usize];

                    if entry.hash() == &hash && entry.name().as_str() == name.as_str() {
                        break if append {
                            // Append
                            entry.push(value);
                            self.extra_len += 1;
                            None
                        } else {
                            // Returns duplicate
                            let entry = replace(entry, Entry::new(hash, name, value));
                            Some(entry.into_parts().1)
                        };
                    }

                    // Insert Collision
                    index = (index + 1) & (mask - 1);
                }
            }
        };

        self.is_full = self.entries.len() as Size > self.delim;

        result
    }

    fn increase_capacity(&mut self) {
        assert!(self.is_full, "[BUG] increasing capacity should only `is_full`");
        let new_cap = (self.indices.len() + 1).next_power_of_two().max(8);

        let mut me = HeaderMap::with_capacity(new_cap);

        for entry in take(&mut self.entries) {
            let (name,value) = entry.into_parts();
            me.try_insert(name, value, true);
        }

        println!("Resize({new_cap})");
        *self = me;
    }
}

impl std::fmt::Debug for HeaderMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Headers")
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn header_map() {
        let mut map = HeaderMap::new();

        assert!(map.get(HeaderName::new("content-type")).is_none());

        map.insert(HeaderName::new("content-type"), HeaderValue::from_string("FOO"));
        assert!(map.contains_key(&HeaderName::new("content-type")));

        map.insert(HeaderName::new("accept"), HeaderValue::from_string("BAR"));
        map.insert(HeaderName::new("content-length"), HeaderValue::from_string("LEN"));
        map.insert(HeaderName::new("host"), HeaderValue::from_string("BAR"));
        map.insert(HeaderName::new("date"), HeaderValue::from_string("BAR"));
        map.insert(HeaderName::new("referer"), HeaderValue::from_string("BAR"));
        map.insert(HeaderName::new("rim"), HeaderValue::from_string("BAR"));

        assert!(map.contains_key(&HeaderName::new("content-type")));
        assert!(map.contains_key(&HeaderName::new("accept")));
        assert!(map.contains_key(&HeaderName::new("content-length")));
        assert!(map.contains_key(&HeaderName::new("host")));
        assert!(map.contains_key(&HeaderName::new("date")));
        assert!(map.contains_key(&HeaderName::new("referer")));
        assert!(map.contains_key(&HeaderName::new("rim")));

        println!("Insert Allocate");

        map.insert(HeaderName::new("lea"), HeaderValue::from_string("BAR"));

        assert!(map.contains_key(&HeaderName::new("content-type")));
        assert!(map.contains_key(&HeaderName::new("accept")));
        assert!(map.contains_key(&HeaderName::new("content-length")));
        assert!(map.contains_key(&HeaderName::new("host")));
        assert!(map.contains_key(&HeaderName::new("date")));
        assert!(map.contains_key(&HeaderName::new("referer")));
        assert!(map.contains_key(&HeaderName::new("rim")));
        assert!(map.contains_key(&HeaderName::new("lea")));

        println!("Insert Multi");

        map.append(HeaderName::new("content-length"), HeaderValue::from_string("BAR"));

        assert!(map.contains_key(&HeaderName::new("content-length")));
        assert!(map.contains_key(&HeaderName::new("host")));
        assert!(map.contains_key(&HeaderName::new("date")));
        assert!(map.contains_key(&HeaderName::new("referer")));
        assert!(map.contains_key(&HeaderName::new("rim")));

        let mut all = map.get_all(HeaderName::new("content-length"));
        assert!(matches!(all.next(), Some(v) if matches!(v.try_as_str(),Ok("LEN"))));
        assert!(matches!(all.next(), Some(v) if matches!(v.try_as_str(),Ok("BAR"))));
        assert!(all.next().is_none());

        assert!(map.remove(HeaderName::new("accept")).is_some());
        assert!(map.contains_key(&HeaderName::new("content-type")));
        assert!(map.contains_key(&HeaderName::new("content-length")));
        assert!(map.contains_key(&HeaderName::new("host")));
        assert!(map.contains_key(&HeaderName::new("date")));
        assert!(map.contains_key(&HeaderName::new("referer")));
        assert!(map.contains_key(&HeaderName::new("rim")));
        assert!(map.contains_key(&HeaderName::new("lea")));

        assert!(map.remove(HeaderName::new("lea")).is_some());
        assert!(map.contains_key(&HeaderName::new("content-type")));
        assert!(map.contains_key(&HeaderName::new("content-length")));
        assert!(map.contains_key(&HeaderName::new("host")));
        assert!(map.contains_key(&HeaderName::new("date")));
        assert!(map.contains_key(&HeaderName::new("referer")));
        assert!(map.contains_key(&HeaderName::new("rim")));

        assert!(map.remove("content-length").is_some());

        dbg!(map.len());
        dbg!(map);
    }
}

