// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use super::traits::*;
use std::collections::HashMap as StdHashMap;
use std::hash::Hash;

/// Insertion-ordered map backed by a `Vec` of entries and a hash index.
/// Iteration follows insertion order. Updating an existing key preserves
/// its position; only new keys are appended.
#[derive(Debug, Clone)]
pub struct LinkedHashMap<K: Eq + Hash, V> {
    entries: Vec<(K, V)>,
    index: StdHashMap<K, usize>,
}

impl<K: Eq + Hash + Clone, V> LinkedHashMap<K, V> {
    pub fn new() -> Self {
        LinkedHashMap {
            entries: Vec::new(),
            index: StdHashMap::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        LinkedHashMap {
            entries: Vec::with_capacity(cap),
            index: StdHashMap::with_capacity(cap),
        }
    }
}

impl<K: Eq + Hash + Clone, V> MapIterable<K, V> for LinkedHashMap<K, V> {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    fn get(&self, key: &K) -> Option<&V> {
        self.index.get(key).map(|&i| &self.entries[i].1)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (&K, &V)> + '_> {
        Box::new(self.entries.iter().map(|(k, v)| (k, v)))
    }
}

impl<K: Eq + Hash + Clone, V> MutableMap<K, V> for LinkedHashMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(&idx) = self.index.get(&key) {
            let old = std::mem::replace(&mut self.entries[idx].1, value);
            Some(old)
        } else {
            let idx = self.entries.len();
            self.index.insert(key.clone(), idx);
            self.entries.push((key, value));
            None
        }
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(idx) = self.index.remove(key) {
            let (_, v) = self.entries.remove(idx);
            // Fix indices for entries that shifted down
            for (_, i) in self.index.iter_mut() {
                if *i > idx {
                    *i -= 1;
                }
            }
            Some(v)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }
}

impl<K: Eq + Hash + Clone, V: Clone> LinkedHashMap<K, V> {
    pub fn keys_to_vec(&self) -> Vec<&K> {
        self.entries.iter().map(|(k, _)| k).collect()
    }
    pub fn values_to_vec(&self) -> Vec<&V> {
        self.entries.iter().map(|(_, v)| v).collect()
    }

    pub fn contains_value(&self, value: &V) -> bool
    where
        V: PartialEq,
    {
        self.entries.iter().any(|(_, v)| v == value)
    }

    pub fn count_where(&self, predicate: impl Fn(&K, &V) -> bool) -> usize {
        self.entries.iter().filter(|(k, v)| predicate(k, v)).count()
    }

    pub fn detect(&self, predicate: impl Fn(&K, &V) -> bool) -> Option<(&K, &V)> {
        self.entries
            .iter()
            .find(|(k, v)| predicate(k, v))
            .map(|(k, v)| (k, v))
    }

    pub fn select(&self, predicate: impl Fn(&K, &V) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in &self.entries {
            if predicate(k, v) {
                result.insert(k.clone(), v.clone());
            }
        }
        result
    }

    pub fn reject(&self, predicate: impl Fn(&K, &V) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in &self.entries {
            if !predicate(k, v) {
                result.insert(k.clone(), v.clone());
            }
        }
        result
    }
}

impl<K: Eq + Hash + Clone, V> Default for LinkedHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut m = LinkedHashMap::new();
        assert!(m.is_empty());
        assert_eq!(m.insert("a", 1), None);
        assert_eq!(m.insert("b", 2), None);
        assert_eq!(m.insert("a", 10), Some(1));
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&"a"), Some(&10));
        assert_eq!(m.remove(&"a"), Some(10));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn test_insertion_order() {
        let mut m = LinkedHashMap::new();
        m.insert("c", 3);
        m.insert("a", 1);
        m.insert("b", 2);
        let keys: Vec<&&str> = m.keys_to_vec();
        assert_eq!(keys, vec![&"c", &"a", &"b"]);
    }

    #[test]
    fn test_overwrite_preserves_order() {
        let mut m = LinkedHashMap::new();
        m.insert("a", 1);
        m.insert("b", 2);
        m.insert("c", 3);
        m.insert("b", 20);
        let keys: Vec<&&str> = m.keys_to_vec();
        assert_eq!(keys, vec![&"a", &"b", &"c"]);
        assert_eq!(m.get(&"b"), Some(&20));
    }

    #[test]
    fn test_remove_preserves_order() {
        let mut m = LinkedHashMap::new();
        m.insert("a", 1);
        m.insert("b", 2);
        m.insert("c", 3);
        m.remove(&"b");
        let keys: Vec<&&str> = m.keys_to_vec();
        assert_eq!(keys, vec![&"a", &"c"]);
    }

    #[test]
    fn test_functional() {
        let mut m = LinkedHashMap::new();
        m.insert("x", 1);
        m.insert("y", 2);
        m.insert("z", 3);
        assert!(m.any_satisfy(|_, v| *v > 2));
        assert!(m.all_satisfy(|_, v| *v > 0));
        assert!(m.none_satisfy(|_, v| *v > 10));
        assert_eq!(m.count_where(|_, v| *v % 2 == 0), 1);
    }

    #[test]
    fn test_select_reject() {
        let mut m = LinkedHashMap::new();
        m.insert(1, 10);
        m.insert(2, 20);
        m.insert(3, 30);
        let big = m.select(|_, v| *v > 15);
        assert_eq!(big.len(), 2);
        let keys: Vec<&i32> = big.keys_to_vec();
        assert_eq!(keys, vec![&2, &3]);
        let small = m.reject(|_, v| *v > 15);
        assert_eq!(small.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut m = LinkedHashMap::new();
        m.insert(1, 10);
        m.clear();
        assert!(m.is_empty());
    }
}
