// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Insertion-ordered map (`LinkedHashMap`) over an intrusive slot arena.
//!
//! Rebuilt for v3 (blueprint doc 14 §5, the "T9 keystone"). The old design
//! stored every key **twice** — once in a `Vec<(K, V)>` and once in a
//! `std::HashMap<K, usize>` — which forced a `K: Clone` bound everywhere and
//! made `remove` an O(n) index-fix-up sweep. The new design keeps each entry
//! exactly once in a [`SlotList`] arena and indexes it with a key-owning-free
//! [`IndexTable`]:
//!
//! - **`remove` is O(1)** — unlink the slot and free it; no index fix-ups ever,
//!   because slot indices are stable (they never shift).
//! - **No `K: Clone`** — the key lives in the arena; the index stores only its
//!   hash and slot number.
//! - **`Borrow<Q>` lookups** — `get`/`remove`/`contains_key` accept any borrowed
//!   form of the key, like `std` maps.
//!
//! Iteration follows the arena's order list (insertion order); updating an
//! existing key preserves its position, only new keys are appended. Java's
//! `LinkedHashMap` makes the same pointer-chasing-vs-dense-scan trade.

use super::traits::*;
use crate::index_table::{IndexTable, RawEntry};
use crate::slot_list::{self, SlotList};
use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::{BuildHasher, Hash};

/// Insertion-ordered map from `K` to `V`. The hasher `S` defaults to
/// [`RandomState`]; the arena keeps keys once, so there is no `K: Clone` bound.
pub struct LinkedHashMap<K, V, S = RandomState> {
    /// Entries in insertion order; the sole owner of every key and value.
    slots: SlotList<(K, V)>,
    /// key-hash → arena slot index. Owns no keys.
    index: IndexTable<S>,
}

impl<K: Eq + Hash, V> LinkedHashMap<K, V, RandomState> {
    /// An empty map with the default hasher.
    pub fn new() -> Self {
        LinkedHashMap {
            slots: SlotList::new(),
            index: IndexTable::new(),
        }
    }

    /// An empty map with room reserved for `cap` entries.
    pub fn with_capacity(cap: usize) -> Self {
        LinkedHashMap {
            slots: SlotList::with_capacity(cap),
            index: IndexTable::with_capacity(cap),
        }
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> LinkedHashMap<K, V, S> {
    /// An empty map that hashes keys with `hasher`.
    pub fn with_hasher(hasher: S) -> Self {
        LinkedHashMap {
            slots: SlotList::new(),
            index: IndexTable::with_hasher(hasher),
        }
    }

    /// An empty map with reserved capacity that hashes keys with `hasher`.
    pub fn with_capacity_and_hasher(cap: usize, hasher: S) -> Self {
        LinkedHashMap {
            slots: SlotList::with_capacity(cap),
            index: IndexTable::with_capacity_and_hasher(cap, hasher),
        }
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Whether `key` is present. Accepts any borrowed form of the key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.find_slot(key).is_some()
    }

    /// A reference to the value for `key`, or `None`.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let slot = self.find_slot(key)?;
        Some(&self.slots.get(slot).1)
    }

    /// A mutable reference to the value for `key`, or `None`. The key is not
    /// exposed, so the hash index stays consistent.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let slot = self.find_slot(key)?;
        Some(&mut self.slots.get_mut(slot).1)
    }

