// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Open-addressing hash map with pluggable [`HashingStrategy`] for keys.

use super::strategy::HashingStrategy;
use std::fmt;

const DEFAULT_CAPACITY: usize = 16;

struct Entry<K, V> {
    key: Option<K>,
    value: Option<V>,
}

impl<K, V> Entry<K, V> {
    fn empty() -> Self {
        Entry {
            key: None,
            value: None,
        }
    }

    fn is_occupied(&self) -> bool {
        self.key.is_some()
    }
}

/// An open-addressing hash map that uses a pluggable [`HashingStrategy`]
/// for key identity. This allows case-insensitive maps, maps keyed by
/// extracted fields, etc.
pub struct HashMapWithStrategy<K, V> {
    entries: Vec<Entry<K, V>>,
    size: usize,
    strategy: HashingStrategy<K>,
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for HashMapWithStrategy<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V> HashMapWithStrategy<K, V> {
    /// Creates an empty map using the given hashing strategy for keys.
    pub fn new(strategy: HashingStrategy<K>) -> Self {
        Self::with_capacity(strategy, DEFAULT_CAPACITY)
    }

    /// Creates an empty map with pre-allocated capacity.
    pub fn with_capacity(strategy: HashingStrategy<K>, capacity: usize) -> Self {
        let cap = capacity
            .max(DEFAULT_CAPACITY)
            .checked_next_power_of_two()
            .unwrap_or(usize::MAX);
        let mut entries = Vec::with_capacity(cap);
        for _ in 0..cap {
            entries.push(Entry::empty());
        }
        HashMapWithStrategy {
            entries,
            size: 0,
            strategy,
        }
    }

    /// Inserts a key-value pair. Returns `Some(old_value)` if the key was
    /// already present (per the strategy's equality), or `None` if it was new.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.entries.len() - 1;
        let mut idx = self.strategy.hash_code(&key) as usize & mask;
        loop {
            if !self.entries[idx].is_occupied() {
                self.entries[idx].key = Some(key);
                self.entries[idx].value = Some(value);
                self.size += 1;
                return None;
            }
            if self
                .strategy
                .equals(self.entries[idx].key.as_ref().unwrap(), &key)
            {
                let old = self.entries[idx].value.take();
                self.entries[idx].value = Some(value);
                return old;
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Returns a reference to the value associated with the key, or `None`.
    pub fn get(&self, key: &K) -> Option<&V> {
        if self.size == 0 {
            return None;
        }
        let mask = self.entries.len() - 1;
        let mut idx = self.strategy.hash_code(key) as usize & mask;
        loop {
            if !self.entries[idx].is_occupied() {
                return None;
            }
            if self
                .strategy
                .equals(self.entries[idx].key.as_ref().unwrap(), key)
            {
                return self.entries[idx].value.as_ref();
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Removes the entry for the given key. Returns `Some(value)` if found.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if self.size == 0 {
            return None;
        }
        let mask = self.entries.len() - 1;
        let mut idx = self.strategy.hash_code(key) as usize & mask;
        loop {
            if !self.entries[idx].is_occupied() {
                return None;
            }
            if self
                .strategy
                .equals(self.entries[idx].key.as_ref().unwrap(), key)
            {
                let old = self.entries[idx].value.take();
                self.entries[idx].key = None;
                self.size -= 1;
                self.rehash_from(idx, mask);
                return old;
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Returns `true` if the map contains the given key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Returns the number of key-value pairs.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns `true` if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Removes all entries.
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.key = None;
            entry.value = None;
        }
        self.size = 0;
    }

    /// Returns an iterator over `(&K, &V)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().filter_map(|e| {
            if e.is_occupied() {
                Some((e.key.as_ref().unwrap(), e.value.as_ref().unwrap()))
            } else {
                None
            }
        })
    }

    /// Returns an iterator over keys.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().filter_map(|e| e.key.as_ref())
    }

    /// Returns an iterator over values.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().filter_map(|e| {
            if e.is_occupied() {
                e.value.as_ref()
            } else {
                None
            }
        })
    }

    /// Calls `f` for each key-value pair.
    pub fn for_each(&self, mut f: impl FnMut(&K, &V)) {
        for entry in &self.entries {
            if let (Some(ref k), Some(ref v)) = (&entry.key, &entry.value) {
                f(k, v);
            }
        }
    }

    // ── internal ────────────────────────────────────────────────────

    fn needs_resize(&self) -> bool {
        // Grow strictly *below* the 0.75 load factor: the table must
        // not hold `(size+1)` entries once that count reaches `cap*3/4`.
        // `>=` (not `>`) ensures we grow when `cap*3 == (size+1)*4`
        // exactly (e.g. the 12th insert into a capacity-16 table).
        (self.size + 1) * 4 >= self.entries.len() * 3
    }

    fn resize(&mut self) {
        let new_cap = self.entries.len() * 2;
        let old = std::mem::replace(&mut self.entries, {
            let mut v = Vec::with_capacity(new_cap);
            for _ in 0..new_cap {
                v.push(Entry::empty());
            }
            v
        });
        self.size = 0;
        for entry in old {
            if let (Some(k), Some(v)) = (entry.key, entry.value) {
                self.insert(k, v);
            }
        }
    }

    fn rehash_from(&mut self, deleted: usize, mask: usize) {
        let cap = self.entries.len();
        let mut gap = deleted;
        let mut idx = (deleted + 1) & mask;
        while self.entries[idx].is_occupied() {
            let ideal =
                self.strategy
                    .hash_code(self.entries[idx].key.as_ref().unwrap()) as usize
                    & mask;
            let dist_current = (idx.wrapping_sub(ideal).wrapping_add(cap)) & mask;
            let dist_gap = (gap.wrapping_sub(ideal).wrapping_add(cap)) & mask;
            if dist_current > dist_gap {
                self.entries.swap(gap, idx);
                gap = idx;
            }
            idx = (idx + 1) & mask;
            if idx == gap {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::strategy::*;

    #[test]
    fn test_case_insensitive_map() {
        let mut m = HashMapWithStrategy::new(case_insensitive_hashing_strategy());
        m.insert("Content-Type".to_string(), 1);
        let old = m.insert("content-type".to_string(), 2); // should overwrite
        assert_eq!(old, Some(1));
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(&"CONTENT-TYPE".to_string()), Some(&2));
    }

    #[derive(Debug, Clone)]
    struct Person {
        name: String,
        _age: i32,
    }

    #[test]
    fn test_by_field_map() {
        let strategy = by_field(|p: &Person| p.name.clone());
        let mut m = HashMapWithStrategy::new(strategy);
        m.insert(
            Person {
                name: "Alice".into(),
                _age: 30,
            },
            "first".to_string(),
        );
        let old = m.insert(
            Person {
                name: "Alice".into(),
                _age: 25,
            },
            "second".to_string(),
        ); // overwrites by name
        assert_eq!(old, Some("first".to_string()));
        assert_eq!(m.len(), 1);
        let v = m.get(&Person {
            name: "Alice".into(),
            _age: 0,
        });
        assert_eq!(v, Some(&"second".to_string()));
    }

    #[test]
    fn test_remove() {
        let mut m = HashMapWithStrategy::new(string_hashing_strategy());
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);
        assert_eq!(m.remove(&"a".to_string()), Some(1));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key(&"a".to_string()));
        assert!(m.contains_key(&"b".to_string()));
    }

    #[test]
    fn test_clear() {
        let mut m = HashMapWithStrategy::new(string_hashing_strategy());
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_resize_stress() {
        let mut m = HashMapWithStrategy::new(string_hashing_strategy());
        for i in 0..500 {
            m.insert(format!("key_{}", i), i);
        }
        assert_eq!(m.len(), 500);
        for i in 0..500 {
            assert_eq!(m.get(&format!("key_{}", i)), Some(&i));
        }
    }

    // Exercises the std `next_power_of_two()` capacity path (previously a
    // hand-rolled `next_pow2` with an ungated `v >> 32` that panicked on
    // 32-bit targets). A from-scratch grow drives the same code path.
    #[test]
    fn test_capacity_growth_via_next_power_of_two() {
        let mut m = HashMapWithStrategy::new(string_hashing_strategy());
        assert_eq!(m.entries.len(), 16);
        for i in 0..200 {
            m.insert(format!("k{}", i), i);
        }
        // Table must have grown to a power of two strictly larger than 16.
        assert!(m.entries.len() > 16);
        assert!(m.entries.len().is_power_of_two());
        assert_eq!(m.len(), 200);
    }

    // Spec: load factor must stay strictly below 0.75. With a capacity-16
    // table, the 12th distinct entry (size would reach 12 == 16*0.75) must
    // trigger a grow before it is stored.
    #[test]
    fn test_load_factor_strictly_below_three_quarters() {
        let mut m = HashMapWithStrategy::new(string_hashing_strategy());
        assert_eq!(m.entries.len(), 16);
        for i in 0..11 {
            m.insert(format!("k{}", i), i);
        }
        // 11 entries: 12*4 = 48 >= 16*3 = 48 would grow on the *next*
        // insert; at 11 the table has not yet grown.
        assert_eq!(m.entries.len(), 16);
        m.insert("k11".to_string(), 11);
        // 12th insert: must have grown strictly below 0.75.
        assert_eq!(m.entries.len(), 32);
        assert_eq!(m.len(), 12);
    }

    #[test]
    fn test_iter() {
        let mut m = HashMapWithStrategy::new(string_hashing_strategy());
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);
        let mut pairs: Vec<_> = m.iter().map(|(k, v)| (k.clone(), *v)).collect();
        pairs.sort();
        assert_eq!(pairs, vec![("a".to_string(), 1), ("b".to_string(), 2)]);
    }
}
