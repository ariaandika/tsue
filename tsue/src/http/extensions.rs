use std::{
    any::{Any, TypeId},
    collections::HashMap, fmt, hash::{BuildHasherDefault, Hasher},
};

#[derive(Default)]
struct NoopHasher(u64);

impl Hasher for NoopHasher {
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// HTTP Extensions.
#[derive(Clone)]
pub struct Extensions {
    map: Option<
        HashMap<TypeId, Box<dyn AnyClone + Send + Sync + 'static>, BuildHasherDefault<NoopHasher>>,
    >,
}

impl Extensions {
    /// Create new [`Extensions`].
    ///
    /// This function does not allocate.
    pub fn new() -> Self {
        Self { map: None }
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.map.as_ref().map(HashMap::len).unwrap_or_default()
    }

    /// Returns `true` if the map contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a reference to the value corresponding to the type.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map
            .as_ref()
            .and_then(|map| map.get(&TypeId::of::<T>()))
            .and_then(|ok| (**ok).as_any().downcast_ref())
    }

    /// Returns a mutable reference to the value corresponding to the type.
    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.map
            .as_mut()
            .and_then(|map| map.get_mut(&TypeId::of::<T>()))
            .and_then(|ok| (**ok).as_mut_any().downcast_mut())
    }

    /// Inserts a value into the map.
    pub fn insert<T: Clone + Send + Sync + 'static>(&mut self, value: T) -> Option<T> {
        self.map
            .get_or_insert_default()
            .insert(TypeId::of::<T>(), Box::new(value))
            .and_then(|ok| ok.into_any().downcast().map(|e| *e).ok())
    }

    /// Removes and returns the value at the type if the type was previously in the map.
    pub fn remove<T: Any>(&mut self) -> Option<T> {
        self.map
            .as_mut()
            .and_then(|map| map.remove(&TypeId::of::<T>()))
            .and_then(|ok| ok.into_any().downcast().map(|e| *e).ok())
    }

    /// Clears the map. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        if let Some(map) = self.map.as_mut() {
            map.clear();
        }
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Extensions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Extensions")
            .field(
                "length",
                &self.map.as_ref().map(HashMap::len).unwrap_or_default(),
            )
            .finish()
    }
}

// ===== AnyMap =====

trait AnyClone {
    fn clone_box(&self) -> Box<dyn AnyClone + Send + Sync>;

    fn as_any(&self) -> &dyn Any;

    fn as_mut_any(&mut self) -> &mut dyn Any;

    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Clone + Send + Sync + 'static> AnyClone for T {
    fn clone_box(&self) -> Box<dyn AnyClone + Send + Sync> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Clone for Box<dyn AnyClone + Send + Sync> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

#[test]
fn test_extensions() {
    #[derive(Clone, Debug, PartialEq)]
    struct MyType(i32);

    let mut extensions = Extensions::new();

    extensions.insert(5i32);
    extensions.insert(MyType(10));

    assert_eq!(extensions.get(), Some(&5i32));
    assert_eq!(extensions.get_mut(), Some(&mut 5i32));

    let ext2 = extensions.clone();

    assert_eq!(extensions.remove::<i32>(), Some(5i32));
    assert!(extensions.get::<i32>().is_none());

    // clone still has it
    assert_eq!(ext2.get(), Some(&5i32));
    assert_eq!(ext2.get(), Some(&MyType(10)));

    assert_eq!(extensions.get::<bool>(), None);
    assert_eq!(extensions.get(), Some(&MyType(10)));
}

