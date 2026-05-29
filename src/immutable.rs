// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Generic immutable wrappers — frozen view + cheap `Arc`-based clone.
//!
//! Replaces the 8× `ImmutableXxxHashSet` and 64× `ImmutableXxxHashMap` files.
//! `ImmutableHashSet<T>` is a real frozen hash set (O(1) `contains`) backed by
//! the project's own [`crate::hash_table::OpenHashSet`], not a sorted/linear-search array.

use crate::hash_table::{OpenHashMap, OpenHashSet};
use std::hash::Hash;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// ImmutableHashSet<T>
// ---------------------------------------------------------------------------

/// Frozen hash set: O(1) `contains`, cheaply cloneable via `Arc`.
#[derive(Debug)]
pub struct ImmutableHashSet<T> {
    inner: Arc<OpenHashSet<T>>,
}

impl<T> Clone for ImmutableHashSet<T> {
    fn clone(&self) -> Self {
        ImmutableHashSet {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: Hash + Eq> ImmutableHashSet<T> {
    pub fn from_mutable(set: OpenHashSet<T>) -> Self {
        ImmutableHashSet {
            inner: Arc::new(set),
        }
    }

    pub fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.inner.iter()
    }
}

// ---------------------------------------------------------------------------
// ImmutableHashMap<K, V>
// ---------------------------------------------------------------------------

/// Frozen hash map: O(1) lookup, cheaply cloneable via `Arc`. Uses our ported
/// `OpenHashMap` (not `std::collections::HashMap`) so the cache-locality
/// interleaved-entry layout carries through to the immutable variant.
#[derive(Debug)]
pub struct ImmutableHashMap<K, V> {
    inner: Arc<OpenHashMap<K, V>>,
}

impl<K, V> Clone for ImmutableHashMap<K, V> {
    fn clone(&self) -> Self {
        ImmutableHashMap {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<K: Hash + Eq, V> ImmutableHashMap<K, V> {
    pub fn from_mutable(map: OpenHashMap<K, V>) -> Self {
        ImmutableHashMap {
            inner: Arc::new(map),
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        self.inner.iter()
    }
}

// ---------------------------------------------------------------------------
// ImmutableList<T>
// ---------------------------------------------------------------------------

/// Frozen ordered list: indexable, cheaply cloneable. The Java-side
/// `ImmutableIntList`/`ImmutableObjectList` analogue.
#[derive(Debug)]
pub struct ImmutableList<T> {
    inner: Arc<[T]>,
}

impl<T> Clone for ImmutableList<T> {
    fn clone(&self) -> Self {
        ImmutableList {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: Clone> ImmutableList<T> {
    pub fn from_slice(values: &[T]) -> Self {
        // `Arc::from(&[T])` exists for `T: Clone` and copies once into a fresh
        // `Arc<[T]>` allocation.
        ImmutableList {
            inner: Arc::from(values),
        }
    }
}

impl<T> ImmutableList<T> {
    pub fn from_vec(values: Vec<T>) -> Self {
        ImmutableList {
            inner: Arc::from(values.into_boxed_slice()),
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    /// Borrows the backing storage as a contiguous slice — the bridge to the
    /// [`parallel`](crate::parallel) module (see [`ArrayList::as_slice`]).
    ///
    /// [`ArrayList::as_slice`]: crate::object::ArrayList::as_slice
    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }
}

impl<T> std::ops::Index<usize> for ImmutableList<T> {
    type Output = T;
    fn index(&self, idx: usize) -> &T {
        &self.inner[idx]
    }
}

// ---- FromIterator ---------------------------------------------------------
//
// Implemented as proper trait impls so `collect::<ImmutableHashSet<_>>()` works
// and `ImmutableHashSet::from_iter(...)` (via the trait, in prelude) is not
// shadowed by an inherent method of the same name.

impl<T: Hash + Eq> FromIterator<T> for ImmutableHashSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut s = OpenHashSet::new();
        for v in iter {
            s.add(v);
        }
        ImmutableHashSet::from_mutable(s)
    }
}

impl<K: Hash + Eq, V> FromIterator<(K, V)> for ImmutableHashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut m = OpenHashMap::new();
        for (k, v) in iter {
            m.insert(k, v);
        }
        ImmutableHashMap::from_mutable(m)
    }
}

impl<T> FromIterator<T> for ImmutableList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        ImmutableList::from_vec(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashable_float::HashableF32;

    #[test]
    fn immutable_hash_set_real_hash_lookup() {
        let s = ImmutableHashSet::from_iter([1, 2, 3, 4, 5]);
        assert!(s.contains(&3));
        assert!(!s.contains(&99));
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn immutable_hash_set_clones_cheaply() {
        let a = ImmutableHashSet::from_iter([1, 2, 3]);
        let b = a.clone();
        assert!(b.contains(&2));
    }

    #[test]
    fn immutable_hash_set_with_floats() {
        let s = ImmutableHashSet::from_iter([
            HashableF32(1.0),
            HashableF32(2.0),
            HashableF32(f32::NAN),
        ]);
        assert!(s.contains(&HashableF32(1.0)));
        assert!(s.contains(&HashableF32(f32::NAN)));
        assert!(!s.contains(&HashableF32(99.0)));
    }

    #[test]
    fn immutable_hash_map_lookup() {
        let m = ImmutableHashMap::from_iter([(1, "one"), (2, "two")]);
        assert_eq!(m.get(&1), Some(&"one"));
        assert_eq!(m.get(&3), None);
    }

    #[test]
    fn immutable_list_index() {
        let l = ImmutableList::from_vec(vec![10, 20, 30]);
        assert_eq!(l[0], 10);
        assert_eq!(l[2], 30);
        assert_eq!(l.len(), 3);
    }
}
