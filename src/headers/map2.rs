//! Second version of the header map API.
use std::num::NonZeroU32;
use std::{mem, ptr, slice};

use crate::headers::error::TryReserveError;
use crate::headers::{HeaderName, HeaderValue};

// space-time tradeoff
// most of integer type is limited
// this limit practically should never exceeded for header length
type Size = u32;

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
pub struct HeaderMap {
    fields: ptr::NonNull<HeaderField>,
    len: Size,
    cap: Size,
}

use inner::HeaderField;
mod inner {
    use crate::headers::{HeaderName, HeaderValue};

    #[derive(Debug, Clone)]
    pub struct HeaderField {
        name: HeaderName,
        value: HeaderValue,
    }

    impl HeaderField {
        pub fn new(name: HeaderName, value: HeaderValue) -> Self {
            Self { name, value }
        }

        pub fn name(&self) -> &HeaderName {
            &self.name
        }

        pub fn value(&self) -> &HeaderValue {
            &self.value
        }
    }
}

type HashIdx = Option<HashField>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HashField {
    hash: u32,
    idx: NonZeroU32,
}

impl HashField {
    fn field<'a>(&self, map: &'a HeaderMap) -> &'a HeaderField {
        unsafe { map.fields.add(self.idx.get() as usize).as_ref() }
    }

    fn field_mut<'a>(&self, map: &'a mut HeaderMap) -> &'a mut HeaderField {
        unsafe { map.fields.add(self.idx.get() as usize).as_mut() }
    }

    fn field_ptr(&self, map: &mut HeaderMap) -> ptr::NonNull<HeaderField> {
        unsafe { map.fields.add(self.idx.get() as usize) }
    }
}

unsafe impl Send for HeaderMap {}
unsafe impl Sync for HeaderMap {}

impl Drop for HeaderMap {
    fn drop(&mut self) {
        // dangling ptr
        if self.cap == 0 {
            return;
        }
        // call drop on fields except the hash table
        unsafe {
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                self.fields.add(alloc::offset(self.cap)).as_ptr(),
                self.len as usize,
            ))
        };
        // deallocate
        alloc::deallocate(self.fields, self.cap);
    }
}

