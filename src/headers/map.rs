use std::{mem, ptr, slice};

use crate::headers::error::TryReserveError;
use crate::headers::field::HeaderField;
use crate::headers::iter::{GetAll, Iter};
use crate::headers::matches;
use crate::headers::{HeaderName, HeaderValue};

// space-time tradeoff
// most of integer type is limited
// this limit practically should never exceeded for header length
type Size = u32;

const MAX_SIZE: Size = !(Size::MAX >> 1);

const fn mask_by_capacity(cap: Size, value: Size) -> Size {
    // capacity is always a power of two
    // any power of two - 1 will have all the appropriate bit set to mask the hash value
    // the result is always equal to to `hash % capacity`
    value & (cap - 1)
}

/// HTTP Headers Multimap.
///
/// # Header Name
///
/// All operations that requires header name as parameter, can accept either static `str` or
/// [`HeaderName`] as the value.
///
/// When using static `str`, it must be valid header name in ASCII lowercase.
///
/// It is prefered to use `HeaderName`'s [provided constants] instead of static `str` as it can
/// utilize the cached hash code as oppose to static `str` which calculate it on demand.
///
/// If there is no constant provided for wanted header, use [`HeaderName::from_static`] to validate
/// and calculate hash code at compile time.
///
/// # Hash Function
///
/// `HeaderMap` **DOES NOT** use hashing algorithm that provide resistance against HashDoS attacks.
/// It is expected that user will limit the number of headers to much lower number than the amount
/// of where HashDoS attack is a concern.
///
/// # Capacity Limitations
///
/// This implementation has a maximum capacity that is lower than the system limit. The exact limit
/// is sufficient for all HTTP headers use cases.
///
/// [provided constants]: crate::headers::standard
pub struct HeaderMap {
    /// - the allocation where the header field stored
    /// - all values are initialized `self.cap` size
    fields: ptr::NonNull<Option<HeaderField>>,
    /// `self.len` is fields that is some, all fields are initialized
    len: Size,
    cap: Size,
}

unsafe impl Send for HeaderMap {}
unsafe impl Sync for HeaderMap {}

impl Drop for HeaderMap {
    fn drop(&mut self) {
        let cap = self.cap as usize;
        let len = if self.is_empty() { 0 } else { cap };
        // SAFETY: `self.len` represent fields that is some, all fields are initialized, len 0 if
        // map empty is to prevent `Vec::drop` iterating what will be all `None` elements
        unsafe { Vec::from_raw_parts(self.fields.as_ptr(), len, cap) };
    }
}

impl Clone for HeaderMap {
    fn clone(&self) -> Self {
        if self.is_empty() {
            return Self::new();
        }

        // SAFETY: `self.cap` is valid capacity
        let mut cloned = unsafe { Self::with_capacity_unchecked(self.cap) };

        for (src, dst) in self.fields().iter().zip(cloned.fields_mut()) {
            dst.clone_from(src);
        }

        cloned.len = self.len;
        cloned
    }
}

impl Default for HeaderMap {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl HeaderMap {
    /// Create new empty [`HeaderMap`].
    ///
    /// This function does not allocate.
    #[inline]
    pub const fn new() -> Self {
        Self {
            fields: const { ptr::NonNull::new(ptr::dangling_mut()).unwrap() },
            len: 0,
            cap: 0,
        }
    }

    /// Creates new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// The header map will be able to hold at least capacity headers without reallocating. If
    /// capacity is zero, the header map will not allocate.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the capacity limit.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("failed to create HeaderMap")
    }

    /// Creates new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// The header map will be able to hold at least capacity headers without reallocating. If
    /// capacity is zero, the header map will not allocate.
    ///
    /// # Errors
    ///
    /// Returns an error if the new capacity exceeds the capacity limit.
    #[inline]
    pub fn try_with_capacity(capacity: usize) -> Result<Self, TryReserveError> {
        if capacity == 0 {
            return Ok(Self::new());
        }
        match Size::try_from(capacity)
            .ok()
            .and_then(Size::checked_next_power_of_two)
        {
            Some(cap) => Ok(Self::with_capacity_size(cap)),
            None => Err(TryReserveError {}),
        }
    }

