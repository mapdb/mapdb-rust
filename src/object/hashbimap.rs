// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Generic bidirectional map.
//!
//! Each `(key, value)` pair lives once in a [`SlotList`] arena; two
//! [`IndexTable`]s map key-hash and value-hash to the pair's slot. The arena
//! stores both hashes beside the pair, so a mutator can update either index
//! without hashing again.
//!
//! **Panic consistency.** Every mutator first runs all the user code it needs
//! (`Hash` and `Eq` on keys and values) without changing the mapping, then
//! edits the arena and both indices with no user code at all: removals match
//! by slot number and stored hash, and growth is reserved up front. Pairs it
//! removes are dropped only after both indices agree again. So a panicking
//! `Hash` or `Eq` leaves the mapping unchanged (an index may already have
//! grown, which is not observable through the API), and a panicking `Drop`
//! leaves it in its new state; either way forward and inverse lookups describe
//! the same bijection.
//!
//! Iteration follows insertion order of the pairs.

use crate::bulk::{BulkError, DuplicatePolicy};
use crate::index_table::{IndexTable, RawEntry};
use crate::slot_list::{self, SlotList};
use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;

/// One pair with the hashes its key and value have in the two indices.
#[derive(Clone)]
struct Pair<K, V> {
    key: K,
    value: V,
    key_hash: u64,
    value_hash: u64,
}

/// Generic bidirectional map. Both keys and values must be unique (bijection).
#[derive(Clone)]
pub struct HashBiMap<K: Eq + Hash + Clone, V: Eq + Hash + Clone> {
    /// Every pair, in insertion order; the sole owner of keys and values.
    pairs: SlotList<Pair<K, V>>,
    /// key-hash → slot.
    forward: IndexTable,
    /// value-hash → slot.
    inverse: IndexTable,
}

/// An insert's hashes and where its key and value stand in their indices.
struct Located {
    key_hash: u64,
    value_hash: u64,
    by_key: RawEntry,
    by_value: RawEntry,
}

/// Add a slot known to be absent from `table`. No user code.
fn index_new(table: &mut IndexTable, hash: u64, slot: usize) {
    match table.probe(hash, |_| false) {
        RawEntry::Vacant(cell) => table.fill_vacant(cell, hash, slot),
        RawEntry::Occupied(_) => unreachable!("probe without a match is never occupied"),
    }
}