impl Clone for HeaderMap {
    fn clone(&self) -> Self {
        if self.is_empty() {
            return Self::new();
        }

        let mut cloned = Self::with_capacity_size(self.cap);
        let offset = alloc::offset(self.cap) as u32;

        // copy the hash table
        unsafe {
            cloned
                .fields
                .copy_from_nonoverlapping(self.fields, offset as usize)
        };

        // clone the fields
        for i in offset..offset + self.len {
            unsafe {
                let dst = cloned.fields.add(i as usize).as_mut();
                let src = self.fields.add(i as usize).as_ref();
                dst.clone_from(src);
            }
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
            fields: ptr::NonNull::dangling(),
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
        match Size::try_from(capacity) {
            Ok(cap) => Ok(Self::with_capacity_size(cap)),
            Err(_) => Err(TryReserveError {}),
        }
    }

    /// Creates new empty [`HeaderMap`] with at least the specified capacity.
    #[inline]
    fn with_capacity_size(cap: Size) -> Self {
        Self {
            fields: alloc::allocate(cap),
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

    /// Returns an iterator over header fields.
    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, HeaderField> {
        self.fields().iter()
    }

    /// Returns an iterator over headers as name and value pair.
    #[inline]
    pub fn pairs(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.fields().iter().map(|f|(f.name(), f.value()))
    }

    /// Returns `true` if the map contains a header value for given header name.
    #[inline]
    pub fn contains_key(&self, name: &HeaderName) -> bool {
        if self.is_empty() {
            return false
        }
        self.hash_field(name.as_str(), name.hash()).is_some()
    }

    /// Returns a reference to the first header value corresponding to the given header name.
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
    #[inline]
    pub fn get(&self, name: &HeaderName) -> Option<&HeaderValue> {
        if self.is_empty() {
            return None;
        }
        Some(
            self.hash_field(name.as_str(), name.hash())?
                .field(self)
                .value(),
        )
    }

    // /// Returns an iterator to all header values corresponding to the given header name.
    // ///
    // /// Note that this is the result of duplicate header fields, *NOT* comma separated list.
    // #[inline]
    // pub fn get_all<'a, K: AsHeaderName>(&'a self, name: &'a K) -> iter::GetAll<'a> {
    //     iter::GetAll::new(self, name.as_lowercase_str(), name.hash())
    // }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did have this key present, the value is updated, and the old value is returned
    /// as `Some`.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn insert(&mut self, name: HeaderName, value: HeaderValue) -> Option<HeaderField> {
        self.reserve_one().expect("cannot insert header");
        unsafe { self.insert_inner(name.hash(), HeaderField::new(name, value), false) }
    }

    /// Append a header key and value into the map.
    ///
    /// Unlike [`insert`][HeaderMap::insert], if header key is present, header value is still
    /// appended as extra value.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn append(&mut self, name: HeaderName, value: HeaderValue) {
        self.reserve_one().expect("cannot append header");
        unsafe { self.insert_inner(name.hash(), HeaderField::new(name, value), true) };
    }

    // pub(crate) fn try_append_field(&mut self, field: HeaderField) -> Result<(), TryReserveError> {
    //     self.reserve_one()?;
    //     unsafe { self.insert_inner(field, true) };
    //     Ok(())
    // }

    /// Removes a header from the map, returning the first header value if it founds.
    #[inline]
    pub fn remove(&mut self, name: &HeaderName) -> Option<HeaderField> {
        if self.is_empty() {
            return None;
        }
        self.remove_inner(name.as_str(), name.hash())
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

        let offset = alloc::offset(self.cap);

        // clear the hash table
        unsafe { std::ptr::write_bytes(self.fields.as_ptr(), 0, offset) };

        // call drop on fields except the hash table
        unsafe {
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                self.fields.add(offset).as_ptr(),
                self.len as usize,
            ))
        };

        self.len = 0;
    }

    pub(crate) const fn fields(&self) -> &[HeaderField] {
        unsafe {
            slice::from_raw_parts(
                self.fields.add(alloc::offset(self.cap)).as_ptr(),
                self.len as usize,
            )
        }
    }
}

// ===== Implementation =====

impl HeaderMap {
    fn hash_field(&self, name: &str, hash: u32) -> Option<&HashField> {
        let ptr = self.fields.cast::<HashIdx>();
        let offset = alloc::offset(self.cap);
        let hash_field_cap = alloc::hash_field_cap(offset) as Size;

        let mut index = hash;

        loop {
            index %= hash_field_cap;
            // `?` is the base case of the loop, there is always `None` because the load
            // factor is capped to less than capacity
            // SAFETY: `index` is masked by hash field capacity
            let hash_field = unsafe { ptr.add(index as usize).as_ref().as_ref()? };
            if hash_field.hash == hash {
                let field = hash_field.field(self);
                if field.name().as_str() == name {
                    return Some(hash_field);
                }
            }

            // linear probing
            index += 1;
        }
    }

    /// # Safety
    ///
    /// `self.len < self.cap`
    unsafe fn insert_inner(&mut self, hash: u32, field: HeaderField, append: bool) -> Option<HeaderField> {
        debug_assert!(self.len < self.cap);

        let ptr = self.fields.cast::<HashIdx>();
        let offset = alloc::offset(self.cap);
        let hash_field_cap = alloc::hash_field_cap(offset) as Size;

        let mut index = hash;

        loop {
            index %= hash_field_cap;
            // SAFETY: `index` is masked by capacity
            let hash_field = unsafe { ptr.add(index as usize).as_mut() };
            let Some(dup_hash_field) = hash_field.as_mut() else {
                // found empty slot

                // SAFETY: this function should be called with non zero capacity, thus offset
                // will never be zero
                let offset_idx = unsafe { NonZeroU32::new_unchecked(self.len + offset as u32) };
                *hash_field = Some(HashField {
                    hash,
                    idx: offset_idx,
                });

                // move the field
                unsafe { self.fields.add(offset_idx.get() as usize).write(field); };

                self.len += 1;

                return None
            };

            if !append && hash == dup_hash_field.hash {
                // duplicate Header

                let dup_field = dup_hash_field.field_mut(self);
                if dup_field.name() == field.name() {
                    // replace and returns duplicate
                    return Some(mem::replace(dup_field, field));
                }
            }

            // appending, look for the next empty slot

            // linear probing
            index += 1;
        }
    }