    /// Creates new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// # Panics
    ///
    /// The capacity must be a power of two, otherwise panics.
    #[doc(hidden)]
    #[inline]
    pub fn with_capacity_size(cap: Size) -> Self {
        assert!(cap.is_power_of_two());
        unsafe { Self::with_capacity_unchecked(cap) }
    }

    /// Creates new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// # Safety
    ///
    /// The capacity must be a power of two.
    #[doc(hidden)]
    #[inline]
    pub unsafe fn with_capacity_unchecked(cap: Size) -> Self {
        // it is required that capacity is power of two,
        // see `fn mask_by_capacity()`
        debug_assert!(cap.is_power_of_two());
        let ptr = Vec::into_raw_parts(vec![None; cap as usize]).0;
        let fields = ptr::NonNull::new(ptr).expect("Vec ptr is non null");
        let len = 0;
        Self { fields, len, cap }
    }

    /// Returns headers length.
    ///
    /// This length includes duplicate header name.
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

    /// Returns an iterator over headers as name and value pair.
    #[inline]
    pub fn pairs(&self) -> Iter<'_> {
        self.into_iter()
    }

    /// Returns `true` if the map contains a header value for given header name.
    ///
    /// For header name it is prefered to use [provided constants] as oppose to static `str`, see
    /// [`HeaderMap`] documentation for more details.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// [provided constants]: crate::headers::standard
    #[inline]
    pub fn contains_key<K: AsHeaderName>(&self, name: K) -> bool {
        if self.is_empty() {
            return false
        }
        self.field(name.as_lowercase_str(), name.hash()).is_some()
    }

    /// Returns a reference to the first header value corresponding to the given header name.
    ///
    /// For header name it is prefered to use [provided constants] as oppose to static `str`, see
    /// [`HeaderMap`] documentation for more details.
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
    /// [provided constants]: crate::headers::standard
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
    /// For header name it is prefered to use [provided constants] as oppose to static `str`, see
    /// [`HeaderMap`] documentation for more details.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// [provided constants]: crate::headers::standard
    #[inline]
    pub fn get_all<K: AsHeaderName>(&self, name: K) -> GetAll<'_> {
        if self.is_empty() {
            return GetAll::empty();
        }
        match self.field(name.as_lowercase_str(), name.hash()) {
            Some(field) => field.iter(),
            None => GetAll::empty(),
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did have this key present, the value is updated, and the old value is returned
    /// as `Some`.
    ///
    /// For header name it is prefered to use [provided constants] as oppose to static `str`, see
    /// [`HeaderMap`] documentation for more details.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// [provided constants]: crate::headers::standard
    #[inline]
    pub fn insert<K: IntoHeaderName>(&mut self, name: K, value: HeaderValue) -> Option<HeaderField> {
        self.reserve_one().expect("cannot insert header");
        unsafe { self.insert_inner(HeaderField::new(name.into_header_name(), value), false) }
    }

    /// Append a header key and value into the map.
    ///
    /// Unlike [`insert`][HeaderMap::insert], if header key is present, header value is still
    /// appended as extra value.
    ///
    /// For header name it is prefered to use [provided constants] as oppose to static `str`, see
    /// [`HeaderMap`] documentation for more details.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// [provided constants]: crate::headers::standard
    #[inline]
    pub fn append<K: IntoHeaderName>(&mut self, name: K, value: HeaderValue) {
        self.reserve_one().expect("cannot append header");
        unsafe { self.insert_inner(HeaderField::new(name.into_header_name(), value), true) };
    }

    pub(crate) fn try_append(
        &mut self,
        name: HeaderName,
        value: HeaderValue,
        hash: u32,
    ) -> Result<(), TryReserveError> {
        self.reserve_one()?;
        unsafe { self.insert_inner(HeaderField::with_hash(name, value, hash), true) };
        Ok(())
    }

    /// Removes a header from the map, returning the first header value if it founds.
    ///
    /// For header name it is prefered to use [provided constants] as oppose to static `str`, see
    /// [`HeaderMap`] documentation for more details.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// [provided constants]: crate::headers::standard
    #[inline]
    pub fn remove<K: AsHeaderName>(&mut self, name: K) -> Option<HeaderField> {
        if self.is_empty() {
            return None;
        }
        self.remove_inner(name.as_lowercase_str(), name.hash())
    }

    /// Reserves capacity for at least `additional` more headers.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.try_reserve(additional).expect("failed to reserve capacity")
    }

    /// Clear headers map, removing all the value.
    #[inline]
    pub fn clear(&mut self) {
        if self.is_empty() {
            return;
        }
        for field_mut in self.fields_mut() {
            field_mut.take();
        }
        self.len = 0;
    }

    // `self.len` represent the fields that is `Some`, the underlying memory is all initialized,
    // thats why we use `self.cap` here

    pub(crate) const fn fields(&self) -> &[Option<HeaderField>] {
        unsafe { slice::from_raw_parts(self.fields.as_ptr(), self.cap as usize) }
    }

    pub(crate) const fn fields_mut(&mut self) -> &mut [Option<HeaderField>] {
        unsafe { slice::from_raw_parts_mut(self.fields.as_ptr(), self.cap as usize) }
    }
}

