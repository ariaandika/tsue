use std::mem::{ManuallyDrop, replace};
use std::ptr::{self, NonNull};
use std::slice;

use crate::headers::HeaderName;
use crate::headers::HeaderValue;
use crate::headers::error::TryReserveError;
use crate::headers::field::{GetAll, HeaderField};
use crate::headers::iter::Iter;
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
#[derive(Clone)]
pub struct HeaderMap {
    /// the allocation where the header field stored
    /// `self.len` represent fields that is initialized
    /// this list is sorted by insertion
    fields: NonNull<HeaderField>,
    /// (hash, index)
    /// hash to the header name for lookup
    /// index to the fields
    /// `self.len` represent slots that is `Some`,
    /// the entire memory is initialized
    slots: NonNull<Slot>,
    /// this `len` of the headers fields, excluding duplicate headers
    len: Size,
    cap: Size,
}

#[derive(Debug, Clone, Copy)]
enum Slot {
    None,
    Some((Size, Size)),
    Tombstone,
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
        let cap = self.cap as usize;
        let len = self.len as usize;
        unsafe {
            // `len` is actually represent the slots that is `Some`,
            // the underlying memory is initialized, so we use `cap` here
            Vec::from_raw_parts(self.slots.as_ptr(), cap, cap);
            Vec::from_raw_parts(self.fields.as_ptr(), len, cap);
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
            slots: unsafe { NonNull::new_unchecked(ptr::dangling_mut()) },
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

        let mut fields = ManuallyDrop::new(Vec::with_capacity(cap as usize));
        let mut slots = ManuallyDrop::new(vec![Slot::None; cap as usize]);

        assert_eq!(
            (fields.capacity(), slots.capacity()),
            (cap as usize, cap as usize)
        );

        Self {
            fields: unsafe { NonNull::new_unchecked(fields.as_mut_ptr()) },
            slots: unsafe { NonNull::new_unchecked(slots.as_mut_ptr()) },
            len: 0,
            cap,
        }
    }

    /// Returns headers length.
    ///
    /// Note that this does not include header name with multiple values.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len as _
    }

    /// Returns headers length, including duplicate headers.
    ///
    /// This function performs a computation, thus it best to cache the result to prevent calling
    /// it multiple time.
    pub fn total_len(&self) -> usize {
        self.fields().iter().fold(0, |acc, next| acc + next.len())
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
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }
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

    /// Removes a header from the map, returning the first header value at the key if the key was
    /// previously in the map.
    ///
    /// Note: Because this shifts over the remaining elements, it has a worst-case performance of
    /// *O*(*n*). If you don't need the order of headers to be preserved, use [`swap_remove`]
    /// instead.
    ///
    /// # Panics
    ///
    /// When using static str, it must be valid header name and in lowercase, otherwise it panics.
    ///
    /// If it unsure that header name is valid, use [`HeaderValue`] directly or its corresponding
    /// constant.
    ///
    /// [`swap_remove`]: Self::swap_remove
    #[inline]
    pub fn remove<K: AsHeaderName>(&mut self, name: K) -> Option<HeaderValue> {
        if self.is_empty() {
            return None;
        }
        // the rest of duplicate header values are dropped
        self.shift_remove_inner(name.as_lowercase_str(), name.hash())
    }

    #[inline]
    pub fn swap_remove<K: AsHeaderName>(&mut self, name: K) -> Option<HeaderValue> {
        if self.is_empty() {
            return None;
        }
        // the rest of duplicate header values are dropped
        self.swap_remove_inner(name.as_lowercase_str(), name.hash())
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

    // private

    pub(crate) const fn fields(&self) -> &[HeaderField] {
        unsafe { slice::from_raw_parts(self.fields.as_ptr(), self.len as usize) }
    }

    const fn fields_mut(&mut self) -> &mut [HeaderField] {
        unsafe { slice::from_raw_parts_mut(self.fields.as_ptr(), self.len as usize) }
    }

    // `self.len` represent the slots that is `Some`
    // the underlying memory is all initialized
    // so we use `self.cap` here

    const fn slots_mut(&mut self) -> &mut [Slot] {
        unsafe { slice::from_raw_parts_mut(self.slots.as_ptr(), self.cap as usize) }
    }
}

// ===== Implementation =====