    /// backward shifting removal
    /// 1. find the target hash field, returns if not found
    /// 2. replace the hash field with other entry that may be displaced by collision
    /// 3. update hash fields indexes that will be shifted
    /// 4. take out the removed field
    /// 5. copy the fields backward
    fn remove_inner(&mut self, name: &str, hash: u32) -> Option<HeaderField> {
        let ptr = self.fields.cast::<HashIdx>();
        let offset = alloc::offset(self.cap);
        let hash_field_cap = alloc::hash_field_cap(offset) as Size;

        let mut index = hash % hash_field_cap;

        // 1. find the target field
        loop {
            // `?` is the base case of the loop, there is always `None` because the load
            // factor is capped to less than capacity
            // SAFETY: `index` is masked by hash table capacity
            let hash_field = unsafe { ptr.add(index as usize).as_mut().as_mut()? };
            if hash_field.hash == hash {
                let field = hash_field.field(self);
                if field.name().as_str() == name {
                    break;
                }
            }

            // linear probing
            index = (index + 1) % hash_field_cap;
        }
        let hash_field_idx = index;

        // 2. replace the hash field with other entry that may be displaced by collision
        let mut swap_candidate = &mut None;
        loop {
            index = (index + 1) % hash_field_cap;
            // SAFETY: `index` is masked by hash table capacity
            let hash_field_mut = unsafe { ptr.add(index as usize).as_mut() };
            let Some(hash_field) = hash_field_mut.as_mut() else {
                break;
            };
            // only hash field that is displaced that should be swapped, other field may just
            // happens to be contiguous
            if hash_field.hash % hash_field_cap == hash_field_idx {
                swap_candidate = hash_field_mut;
            }
        }
        // SAFETY:
        // - `target_idx` is result of the search, its valid index
        // - `unwrap_unchecked` is safe because if the target hash field is `None`, this function
        // already returned
        let hash_field = unsafe {
            ptr.add(hash_field_idx as usize)
                .replace(swap_candidate.take())
                .unwrap_unchecked()
        };

        // 3. update hash fields indexes that will be shifted
        // `hash_field.idx` are index with `offset` applied, but `self.len` is not
        let max_bounds = self.len + offset as u32;
        let mut i = hash_field.idx.get() + 1;
        while i < max_bounds {
            let field_mut = unsafe { self.fields.add(i as usize).as_mut() };
            let hash = field_mut.name().hash();
            let mut hash_idx = hash % hash_field_cap;

            loop {
                let hash_field = unsafe { ptr.add(hash_idx as usize).as_mut().as_mut() };
                let Some(hash_field) = hash_field else {
                    unreachable!("fields with no hash entry")
                };
                // check whether this is the correct hash fields, not displaced
                if hash_field.hash == hash {
                    // affected fields is backshifted
                    hash_field.idx = unsafe { NonZeroU32::new_unchecked(i - 1) };
                    break;
                }
                // displaced, search next entry
                hash_idx = (hash_idx + 1) % hash_field_cap;
            }

            i += 1;
        }

        // 4. take out the removed field
        let field_ptr = hash_field.field_ptr(self);
        // the ptr here will be overwritten
        let field = unsafe { field_ptr.read() };

        // 5. copy the fields backward
        // do this AFTER all hash tables updated
        let n = self.len - (hash_field.idx.get() - offset as u32);
        unsafe { field_ptr.copy_from(field_ptr.add(1), n as usize) };

        // update the length
        self.len -= 1;
        Some(field)
    }