// ===== Implementation =====

impl HeaderMap {
    fn field(&self, name: &str, hash: Size) -> Option<&HeaderField> {
        let mut index = hash;

        loop {
            index = mask_by_capacity(self.cap, index);
            // SAFETY: `index` is masked by capacity
            // `?` is the base case of the loop, there is always `None` because the load
            // factor is capped to less than capacity
            let field = unsafe { self.fields.add(index as usize).as_ref().as_ref()? };
            if field.cached_hash() == hash && field.name().as_str() == name {
                return Some(field);
            }

            // linear probing
            index += 1;
        }
    }

    /// # Safety
    ///
    /// `self.len < self.cap`
    unsafe fn insert_inner(&mut self, field: HeaderField, append: bool) -> Option<HeaderField> {
        debug_assert!(self.len < self.cap);

        let mut index = field.cached_hash();

        loop {
            index = mask_by_capacity(self.cap, index);
            // SAFETY: `index` is masked by capacity
            let field_mut = unsafe { self.fields.add(index as usize).as_mut() };
            let Some(dup_field) = field_mut.as_mut() else {
                // found empty slot
                *field_mut = Some(field);
                self.len += 1;
                return None
            };

            if field.cached_hash() == dup_field.cached_hash() && field.name() == dup_field.name() {
                // duplicate Header

                if !append {
                    // replace and returns duplicate
                    return Some(mem::replace(dup_field, field));
                }

                // appending, look for the next empty slot
            }

            // linear probing
            index += 1;
        }
    }

    /// Removing is not an O(1) operation if there is hash collision
    fn remove_inner(&mut self, name: &str, hash: Size) -> Option<HeaderField> {
        let mut removed_index = hash;
        let mut field_mut;

        loop {
            removed_index = mask_by_capacity(self.cap, removed_index);
            // SAFETY: `removed_index` is masked by capacity
            field_mut = unsafe { self.fields.add(removed_index as usize).as_mut() };
            // `?` is the base case
            let field_ref = field_mut.as_ref()?;

            if hash == field_ref.cached_hash() && name == field_ref.name().as_str() {
                break;
            }

            // linear probing
            removed_index += 1;
        };

        let mut swap_candidate = &mut None;
        let mut index = removed_index;

        loop {
            index = mask_by_capacity(self.cap, index + 1);
            // SAFETY: `index` is masked by capacity
            let next_field_mut = unsafe { self.fields.add(index as usize).as_mut() };
            let Some(next_field) = next_field_mut.as_mut() else {
                break;
            };
            let ideal_index = mask_by_capacity(self.cap, next_field.cached_hash());

            // if the ideal index is same as the removed index, this field is a victim of hash
            // collision, make it candidate for swapping
            //
            // this also make sure to not swap the field that is in its ideal index
            if ideal_index == removed_index {
                swap_candidate = next_field_mut;
            }
        }

        debug_assert!(field_mut.is_some());

        self.len -= 1;
        mem::replace(field_mut, swap_candidate.take())
    }