    /// Locate the arena slot for `key` through the index. The `eq` closure and
    /// `find` borrow disjoint fields (`slots` vs `index`), so this type-checks
    /// without splitting the struct.
    #[inline]
    fn find_slot<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = self.index.hash(key);
        let slots = &self.slots;
        self.index.find(hash, |s| slots.get(s).0.borrow() == key)
    }

    /// Insert or update `key`. A new key is appended (insertion order); updating
    /// an existing key replaces the value and keeps its position. Returns the
    /// previous value, if any. Single probe — no contains-then-insert.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = self.index.hash(&key);
        let slots = &self.slots;
        match self.index.probe(hash, |s| slots.get(s).0 == key) {
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

    /// Remove `key`, returning its value. O(1): unlink the slot and recycle it;
    /// the positions of all other entries are unchanged. Accepts any borrowed
    /// form of the key.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = self.index.hash(key);
        let slots = &self.slots;
        let slot = self
            .index
            .remove(hash, |s| slots.get(s).0.borrow() == key)?;
        let (_k, v) = self.slots.unlink_free(slot);
        Some(v)
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        // Clear the index first: should a value's `Drop` panic while the arena
        // is being cleared, the index is already empty and can never reference a
        // half-cleared arena (worst case stays a safe-Rust panic, never UB).
        self.index.clear();
        self.slots.clear();
    }

    /// Iterate `(&K, &V)` in insertion order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.slots.iter(),
        }
    }

    /// Iterate `(&K, &mut V)` in insertion order. Keys stay immutable, so the
    /// index and ordering invariants are preserved.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            inner: self.slots.iter_mut(),
        }
    }

    /// Iterate keys in insertion order.
    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.iter().map(|(k, _)| k)
    }

    /// Iterate values in insertion order.
    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.iter().map(|(_, v)| v)
    }
}

// ---- borrow-free convenience/functional methods (no `K: Clone`) ------------

impl<K: Eq + Hash, V, S: BuildHasher> LinkedHashMap<K, V, S> {
    /// Keys in insertion order, as references.
    pub fn keys_to_vec(&self) -> Vec<&K> {
        self.iter().map(|(k, _)| k).collect()
    }

    /// Values in insertion order, as references.
    pub fn values_to_vec(&self) -> Vec<&V> {
        self.iter().map(|(_, v)| v).collect()
    }

    /// Whether any value equals `value`.
    pub fn contains_value(&self, value: &V) -> bool
    where
        V: PartialEq,
    {
        self.iter().any(|(_, v)| v == value)
    }

    /// Count entries matching `predicate`.
    pub fn count_where(&self, predicate: impl Fn(&K, &V) -> bool) -> usize {
        self.iter().filter(|(k, v)| predicate(k, v)).count()
    }

    /// The first entry (insertion order) matching `predicate`.
    pub fn detect(&self, predicate: impl Fn(&K, &V) -> bool) -> Option<(&K, &V)> {
        self.iter().find(|(k, v)| predicate(k, v))
    }
}

// ---- select/reject: build a new owned map (needs Clone) --------------------

impl<K: Eq + Hash + Clone, V: Clone, S: BuildHasher + Default> LinkedHashMap<K, V, S> {
    /// A new map of the entries matching `predicate` (insertion order kept).
    pub fn select(&self, predicate: impl Fn(&K, &V) -> bool) -> Self {
        let mut result = Self::with_hasher(S::default());
        for (k, v) in self.iter() {
            if predicate(k, v) {
                result.insert(k.clone(), v.clone());
            }
        }
        result
    }

    /// A new map of the entries *not* matching `predicate` (order kept).
    pub fn reject(&self, predicate: impl Fn(&K, &V) -> bool) -> Self {
        let mut result = Self::with_hasher(S::default());
        for (k, v) in self.iter() {
            if !predicate(k, v) {
                result.insert(k.clone(), v.clone());
            }
        }
        result
    }
}

// ---- trait-tower impls (Stage-C deletion targets; kept working for now) ----

impl<K: Eq + Hash, V, S: BuildHasher> MapIterable<K, V> for LinkedHashMap<K, V, S> {
    fn len(&self) -> usize {
        self.len()
    }

    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }

    fn get(&self, key: &K) -> Option<&V> {
        self.get(key)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (&K, &V)> + '_> {
        Box::new(self.iter())
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> MutableMap<K, V> for LinkedHashMap<K, V, S> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.insert(key, value)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self.remove(key)
    }

    fn clear(&mut self) {
        self.clear()
    }
}

// ---- std-style trait impls -------------------------------------------------

