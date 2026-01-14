use std::mem::replace;
use std::ptr::{self, NonNull};
use std::slice;

use crate::headers::HeaderName;
use crate::headers::HeaderValue;
use crate::headers::error::TryReserveError;
use crate::headers::field::{GetAll, HeaderField};
use crate::headers::matches;

// space-time tradeoff
// most of integer type is limited
// this limit practically should never exceeded for header length
type Size = u32;

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
    fields: NonNull<Option<HeaderField>>,
    /// this `len` of the headers fields, including duplicate headers
    len: Size,
    cap: Size,
}

unsafe impl Send for HeaderMap {}
unsafe impl Sync for HeaderMap {}

impl Drop for HeaderMap {
    fn drop(&mut self) {
        unsafe {
            // `self.len` represent headers length, not allocation size, all fields are
            // initialized
            let cap = self.cap as usize;
            let len = if self.is_empty() { 0 } else { cap };
            Vec::from_raw_parts(self.fields.as_ptr(), len, cap);
        }
    }
}

impl Clone for HeaderMap {
    fn clone(&self) -> Self {
        if self.is_empty() {
            return Self::new();
        }
        unsafe {
            let mut cloned = Self::with_capacity_unchecked(self.cap);

            for (src, dst) in self.fields().iter().zip(cloned.fields_mut()) {
                dst.clone_from(src);
            }

            cloned.len = self.len;
            cloned
        }
    }
}

impl HeaderMap {
    /// Create new empty [`HeaderMap`].
    ///
    /// This function does not allocate.
    #[inline]
    pub const fn new() -> Self {
        Self {
            fields: unsafe { NonNull::new_unchecked(ptr::dangling_mut()) },
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
            Some(cap) => unsafe { Ok(Self::with_capacity_unchecked(cap)) },
            None => Err(TryReserveError {}),
        }
    }

    /// Creates new empty [`HeaderMap`] with at least the specified capacity.
    ///
    /// # Safety
    ///
    /// The capacity must be a power of two.
    #[doc(hidden)]
    pub unsafe fn with_capacity_unchecked(cap: Size) -> Self {
        // it is required that capacity is power of two,
        // see `fn mask_by_capacity()`
        debug_assert!(cap.is_power_of_two());

        let fields = Box::leak(Vec::into_boxed_slice(vec![None; cap as usize]));

        Self {
            fields: unsafe { NonNull::new_unchecked(fields.as_mut_ptr()) },
            len: 0,
            cap,
        }
    }

    /// Returns headers length.
    ///
    /// This length includes duplicate header name.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len as _
    }

    /// Returns headers length, including duplicate headers.
    ///
    /// This function performs a computation, thus it best to cache the result to prevent calling
    /// it multiple time.
    pub fn total_len(&self) -> usize {
        self.fields()
            .iter()
            .filter_map(|e| e.as_ref())
            .fold(0, |acc, next| acc + next.len())
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
    pub fn iter(&self) -> crate::headers::iter::Iter<'_> {
        <&Self>::into_iter(self)
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
            Some(field) => GetAll::new(field),
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
    pub fn insert<K: IntoHeaderName>(&mut self, name: K, value: HeaderValue) -> Option<HeaderValue> {
        self.insert_inner(HeaderField::new(name.into_header_name(), value), false)
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
        self.insert_inner(HeaderField::new(name.into_header_name(), value), true);
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
    pub fn remove<K: AsHeaderName>(&mut self, name: K) -> Option<HeaderValue> {
        if self.is_empty() {
            return None;
        }
        // the rest of duplicate header values are dropped
        self.remove_inner(name.as_lowercase_str(), name.hash())
    }

    /// Reserves capacity for at least `additional` more headers.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.try_reserve(additional).unwrap()
    }

    // `self.len` represent the fields that is `Some`, the underlying memory is all initialized,
    // thats why we use `self.cap` here

    pub(crate) const fn fields(&self) -> &[Option<HeaderField>] {
        unsafe { slice::from_raw_parts(self.fields.as_ptr(), self.cap as usize) }
    }

    const fn fields_mut(&mut self) -> &mut [Option<HeaderField>] {
        unsafe { slice::from_raw_parts_mut(self.fields.as_ptr(), self.cap as usize) }
    }
}

// ===== Implementation =====

