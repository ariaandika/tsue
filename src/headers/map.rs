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
#[derive(Default, Clone)]
pub struct HeaderMap {
    indices: Box<[Slot]>,
    entries: Vec<Entry>,
    extra_len: Size,
    delim: Size,
    is_full: bool,
}

#[derive(Debug, Default, Clone, Copy)]
enum Slot {
    Some(Size),
    /// there is collision previously, but index removed,
    /// keed searching forward instead of giveup searching
    Tombstone,
    #[default]
    None,
}

impl Slot {
    fn take_as_tombstone(&mut self) -> Self {
        std::mem::replace(self, Self::Tombstone)
    }
}

impl HeaderMap {
    /// Create new empty [`HeaderMap`].
    ///
    /// This function does not allocate.
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
}

// ===== Lookup =====

impl HeaderMap {
    /// Returns `true` if the map contains a header value for the header key.
    #[inline]
    pub fn contains_key<K: AsHeaderName>(&self, name: K) -> bool {
        if self.is_empty() {
            return false
        }

        // `to_header_ref` may calculate hash
        self.try_get(name.to_header_ref()).is_some()
    }

    /// Returns a reference to the first header value corresponding to the header name.
    #[inline]
    pub fn get<K: AsHeaderName>(&self, name: K) -> Option<&HeaderValue> {
        if self.is_empty() {
            return None;
        }

        // `to_header_ref` may calculate hash
        self.try_get(name.to_header_ref())
    }

    fn try_get(&self, name: HeaderNameRef) -> Option<&HeaderValue> {
        let mask = self.indices.len() as Size;
        let hash = name.hash;
        let mut index = hash & (mask - 1);

        loop {
            match self.indices[index as usize] {
                Slot::Some(entry_index) => {
                    let entry = &self.entries[entry_index as usize];

                    // TODO: case sensitivity
                    if entry.get_hashed() == &hash && entry.name().as_str() == name.name {
                        return Some(entry.value());
                    }
                },
                Slot::Tombstone => { },
                Slot::None => return None,
            }

            // Get Collision
            index = (index + 1) & (mask - 1);
        }
    }

    /// Returns a reference to all header values corresponding to the header name.
    #[inline]
    pub fn get_all<K: AsHeaderName>(&self, name: K) -> GetAll {
        if self.is_empty() {
            return GetAll::empty();
        }

        // `to_header_ref` may calculate hash
        self.try_get_all(name.to_header_ref())
    }

    fn try_get_all(&self, name: HeaderNameRef) -> GetAll {
        let mask = self.indices.len() as Size;
        let hash = name.hash;
        let mut index = hash & (mask - 1);

        loop {
            match self.indices[index as usize] {
                Slot::Some(entry_index) => {
                    let entry = &self.entries[entry_index as usize];

                    if entry.get_hashed() == &hash && entry.name().as_str() == name.name {
                        return GetAll::new(entry);
                    }
                },
                Slot::Tombstone => { },
                Slot::None => {
                    return GetAll::empty();
                },
            }

            // Get Collision
            index = (index + 1) & (mask - 1);
        }
    }

    /// Returns an iterator over the headers.
    #[inline]
    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }

    #[inline]
    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    // pub(crate) fn entries_mut(&mut self) -> &mut Vec<Entry> {
    //     &mut self.entries
    // }
}

// ===== Mutation =====

impl HeaderMap {
    pub fn reserve(&mut self, _capacity: usize) {
        // TODO: HeaderMap::reserve
    }

    /// Removes a header from the map, returning the first header value at the key if the key was
    /// previously in the map.
    pub fn remove<K: AsHeaderName>(&mut self, name: K) -> Option<HeaderValue> {
        if self.is_empty() {
            return None;
        }

        // `to_header_ref` may calculate hash
        let entry = self.try_remove_entry(name.to_header_ref())?;

        // the rest ot duplicate header values are dropped
        let (_, val) = entry.into_parts();
        Some(val)
    }