impl<K: Eq + Hash, V, S: BuildHasher + Default> Default for LinkedHashMap<K, V, S> {
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K, V, S> Clone for LinkedHashMap<K, V, S>
where
    K: Clone,
    V: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        // Structural: `SlotList`'s clone preserves slot indices, so the verbatim
        // `IndexTable` clone stays consistent with it.
        LinkedHashMap {
            slots: self.slots.clone(),
            index: self.index.clone(),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for LinkedHashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: Eq + Hash, V, S: BuildHasher + Default> FromIterator<(K, V)> for LinkedHashMap<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut m = Self::default();
        m.extend(iter);
        m
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> Extend<(K, V)> for LinkedHashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

/// Order-insensitive map equality: same keys mapping to equal values.
impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for LinkedHashMap<K, V, S> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for LinkedHashMap<K, V, S> {}

// ---- iterators -------------------------------------------------------------

/// Shared-reference iterator over `(&K, &V)` in insertion order.
pub struct Iter<'a, K, V> {
    inner: slot_list::Iter<'a, (K, V)>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (k, v))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}
impl<K, V> std::iter::FusedIterator for Iter<'_, K, V> {}

/// Mutable-reference iterator over `(&K, &mut V)` in insertion order.
pub struct IterMut<'a, K, V> {
    inner: slot_list::IterMut<'a, (K, V)>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|kv| {
            let (k, v) = kv;
            (&*k, v)
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {}
impl<K, V> std::iter::FusedIterator for IterMut<'_, K, V> {}

/// Owned iterator over `(K, V)` in insertion order.
pub struct IntoIter<K, V> {
    inner: slot_list::IntoIter<(K, V)>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {}
impl<K, V> std::iter::FusedIterator for IntoIter<K, V> {}

impl<'a, K: Eq + Hash, V, S: BuildHasher> IntoIterator for &'a LinkedHashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K: Eq + Hash, V, S: BuildHasher> IntoIterator for &'a mut LinkedHashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<K, V, S> IntoIterator for LinkedHashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.slots.into_iter(),
        }
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

    #[test]
    fn test_into_iter_ref_mut_owned() {
        let mut m = LinkedHashMap::new();
        m.insert("a", 1);
        m.insert("b", 2);
        let pairs: Vec<(&&str, &i32)> = (&m).into_iter().collect();
        assert_eq!(pairs, vec![(&"a", &1), (&"b", &2)]);
        for (_k, v) in &mut m {
            *v *= 10;
        }
        let owned: Vec<(&str, i32)> = m.into_iter().collect();
        assert_eq!(owned, vec![("a", 10), ("b", 20)]);
    }

    #[test]
    fn test_from_iterator_and_extend() {
        let mut m: LinkedHashMap<&str, i32> = [("a", 1), ("b", 2)].into_iter().collect();
        assert_eq!(m.len(), 2);
        m.extend([("c", 3)]);
        let keys = m.keys_to_vec();
        assert_eq!(keys, vec![&"a", &"b", &"c"]);
    }

    #[test]
    fn test_partial_eq_order_insensitive() {
        let mut a = LinkedHashMap::new();
        a.insert(1, 10);
        a.insert(2, 20);
        let mut b = LinkedHashMap::new();
        b.insert(2, 20);
        b.insert(1, 10);
        assert_eq!(a, b);
    }

    // ---- v3 additions --------------------------------------------------

    #[test]
    fn get_mut_updates_value_in_place() {
        let mut m = LinkedHashMap::new();
        m.insert("a", 1);
        m.insert("b", 2);
        *m.get_mut(&"b").unwrap() += 100;
        assert_eq!(m.get(&"b"), Some(&102));
        // order untouched
        assert_eq!(m.keys_to_vec(), vec![&"a", &"b"]);
    }

    #[test]
    fn borrow_lookup_with_str() {
        // Key is `String`; lookups accept `&str` (no `K: Clone`, no allocation).
        let mut m: LinkedHashMap<String, i32> = LinkedHashMap::new();
        m.insert("hello".to_string(), 1);
        m.insert("world".to_string(), 2);
        assert_eq!(m.get("hello"), Some(&1));
        assert!(m.contains_key("world"));
        assert_eq!(m.remove("hello"), Some(1));
        assert!(!m.contains_key("hello"));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn non_clone_key_and_value_work() {
        // A key/value that is NOT `Clone` — impossible with the old design.
        struct NoClone(i32);
        impl PartialEq for NoClone {
            fn eq(&self, o: &Self) -> bool {
                self.0 == o.0
            }
        }
        impl Eq for NoClone {}
        impl Hash for NoClone {
            fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
                self.0.hash(h);
            }
        }
        let mut m: LinkedHashMap<NoClone, NoClone> = LinkedHashMap::new();
        m.insert(NoClone(1), NoClone(10));
        m.insert(NoClone(2), NoClone(20));
        assert_eq!(m.get(&NoClone(1)).map(|v| v.0), Some(10));
        assert_eq!(m.remove(&NoClone(1)).map(|v| v.0), Some(10));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn remove_is_o1_and_slots_recycle() {
        // Heavy churn: many inserts/removes must not grow storage without bound
        // and must keep insertion order among survivors.
        let mut m: LinkedHashMap<i32, i32> = LinkedHashMap::new();
        for k in 0..1000 {
            m.insert(k, k * 2);
        }
        for k in 0..1000 {
            if k % 2 == 0 {
                assert_eq!(m.remove(&k), Some(k * 2));
            }
        }
        assert_eq!(m.len(), 500);
        let expected: Vec<i32> = (0..1000).filter(|k| k % 2 == 1).collect();
        let got: Vec<i32> = m.keys().copied().collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn clone_is_consistent_after_clone() {
        let mut m = LinkedHashMap::new();
        m.insert(1, 10);
        m.insert(2, 20);
        m.insert(3, 30);
        m.remove(&2); // leaves a hole
        let c = m.clone();
        // Clone must resolve every surviving key correctly (index/slot in sync).
        assert_eq!(c.get(&1), Some(&10));
        assert_eq!(c.get(&3), Some(&30));
        assert_eq!(c.get(&2), None);
        assert_eq!(c.keys().copied().collect::<Vec<_>>(), vec![1, 3]);
        // Mutating the clone does not touch the original.
        let mut c2 = c.clone();
        c2.insert(4, 40);
        assert_eq!(m.len(), 2);
        assert_eq!(c2.len(), 3);
    }

    /// Differential fuzz: the rebuilt map must match a naive `Vec`-ordered
    /// reference model across a long randomized op stream.
    #[test]
    fn differential_against_reference_model() {
        // Reference: insertion-ordered Vec of (k, v); linear ops.
        fn ref_insert(model: &mut Vec<(i32, i32)>, k: i32, v: i32) -> Option<i32> {
            if let Some(e) = model.iter_mut().find(|(ek, _)| *ek == k) {
                Some(std::mem::replace(&mut e.1, v))
            } else {
                model.push((k, v));
                None
            }
        }
        fn ref_remove(model: &mut Vec<(i32, i32)>, k: i32) -> Option<i32> {
            let pos = model.iter().position(|(ek, _)| *ek == k)?;
            Some(model.remove(pos).1)
        }

        let mut map: LinkedHashMap<i32, i32> = LinkedHashMap::new();
        let mut model: Vec<(i32, i32)> = Vec::new();
        let mut state: u64 = 0xDEAD_BEEF_0BAD_F00D;
        for _ in 0..20_000 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let k = ((state >> 33) % 64) as i32;
            let v = ((state >> 20) & 0xFFFF) as i32;
            match (state >> 30) & 3 {
                0 | 1 => assert_eq!(map.insert(k, v), ref_insert(&mut model, k, v)),
                2 => assert_eq!(map.remove(&k), ref_remove(&mut model, k)),
                _ => assert_eq!(
                    map.get(&k),
                    model.iter().find(|(ek, _)| *ek == k).map(|(_, v)| v)
                ),
            }
            assert_eq!(map.len(), model.len());
        }
        // Full order + content agreement at the end.
        let map_pairs: Vec<(i32, i32)> = map.iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(map_pairs, model);
    }
}