impl HeaderMap {
    fn field(&self, name: &str, hash: Size) -> Option<&HeaderField> {
        let mut index = mask_by_capacity(self.cap, hash);

        loop {
            // SAFETY: `index` is masked by capacity
            // operator `?` is the base case of the loop, there is always `None` because the load
            // factor is capped to less than capacity
            let field = unsafe { self.fields.add(index as usize).as_ref().as_ref()? };
            if field.cached_hash() == hash && field.name().as_str() == name {
                return Some(field);
            }

            // hash collision, open address linear probing
            index = mask_by_capacity(self.cap, index + 1);
        }
    }

    fn field_mut(&mut self, name: &str, hash: Size) -> &mut Option<HeaderField> {
        let mut index = mask_by_capacity(self.cap, hash);

        loop {
            // SAFETY: `index` is masked by capacity
            let field_mut = unsafe { self.fields.add(index as usize).as_mut() };
            match field_mut.as_mut() {
                Some(field) => {
                    if field.cached_hash() == hash && field.name().as_str() == name {
                        return field_mut;
                    }
                },
                // base case of the loop, there is always `None` because the load factor is capped
                // to less than capacity
                None => return field_mut
            }

            // hash collision, open address linear probing
            index = mask_by_capacity(self.cap, index + 1);
        }
    }

    fn insert_inner(&mut self, field: HeaderField, append: bool) -> Option<HeaderValue> {
        let field_len = field.len();
        self.reserve_one();

        let field_mut = self.field_mut(field.name().as_str(), field.cached_hash());
        match field_mut.as_mut() {
            Some(dup_field) => {
                debug_assert!(dup_field.cached_hash() == field.cached_hash() && dup_field.name() == field.name());

                // duplicate header
                if append {
                    // Append
                    dup_field.merge(field);
                    self.len += field_len as u32;
                    None
                } else {
                    // Returns duplicate, rest of multiple header values are dropped
                    Some(replace(dup_field, field).into_parts().1)
                }
            },
            None => {
                *field_mut = Some(field);
                self.len += field_len as u32;
                None
            },
        }
    }

    /// Perform a `swap_remove`.
    fn remove_inner(&mut self, name: &str, hash: Size) -> Option<HeaderValue> {
        let Self { cap, fields, .. } = *self;

        let field_mut = self.field_mut(name, hash);
        let field = field_mut.take()?;
        let mut swap_candidate = &mut None;

        unsafe {
            let removed_index = <*const _>::offset_from_unsigned(field_mut, fields.as_ptr()) as u32;
            let mut index = removed_index;

            loop {
                index = mask_by_capacity(cap, index + 1);
                let next_field_mut = fields.add(index as usize).as_mut();
                let Some(next_field) = next_field_mut.as_mut() else {
                    break;
                };
                let ideal_index = mask_by_capacity(cap, next_field.cached_hash());

                // if the ideal index is same as the removed index, this field is a victim of hash
                // collision, make it candidate for swapping
                if ideal_index == removed_index {
                    swap_candidate = next_field_mut;
                }
            }

            *field_mut = swap_candidate.take();
            self.len -= field.len() as u32;

            Some(field.into_parts().1)
        }
    }

    /// Reserves capacity for at least `additional` more headers.
    ///
    /// # Errors
    ///
    /// Returns error if the new capacity exceeds the HeaderMap capacity limit.
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

    fn reserve_one(&mut self) {
        const DEFAULT_MIN_ALLOC: Size = 4;

        // more optimized of `self.len as f64 / self.cap as f64 >= LOAD_FACTOR`
        // this also handle 0 capacity
        let is_load_factor_exceeded = self.len * 4 >= self.cap * 3;

        if is_load_factor_exceeded {
            let cap = if self.cap == 0 {
                DEFAULT_MIN_ALLOC
            } else {
                self.cap << 1
            };

            // SAFETY: `cap` is derived from `self.cap` or literal DEFAULT_MIN_ALLOC
            unsafe { self.reserve_unchecked(cap) };
        }
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
        for field in self.fields().iter().filter_map(|e| e.as_ref()) {
            let field = unsafe { ptr::read(field) };
            new_map.insert_inner(field, false);
        }

        self.len = 0;
        *self = new_map;
    }

    /// Clear headers map, removing all the value.
    pub fn clear(&mut self) {
        if self.is_empty() {
            return;
        }
        self.fields_mut().iter_mut().for_each(|e| {
            e.take();
        });
        self.len = 0;
    }
}

impl Default for HeaderMap {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for HeaderMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
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