impl HeaderMap {
    fn insert_inner(&mut self, field: HeaderField, append: bool) -> Option<HeaderValue> {
        self.reserve_one();

        let hash = field.cached_hash();
        let mut index = mask_by_capacity(self.cap, hash);

        loop {
            // SAFETY: `index` is masked by capacity
            let slot_mut = unsafe { self.slots.add(index as usize).as_mut() };
            match slot_mut {
                Slot::Some((dup_hash, dup_index)) => {
                    if *dup_hash == hash {
                        let dup_field = unsafe { self.fields.add(*dup_index as usize).as_mut() };
                        if dup_field.name() == field.name() {
                            // duplicate header
                            let field = if append {
                                // Append
                                // note that `len` is unchanged because it represent the
                                // initialization length *NOT* the total headers length
                                dup_field.merge(field);
                                None
                            } else {
                                // Returns duplicate, rest of multiple header values are dropped
                                Some(replace(dup_field, field).into_parts().1)
                            };
                            return field;
                        }
                    }

                    // hash collision, open address linear probing
                    index = mask_by_capacity(self.cap, index + 1);
                },
                // this is the base case of the loop, there is always `None`
                // because the load factor is limited to less than capacity
                slot_mut @ (Slot::None | Slot::Tombstone) => {
                    *slot_mut = Slot::Some((hash, self.len));
                    unsafe { self.fields.add(self.len as usize).write(field); }
                    self.len += 1;
                    return None;
                },
            }
            // ..
        }
    }

    fn field(&self, name: &str, hash: Size) -> Option<&HeaderField> {
        let mut index = mask_by_capacity(self.cap, hash);

        loop {
            let slot = unsafe { self.slots.add(index as usize).as_ref() };
            match *slot {
                Slot::Some((fd_hash, fd_idx)) => {
                    if fd_hash == hash {
                        let field_ptr = unsafe { self.fields.add(fd_idx as usize) };
                        let field = unsafe { field_ptr.as_ref() };
                        if field.name().as_str() == name {
                            return Some(field);
                        }
                    }
                }
                // base case of the loop, there is always `None` because the load factor is capped
                // to less than capacity
                Slot::None => return None,
                // hash collision with previously removed field
                Slot::Tombstone => {}
            }

            // hash collision, open address linear probing
            index = mask_by_capacity(self.cap, index + 1);
        }
    }

    #[allow(clippy::type_complexity)]
    fn field_pair_ptr(&mut self, name: &str, hash: Size) -> Option<(NonNull<Slot>, NonNull<HeaderField>)> {
        let mut index = mask_by_capacity(self.cap, hash);

        loop {
            let slot_ptr = unsafe { self.slots.add(index as usize) };
            match unsafe { slot_ptr.read() } {
                Slot::Some((fd_hash, fd_idx)) => {
                    if fd_hash == hash {
                        let field_ptr = unsafe { self.fields.add(fd_idx as usize) };
                        let field_mut = unsafe { field_ptr.as_ref() };
                        if field_mut.name().as_str() == name {
                            return Some((slot_ptr, field_ptr));
                        }
                    }
                }
                // base case of the loop, there is always `None` because the load factor is capped
                // to less than capacity
                Slot::None => return None,
                // hash collision with previously removed field
                Slot::Tombstone => {}
            }

            // hash collision, open address linear probing
            index = mask_by_capacity(self.cap, index + 1);
        }
    }

    fn shift_remove_inner(&mut self, name: &str, hash: Size) -> Option<HeaderValue> {
        let (slot_ptr, field_ptr) = self.field_pair_ptr(name, hash)?;

        unsafe {
            // take out the target field ownership and remove the slot
            let field = field_ptr.read();
            slot_ptr.write(Slot::Tombstone);

            // casting will not cause data loss because max length is u32::MAX
            let index = field_ptr.offset_from_unsigned(self.fields) as u32;
            let shifted_count = self.len - index - 1;

            // update all shifted field's slots
            let shifted_fields = slice::from_raw_parts(field_ptr.add(1).as_ptr(), shifted_count as usize);

            for (field, i) in shifted_fields.iter().zip(0u32..) {
                let new_index = i + index;

                // all unchecked in this loop is safe because the fact that one field have one
                // corresponding slot

                let (slot_ptr, _) = self
                    .field_pair_ptr(field.name().as_str(), field.cached_hash())
                    .unwrap_unchecked();
                let Slot::Some((hash, _)) = slot_ptr.read() else {
                    std::hint::unreachable_unchecked()
                };
                slot_ptr.write(Slot::Some((hash, new_index)));
            }

            // backshift the fields memory
            // note that we shift *AFTER* updating the slots, otherwise hash collision probing will
            // not work
            field_ptr.add(1).copy_to(field_ptr, shifted_count as usize);

            // update the length
            self.len -= 1;

            Some(field.into_parts().1)
        }
    }

