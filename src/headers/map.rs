use std::{
    iter::repeat_with,
    mem::{replace, take},
};
use tcio::bytes::ByteStr;

use super::{
    HeaderName, HeaderValue,
    field::{GetAll, HeaderField},
    iter::Iter,
    matches,
};

type Size = u32;

/// HTTP Headers Multimap.
#[derive(Default, Clone)]
pub struct HeaderMap {
    indices: Box<[Slot]>,
    fields: Vec<HeaderField>,
    extra_len: Size,
    delim: Size,
    is_full: bool,
}

#[derive(Debug, Default, Clone)]
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
        replace(self, Self::Tombstone)
    }
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
            fields: Vec::new(),
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
            fields: Vec::with_capacity(new_cap),
            extra_len: 0,
            delim: new_cap as Size * 3 / 4,
            is_full: false,
        }
    }

    /// Returns headers length.
    #[inline]
    pub const fn len(&self) -> usize {
        self.fields.len() + self.extra_len as usize
    }

    /// Returns the total number of elements the map can hold without reallocating.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.fields.capacity()
    }

    /// Returns `true` if headers has no element.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.fields.len() == 0 && self.extra_len == 0
    }
}

// ===== Lookup =====

impl HeaderMap {
    /// Returns `true` if the map contains a header value for given header name.
    #[inline]
    pub fn contains_key<K: AsHeaderName>(&self, name: K) -> bool {
        if self.is_empty() {
            return false
        }

        // `to_header_ref` may calculate hash
        self.try_get(name.as_str(), name.hash()).is_some()
    }

    /// Returns a reference to the first header value corresponding to the given header name.
    #[inline]
    pub fn get<K: AsHeaderName>(&self, name: K) -> Option<&HeaderValue> {
        if self.is_empty() {
            return None;
        }

        // `to_header_ref` may calculate hash
        self.try_get(name.as_str(), name.hash())
    }

    fn try_get(&self, name: &str, hash: Size) -> Option<&HeaderValue> {
        let mask = self.indices.len() as Size;
        let mut index = hash & (mask - 1);

        loop {
            match self.indices[index as usize] {
                Slot::Some(field_index) => {
                    let field = &self.fields[field_index as usize];

                    if field.get_hashed() == &hash && field.name().as_str().eq_ignore_ascii_case(name) {
                        return Some(field.value());
                    }
                },
                Slot::Tombstone => { },
                Slot::None => return None,
            }

            // Get Collision
            index = (index + 1) & (mask - 1);
        }
    }

    /// Returns an iterator to all header values corresponding to the given header name.
    #[inline]
    pub fn get_all<K: AsHeaderName>(&self, name: K) -> GetAll<'_> {
        if self.is_empty() {
            return GetAll::empty();
        }