    #[inline]
    fn reserve_one(&mut self) -> Result<(), TryReserveError> {
        if !alloc::is_load_factor_exceeded(self.len, self.cap) {
            return Ok(());
        }
        self.try_reserve_size(1)
    }

    /// Reserves capacity for at least `additional` more headers.
    ///
    /// # Errors
    ///
    /// Returns error if the new capacity exceeds the HeaderMap capacity limit.
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        let Ok(add) = Size::try_from(additional) else {
            return Err(TryReserveError {});
        };
        if self.cap - self.len > add {
            return Ok(());
        }
        let Some(new_cap) = self.len.checked_add(add) else {
            return Err(TryReserveError {});
        };
        self.try_reserve_size(new_cap)
    }

    fn try_reserve_size(&mut self, add: Size) -> Result<(), TryReserveError> {
        const DEFAULT_MIN_ALLOC: Size = 2;

        let Some(cap) = self
            .cap
            .max(DEFAULT_MIN_ALLOC)
            .checked_mul(2)
            .max(self.len.checked_add(add))
        else {
            return Err(TryReserveError {});
        };

        let mut new_map = Self::with_capacity_size(cap);

        // copy to new map
        self.copy_to(&mut new_map);

        // skip drop, just deallocate
        if self.cap != 0 {
            alloc::deallocate(self.fields, self.cap);
        }
        let _ = mem::ManuallyDrop::new(mem::replace(self, new_map));

        Ok(())
    }

    fn copy_to(&self, new_map: &mut Self) {
        // recalculate hash table
        let ptr = self.fields.cast::<HashIdx>();
        let new_ptr = new_map.fields.cast::<HashIdx>();

        let offset = alloc::offset(self.cap);

        let new_offset = alloc::offset(new_map.cap);
        let new_hash_field_cap = alloc::hash_field_cap(new_offset) as Size;

        // when the hash table size changes, every index will also change
        let offset_delta = (new_offset - offset) as u32;

        let mut i = 0;

        while new_map.len < self.len {
            let hash_field = unsafe { ptr.add(i as usize).as_ref() };
            let Some(hash_field_ref) = hash_field.as_ref() else {
                i += 1;
                continue;
            };

            let mut new_index = hash_field_ref.hash;
            loop {
                new_index %= new_hash_field_cap;
                let new_field = unsafe { new_ptr.add(new_index as usize).as_mut() };

                match new_field.as_mut() {
                    Some(_) => {
                        // collision
                        new_index += 1;
                    },
                    None => {
                        *new_field = Some(HashField {
                            hash: hash_field_ref.hash,
                            idx: unsafe { NonZeroU32::new_unchecked(hash_field_ref.idx.get() + offset_delta) },
                        });
                        new_map.len += 1;
                        break;
                    }
                }
            }

            i += 1;
        }

        // copy all fields
        unsafe {
            self.fields
                .add(offset)
                .copy_to_nonoverlapping(new_map.fields.add(new_offset), self.len as usize)
        };

        debug_assert_eq!(new_map.len, self.len);
    }
}

impl std::fmt::Debug for HeaderMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.pairs()).finish()
    }
}

mod alloc {
    //! Allocation for the HeaderMap is divided into two region. The first region is used to store
    //! hash and index pair for lookups, then the rest is where the fields are stored.
    //!
    //! ```not_rust
    //! SIZE = 48
    //! LOAD_FACTOR = 3/4
    //! load = cap * LOAD_FACTOR
    //! off = cap * (1 - LOAD_FACTOR)
    //! [ off | load ]
    //! ```

    use std::alloc::{Layout, handle_alloc_error, alloc, dealloc};
    use std::ptr::NonNull;

    use super::{HeaderField, HashIdx, Size};

    // const LOAD_FACTOR: f32  = 3 / 4;

    const HASH_SIZE: usize = size_of::<HashIdx>();

    pub const SIZE: usize = size_of::<HeaderField>();
    pub const ALIGN: usize = align_of::<HeaderField>();

    // how many hash field can be stored in one SIZE.
    pub const OFFSET_SCALE: usize = SIZE / HASH_SIZE;