    fn try_remove_entry(&mut self, name: HeaderNameRef) -> Option<Entry> {
        let mask = self.indices.len() as Size;
        let hash = name.hash;
        let mut index = hash & (mask - 1);

        loop {
            match &mut self.indices[index as usize] {
                Slot::Some(entry_index) => {
                    let entry_index = *entry_index as usize;
                    let entry = &self.entries[entry_index];

                    if entry.get_hashed() == &hash && entry.name().as_str() == name.name {

                        // prepare for `swap_remove` below, change indices of to be swaped entry
                        if let Some(last_entry) = self.entries.last().filter(|last|last.get_hashed() != entry.get_hashed()) {
                            // this still possibly collisioned index
                            let mut index = last_entry.get_hashed() & (mask - 1);

                            loop {
                                let Slot::Some(inner_entry_index) = &mut self.indices[index as usize] else {
                                    unreachable!("[BUG] entry does not have slot index")
                                };

                                let inner_entry = &self.entries[*inner_entry_index as usize];

                                if inner_entry.get_hashed() == last_entry.get_hashed()
                                    && inner_entry.name().as_str() == last_entry.name().as_str()
                                {
                                    *inner_entry_index = entry_index as Size;
                                    break;
                                }

                                // Index swapping lookup collision
                                index = (index + 1) & (mask - 1);
                            }
                        }

                        // make it tombstone
                        let Slot::Some(entry_index) = self.indices[index as usize].take_as_tombstone() else {
                            unreachable!("matched in the first loop")
                        };

                        let entry = self.entries.swap_remove(entry_index as usize);
                        self.extra_len -= entry.extra_len();
                        return Some(entry);
                    }
                }
                Slot::Tombstone => { },
                Slot::None => return None,
            }

            // Remove lookup collision
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
        self.try_insert(name.into_header_name(), value, true);
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
                index @ Slot::None | index @ Slot::Tombstone => {
                    let entry_index = self.entries.len();
                    *index = Slot::Some(entry_index as _);
                    self.entries.push(Entry::new(hash, name, value));
                    break None
                },

                Slot::Some(entry_index) => {
                    let entry = &mut self.entries[*entry_index as usize];

                    if entry.get_hashed() == &hash && entry.name().as_str() == name.as_str() {
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
                }
            }

            // Insert lookup Collision
            index = (index + 1) & (mask - 1);
        };

        self.is_full = self.entries.len() as Size > self.delim;

        result
    }

    fn increase_capacity(&mut self) {
        debug_assert!(self.is_full, "[BUG] increasing capacity should only `is_full`");
        let new_cap = (self.indices.len() + 1).next_power_of_two().max(8);

        let mut me = HeaderMap::with_capacity(new_cap);

        for entry in take(&mut self.entries) {
            let (name,value) = entry.into_parts();
            me.try_insert(name, value, true);
        }

        *self = me;
    }

    // TODO: HeaderMap::clear
}

impl std::fmt::Debug for HeaderMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn header_map() {
        let mut map = HeaderMap::new();

        assert!(map.get("content-type").is_none());

        map.insert("content-type", HeaderValue::from_string("FOO"));
        assert!(map.contains_key("content-type"));

        map.insert("accept", HeaderValue::from_string("BAR"));
        map.insert("content-length", HeaderValue::from_string("LEN"));
        map.insert("host", HeaderValue::from_string("BAR"));
        map.insert("date", HeaderValue::from_string("BAR"));
        map.insert("referer", HeaderValue::from_string("BAR"));
        map.insert("rim", HeaderValue::from_string("BAR"));

        assert!(map.contains_key("content-type"));
        assert!(map.contains_key("accept"));
        assert!(map.contains_key("content-length"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("date"));
        assert!(map.contains_key("referer"));
        assert!(map.contains_key("rim"));

        println!("Insert Allocate");

        map.insert("lea", HeaderValue::from_string("BAR"));

        assert!(map.contains_key("content-type"));
        assert!(map.contains_key("accept"));
        assert!(map.contains_key("content-length"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("date"));
        assert!(map.contains_key("referer"));
        assert!(map.contains_key("rim"));
        assert!(map.contains_key("lea"));

        println!("Insert Multi");

        map.append("content-length", HeaderValue::from_string("BAR"));

        assert!(map.contains_key("content-length"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("date"));
        assert!(map.contains_key("referer"));
        assert!(map.contains_key("rim"));

        let mut all = map.get_all("content-length");
        assert!(matches!(all.next(), Some(v) if matches!(v.try_as_str(),Ok("LEN"))));
        assert!(matches!(all.next(), Some(v) if matches!(v.try_as_str(),Ok("BAR"))));
        assert!(all.next().is_none());

        println!("Remove accept");

        assert!(map.remove("accept").is_some());
        assert!(map.contains_key("content-type"));
        assert!(map.contains_key("content-length"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("date"));
        assert!(map.contains_key("referer"));
        assert!(map.contains_key("rim"));
        assert!(map.contains_key("lea"));

        println!("Remove lea");

        assert!(map.remove("lea").is_some());
        assert!(map.contains_key("content-type"));
        assert!(map.contains_key("content-length"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("date"));
        assert!(map.contains_key("referer"));
        assert!(map.contains_key("rim"));

        assert!(map.remove("content-length").is_some());

        dbg!(map.len());
        dbg!(map);
    }
}