        // `to_header_ref` may calculate hash
        self.try_get_all(name.as_str(), name.hash())
    }

    fn try_get_all(&self, name: &str, hash: Size) -> GetAll<'_> {
        let mask = self.indices.len() as Size;
        let mut index = hash & (mask - 1);

        loop {
            match self.indices[index as usize] {
                Slot::Some(field_index) => {
                    let field = &self.fields[field_index as usize];

                    if field.get_hashed() == &hash && field.name().as_str() == name {
                        return GetAll::new(field);
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

    /// Returns an iterator over headers as name and value pair.
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }

    /// Returns an iterator over header [`HeaderField`].
    #[inline]
    pub const fn fields(&self) -> &[HeaderField] {
        self.fields.as_slice()
    }

    // pub(crate) fn fields_mut(&mut self) -> &mut Vec<HeaderField> {
    //     &mut self.fields
    // }
}

// ===== Mutation =====

impl HeaderMap {
    /// Removes a header from the map, returning the first header value at the key if the key was
    /// previously in the map.
    pub fn remove<K: AsHeaderName>(&mut self, name: K) -> Option<HeaderValue> {
        if self.is_empty() {
            return None;
        }

        // `to_header_ref` may calculate hash
        let field = self.try_remove_field(name.as_str(), name.hash())?;

        // the rest ot duplicate header values are dropped
        let (_, val) = field.into_parts();
        Some(val)
    }

    fn try_remove_field(&mut self, name: &str, hash: Size) -> Option<HeaderField> {
        let mask = self.indices.len() as Size;
        let mut index = hash & (mask - 1);

        loop {
            match &mut self.indices[index as usize] {
                Slot::Some(field_index) => {
                    let field_index = *field_index as usize;
                    let field = &self.fields[field_index];

                    if field.get_hashed() == &hash && field.name().as_str() == name {

                        // prepare for `swap_remove` below, change indices of to be swaped field
                        if let Some(last_field) = self.fields.last().filter(|last|last.get_hashed() != field.get_hashed()) {
                            // this still possibly collisioned index
                            let mut index = last_field.get_hashed() & (mask - 1);

                            loop {
                                let Slot::Some(inner_field_index) = &mut self.indices[index as usize] else {
                                    unreachable!("[BUG] field does not have slot index")
                                };

                                let inner_field = &self.fields[*inner_field_index as usize];

                                if inner_field.get_hashed() == last_field.get_hashed()
                                    && inner_field.name().as_str() == last_field.name().as_str()
                                {
                                    *inner_field_index = field_index as Size;
                                    break;
                                }

                                // Index swapping lookup collision
                                index = (index + 1) & (mask - 1);
                            }
                        }

                        // make it tombstone
                        let Slot::Some(field_index) = self.indices[index as usize].take_as_tombstone() else {
                            unreachable!("matched in the first loop")
                        };

                        let field = self.fields.swap_remove(field_index as usize);
                        self.extra_len -= (field.len() - 1) as Size;
                        return Some(field);
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
                    let field_index = self.fields.len();
                    *index = Slot::Some(field_index as _);
                    self.fields.push(HeaderField::new(hash, name, value));
                    break None
                },

                Slot::Some(field_index) => {
                    let field = &mut self.fields[*field_index as usize];

                    if field.get_hashed() == &hash && field.name().as_str() == name.as_str() {
                        break if append {
                            // Append
                            field.push(value);
                            self.extra_len += 1;
                            None
                        } else {
                            // Returns duplicate
                            let field = replace(field, HeaderField::new(hash, name, value));
                            Some(field.into_parts().1)
                        };
                    }
                }
            }

            // Insert lookup Collision
            index = (index + 1) & (mask - 1);
        };

        self.is_full = self.fields.len() as Size > self.delim;

        result
    }

    fn increase_capacity(&mut self) {
        debug_assert!(self.is_full, "[BUG] increasing capacity should only `is_full`");
        let new_cap = (self.indices.len() + 1).next_power_of_two().max(8);

        let mut me = HeaderMap::with_capacity(new_cap);

        for field in take(&mut self.fields) {
            let (name,value) = field.into_parts();
            me.try_insert(name, value, true);
        }

        *self = me;
    }

    /// Reserves capacity for at least `additional` more headers.
    pub fn reserve(&mut self, additional: usize) {
        if self.fields.capacity() - self.fields.len() > additional {
            return;
        }

        let mut me = HeaderMap::with_capacity(self.fields.capacity() + additional);

        for field in take(&mut self.fields) {
            let (name,value) = field.into_parts();
            me.try_insert(name, value, true);
        }

        *self = me;
    }

    /// Clear headers map, removing all the value.
    pub fn clear(&mut self) {
        for index in &mut self.indices {
            take(index);
        }
        self.fields.clear();
        self.is_full = self.fields.capacity() == 0;
    }
}

impl std::fmt::Debug for HeaderMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

// ===== Ref Traits =====

/// A type that can be used for [`HeaderMap`] operation.
#[allow(private_bounds)]
pub trait AsHeaderName: SealedRef { }
trait SealedRef: Sized {
    fn hash(&self) -> Size;

    fn as_str(&self) -> &str;
}

/// for str input, calculate hash
impl AsHeaderName for &str { }
impl SealedRef for &str {
    fn hash(&self) -> Size {
        matches::hash_32(self.as_bytes())
    }

    fn as_str(&self) -> &str {
        self
    }
}

/// for HeaderName, hash may be cacheed
impl AsHeaderName for HeaderName { }
impl SealedRef for HeaderName {
    fn hash(&self) -> Size {
        HeaderName::hash(self)
    }

    fn as_str(&self) -> &str {
        HeaderName::as_str(self)
    }
}

// blanket implementation
impl<K: AsHeaderName> AsHeaderName for &K { }
impl<S: SealedRef> SealedRef for &S {
    fn hash(&self) -> Size {
        S::hash(self)
    }

    fn as_str(&self) -> &str {
        S::as_str(self)
    }
}

// ===== Owned Traits =====

/// A type that can be used for name consuming [`HeaderMap`] operation.
#[allow(private_bounds)]
pub trait IntoHeaderName: Sealed {}
trait Sealed: Sized {
    fn into_header_name(self) -> HeaderName;
}

impl IntoHeaderName for ByteStr {}
impl Sealed for ByteStr {
    fn into_header_name(self) -> HeaderName {
        // HeaderName::from_bytes(self.into_bytes())
        todo!()
    }
}

// for static data use provided constants, not static str
impl IntoHeaderName for &str {}
impl Sealed for &str {
    fn into_header_name(self) -> HeaderName {
        // HeaderName::from_bytes(self)
        todo!()
    }
}

impl IntoHeaderName for HeaderName {}
impl Sealed for HeaderName {
    fn into_header_name(self) -> HeaderName {
        self
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

        let ptr = map.fields.as_ptr();
        let cap = map.capacity();

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

        assert_eq!(ptr, map.fields.as_ptr());
        assert_eq!(cap, map.capacity());

        // Insert Allocate

        map.insert("lea", HeaderValue::from_string("BAR"));

        assert_ne!(ptr, map.fields.as_ptr());
        assert_ne!(cap, map.capacity());

        assert!(map.contains_key("Content-type"));
        assert!(map.contains_key("Accept"));
        assert!(map.contains_key("Content-length"));
        assert!(map.contains_key("Host"));
        assert!(map.contains_key("Date"));
        assert!(map.contains_key("Referer"));
        assert!(map.contains_key("Rim"));
        assert!(map.contains_key("Lea"));

        // Insert Multi

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

        // Remove accept

        assert!(map.remove("accept").is_some());
        assert!(map.contains_key("content-type"));
        assert!(map.contains_key("content-length"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("date"));
        assert!(map.contains_key("referer"));
        assert!(map.contains_key("rim"));
        assert!(map.contains_key("lea"));

        // Remove lea

        assert!(map.remove("lea").is_some());
        assert!(map.contains_key("content-type"));
        assert!(map.contains_key("content-length"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("date"));
        assert!(map.contains_key("referer"));
        assert!(map.contains_key("rim"));

        assert!(map.remove("content-length").is_some());

        // Clear

        map.clear();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
        assert!(!map.contains_key("content-type"));
        assert!(!map.contains_key("host"));
        assert!(!map.contains_key("date"));
        assert!(!map.contains_key("referer"));
        assert!(!map.contains_key("rim"));
    }
}