    /// Reserves capacity for at least `additional` more headers.
    ///
    /// # Errors
    ///
    /// Returns error if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        if (self.cap - self.len) as usize > additional {
            return Ok(());
        }

        let Some(new_cap) = Size::try_from(additional)
            .ok()
            .and_then(|e|e.checked_add(self.cap))
            .and_then(Size::checked_next_power_of_two)
        else {
            return Err(TryReserveError {});
        };

        // SAFETY: `new_cap` is power of two
        unsafe { self.reserve_unchecked(new_cap) };
        Ok(())
    }

    #[inline]
    fn reserve_one(&mut self) -> Result<(), TryReserveError> {
        const DEFAULT_MIN_ALLOC: Size = 4;

        // more optimized of `self.len as f64 / self.cap as f64 >= LOAD_FACTOR`
        // this also handle 0 capacity
        let is_load_factor_exceeded = self.len * 4 >= self.cap * 3;

        if is_load_factor_exceeded {
            if self.cap == MAX_SIZE {
                return Err(TryReserveError {});
            }

            let cap = if self.cap == 0 {
                DEFAULT_MIN_ALLOC
            } else {
                self.cap << 1
            };

            // SAFETY: `cap` is derived from `self.cap` or literal DEFAULT_MIN_ALLOC
            unsafe { self.reserve_unchecked(cap) };
        }

        Ok(())
    }

    /// Reserves capacity for exactly `new_cap` headers.
    ///
    /// # Safety
    ///
    /// `new_cap` must be a power of two.
    unsafe fn reserve_unchecked(&mut self, new_cap: Size) {
        debug_assert!(new_cap.is_power_of_two());

        // SAFETY: `new_cap` is power of two
        let mut new_map = unsafe { Self::with_capacity_unchecked(new_cap) };

        // move all values to the newly allocated map
        for field in self.fields_mut().iter_mut().filter_map(Option::take) {
            // SAFETY: capacity is more than current capacity
            unsafe { new_map.insert_inner(field, true) };
        }

        self.len = 0;
        *self = new_map;
    }
}

impl std::fmt::Debug for HeaderMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.pairs()).finish()
    }
}

// ===== Ref Traits =====

/// A type that can be used for [`HeaderMap`]'s lookup operations.
///
/// It is prefered to use [provided constants] as oppose to static `str`, see [`HeaderMap`]
/// documentation for more details.
///
/// [provided constants]: crate::headers::standard
pub trait AsHeaderName: sealed_ref::SealedRef { }
mod sealed_ref {
    use super::*;

    pub trait SealedRef {
        fn hash(&self) -> Size;

        /// Returns lowercase string
        fn as_lowercase_str(&self) -> &str;
    }

    /// for str input, calculate hash
    ///
    /// will panics if it contains invalid header name character.
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
}

// ===== Owned Traits =====

/// A type that can be used for [`HeaderMap`]'s `insert` or `append` operation.
///
/// It is prefered to use [provided constants] as oppose to static `str`, see [`HeaderMap`]
/// documentation for more details.
///
/// [provided constants]: crate::headers::standard
pub trait IntoHeaderName: sealed::Sealed {}
mod sealed {
    use super::*;

    pub trait Sealed {
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
}