/// Remove `slot` from `table`, matching by slot number. No user code.
fn index_remove(table: &mut IndexTable, hash: u64, slot: usize) {
    let removed = table.remove(hash, |s| s == slot);
    debug_assert_eq!(removed, Some(slot), "bimap index lost a slot");
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> HashBiMap<K, V> {
    pub fn new() -> Self {
        HashBiMap {
            pairs: SlotList::new(),
            forward: IndexTable::new(),
            inverse: IndexTable::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        HashBiMap {
            pairs: SlotList::with_capacity(cap),
            forward: IndexTable::with_capacity(cap),
            inverse: IndexTable::with_capacity(cap),
        }
    }

    /// Bulk-loads a fresh bijective map. A BiMap requires a bijection, so
    /// [`DuplicatePolicy`] **does not apply**: a duplicate **key** OR a
    /// duplicate **value** is *always* a [`BulkError::Duplicate`], even under
    /// [`DuplicatePolicy::IgnoreDuplicates`] and even for an identical `(k, v)`
    /// pair (an identical pair repeats the key, which breaks the single-pass
    /// bijection build). The `dup` parameter is accepted only for API symmetry
    /// with the other bulk loaders and is otherwise ignored.
    ///
    /// The capacity reserved up front is the iterator's lower size hint.
    pub fn bulk_load<I: IntoIterator<Item = (K, V)>>(
        iter: I,
        _dup: DuplicatePolicy,
    ) -> Result<Self, BulkError> {
        let iter = iter.into_iter();
        let mut map = Self::with_capacity(iter.size_hint().0);
        for (index, (k, v)) in iter.enumerate() {
            // BiMap ignores DuplicatePolicy: any duplicate key or value (incl.
            // an identical pair) is always an error.
            let at = map.locate(&k, &v);
            match (at.by_key, at.by_value) {
                (RawEntry::Vacant(key_cell), RawEntry::Vacant(value_cell)) => {
                    map.add_at(k, v, at.key_hash, at.value_hash, key_cell, value_cell)
                }
                _ => return Err(BulkError::Duplicate { index }),
            }
        }
        Ok(map)
    }

    /// Hash `key` and `value` and probe both indices. The only user code an
    /// insert runs. It may grow an index, which runs no user code and leaves
    /// the mapping unchanged; a `Vacant` cell stays valid until its index is
    /// next changed.
    fn locate(&mut self, key: &K, value: &V) -> Located {
        let key_hash = self.forward.hash(key);
        let value_hash = self.inverse.hash(value);
        let pairs = &self.pairs;
        let by_key = self.forward.probe(key_hash, |s| pairs.get(s).key == *key);
        let by_value = self
            .inverse
            .probe(value_hash, |s| pairs.get(s).value == *value);
        Located {
            key_hash,
            value_hash,
            by_key,
            by_value,
        }
    }

    /// Add a pair whose key and value are both absent, into the vacant index
    /// cells [`locate`](Self::locate) found. No user code.
    fn add_at(
        &mut self,
        key: K,
        value: V,
        key_hash: u64,
        value_hash: u64,
        key_cell: usize,
        value_cell: usize,
    ) {
        let slot = self.pairs.push_back(Pair {
            key,
            value,
            key_hash,
            value_hash,
        });
        self.forward.fill_vacant(key_cell, key_hash, slot);
        self.inverse.fill_vacant(value_cell, value_hash, slot);
    }

    /// Add a pair whose key and value are both absent, probing for cells again
    /// (for callers that changed an index since `locate`). No user code.
    fn add_new(&mut self, key: K, value: V, key_hash: u64, value_hash: u64) {
        self.forward.reserve_one();
        self.inverse.reserve_one();
        let slot = self.pairs.push_back(Pair {
            key,
            value,
            key_hash,
            value_hash,
        });
        index_new(&mut self.forward, key_hash, slot);
        index_new(&mut self.inverse, value_hash, slot);
    }

    /// Unlink the pair in `slot` from both indices and the arena, handing it
    /// back for the caller to drop. No user code.
    fn take_slot(&mut self, slot: usize) -> Pair<K, V> {
        let pair = self.pairs.get(slot);
        let (key_hash, value_hash) = (pair.key_hash, pair.value_hash);
        index_remove(&mut self.forward, key_hash, slot);
        index_remove(&mut self.inverse, value_hash, slot);
        self.pairs.unlink_free(slot)
    }

    /// Insert a key-value pair. If the value already exists under a different key,
    /// that old key is removed (bijection invariant). Returns the old value for the
    /// key if it existed.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let at = self.locate(&key, &value);
        let (key_hash, value_hash) = (at.key_hash, at.value_hash);
        // From here on no user code runs until the removed pair is dropped.
        match (at.by_key, at.by_value) {
            (RawEntry::Occupied(slot), RawEntry::Occupied(other)) if slot == other => {
                // The pair is already present: replace the stored value object.
                Some(std::mem::replace(
                    &mut self.pairs.get_mut(slot).value,
                    value,
                ))
            }
            (RawEntry::Occupied(slot), by_value) => {
                // The key keeps its slot and takes the new value; a different
                // key holding that value loses its pair.
                let evicted = match by_value {
                    RawEntry::Occupied(other) => Some(self.take_slot(other)),
                    RawEntry::Vacant(_) => None,
                };
                let old_value_hash = self.pairs.get(slot).value_hash;
                index_remove(&mut self.inverse, old_value_hash, slot);
                index_new(&mut self.inverse, value_hash, slot);
                let pair = self.pairs.get_mut(slot);
                pair.value_hash = value_hash;
                let old = std::mem::replace(&mut pair.value, value);
                drop(evicted);
                Some(old)
            }
            (RawEntry::Vacant(key_cell), RawEntry::Vacant(value_cell)) => {
                self.add_at(key, value, key_hash, value_hash, key_cell, value_cell);
                None
            }
            (RawEntry::Vacant(_), RawEntry::Occupied(other)) => {
                // A different key holds the value: its pair goes, then the new
                // pair is added (the removal moved index cells, so re-probe).
                let evicted = self.take_slot(other);
                self.add_new(key, value, key_hash, value_hash);
                drop(evicted);
                None
            }
        }
    }

    /// Looks up a value by any borrowed form of the key (`K: Borrow<Q>`),
    /// e.g. `bimap.get("str")` on a `HashBiMap<String, _>`.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.slot_of_key(key).map(|s| &self.pairs.get(s).value)
    }

    /// Reverse lookup by any borrowed form of the value (`V: Borrow<Q>`).
    pub fn get_inverse<Q>(&self, value: &Q) -> Option<&K>
    where
        V: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.slot_of_value(value).map(|s| &self.pairs.get(s).key)
    }

    fn slot_of_key<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let pairs = &self.pairs;
        self.forward
            .find(self.forward.hash(key), |s| pairs.get(s).key.borrow() == key)
    }

    fn slot_of_value<Q>(&self, value: &Q) -> Option<usize>
    where
        V: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let pairs = &self.pairs;
        self.inverse.find(self.inverse.hash(value), |s| {
            pairs.get(s).value.borrow() == value
        })
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.slot_of_key(key).is_some()
    }
    pub fn contains_value<Q>(&self, value: &Q) -> bool
    where
        V: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.slot_of_value(value).is_some()
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let slot = self.slot_of_key(key)?;
        Some(self.take_slot(slot).value)
    }

    pub fn remove_inverse<Q>(&mut self, value: &Q) -> Option<K>
    where
        V: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let slot = self.slot_of_value(value)?;
        Some(self.take_slot(slot).key)
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub fn clear(&mut self) {
        self.forward.clear();
        self.inverse.clear();
        // Drop the pairs only once the map is already empty.
        drop(std::mem::take(&mut self.pairs));
    }

    /// Returns a snapshot copy with keys and values swapped.
    pub fn inverse(&self) -> HashBiMap<V, K> {
        HashBiMap {
            pairs: self.pairs.map_structural(|p| Pair {
                key: p.value.clone(),
                value: p.key.clone(),
                key_hash: p.value_hash,
                value_hash: p.key_hash,
            }),
            forward: self.inverse.clone(),
            inverse: self.forward.clone(),
        }
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.pairs.iter(),
        }
    }
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.pairs.iter().map(|p| &p.key)
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.pairs.iter().map(|p| &p.value)
    }

    pub fn for_each(&self, mut f: impl FnMut(&K, &V)) {
        for p in self.pairs.iter() {
            f(&p.key, &p.value);
        }
    }
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> Default for HashBiMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> fmt::Debug for HashBiMap<K, V>
where
    K: Eq + Hash + Clone + fmt::Debug,
    V: Eq + Hash + Clone + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Shared-reference iterator over `(&K, &V)` in insertion order.
