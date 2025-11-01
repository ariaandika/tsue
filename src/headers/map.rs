use std::{
    mem::{ManuallyDrop, replace},
    ptr::NonNull,
};

use super::{
    HeaderName, HeaderValue,
    field::{GetAll, HeaderField},
    iter::Iter,
    matches,
};

type Size = u32;

/// HTTP Headers Multimap.
#[derive(Clone)]
pub struct HeaderMap {
    ptr: NonNull<Option<HeaderField>>,
    len: usize,
    cap: usize,
}

unsafe impl Send for HeaderMap { }
unsafe impl Sync for HeaderMap { }

impl Default for HeaderMap {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HeaderMap {
    fn drop(&mut self) {
        // `self.len` is actually represent the field that `Some`
        // the underlying memory is actually all initialized
        // so we use `self.cap` here
        unsafe { Vec::from_raw_parts(self.ptr.as_ptr(), self.cap, self.cap) };
    }
}

impl HeaderMap {
    /// Create new empty [`HeaderMap`].
    ///
    /// This function does not allocate.
    #[inline]
    pub const fn new() -> Self {
        let mut vec = Vec::new();
        let ptr = unsafe { NonNull::new_unchecked(vec.as_mut_ptr()) };
        let _ = ManuallyDrop::new(vec);
        Self {
            ptr,
            len: 0,
            cap: 0,
        }
    }

    /// Create new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// If the `capacity` is `0`, this function does not allocate.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            return Self::new();
        }
        Self::with_capacity_unchecked(capacity.next_power_of_two())
    }

    fn with_capacity_unchecked(capacity: usize) -> Self {
        // it is required that capacity is power of two,
        // see `fn mask_capacity()`
        debug_assert!(capacity.is_power_of_two());

        let mut vec = ManuallyDrop::new(vec![None; capacity]);

        // `self.len` is actually represent the field that is `Some`
        // the underlying memory is actually all initialized
        Self {
            ptr: unsafe { NonNull::new_unchecked(vec.as_mut_ptr()) },
            len: 0,
            cap: vec.capacity(),
        }
    }

    /// Returns headers length.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns the total number of elements the map can hold without reallocating.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.cap
    }

    /// Returns `true` if headers has no element.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

const fn mask_capacity(cap: usize, hash: Size) -> Size {
    // capacity is always a power of two
    // any power of two - 1 will have all the appropriate bit set to mask the hash value
    // the result is always equal to to `hash % capacity`
    hash & (cap - 1) as Size
}

// ===== Lookup =====

impl HeaderMap {
    /// Returns `true` if the map contains a header value for given header name.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    #[inline]
    pub fn contains_key<K: AsHeaderName>(&self, name: K) -> bool {
        if self.is_empty() {
            return false
        }
        self.field(name.as_lowercase_str(), name.hash()).is_some()
    }

    /// Returns a reference to the first header value corresponding to the given header name.
    ///
    /// Key can be valid static str, but note that using [`HeaderName`] directly is more performant
    /// because the hash is calculated at compile time.
    ///
    /// ```rust
    /// use tsue::headers::{standard::{CONTENT_TYPE, DATE}, HeaderMap, HeaderValue};
    ///
    /// let mut map = HeaderMap::new();
    /// map.insert(CONTENT_TYPE, HeaderValue::from_static(b"text/html"));
    /// assert_eq!(map.get(CONTENT_TYPE).unwrap().as_str(), "text/html");
    ///
    /// let ctype = map.get(CONTENT_TYPE);
    /// ```
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
    #[inline]
    pub fn get<K: AsHeaderName>(&self, name: K) -> Option<&HeaderValue> {
        if self.is_empty() {
            return None;
        }
        match self.field(name.as_lowercase_str(), name.hash()) {
            Some(field) => Some(field.value()),
            None => None,
        }
    }