    // no allocation overflow
    const _: () = assert!(((Size::MAX as usize).strict_mul(SIZE)) < isize::MAX as usize);

    // unused capacity in remaining of the load factor is enough to store hash table
    const _: () = assert!(offset(3) * HASH_SIZE <= SIZE * 3);

    /// Calculate offset to the first pointer of the fields.
    ///
    /// Returned `offset` is in [`SIZE`] bytes.
    pub const fn offset(cap: Size) -> usize {
        // cap * (1 - LOAD_FACTOR)
        cap as usize / 4
    }

    /// Calculate capacity of hash field.
    pub const fn hash_field_cap(offset: usize) -> usize {
        offset * OFFSET_SCALE
    }

    pub const fn is_load_factor_exceeded(len: Size, cap: Size) -> bool {
        // more optimized of `self.len as f64 / self.cap as f64 >= LOAD_FACTOR`
        // this also handle 0 capacity
        len * 4 >= cap * 3
    }

    const fn layout(cap: Size) -> Layout {
        // `Size::MAX * SIZE` is below `isize::MAX`
        unsafe { Layout::from_size_align_unchecked((cap as usize).unchecked_mul(SIZE), ALIGN) }
    }

    pub fn allocate(cap: Size) -> NonNull<HeaderField> {
        unsafe {
            let layout = layout(cap);
            let Some(ok) = NonNull::new(alloc(layout)) else {
                handle_alloc_error(layout)
            };
            let ptr = ok.cast();
            // initialized the hash table
            std::ptr::write_bytes(ptr.as_ptr(), 0, offset(cap));
            ptr
        }
    }

    pub fn deallocate(ptr: NonNull<HeaderField>, cap: Size) {
        unsafe { dealloc(ptr.cast().as_ptr(), layout(cap)) };
    }
}

#[test]
fn test_zeroed_hash_idx() {
    // `allocate` use zero bytes write to initialized the hash table
    unsafe { assert_eq!(None::<HashIdx>, std::mem::zeroed()) };
}

#[test]
#[allow(clippy::borrow_interior_mutable_const)]
#[allow(clippy::declare_interior_mutable_const)]
fn test_header_map() {
    use super::name::standard as s;

    const FOO: HeaderValue = HeaderValue::from_static(b"FOO");

    // dangling ptr
    drop(HeaderMap::new());

    let mut map = HeaderMap::new();

    assert!(map.insert(s::DATE, FOO).is_none());
    assert!(map.contains_key(&s::DATE));

    let field = map.insert(s::DATE, FOO).unwrap();
    assert!(map.contains_key(&s::DATE));
    assert_eq!(field.name(), &s::DATE);
    assert_eq!(field.value(), &FOO);

    assert!(map.insert(s::AGE, FOO).is_none());
    assert!(map.insert(s::HOST, FOO).is_none());
    assert!(map.insert(s::ACCEPT, FOO).is_none());
    assert!(map.insert(s::TE, FOO).is_none());
    assert!(map.insert(s::CONTENT_TYPE, FOO).is_none());
    assert!(map.insert(s::CONTENT_LENGTH, FOO).is_none());

    let len = map.len();

    map.append(s::DATE, FOO);
    assert!(map.contains_key(&s::DATE));

    assert_eq!(map.len(), len + 1);

    // let mut fields = map.get_all(&s::DATE);
    // assert_eq!(fields.next(), Some(&FOO));
    // assert_eq!(fields.next(), Some(&FOO));
    // assert!(fields.next().is_none());

    let mut i = 0;
    for field in map.iter() {
        assert!(matches!(field.name().as_str(), "date" | "age" | "host" | "accept" | "te" | "content-type" | "content-length"));
        i += 1;
    }
    assert_eq!(map.len(), i);

    // let field = map.remove(s::HOST).unwrap();
    // assert!(!map.contains_key(s::HOST));
    // assert_eq!(field.into_parts(), (s::HOST, FOO));
    //
    // let field = map.remove(s::DATE).unwrap();
    // assert!(map.contains_key(s::DATE));
    // assert_eq!(field.into_parts(), (s::DATE, FOO));
}
