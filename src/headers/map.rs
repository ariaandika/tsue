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

// space-time tradeoff
// most of integer type is limited
// this limit practically should never exceeded for header length
type Size = u32;

/// Panics if the new capacity exceeds the HeaderMap capacity limit.
const fn limit_cap(cap: usize) -> Size {
    if cap <= Size::MAX as usize {
        cap as Size
    } else {
        panic!("HeaderMap capacity limit exceeded")
    }
}

/// HTTP Headers Multimap.
///
/// # Capacity Limitations
///
/// This implementation has a maximum capacity that is lower than the theoretical system limit for
/// performance reason. The exact limit is sufficient for all realistic HTTP header scenarios, as
/// even extreme cases rarely approach this boundary.
#[derive(Clone)]
pub struct HeaderMap {
    fields: NonNull<Option<HeaderField>>,
    len: Size,
    cap: Size,
}

unsafe impl Send for HeaderMap {}
unsafe impl Sync for HeaderMap {}

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
        let cap = self.cap as usize;
        unsafe { Vec::from_raw_parts(self.fields.as_ptr(), cap, cap) };
    }
}

const fn new_dangling_ptr<T>() -> NonNull<T> {
    let mut vec = Vec::<T>::new();
    let ptr = unsafe { NonNull::new_unchecked(vec.as_mut_ptr()) };
    let _ = ManuallyDrop::new(vec);
    ptr
}

impl HeaderMap {
    /// Create new empty [`HeaderMap`].
    ///
    /// This function does not allocate.
    #[inline]
    pub const fn new() -> Self {
        Self {
            fields: new_dangling_ptr(),
            len: 0,
            cap: 0,
        }
    }

    /// Create new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// If the `capacity` is `0`, this function does not allocate.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            return Self::new();
        }
        Self::with_capacity_unchecked(limit_cap(capacity).next_power_of_two())
    }

    fn with_capacity_unchecked(cap: Size) -> Self {
        // it is required that capacity is power of two,
        // see `fn mask_capacity()`
        debug_assert!(cap.is_power_of_two());

        let mut fields = ManuallyDrop::new(vec![None; cap as usize]);

        debug_assert_eq!(fields.capacity(), cap as usize);

        // `self.len` represent the field that is `Some`,
        // the underlying memory is all initialized
        Self {
            fields: unsafe { NonNull::new_unchecked(fields.as_mut_ptr()) },
            len: 0,
            cap,
        }
    }

    /// Returns headers length.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len as _
    }

    /// Returns the total number of elements the map can hold without reallocating.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.cap as _
    }

    /// Returns `true` if headers has no element.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

const fn mask_capacity(cap: Size, hash: Size) -> Size {
    // capacity is always a power of two
    // any power of two - 1 will have all the appropriate bit set to mask the hash value
    // the result is always equal to to `hash % capacity`
    hash & (cap - 1)
}

// ===== Lookup =====

