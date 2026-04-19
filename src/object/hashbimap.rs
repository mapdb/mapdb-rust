// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use super::traits::*;
use std::collections::HashMap;
use std::hash::Hash;

/// Generic bidirectional map. Both keys and values must be unique (bijection).
/// Backed by two `HashMap`s (forward and inverse).
#[derive(Debug, Clone)]
pub struct HashBiMap<K: Eq + Hash + Clone, V: Eq + Hash + Clone> {
    forward: HashMap<K, V>,
    inverse: HashMap<V, K>,
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> HashBiMap<K, V> {
    pub fn new() -> Self {
        HashBiMap {
            forward: HashMap::new(),
            inverse: HashMap::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        HashBiMap {
            forward: HashMap::with_capacity(cap),
            inverse: HashMap::with_capacity(cap),
        }
    }

    /// Insert a key-value pair. If the value already exists under a different key,
    /// that old key is removed (bijection invariant). Returns the old value for the
    /// key if it existed.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        // If value exists under a different key, remove that key
        if let Some(existing_key) = self.inverse.get(&value) {
            if *existing_key != key {
                let ek = existing_key.clone();
                self.forward.remove(&ek);
            }
        }
        // If key already maps to a different value, remove old inverse
        let old = if let Some(old_value) = self.forward.get(&key) {
            let ov = old_value.clone();
            self.inverse.remove(&ov);
            Some(ov)
        } else {
            None
        };
        self.inverse.insert(value.clone(), key.clone());
        self.forward.insert(key, value);
        old
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.forward.get(key)
    }

    pub fn get_inverse(&self, value: &V) -> Option<&K> {
        self.inverse.get(value)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.forward.contains_key(key)
    }
    pub fn contains_value(&self, value: &V) -> bool {
        self.inverse.contains_key(value)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(v) = self.forward.remove(key) {
            self.inverse.remove(&v);
            Some(v)
        } else {
            None
        }
    }

    pub fn remove_inverse(&mut self, value: &V) -> Option<K> {
        if let Some(k) = self.inverse.remove(value) {
            self.forward.remove(&k);
            Some(k)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.forward.len()
    }
    pub fn is_empty(&self) -> bool {
        self.forward.is_empty()
    }

    pub fn clear(&mut self) {
        self.forward.clear();
        self.inverse.clear();
    }

    /// Returns a snapshot copy with keys and values swapped.
    pub fn inverse(&self) -> HashBiMap<V, K> {
        HashBiMap {
            forward: self.inverse.clone(),
            inverse: self.forward.clone(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.forward.iter()
    }
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.forward.keys()
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.forward.values()
    }

    pub fn for_each(&self, mut f: impl FnMut(&K, &V)) {
        for (k, v) in &self.forward {
            f(k, v);
        }
    }
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> MapIterable<K, V> for HashBiMap<K, V> {
    fn len(&self) -> usize {
        self.forward.len()
    }
    fn contains_key(&self, key: &K) -> bool {
        self.forward.contains_key(key)
    }
    fn get(&self, key: &K) -> Option<&V> {
        self.forward.get(key)
    }
    fn iter(&self) -> Box<dyn Iterator<Item = (&K, &V)> + '_> {
        Box::new(self.forward.iter())
    }
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> Default for HashBiMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut bm = HashBiMap::new();
        assert_eq!(bm.put("a", 1), None);
        assert_eq!(bm.put("b", 2), None);
        assert_eq!(bm.get(&"a"), Some(&1));
        assert_eq!(bm.get_inverse(&2), Some(&"b"));
        assert_eq!(bm.len(), 2);
    }

    #[test]
    fn test_bijection_enforcement() {
        let mut bm = HashBiMap::new();
        bm.put("a", 1);
        bm.put("b", 2);
        // Insert value 1 under key "c" — should remove "a"
        bm.put("c", 1);
        assert!(!bm.contains_key(&"a"));
        assert_eq!(bm.get(&"c"), Some(&1));
        assert_eq!(bm.get_inverse(&1), Some(&"c"));
        assert_eq!(bm.len(), 2);
    }

    #[test]
    fn test_overwrite_same_key() {
        let mut bm = HashBiMap::new();
        bm.put("a", 1);
        let old = bm.put("a", 2);
        assert_eq!(old, Some(1));
        assert_eq!(bm.get(&"a"), Some(&2));
        assert!(!bm.contains_value(&1));
        assert!(bm.contains_value(&2));
    }

    #[test]
    fn test_remove_and_inverse() {
        let mut bm = HashBiMap::new();
        bm.put("x", 10);
        bm.put("y", 20);
        assert_eq!(bm.remove(&"x"), Some(10));
        assert!(!bm.contains_value(&10));
        assert_eq!(bm.remove_inverse(&20), Some("y"));
        assert!(bm.is_empty());
    }

    #[test]
    fn test_inverse_snapshot() {
        let mut bm = HashBiMap::new();
        bm.put("a", 1);
        bm.put("b", 2);
        let inv = bm.inverse();
        assert_eq!(inv.get(&1), Some(&"a"));
        assert_eq!(inv.get(&2), Some(&"b"));
    }
}