    /// Returns an iterator to all header values corresponding to the given header name.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
    #[inline]
    pub fn get_all<K: AsHeaderName>(&self, name: K) -> Option<GetAll<'_>> {
        if self.is_empty() {
            return None;
        }
        self.field(name.as_lowercase_str(), name.hash()).map(GetAll::new)
    }

    /// Returns an iterator over headers as name and value pair.
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }

    fn field(&self, name: &str, hash: Size) -> Option<&HeaderField> {
        let start_index = mask_capacity(self.cap, hash);
        let mut index = start_index;

        loop {
            match self.get_index(index as usize) {
                Some(field) => {
                    if field.eq_hash_and_name(hash, name) {
                        return Some(field);
                    }
                },
                // this is the base case of the loop, there is always `None`
                // because the load factor is limited
                None => return None,
            }

            // hash collision, open address linear probing
            index = mask_capacity(self.cap, index + 1);
        }
    }

    const fn get_index(&self, index: usize) -> &Option<HeaderField> {
        unsafe { self.ptr.add(index).as_ref() }
    }

    const fn get_index_mut(&mut self, index: usize) -> &mut Option<HeaderField> {
        unsafe { self.ptr.add(index).as_mut() }
    }

    // `self.len` is actually represent the field that is `Some`
    // the underlying memory is actually all initialized
    // so we use `self.cap` here

    pub(crate) const fn fields(&self) -> &[Option<HeaderField>] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.cap) }
    }

    const fn fields_mut(&mut self) -> &mut [Option<HeaderField>] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.cap) }
    }
}

// ===== Mutation =====

impl HeaderMap {
    /// Removes a header from the map, returning the first header value at the key if the key was
    /// previously in the map.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
    pub fn remove<K: AsHeaderName>(&mut self, name: K) -> Option<HeaderValue> {
        if self.is_empty() {
            return None;
        }
        // the rest of duplicate header values are dropped
        self.try_remove_field(name.as_lowercase_str(), name.hash()).map(|field| field.into_parts().1)
    }

