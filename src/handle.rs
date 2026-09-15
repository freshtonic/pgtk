//! Typed identity for one entity inside one catalog.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A typed index into one [`crate::Catalog`] arena.
///
/// A handle is only meaningful for the catalog that issued it. The model does
/// not check that at run time. A handle for one kind of entity cannot address
/// another kind, because the type parameter keeps the kinds apart.
///
/// The type parameter is a marker only. `Handle<T>` is `Copy`, `Send`, and
/// `Sync` for every `T`, because it holds `PhantomData<fn() -> T>`.
pub struct Handle<T>(u32, PhantomData<fn() -> T>);

impl<T> Handle<T> {
    /// Make a handle for an arena index.
    ///
    /// Only the builder and the loader make handles. Both push entities in
    /// the order that gives each handle its index.
    pub(crate) fn new(index: u32) -> Self {
        Handle(index, PhantomData)
    }

    /// The raw arena index.
    pub fn index(self) -> u32 {
        self.0
    }
}

/// The last segment of a Rust path, for example `Type` in `pg_catalog::Type`.
fn short_type_name<T>() -> &'static str {
    let full = std::any::type_name::<T>();
    match full.rsplit_once("::") {
        Some((_, last)) => last,
        None => full,
    }
}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle<{}>({})", short_type_name::<T>(), self.0)
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Handle<T> {}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for Handle<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::Deserialize<'de> for Handle<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let index = u32::deserialize(deserializer)?;
        Ok(Handle::new(index))
    }
}

#[cfg(test)]
mod tests {
    use super::Handle;
    use crate::types::Type;

    #[test]
    fn debug_names_the_entity_kind() {
        let handle: Handle<Type> = Handle::new(12);
        assert_eq!(format!("{handle:?}"), "Handle<Type>(12)");
    }

    #[test]
    fn handles_compare_by_index() {
        let a: Handle<Type> = Handle::new(1);
        let b: Handle<Type> = Handle::new(2);
        assert!(a < b);
        assert_eq!(a, Handle::<Type>::new(1));
        assert_eq!(a.index(), 1);
    }

    /// A type with no trait implementations still works as a marker.
    struct Bare;

    #[test]
    fn the_marker_type_needs_no_bounds() {
        let a: Handle<Bare> = Handle::new(3);
        let b = a;
        assert_eq!(a, b);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_uses_a_bare_number() {
        let handle: Handle<Type> = Handle::new(7);
        let text = serde_json::to_string(&handle).unwrap();
        assert_eq!(text, "7");
        let back: Handle<Type> = serde_json::from_str(&text).unwrap();
        assert_eq!(back, handle);
    }
}
