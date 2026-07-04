// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Open-addressing hash map with pluggable [`HashingStrategy`] for keys.
//!
//! Built on the crate's shared kernel rather than a private probe loop
//! (blueprint M4/M5): a [`SlotList`] arena owns the `(K, V)` entries and an
//! [`IndexTable`] maps each key's strategy hash to its arena slot. This deletes
//! the duplicate Robin-Hood implementation the type used to carry, and inherits
//! the index's hardening — because the index stores each key's hash inline,
//! backward-shift deletion and resize re-derive ideal positions *without*
//! calling the user's strategy, so a panic in a `HashingStrategy` closure can
//! only occur during the read-only probe, never mid-shift. (The old
//! `rehash_from` re-invoked `strategy.hash_code` while shifting.)
//!
//! Identity comes entirely from the strategy (there is no `K: Hash + Eq`
//! bound); the index's own `BuildHasher` is unused — lookups pass the
//! strategy-computed hash directly.

use super::strategy::HashingStrategy;
use crate::index_table::{IndexTable, RawEntry};
use crate::slot_list::SlotList;
use std::fmt;

/// An open-addressing hash map that uses a pluggable [`HashingStrategy`]
/// for key identity. This allows case-insensitive maps, maps keyed by
/// extracted fields, etc.
pub struct HashMapWithStrategy<K, V> {
    /// Sole owner of every key and value (insertion order).
    slots: SlotList<(K, V)>,
    /// Strategy-hash → arena slot index. Owns no keys; its `BuildHasher` is
    /// unused (hashes come from the strategy).
    index: IndexTable,
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
        HashMapWithStrategy {
            slots: SlotList::new(),
            index: IndexTable::new(),
            strategy,
        }
    }

    /// Creates an empty map with pre-allocated capacity.
    pub fn with_capacity(strategy: HashingStrategy<K>, capacity: usize) -> Self {
        HashMapWithStrategy {
            slots: SlotList::with_capacity(capacity),
            index: IndexTable::with_capacity(capacity),
            strategy,
        }
    }

    /// Inserts a key-value pair. Returns `Some(old_value)` if the key was
    /// already present (per the strategy's equality), or `None` if it was new.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = self.strategy.hash_code(&key);
        // Disjoint-field borrows: `probe` borrows `index` mutably while the
        // `eq` closure reads `slots`/`strategy`. No user code runs between the
        // probe and `fill_vacant`.
        let slots = &self.slots;
        let strategy = &self.strategy;
        match self
            .index
            .probe(hash, |s| strategy.equals(&slots.get(s).0, &key))
        {
            RawEntry::Occupied(slot) => {
                Some(std::mem::replace(&mut self.slots.get_mut(slot).1, value))
            }
            RawEntry::Vacant(cell) => {
                let slot = self.slots.push_back((key, value));
                self.index.fill_vacant(cell, hash, slot);
                None
            }
        }
    }

    /// Returns a reference to the value associated with the key, or `None`.
    pub fn get(&self, key: &K) -> Option<&V> {
        let slot = self.find_slot(key)?;
        Some(&self.slots.get(slot).1)
    }

    /// Removes the entry for the given key. Returns `Some(value)` if found.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let hash = self.strategy.hash_code(key);
        let slots = &self.slots;
        let strategy = &self.strategy;
        let slot = self
            .index
            .remove(hash, |s| strategy.equals(&slots.get(s).0, key))?;
        let (_k, v) = self.slots.unlink_free(slot);
        Some(v)
    }

    /// Returns `true` if the map contains the given key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.find_slot(key).is_some()
    }

    /// Returns the number of key-value pairs.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns `true` if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Removes all entries.
    pub fn clear(&mut self) {
        // Clear the index first so a panicking value `Drop` during the arena
        // clear can never leave the index referencing a half-cleared arena.
        self.index.clear();
        self.slots.clear();
    }

    /// Returns an iterator over `(&K, &V)` pairs (insertion order).
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.slots.iter().map(|(k, v)| (k, v))
    }

    /// Returns an iterator over keys (insertion order).
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.slots.iter().map(|(k, _)| k)
    }

    /// Returns an iterator over values (insertion order).
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.slots.iter().map(|(_, v)| v)
    }

    /// Calls `f` for each key-value pair.
    pub fn for_each(&self, mut f: impl FnMut(&K, &V)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    /// Locate the arena slot for `key` through the index. The `eq` closure and
    /// `find` borrow disjoint fields (`slots`/`strategy` vs `index`), so this
    /// type-checks without splitting the struct.
    #[inline]
    fn find_slot(&self, key: &K) -> Option<usize> {
        if self.slots.is_empty() {
            return None;
        }
        let hash = self.strategy.hash_code(key);
        let slots = &self.slots;
        let strategy = &self.strategy;
        self.index
            .find(hash, |s| strategy.equals(&slots.get(s).0, key))
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

    // Resize/backward-shift correctness through many grows: the capacity policy
    // itself now lives in (and is unit-tested by) `IndexTable`; here we only
    // pin the behavioral contract — every key survives repeated resizes.
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

    // Remove-heavy churn exercises the index's backward-shift deletion against
    // the arena's slot recycling: interleave inserts and removes, then verify
    // the surviving set is exactly correct.
    #[test]
    fn test_remove_churn_keeps_probe_chains_intact() {
        let mut m = HashMapWithStrategy::new(string_hashing_strategy());
        for i in 0..200 {
            m.insert(format!("k{}", i), i);
        }
        // Remove every even key.
        for i in (0..200).step_by(2) {
            assert_eq!(m.remove(&format!("k{}", i)), Some(i));
        }
        assert_eq!(m.len(), 100);
        for i in 0..200 {
            let got = m.get(&format!("k{}", i));
            if i % 2 == 0 {
                assert_eq!(got, None);
            } else {
                assert_eq!(got, Some(&i));
            }
        }
        // Re-insert the removed keys (drives slot reuse + fresh index cells).
        for i in (0..200).step_by(2) {
            assert!(m.insert(format!("k{}", i), i * 10).is_none());
        }
        assert_eq!(m.len(), 200);
        assert_eq!(m.get(&"k0".to_string()), Some(&0));
        assert_eq!(m.get(&"k2".to_string()), Some(&20));
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