    fn try_remove_field(&mut self, name: &str, hash: Size) -> Option<HeaderField> {
        let start_index = mask_capacity(self.cap, hash);
        let mut index = start_index;

        loop {
            let slot = self.get_index_mut(index as usize);

            // LATEST: robin hood hashing
            // use two allocation for the hash and header field, that way when eviction as of robin hood
            // hashing happens, only small amount of memory (the hash value) is copied

            if slot.as_ref()?.eq_hash_and_name(hash, name) {
                let Some(field) = slot.take() else {
                    // guaranteed by the `?` operator
                    unsafe { std::hint::unreachable_unchecked() }
                };
                self.len -= 1;

                // backward shifting
                let cap = self.cap;
                let mut next_index = mask_capacity(cap, index + 1);

                loop {
                    let Some(next_slot) = self.get_index_mut(next_index as usize) else {
                        break;
                    };

                    let ideal_index = mask_capacity(cap, next_slot.cached_hash());

                    if ideal_index == index {
                        let Some(slot) = self.get_index_mut(next_index as usize).take() else {
                            // guaranteed by the `let else` at the start of the loop
                            unsafe { std::hint::unreachable_unchecked() }
                        };
                        self.get_index_mut(index as usize).replace(slot);
                        self.len -= 1;

                        index = next_index;
                    }

                    next_index = mask_capacity(cap, next_index + 1);
                }


                return Some(field);
            }

            // hash collision, open address linear probing
            index = mask_capacity(self.cap, index + 1);
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
    ///
    /// If the map did not have this header key present, [`None`] is returned.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
    #[inline]
    pub fn insert<K: IntoHeaderName>(&mut self, name: K, value: HeaderValue) -> Option<HeaderValue> {
        self.try_insert(name.into_header_name(), value, false)
    }

    /// Append a header key and value into the map.
    ///
    /// Unlike [`insert`][HeaderMap::insert], if header key is present, header value is still
    /// appended as extra value.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
    #[inline]
    pub fn append<K: IntoHeaderName>(&mut self, name: K, value: HeaderValue) {
        self.try_insert(name.into_header_name(), value, true);
    }

    fn try_insert(&mut self, name: HeaderName, value: HeaderValue, append: bool) -> Option<HeaderValue> {
        self.reserve_one();

        let hash = name.hash();
        let start_index = mask_capacity(self.cap, hash);
        let mut index = start_index;

        loop {
            match self.get_index_mut(index as usize) {
                Some(field) => {
                    if field.eq_hash_and_name(hash, name.as_str()) {
                        // duplicate header
                        break if append {
                            // Append
                            field.push(value);
                            self.len += 1;
                            None
                        } else {
                            // Returns duplicate
                            Some(replace(field, HeaderField::new(hash, name, value)).into_parts().1)
                        };
                    }
                }
                // this is the base case of the loop, there is always `None`
                // because the load factor is limited
                slot @ None => {
                    slot.replace(HeaderField::new(hash, name, value));
                    self.len += 1;
                    return None;
                }
            }

            // hash collision, open address linear probing
            index = mask_capacity(self.cap, index + 1);
        }
    }

    fn reserve_one(&mut self) {
        const LOAD_FACTOR: f64 = 0.7;

        if self.cap == 0 || self.len as f64 / self.cap as f64 >= LOAD_FACTOR {
            let cap = if self.cap == 0 { 2 } else { self.cap << 1 };
            let mut me = Self::with_capacity_unchecked(cap);

            for field in self.fields_mut().iter_mut().filter_map(Option::take) {
                // FIXME: extra header value is dropped
                let (name,value) = field.into_parts();
                me.try_insert(name, value, true);
            }

            *self = me;

            debug_assert!((self.len as f64 / self.cap as f64) < LOAD_FACTOR)
        }
    }

    /// Reserves capacity for at least `additional` more headers.
    pub fn reserve(&mut self, additional: usize) {
        if self.cap - self.len > additional {
            return;
        }

        let mut me = Self::with_capacity_unchecked(self.cap << 1);

        for field in self.fields_mut().iter_mut().filter_map(Option::take) {
            let (name,value) = field.into_parts();
            me.try_insert(name, value, true);
        }

        *self = me;
    }

    /// Clear headers map, removing all the value.
    pub fn clear(&mut self) {
        for _ in self.fields_mut().iter_mut().filter_map(Option::take) { }
        self.len = 0;
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

    /// Returns lowercase string
    fn as_lowercase_str(&self) -> &str;
}

/// for str input, calculate hash
impl AsHeaderName for &'static str { }
impl SealedRef for &'static str {
    #[inline]
    fn hash(&self) -> Size {
        matches::hash_32(self.as_bytes())
    }

    #[inline]
    fn as_lowercase_str(&self) -> &str {
        assert!(self.chars().all(|e|!e.is_ascii_uppercase()), "static header name must be in lowercase");
        self
    }
}

/// for HeaderName, hash may be cached
impl AsHeaderName for HeaderName { }
impl SealedRef for HeaderName {
    #[inline]
    fn hash(&self) -> Size {
        HeaderName::hash(self)
    }

    #[inline]
    fn as_lowercase_str(&self) -> &str {
        HeaderName::as_str(self)
    }
}

// blanket implementation
impl<K: AsHeaderName> AsHeaderName for &K { }
impl<S: SealedRef> SealedRef for &S {
    #[inline]
    fn hash(&self) -> Size {
        S::hash(self)
    }

    #[inline]
    fn as_lowercase_str(&self) -> &str {
        S::as_lowercase_str(self)
    }
}

// ===== Owned Traits =====

/// A type that can be used for name consuming [`HeaderMap`] operation.
#[allow(private_bounds)]
pub trait IntoHeaderName: Sealed {}
trait Sealed: Sized {
    fn into_header_name(self) -> HeaderName;
}

// for static data use provided constants, not static str
impl IntoHeaderName for &'static str {}
impl Sealed for &'static str {
    #[inline]
    fn into_header_name(self) -> HeaderName {
        HeaderName::from_static(self.as_bytes())
    }
}

impl IntoHeaderName for HeaderName {}
impl Sealed for HeaderName {
    #[inline]
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

        let ptr = map.ptr.as_ptr();
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

        // assert_eq!(ptr, map.ptr.as_ptr());
        // assert_eq!(cap, map.capacity());

        // Insert Allocate

        map.insert("lea", HeaderValue::from_string("BAR"));

        assert_ne!(ptr, map.ptr.as_ptr());
        assert_ne!(cap, map.capacity());

        assert!(map.contains_key("content-type"));
        assert!(map.contains_key("accept"));
        assert!(map.contains_key("content-length"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("date"));
        assert!(map.contains_key("referer"));
        assert!(map.contains_key("rim"));
        assert!(map.contains_key("lea"));

        // Insert Multi

        map.append("content-length", HeaderValue::from_string("BAR"));

        assert!(map.contains_key("content-length"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("date"));
        assert!(map.contains_key("referer"));
        assert!(map.contains_key("rim"));

        let mut all = map.get_all("content-length").unwrap();
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

