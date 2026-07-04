// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use crate::bulk::{BulkError, DuplicatePolicy};
use crate::hash_table::OpenHashMap;
use std::borrow::Borrow;
use std::hash::Hash;

/// Generic unordered map backed by [`crate::hash_table::OpenHashMap`] — the
/// project's port of Eclipse Collections' open-addressing hash map with
/// niche-packed key/value slots for cache locality. (Not `std::HashMap`.)
#[derive(Debug, Clone)]
pub struct HashMap<K: Eq + Hash, V> {
    inner: OpenHashMap<K, V>,
}

impl<K: Eq + Hash, V> HashMap<K, V> {
    pub fn new() -> Self {
        HashMap {
            inner: OpenHashMap::new(),
        }
    }
    pub fn with_capacity(cap: usize) -> Self {
        HashMap {
            inner: OpenHashMap::with_capacity(cap),
        }
    }

    /// Bulk-loads a fresh map (size-hint path; may rehash). See
    /// [`OpenHashMap::bulk_load`].
    pub fn bulk_load<I: IntoIterator<Item = (K, V)>>(
        iter: I,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError> {
        Ok(HashMap {
            inner: OpenHashMap::bulk_load(iter, dup)?,
        })
    }

    /// Zero-rehash bulk load for an exactly-`n`-element source. See
    /// [`OpenHashMap::bulk_load_exact`].
    pub fn bulk_load_exact<I: IntoIterator<Item = (K, V)>>(
        iter: I,
        n: usize,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError> {
        Ok(HashMap {
            inner: OpenHashMap::bulk_load_exact(iter, n, dup)?,
        })
    }
}

// ---- core + functional API (formerly the trait tower) ----------------------

impl<K: Eq + Hash, V> HashMap<K, V> {
    /// The number of entries.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    /// Insert `key`→`value`, returning the previous value if the key existed.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }
    /// Remove all entries.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Apply `f` to each entry.
    pub fn for_each(&self, mut f: impl FnMut(&K, &V)) {
        for (k, v) in self.inner.iter() {
            f(k, v);
        }
    }
    /// Whether any entry satisfies `predicate`.
    pub fn any_satisfy(&self, predicate: impl Fn(&K, &V) -> bool) -> bool {
        self.inner.iter().any(|(k, v)| predicate(k, v))
    }
    /// Whether every entry satisfies `predicate`.
    pub fn all_satisfy(&self, predicate: impl Fn(&K, &V) -> bool) -> bool {
        self.inner.iter().all(|(k, v)| predicate(k, v))
    }
    /// Whether no entry satisfies `predicate`.
    pub fn none_satisfy(&self, predicate: impl Fn(&K, &V) -> bool) -> bool {
        !self.inner.iter().any(|(k, v)| predicate(k, v))
    }
}

impl<K: Eq + Hash, V> HashMap<K, V> {
    pub fn contains_value(&self, value: &V) -> bool
    where
        V: PartialEq,
    {
        self.inner.iter().any(|(_, v)| v == value)
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
        let mut out = HashMap::new();
        for (k, v) in self.inner.iter() {
            if predicate(k, v) {
                out.inner.insert(k.clone(), v.clone());
            }
        }
        out
    }
    pub fn reject(&self, predicate: impl Fn(&K, &V) -> bool) -> Self {
        let mut out = HashMap::new();
        for (k, v) in self.inner.iter() {
            if !predicate(k, v) {
                out.inner.insert(k.clone(), v.clone());
            }
        }
        out
    }
}

impl<K: Eq + Hash, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

// ---- idiomatic std-style additions ----------------------------------------

impl<K: Eq + Hash, V> HashMap<K, V> {
    /// Borrowed `(&K, &V)` iterator, so `for (k, v) in &map` and `map.iter()`
    /// both work.
    pub fn iter(&self) -> crate::hash_table::OpenHashMapIter<'_, K, V> {
        self.inner.iter()
    }

    /// Looks up a value by any borrowed form of the key
    /// (`K: Borrow<Q>`), e.g. `map.get("str")` on a `HashMap<String, _>`.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get(key)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get_mut(key)
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains_key(key)
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(key)
    }
}

impl<'a, K: Eq + Hash, V> IntoIterator for &'a HashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = crate::hash_table::OpenHashMapIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<K: Eq + Hash, V> IntoIterator for HashMap<K, V> {
    type Item = (K, V);
    type IntoIter = crate::hash_table::OpenHashMapIntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<K: Eq + Hash, V> FromIterator<(K, V)> for HashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        HashMap {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<K: Eq + Hash, V> Extend<(K, V)> for HashMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.inner.insert(k, v);
        }
    }
}

/// Order-insensitive structural equality: same key set with equal values.
impl<K: Eq + Hash, V: PartialEq> PartialEq for HashMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.len() == other.inner.len()
            && self
                .inner
                .iter()
                .all(|(k, v)| other.inner.get(k) == Some(v))
    }
}

impl<K: Eq + Hash, V: Eq> Eq for HashMap<K, V> {}

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

    #[test]
    fn test_into_iter_borrowing() {
        let mut m = HashMap::new();
        m.insert("a", 1);
        m.insert("b", 2);
        let mut sum = 0;
        for (_k, v) in &m {
            sum += *v;
        }
        assert_eq!(sum, 3);
    }

    #[test]
    fn test_into_iter_owned() {
        let mut m = HashMap::new();
        m.insert("a", 1);
        m.insert("b", 2);
        let mut collected: Vec<(&str, i32)> = m.into_iter().collect();
        collected.sort();
        assert_eq!(collected, vec![("a", 1), ("b", 2)]);
    }

    #[test]
    fn test_from_iterator_and_extend() {
        let mut m: HashMap<&str, i32> = [("a", 1), ("b", 2)].into_iter().collect();
        assert_eq!(m.len(), 2);
        m.extend([("c", 3), ("d", 4)]);
        assert_eq!(m.len(), 4);
        assert_eq!(m.get(&"c"), Some(&3));
    }

    #[test]
    fn test_partial_eq_order_insensitive() {
        let a: HashMap<i32, i32> = [(1, 10), (2, 20)].into_iter().collect();
        let b: HashMap<i32, i32> = [(2, 20), (1, 10)].into_iter().collect();
        assert_eq!(a, b);
        let c: HashMap<i32, i32> = [(1, 10), (2, 99)].into_iter().collect();
        assert_ne!(a, c);
    }

    #[test]
    fn test_borrow_lookup_str() {
        let mut m: HashMap<String, i32> = HashMap::new();
        m.insert("hello".to_string(), 1);
        // Look up with &str on a String-keyed map (Borrow).
        assert_eq!(m.get("hello"), Some(&1));
        assert!(m.contains_key("hello"));
        assert_eq!(m.remove("hello"), Some(1));
        assert!(!m.contains_key("hello"));
    }

    #[test]
    fn test_get_mut() {
        let mut m: HashMap<&str, i32> = HashMap::new();
        m.insert("a", 1);
        if let Some(v) = m.get_mut(&"a") {
            *v = 99;
        }
        assert_eq!(m.get(&"a"), Some(&99));
    }

    #[test]
    fn bulk_load_equals_incremental() {
        use crate::bulk::DuplicatePolicy;
        let data: Vec<(i32, i32)> = (0..50).map(|i| (i, i * 2)).collect();
        let bulk =
            HashMap::bulk_load_exact(data.clone(), data.len(), DuplicatePolicy::Error).unwrap();
        let mut inc = HashMap::new();
        for (k, v) in &data {
            inc.insert(*k, *v);
        }
        assert_eq!(bulk, inc);
    }
}