impl HeaderMap {
    /// Returns `true` if the map contains a header value for given header name.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
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
        self.field(name.as_lowercase_str(), name.hash()).map(HeaderField::value)
    }

    /// Returns an iterator to all header values corresponding to the given header name.
    ///
    /// Note that this is the result of duplicate header fields, *NOT* comma separated list.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
    #[inline]
    pub fn get_all<K: AsHeaderName>(&self, name: K) -> GetAll<'_> {
        if self.is_empty() {
            return GetAll::empty();
        }
        match self.field(name.as_lowercase_str(), name.hash()) {
            Some(field) => GetAll::new(field),
            None => GetAll::empty(),
        }
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
            // the `?` is the base case of the loop, there is always `None`
            // because the load factor is capped to less than capacity
            let field = self.get_index(index as usize)?;

            if field.eq_hash_and_name(hash, name) {
                return Some(field);
            }

            // hash collision, open address linear probing
            index = mask_capacity(self.cap, index + 1);
        }
    }

    const fn get_index(&self, index: usize) -> Option<&HeaderField> {
        unsafe { self.fields.add(index).as_ref().as_ref() }
    }

    const fn get_index_mut(&mut self, index: usize) -> &mut Option<HeaderField> {
        unsafe { self.fields.add(index).as_mut() }
    }

    // `self.len` represent the field that is `Some`
    // the underlying memory is all initialized
    // so we use `self.cap` here

    pub(crate) const fn fields(&self) -> &[Option<HeaderField>] {
        unsafe { std::slice::from_raw_parts(self.fields.as_ptr(), self.cap as usize) }
    }

    const fn fields_mut(&mut self) -> &mut [Option<HeaderField>] {
        unsafe { std::slice::from_raw_parts_mut(self.fields.as_ptr(), self.cap as usize) }
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
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    ///
    /// Additionally, when using static str as the name, it must be valid header name and in
    /// lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
    #[inline]
    pub fn insert<K: IntoHeaderName>(&mut self, name: K, value: HeaderValue) -> Option<HeaderValue> {
        self.insert_inner(HeaderField::new(name.into_header_name(), value), false)
    }

    /// Append a header key and value into the map.
    ///
    /// Unlike [`insert`][HeaderMap::insert], if header key is present, header value is still
    /// appended as extra value.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    ///
    /// Additionally, when using static str as the name, it must be valid header name and in
    /// lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
    #[inline]
    pub fn append<K: IntoHeaderName>(&mut self, name: K, value: HeaderValue) {
        self.insert_inner(HeaderField::new(name.into_header_name(), value), true);
    }

    fn insert_inner(&mut self, field: HeaderField, append: bool) -> Option<HeaderValue> {
        self.reserve_one();

        let hash = field.cached_hash();
        let start_index = mask_capacity(self.cap, hash);
        let mut index = start_index;

        loop {
            match self.get_index_mut(index as usize) {
                Some(dup_field) => {
                    if dup_field.eq_hash_and_name(hash, field.name().as_str()) {
                        // duplicate header
                        break if append {
                            // Append
                            dup_field.merge(field);
                            self.len += 1;
                            None
                        } else {
                            // Returns duplicate, rest of multiple header values are dropped
                            Some(replace(dup_field, field).into_parts().1)
                        };
                    }
                }
                // this is the base case of the loop, there is always `None`
                // because the load factor is limited
                slot @ None => {
                    slot.replace(field);
                    self.len += 1;
                    return None;
                }
            }

            // hash collision, open address linear probing
            index = mask_capacity(self.cap, index + 1);
        }
    }

    fn reserve_one(&mut self) {
        const DEFAULT_MIN_ALLOC: Size = 4;

        // more optimized of `self.len as f64 / self.cap as f64 >= LOAD_FACTOR`
        // this also handle 0 capacity
        let is_load_factor_exceeded = self.len * 4 >= self.cap * 3;

        if is_load_factor_exceeded {
            let cap = if self.cap == 0 {
                DEFAULT_MIN_ALLOC
            } else {
                limit_cap((self.cap as usize) << 1)
            };
            let mut me = Self::with_capacity_unchecked(cap);

            for field in self.fields_mut().iter_mut().filter_map(Option::take) {
                me.insert_inner(field, true);
            }

            *self = me;

            debug_assert!({
                const LOAD_FACTOR: f64 = 3.0 / 4.0;

                (self.len as f64 / self.cap as f64) < LOAD_FACTOR
            })
        }
    }

    /// Reserves capacity for at least `additional` more headers.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    pub fn reserve(&mut self, additional: usize) {
        if (self.cap - self.len) as usize > additional {
            return;
        }

        let mut me = Self::with_capacity_unchecked(limit_cap((self.cap as usize) << 1));

        for field in self.fields_mut().iter_mut().filter_map(Option::take) {
            me.insert_inner(field, true);
        }

        *self = me;
    }

    /// Clear headers map, removing all the value.
    pub fn clear(&mut self) {
        if self.is_empty() {
            return;
        }
        for _ in self.fields_mut().iter_mut().map(Option::take) { }
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
        HeaderName::validate_lowercase(self.as_bytes());
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

        // assert_eq!(ptr, map.ptr.as_ptr());
        // assert_eq!(cap, map.capacity());

        // Insert Allocate

        map.insert("lea", HeaderValue::from_string("BAR"));

        assert_ne!(ptr, map.fields.as_ptr());
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

