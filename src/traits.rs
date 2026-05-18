// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Generic collection contracts — replaces the 208 per-primitive trait files
//! with ~6 generic traits.
//!
//! The Java API has `IntCollection`, `IntList`, `IntSet`, `MutableIntList`,
//! etc. as separate interfaces *per primitive type* because Java needs each
//! interface monomorphised by hand to avoid boxing. In Rust, monomorphisation
//! is automatic, so one generic trait covers every element type.

/// A read-only contract for any collection of `T`.
pub trait PrimitiveCollection<T> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn contains(&self, value: &T) -> bool;
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_>;
}

pub trait MutableCollection<T>: PrimitiveCollection<T> {
    fn clear(&mut self);
}

/// Indexable, ordered list.
pub trait PrimitiveList<T>: PrimitiveCollection<T> {
    fn get(&self, index: usize) -> Option<&T>;
    fn index_of(&self, value: &T) -> Option<usize>;
}

pub trait MutableList<T>: PrimitiveList<T> + MutableCollection<T> {
    fn push(&mut self, value: T);
    fn set(&mut self, index: usize, value: T) -> T;
}

/// Unordered set.
pub trait PrimitiveSet<T>: PrimitiveCollection<T> {}

pub trait MutableSet<T>: PrimitiveSet<T> + MutableCollection<T> {
    fn add(&mut self, value: T) -> bool;
    fn remove(&mut self, value: &T) -> bool;
}

/// Key→value map.
pub trait PrimitiveMap<K, V> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get(&self, key: &K) -> Option<&V>;
    fn contains_key(&self, key: &K) -> bool;
}

pub trait MutableMap<K, V>: PrimitiveMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V>;
    fn remove(&mut self, key: &K) -> Option<V>;
    fn clear(&mut self);
}

// ---- impls on concrete types ---------------------------------------------

impl<T: PartialEq> PrimitiveCollection<T> for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
    fn contains(&self, value: &T) -> bool {
        self.as_slice().contains(value)
    }
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(self.as_slice().iter())
    }
}

impl<T: PartialEq> MutableCollection<T> for Vec<T> {
    fn clear(&mut self) {
        Vec::clear(self)
    }
}

impl<T: PartialEq> PrimitiveList<T> for Vec<T> {
    fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
    }
    fn index_of(&self, value: &T) -> Option<usize> {
        self.iter().position(|v| v == value)
    }
}

impl<T: PartialEq> MutableList<T> for Vec<T> {
    fn push(&mut self, value: T) {
        Vec::push(self, value)
    }
    fn set(&mut self, index: usize, value: T) -> T {
        std::mem::replace(&mut self[index], value)
    }
}

impl<T: std::hash::Hash + Eq> PrimitiveCollection<T> for crate::hash_table::OpenHashSet<T> {
    fn len(&self) -> usize {
        crate::hash_table::OpenHashSet::len(self)
    }
    fn contains(&self, value: &T) -> bool {
        crate::hash_table::OpenHashSet::contains(self, value)
    }
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(crate::hash_table::OpenHashSet::iter(self))
    }
}

impl<T: std::hash::Hash + Eq> MutableCollection<T> for crate::hash_table::OpenHashSet<T> {
    fn clear(&mut self) {
        crate::hash_table::OpenHashSet::clear(self);
    }
}

impl<T: std::hash::Hash + Eq> PrimitiveSet<T> for crate::hash_table::OpenHashSet<T> {}

impl<T: std::hash::Hash + Eq> MutableSet<T> for crate::hash_table::OpenHashSet<T> {
    fn add(&mut self, value: T) -> bool {
        crate::hash_table::OpenHashSet::add(self, value)
    }
    fn remove(&mut self, value: &T) -> bool {
        crate::hash_table::OpenHashSet::remove(self, value)
    }
}

impl<K: std::hash::Hash + Eq, V> PrimitiveMap<K, V> for crate::hash_table::OpenHashMap<K, V> {
    fn len(&self) -> usize {
        crate::hash_table::OpenHashMap::len(self)
    }
    fn get(&self, key: &K) -> Option<&V> {
        crate::hash_table::OpenHashMap::get(self, key)
    }
    fn contains_key(&self, key: &K) -> bool {
        crate::hash_table::OpenHashMap::contains_key(self, key)
    }
}

impl<K: std::hash::Hash + Eq, V> MutableMap<K, V> for crate::hash_table::OpenHashMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        crate::hash_table::OpenHashMap::insert(self, key, value)
    }
    fn remove(&mut self, key: &K) -> Option<V> {
        crate::hash_table::OpenHashMap::remove(self, key)
    }
    fn clear(&mut self) {
        crate::hash_table::OpenHashMap::clear(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash_table::{OpenHashMap, OpenHashSet};

    #[test]
    fn vec_is_a_primitive_list() {
        let v: Vec<i32> = vec![10, 20, 30];
        let l: &dyn PrimitiveList<i32> = &v;
        assert_eq!(l.len(), 3);
        assert_eq!(l.get(1), Some(&20));
        assert_eq!(l.index_of(&30), Some(2));
        assert!(l.contains(&20));
    }

    #[test]
    fn openhashset_is_a_set() {
        let mut s: OpenHashSet<i32> = OpenHashSet::new();
        MutableSet::add(&mut s, 1);
        MutableSet::add(&mut s, 2);
        assert_eq!(PrimitiveCollection::len(&s), 2);
        assert!(PrimitiveCollection::contains(&s, &1));
    }

    #[test]
    fn openhashmap_is_a_map() {
        let mut m: OpenHashMap<i32, String> = OpenHashMap::new();
        MutableMap::insert(&mut m, 1, "one".into());
        assert_eq!(PrimitiveMap::get(&m, &1), Some(&"one".to_string()));
    }
}