pub struct Iter<'a, K, V> {
    inner: slot_list::Iter<'a, Pair<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|p| (&p.key, &p.value))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}
impl<K, V> std::iter::FusedIterator for Iter<'_, K, V> {}

/// Owned iterator over `(K, V)` in insertion order.
pub struct IntoIter<K, V> {
    inner: slot_list::IntoIter<Pair<K, V>>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|p| (p.key, p.value))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {}
impl<K, V> std::iter::FusedIterator for IntoIter<K, V> {}

// ---- idiomatic std-style additions ----------------------------------------

impl<'a, K: Eq + Hash + Clone, V: Eq + Hash + Clone> IntoIterator for &'a HashBiMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> IntoIterator for HashBiMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.pairs.into_iter(),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> FromIterator<(K, V)> for HashBiMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut m = Self::new();
        m.extend(iter);
        m
    }
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> Extend<(K, V)> for HashBiMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

/// Order-insensitive equality on the forward mapping (the bijection invariant
/// makes the inverse follow).
impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> PartialEq for HashBiMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> Eq for HashBiMap<K, V> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrow_lookup_str_on_string_key() {
        let mut bm: HashBiMap<String, i32> = HashBiMap::new();
        bm.insert("alpha".to_string(), 1);
        // get/contains_key/remove accept a borrowed form of the key (&str)
        assert_eq!(bm.get("alpha"), Some(&1));
        assert!(bm.contains_key("alpha"));
        assert!(!bm.contains_key("missing"));
        assert_eq!(bm.remove("alpha"), Some(1));
        assert!(bm.get("alpha").is_none());
    }

    #[test]
    fn test_basic() {
        let mut bm = HashBiMap::new();
        assert_eq!(bm.insert("a", 1), None);
        assert_eq!(bm.insert("b", 2), None);
        assert_eq!(bm.get(&"a"), Some(&1));
        assert_eq!(bm.get_inverse(&2), Some(&"b"));
        assert_eq!(bm.len(), 2);
    }

    #[test]
    fn test_bijection_enforcement() {
        let mut bm = HashBiMap::new();
        bm.insert("a", 1);
        bm.insert("b", 2);
        // Insert value 1 under key "c" — should remove "a"
        bm.insert("c", 1);
        assert!(!bm.contains_key(&"a"));
        assert_eq!(bm.get(&"c"), Some(&1));
        assert_eq!(bm.get_inverse(&1), Some(&"c"));
        assert_eq!(bm.len(), 2);
    }

    #[test]
    fn test_overwrite_same_key() {
        let mut bm = HashBiMap::new();
        bm.insert("a", 1);
        let old = bm.insert("a", 2);
        assert_eq!(old, Some(1));
        assert_eq!(bm.get(&"a"), Some(&2));
        assert!(!bm.contains_value(&1));
        assert!(bm.contains_value(&2));
    }

    #[test]
    fn test_remove_and_inverse() {
        let mut bm = HashBiMap::new();
        bm.insert("x", 10);
        bm.insert("y", 20);
        assert_eq!(bm.remove(&"x"), Some(10));
        assert!(!bm.contains_value(&10));
        assert_eq!(bm.remove_inverse(&20), Some("y"));
        assert!(bm.is_empty());
    }

    #[test]
    fn test_inverse_snapshot() {
        let mut bm = HashBiMap::new();
        bm.insert("a", 1);
        bm.insert("b", 2);
        let inv = bm.inverse();
        assert_eq!(inv.get(&1), Some(&"a"));
        assert_eq!(inv.get(&2), Some(&"b"));
    }

    #[test]
    fn test_into_iter_and_collect() {
        let bm: HashBiMap<&str, i32> = [("a", 1), ("b", 2)].into_iter().collect();
        let sum: i32 = (&bm).into_iter().map(|(_, v)| *v).sum();
        assert_eq!(sum, 3);
        let owned: i32 = bm.into_iter().map(|(_, v)| v).sum();
        assert_eq!(owned, 3);
    }

    #[test]
    fn test_extend_and_eq() {
        let mut bm = HashBiMap::new();
        bm.extend([("a", 1), ("b", 2)]);
        let other: HashBiMap<&str, i32> = [("b", 2), ("a", 1)].into_iter().collect();
        assert_eq!(bm, other);
    }

    #[test]
    fn bulk_load_bijection_equal_incremental() {
        let data = [("a", 1), ("b", 2), ("c", 3)];
        let bulk = HashBiMap::bulk_load(data, DuplicatePolicy::Error).unwrap();
        let mut inc = HashBiMap::new();
        for (k, v) in data {
            inc.insert(k, v);
        }
        assert_eq!(bulk, inc);
        assert_eq!(bulk.get_inverse(&2), Some(&"b"));
    }

    #[test]
    fn bulk_load_rejects_duplicate_key_and_value() {
        // duplicate key at index 2
        let err = HashBiMap::bulk_load([("a", 1), ("b", 2), ("a", 3)], DuplicatePolicy::Error)
            .unwrap_err();
        assert!(matches!(err, BulkError::Duplicate { index: 2 }));
        // duplicate value at index 1
        let err = HashBiMap::bulk_load([("a", 1), ("b", 1)], DuplicatePolicy::Error).unwrap_err();
        assert!(matches!(err, BulkError::Duplicate { index: 1 }));
    }

    #[test]
    fn bulk_load_ignores_policy_every_duplicate_errors() {
        // BiMap ignores DuplicatePolicy: an identical (k,v) pair is an error
        // under IgnoreDuplicates too (the repeated key breaks the bijection
        // build).
        let err = HashBiMap::bulk_load(
            [("a", 1), ("a", 1), ("b", 2)],
            DuplicatePolicy::IgnoreDuplicates,
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::Duplicate { index: 1 }));

        // A partial collision (same key, new value) likewise errors.
        let err = HashBiMap::bulk_load([("a", 1), ("a", 9)], DuplicatePolicy::IgnoreDuplicates)
            .unwrap_err();
        assert!(matches!(err, BulkError::Duplicate { index: 1 }));

        // A duplicate value under IgnoreDuplicates also errors.
        let err = HashBiMap::bulk_load([("a", 1), ("b", 1)], DuplicatePolicy::IgnoreDuplicates)
            .unwrap_err();
        assert!(matches!(err, BulkError::Duplicate { index: 1 }));
    }

    // ---- panic consistency --------------------------------------------------

    use std::cell::Cell;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    thread_local! {
        /// User `Hash`/`Eq` calls left before one panics; negative = disarmed.
        static COUNTDOWN: Cell<i64> = const { Cell::new(-1) };
        /// While set, dropping a `Tick` holding this id panics.
        static DROP_BOMB: Cell<Option<i32>> = const { Cell::new(None) };
    }

    fn tick() {
        COUNTDOWN.with(|c| {
            let n = c.get();
            if n == 0 {
                c.set(-1);
                panic!("armed Hash/Eq");
            }
            if n > 0 {
                c.set(n - 1);
            }
        });
    }

    /// Key/value whose `Hash` and `Eq` count down to a panic, and whose `Drop`
    /// can be armed to panic.
    #[derive(Clone, Debug)]
    struct Tick(i32);

    impl PartialEq for Tick {
        fn eq(&self, other: &Self) -> bool {
            tick();
            self.0 == other.0
        }
    }
    impl Eq for Tick {}
    impl Hash for Tick {
        fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
            tick();
            // A few buckets, so probes compare several candidates.
            (self.0 % 3).hash(h);
        }
    }
    impl Drop for Tick {
        fn drop(&mut self) {
            if DROP_BOMB.with(|b| b.get()) == Some(self.0) && !std::thread::panicking() {
                DROP_BOMB.with(|b| b.set(None));
                panic!("armed Drop");
            }
        }
    }

    fn disarm() {
        COUNTDOWN.with(|c| c.set(-1));
        DROP_BOMB.with(|b| b.set(None));
    }

    fn pairs_of(bm: &HashBiMap<Tick, Tick>) -> Vec<(i32, i32)> {
        let mut out: Vec<_> = bm.iter().map(|(k, v)| (k.0, v.0)).collect();
        out.sort();
        out
    }

    /// Forward and inverse describe the same bijection: every pair is found
    /// both ways, keys and values are unique, and both indices hold `len`.
    fn assert_consistent(bm: &HashBiMap<Tick, Tick>) {
        let pairs = pairs_of(bm);
        assert_eq!(pairs.len(), bm.len());
        assert_eq!(bm.forward.len(), bm.len(), "forward index size");
        assert_eq!(bm.inverse.len(), bm.len(), "inverse index size");
        let mut keys: Vec<i32> = pairs.iter().map(|p| p.0).collect();
        let mut values: Vec<i32> = pairs.iter().map(|p| p.1).collect();
        keys.dedup();
        values.sort();
        values.dedup();
        assert_eq!(keys.len(), pairs.len(), "duplicate key");
        assert_eq!(values.len(), pairs.len(), "duplicate value");
        for &(k, v) in &pairs {
            assert_eq!(bm.get(&Tick(k)).map(|t| t.0), Some(v), "forward {k}");
            assert_eq!(
                bm.get_inverse(&Tick(v)).map(|t| t.0),
                Some(k),
                "inverse {v}"
            );
        }
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self, n: u64) -> i32 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) % n) as i32
        }
    }

    /// A panicking `Hash` or `Eq` at any point of any mutator leaves the
    /// mapping exactly as it was, with forward and inverse in agreement.
    #[test]
    fn hash_or_eq_panic_leaves_map_unchanged_and_consistent() {
        let mut rng = Rng(0x5eed);
        let mut panics = 0;
        for _ in 0..40 {
            let mut bm: HashBiMap<Tick, Tick> = HashBiMap::new();
            for _ in 0..300 {
                let (k, v) = (rng.next(24), rng.next(24));
                let op = rng.next(4);
                let before = pairs_of(&bm);
                COUNTDOWN.with(|c| c.set(rng.next(12) as i64));
                let r = catch_unwind(AssertUnwindSafe(|| match op {
                    0 | 1 => {
                        bm.insert(Tick(k), Tick(v));
                    }
                    2 => {
                        bm.remove(&Tick(k));
                    }
                    _ => {
                        bm.remove_inverse(&Tick(v));
                    }
                }));
                disarm();
                if r.is_err() {
                    panics += 1;
                    assert_eq!(pairs_of(&bm), before, "op {op} ({k}, {v}) changed the map");
                }
                assert_consistent(&bm);
            }
        }
        assert!(panics > 1000, "the countdown fired ({panics})");
    }

    /// An `Eq` panic in the probe right after `locate` grew the forward index:
    /// the index is larger, the mapping is unchanged and still found both ways.
    #[test]
    fn eq_panic_after_index_growth_leaves_mapping_unchanged() {
        let mut bm: HashBiMap<Tick, Tick> = HashBiMap::new();
        let mut i = 0;
        // Fill to the growth boundary: the next insert's probe resizes first.
        while !bm.forward.needs_resize() {
            bm.insert(Tick(i), Tick(100 + i));
            i += 1;
        }
        let before = pairs_of(&bm);
        let cap = bm.forward.allocated_slots();
        // Tick(i + 3) hashes like Tick(i), so its probe compares keys with Eq.
        // Two ticks hash the key and the value; the third, the first Eq,
        // panics.
        COUNTDOWN.with(|c| c.set(2));
        let r = catch_unwind(AssertUnwindSafe(|| bm.insert(Tick(i + 3), Tick(999))));
        disarm();
        assert!(r.is_err(), "the Eq panicked");
        assert!(bm.forward.allocated_slots() > cap, "the forward index grew");
        assert_eq!(pairs_of(&bm), before);
        assert_consistent(&bm);
    }

    /// A panicking `Drop` of a displaced pair fires after both indices are
    /// updated: the map is in its new state and consistent.
    #[test]
    fn drop_panic_of_displaced_pair_leaves_new_state() {
        let fresh = || {
            let mut bm: HashBiMap<Tick, Tick> = HashBiMap::new();
            for i in 0..6 {
                bm.insert(Tick(i), Tick(100 + i));
            }
            bm
        };
        type Op = fn(&mut HashBiMap<Tick, Tick>);
        type Case = (Op, i32, Vec<(i32, i32)>);
        // (op, dropped id, expected pairs afterwards)
        let cases: [Case; 5] = [
            // value 101 moves from key 1 to key 0: pair (1, 101) is dropped.
            (
                |bm| drop(bm.insert(Tick(0), Tick(101))),
                1,
                vec![(0, 101), (2, 102), (3, 103), (4, 104), (5, 105)],
            ),
            // new key 9 takes value 102 from key 2.
            (
                |bm| drop(bm.insert(Tick(9), Tick(102))),
                2,
                vec![(0, 100), (1, 101), (3, 103), (4, 104), (5, 105), (9, 102)],
            ),
            (
                |bm| drop(bm.remove(&Tick(3))),
                3,
                vec![(0, 100), (1, 101), (2, 102), (4, 104), (5, 105)],
            ),
            (
                |bm| drop(bm.remove_inverse(&Tick(104))),
                4,
                vec![(0, 100), (1, 101), (2, 102), (3, 103), (5, 105)],
            ),
            (|bm| bm.clear(), 5, vec![]),
        ];
        for (op, id, expected) in cases {
            let mut bm = fresh();
            DROP_BOMB.with(|b| b.set(Some(id)));
            let r = catch_unwind(AssertUnwindSafe(|| op(&mut bm)));
            disarm();
            assert!(r.is_err(), "drop of {id} panicked");
            assert_eq!(pairs_of(&bm), expected);
            assert_consistent(&bm);
        }
    }

    #[test]
    fn inverse_snapshot_is_consistent_after_churn() {
        let mut rng = Rng(7);
        let mut bm: HashBiMap<Tick, Tick> = HashBiMap::new();
        for _ in 0..500 {
            let (k, v) = (rng.next(40), rng.next(40));
            if rng.next(3) == 0 {
                bm.remove(&Tick(k));
            } else {
                bm.insert(Tick(k), Tick(v));
            }
        }
        let inv = bm.inverse();
        let mut swapped: Vec<(i32, i32)> = pairs_of(&bm).into_iter().map(|(k, v)| (v, k)).collect();
        swapped.sort();
        assert_eq!(pairs_of(&inv), swapped);
        assert_consistent(&inv);
        assert_consistent(&bm);
    }
}
