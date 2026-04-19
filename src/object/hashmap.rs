// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use super::traits::*;
use std::collections::HashMap as StdHashMap;
use std::hash::Hash;

/// Generic unordered map backed by `std::collections::HashMap`.
#[derive(Debug, Clone)]
pub struct HashMap<K: Eq + Hash, V> {
    inner: StdHashMap<K, V>,
}

impl<K: Eq + Hash, V> HashMap<K, V> {
    pub fn new() -> Self {
        HashMap {
            inner: StdHashMap::new(),
        }
    }
    pub fn with_capacity(cap: usize) -> Self {
        HashMap {
            inner: StdHashMap::with_capacity(cap),
        }
    }
}

impl<K: Eq + Hash, V> MapIterable<K, V> for HashMap<K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
    fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }
    fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }
    fn iter(&self) -> Box<dyn Iterator<Item = (&K, &V)> + '_> {
        Box::new(self.inner.iter())
    }
}

impl<K: Eq + Hash, V> MutableMap<K, V> for HashMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }
    fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.remove(key)
    }
    fn clear(&mut self) {
        self.inner.clear();
    }
}

impl<K: Eq + Hash, V> HashMap<K, V> {
    pub fn keys_to_vec(&self) -> Vec<&K> {
        self.inner.keys().collect()
    }
    pub fn values_to_vec(&self) -> Vec<&V> {
        self.inner.values().collect()
    }
    pub fn contains_value(&self, value: &V) -> bool
    where
        V: PartialEq,
    {
        self.inner.values().any(|v| v == value)
    }
    pub fn count_where(&self, predicate: impl Fn(&K, &V) -> bool) -> usize {
        self.inner.iter().filter(|(k, v)| predicate(k, v)).count()
    }
    pub fn detect(&self, predicate: impl Fn(&K, &V) -> bool) -> Option<(&K, &V)> {
        self.inner.iter().find(|(k, v)| predicate(k, v))
    }
}

impl<K: Eq + Hash + Clone, V: Clone> HashMap<K, V> {
    pub fn select(&self, predicate: impl Fn(&K, &V) -> bool) -> Self {
        HashMap {
            inner: self
                .inner
                .iter()
                .filter(|(k, v)| predicate(k, v))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
    pub fn reject(&self, predicate: impl Fn(&K, &V) -> bool) -> Self {
        HashMap {
            inner: self
                .inner
                .iter()
                .filter(|(k, v)| !predicate(k, v))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
}

impl<K: Eq + Hash, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut m = HashMap::new();
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
    fn test_functional() {
        let mut m = HashMap::new();
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
        let mut m = HashMap::new();
        m.insert(1, 10);
        m.insert(2, 20);
        m.insert(3, 30);
        let big = m.select(|_, v| *v > 15);
        assert_eq!(big.len(), 2);
        let small = m.reject(|_, v| *v > 15);
        assert_eq!(small.len(), 1);
    }
}