    fn swap_remove_inner(&mut self, name: &str, hash: Size) -> Option<HeaderValue> {
        let (slot_ptr, field_ptr) = self.field_pair_ptr(name, hash)?;

        unsafe {
            // take out the target field ownership and remove the slot
            let field = field_ptr.read();
            slot_ptr.write(Slot::Tombstone);

            // casting will not cause data loss because max length is u32::MAX
            let new_index = field_ptr.offset_from_unsigned(self.fields) as u32;
            let last_field = self.fields.add(self.len as usize).as_mut();

            // update the last field's slot
            let (slot_ptr, _) = self.field_pair_ptr(
                last_field.name().as_str(),
                last_field.cached_hash()
            ).unwrap_unchecked();
            let Slot::Some((hash, _)) = slot_ptr.read() else {
                std::hint::unreachable_unchecked()
            };
            slot_ptr.write(Slot::Some((hash, new_index)));

            // move last element to target field position
            field_ptr.copy_from_nonoverlapping(last_field.into(), 1);

            // update the length
            self.len -= 1;

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
        for field in self.fields() {
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
        for slot_mut in self.slots_mut() {
            *slot_mut = Slot::None;
        }
        for field_mut in self.fields_mut() {
            // we set `self.len` to 0 below to prevent deallocation in `Drop` implementation
            unsafe { ptr::drop_in_place(field_mut) };
        }
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

// ===== Test =====

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn header_map() {
        let mut map = HeaderMap::new();

        map.insert("content-type", HeaderValue::from_string("FOO"));
        assert!(map.contains_key("content-type"));

        let ptr = map.fields.as_ptr();
        let cap = map.capacity();

        assert!(map.insert("accept", HeaderValue::from_string("BAR")).is_none());
        assert!(map.insert("content-length", HeaderValue::from_string("LEN")).is_none());
        assert!(map.insert("host", HeaderValue::from_string("BAR")).is_none());
        assert!(map.insert("date", HeaderValue::from_string("BAR")).is_none());
        assert!(map.insert("referer", HeaderValue::from_string("BAR")).is_none());
        assert!(map.insert("rim", HeaderValue::from_string("BAR")).is_none());

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

        assert!(map.insert("lea", HeaderValue::from_string("BAR")).is_none());

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

    // const fn slots(map: &HeaderMap) -> &[Slot] {
    //     unsafe { slice::from_raw_parts(map.slots.as_ptr(), map.cap as usize) }
    // }
    //
    // pub struct MapDbg<'a>(pub &'a HeaderMap);
    // pub struct FieldsDbg<'a>(pub &'a HeaderMap);
    // pub struct SlotsDbg<'a>(pub &'a HeaderMap);
    //
    // impl std::fmt::Debug for MapDbg<'_> {
    //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //         let mut m = f.debug_struct("HeaderMap");
    //         m.field("len", &self.0.len);
    //         m.field("cap", &self.0.cap);
    //         m.field("fields", &FieldsDbg(self.0));
    //         m.field("slots", &SlotsDbg(self.0));
    //         m.finish()
    //     }
    // }
    //
    // impl std::fmt::Debug for FieldsDbg<'_> {
    //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //         let mut m = f.debug_list();
    //         for field in self.0.fields() {
    //             m.entry(&format_args!(
    //                 "{}({}): {:?}",
    //                 field.name().as_str(),
    //                 field.cached_hash(),
    //                 field.value(),
    //             ));
    //         }
    //         m.finish()
    //     }
    // }
    //
    // impl std::fmt::Debug for SlotsDbg<'_> {
    //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //         let mut m = f.debug_list();
    //         for slot in slots(self.0) {
    //             match slot {
    //                 Slot::None => m.entry(&None::<()>),
    //                 Slot::Some((hash, index)) => m.entry(&format_args!("{}: {}", hash, index)),
    //                 Slot::Tombstone => m.entry(&"Tombstone"),
    //             };
    //         }
    //         m.finish()
    //     }
    // }
}

