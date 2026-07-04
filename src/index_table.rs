// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Raw open-addressing index (`IndexTable<S>`): a hash table whose entries are
//! `(hash, slot)` pairs rather than owned keys.
//!
//! It is the "raw-table variant of the kernel" that blueprint doc 14 §5 calls
//! for: the same linear-probing + Robin-Hood backward-shift-deletion algorithm
//! as [`crate::hash_table::OpenHashMap`], but the table owns **no keys**. The
//! keys live elsewhere (a [`crate::slot_list::SlotList`] arena); every lookup
//! takes the query's precomputed `hash` plus an `eq(slot) -> bool` closure that
//! the caller implements against that external storage. This lets an
//! insertion-ordered map keep its keys exactly once (killing the `K: Clone`
//! double-storage of the old `Vec`+`HashMap` `LinkedHashMap`).
//!
//! ## Why store the hash inline
//!
//! Each occupied table cell stores the key's full 64-bit `hash` beside the slot
//! index. Two consequences, both deliberate:
//!
//! 1. **Backward-shift never calls user code.** Robin-Hood displacement needs
//!    each shifted entry's *ideal* position, i.e. its hash. Because the hash is
//!    stored, deletion re-derives it locally instead of calling the caller's
//!    hash/eq — so a panic in a user `Hash`/`Eq` impl can occur only during the
//!    read-only probe *before* any mutation, never mid-shift (hardening item (a)
//!    and (b) from doc 14 §5). Resize likewise rehashes from stored hashes.
//! 2. **Probing rejects most mismatches without the `eq` closure**, comparing a
//!    machine word first and only calling `eq` on a full-hash hit.
//!
//! ## Invariants
//!
//! - Capacity is a power of two ≥ `DEFAULT_CAPACITY`; `mask == cap - 1`.
//! - `len` occupied cells, kept strictly below the 0.75 load factor by resizing
//!   before each new insert (identical policy and formula to `OpenHashMap`, so
//!   the brute-forced capacity tests transfer).
//! - An occupied run is contiguous under linear probing; backward-shift
//!   preserves that after a removal.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};

const DEFAULT_CAPACITY: usize = 16;
const LOAD_FACTOR_NUM: usize = 3;
const LOAD_FACTOR_DEN: usize = 4; // 0.75

/// One table cell: empty, or an occupied `(hash, slot)` pair.
#[derive(Clone)]
enum IdxSlot {
    Empty,
    Full { hash: u64, slot: usize },
}

/// The result of probing for a key: either it is present (its external slot
/// index), or absent (the empty table cell where it would be inserted).
pub(crate) enum RawEntry {
    Occupied(usize),
    Vacant(usize),
}

/// A key-owning-free open-addressing index from key-hash to an external slot
/// index.
pub(crate) struct IndexTable<S = RandomState> {
    slots: Vec<IdxSlot>,
    len: usize,
    hasher: S,
}

impl<S: Clone> Clone for IndexTable<S> {
    fn clone(&self) -> Self {
        IndexTable {
            slots: self.slots.clone(),
            len: self.len,
            hasher: self.hasher.clone(),
        }
    }
}

impl IndexTable<RandomState> {
    /// A new empty index using the default (`RandomState`) hasher.
    pub(crate) fn new() -> Self {
        Self::with_hasher(RandomState::new())
    }

    /// A new empty index sized for `cap` items, default hasher.
    pub(crate) fn with_capacity(cap: usize) -> Self {
        Self::with_capacity_and_hasher(cap, RandomState::new())
    }
}

impl<S> IndexTable<S> {
    /// A new empty index using `hasher`.
    pub(crate) fn with_hasher(hasher: S) -> Self {
        Self::with_capacity_and_hasher(DEFAULT_CAPACITY, hasher)
    }

    /// A new empty index sized for `cap` items, using `hasher`.
    pub(crate) fn with_capacity_and_hasher(cap: usize, hasher: S) -> Self {
        let cap = cap.max(DEFAULT_CAPACITY).next_power_of_two();
        let mut slots = Vec::with_capacity(cap);
        slots.resize_with(cap, || IdxSlot::Empty);
        IndexTable {
            slots,
            len: 0,
            hasher,
        }
    }

    #[inline]
    fn cap(&self) -> usize {
        self.slots.len()
    }

    #[inline]
    fn mask(&self) -> usize {
        self.slots.len() - 1
    }

    /// Occupied-cell count. Should equal the owning structure's live-entry
    /// count; exposed for tests and invariant checks.
    #[cfg(test)]
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Resolve a resize before inserting, matching `OpenHashMap`'s policy: grow
    /// when the *next* insert would reach the 0.75 load factor.
    #[inline]
    fn needs_resize(&self) -> bool {
        (self.len + 1) * LOAD_FACTOR_DEN >= self.cap() * LOAD_FACTOR_NUM
    }

    /// Empty every cell (keeps the current capacity).
    pub(crate) fn clear(&mut self) {
        for s in &mut self.slots {
            *s = IdxSlot::Empty;
        }
        self.len = 0;
    }

    /// Find the external slot for a key already hashed to `hash`, using `eq` to
    /// confirm a full-hash hit against the caller's storage. Read-only.
    pub(crate) fn find(&self, hash: u64, mut eq: impl FnMut(usize) -> bool) -> Option<usize> {
        let mask = self.mask();
        let mut idx = (hash as usize) & mask;
        loop {
            match &self.slots[idx] {
                IdxSlot::Empty => return None,
                IdxSlot::Full { hash: h, slot } if *h == hash && eq(*slot) => return Some(*slot),
                IdxSlot::Full { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    /// Probe for `hash`, resolving a resize first so the returned `Vacant`
    /// cell index stays valid until [`fill_vacant`](Self::fill_vacant). On an
    /// `Occupied` hit returns the external slot; on a miss returns the empty
    /// cell where the key belongs.
    ///
    /// Resize happens *before* the probe (as `OpenHashMap::insert` does), so the
    /// growth schedule — and the capacity tests that pin it — are unchanged.
    pub(crate) fn probe(&mut self, hash: u64, mut eq: impl FnMut(usize) -> bool) -> RawEntry {
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.mask();
        let mut idx = (hash as usize) & mask;
        loop {
            match &self.slots[idx] {
                IdxSlot::Empty => return RawEntry::Vacant(idx),
                IdxSlot::Full { hash: h, slot } if *h == hash && eq(*slot) => {
                    return RawEntry::Occupied(*slot)
                }
                IdxSlot::Full { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    /// Fill a `Vacant` cell (from [`probe`](Self::probe)) with a fresh mapping.
    /// The caller must not have mutated the table between the probe and this
    /// call (no user code runs in between, so this holds by construction in the
    /// `&mut self` window).
    pub(crate) fn fill_vacant(&mut self, cell: usize, hash: u64, slot: usize) {
        debug_assert!(matches!(self.slots[cell], IdxSlot::Empty));
        self.slots[cell] = IdxSlot::Full { hash, slot };
        self.len += 1;
    }

    /// Remove the mapping for a key hashed to `hash` (confirmed by `eq`),
    /// returning its external slot index. Deletion is Robin-Hood backward-shift
    /// driven entirely by stored hashes — no user code runs during the shift.
    pub(crate) fn remove(&mut self, hash: u64, mut eq: impl FnMut(usize) -> bool) -> Option<usize> {
        let mask = self.mask();
        let mut idx = (hash as usize) & mask;
        loop {
            match &self.slots[idx] {
                IdxSlot::Empty => return None,
                IdxSlot::Full { hash: h, slot } if *h == hash && eq(*slot) => {
                    let removed = *slot;
                    self.slots[idx] = IdxSlot::Empty;
                    self.len -= 1;
                    self.backward_shift(idx);
                    return Some(removed);
                }
                IdxSlot::Full { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    /// Rewrite the external slot index stored for an existing key. Used when the
    /// key survives but its arena slot moves; a no-op here because slots are
    /// stable, but exposed for completeness of the primitive.
    #[cfg(test)]
    pub(crate) fn contains(&self, hash: u64, eq: impl FnMut(usize) -> bool) -> bool {
        self.find(hash, eq).is_some()
    }

    /// Robin-Hood backward-shift from a just-emptied cell `deleted`, using the
    /// stored hash of each candidate to compute its ideal position. Identical in
    /// shape to `OpenHashMap::rehash_from`.
    fn backward_shift(&mut self, deleted: usize) {
        let mask = self.mask();
        let mut gap = deleted;
        let mut idx = (deleted + 1) & mask;
        while let IdxSlot::Full { hash, .. } = &self.slots[idx] {
            let ideal = (*hash as usize) & mask;
            let dist_current = idx.wrapping_sub(ideal) & mask;
            let dist_gap = gap.wrapping_sub(ideal) & mask;
            if dist_current > dist_gap {
                self.slots.swap(gap, idx);
                gap = idx;
            }
            idx = (idx + 1) & mask;
            if idx == deleted {
                break;
            }
        }
    }

    fn resize(&mut self) {
        let new_cap = (self.slots.len() * 2).max(DEFAULT_CAPACITY);
        let mut fresh = Vec::with_capacity(new_cap);
        fresh.resize_with(new_cap, || IdxSlot::Empty);
        let old = std::mem::replace(&mut self.slots, fresh);
        // Re-place every occupied cell from its stored hash (no user code).
        let mask = new_cap - 1;
        for cell in old {
            if let IdxSlot::Full { hash, slot } = cell {
                let mut idx = (hash as usize) & mask;
                while !matches!(self.slots[idx], IdxSlot::Empty) {
                    idx = (idx + 1) & mask;
                }
                self.slots[idx] = IdxSlot::Full { hash, slot };
            }
        }
    }
}

impl<S: BuildHasher> IndexTable<S> {
    /// Hash a key through the index's [`BuildHasher`]. The only method that runs
    /// the user's `Hash` impl; callers pass the result to `find`/`probe`/`remove`.
    #[inline]
    pub(crate) fn hash<Q: Hash + ?Sized>(&self, key: &Q) -> u64 {
        self.hasher.hash_one(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the index against an external key arena and a `std` reference map,
    /// asserting `find` agreement across a long randomized insert/remove stream.
    /// This exercises resize and backward-shift heavily.
    #[test]
    fn differential_against_std_over_random_ops() {
        use std::collections::HashMap as Std;

        // External "arena": keys[slot] = key. Slots are never reused here (a
        // fresh slot per insert) which is the worst case for the table's len.
        let mut keys: Vec<i64> = Vec::new();
        let mut table: IndexTable = IndexTable::new();
        let mut model: Std<i64, usize> = Std::new();

        let mut state: u64 = 0x00C0_FFEE_1234_5678;
        for _ in 0..20_000 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let key = ((state >> 33) % 500) as i64;
            let op = (state >> 30) & 3;
            let hash = table.hash(&key);
            let k = keys.clone();
            let eq = |s: usize| k[s] == key;
            match op {
                0 | 1 => {
                    // insert-or-noop
                    match table.probe(hash, eq) {
                        RawEntry::Occupied(slot) => {
                            assert_eq!(model.get(&key), Some(&slot));
                        }
                        RawEntry::Vacant(cell) => {
                            assert!(!model.contains_key(&key));
                            let slot = keys.len();
                            keys.push(key);
                            table.fill_vacant(cell, hash, slot);
                            model.insert(key, slot);
                        }
                    }
                }
                2 => {
                    let removed = table.remove(hash, eq);
                    assert_eq!(removed, model.remove(&key));
                }
                _ => {
                    let found = table.find(hash, eq);
                    assert_eq!(found, model.get(&key).copied());
                }
            }
            assert_eq!(table.len(), model.len());
        }
        // Final agreement: every model key resolves, absent keys don't.
        for (&key, &slot) in &model {
            let k = keys.clone();
            assert_eq!(table.find(table.hash(&key), |s| k[s] == key), Some(slot));
        }
    }

    #[test]
    fn find_insert_remove_basic() {
        let mut keys: Vec<&str> = Vec::new();
        let mut t: IndexTable = IndexTable::new();
        for key in ["a", "b", "c"] {
            let h = t.hash(key);
            let k = keys.clone();
            match t.probe(h, |s| k[s] == key) {
                RawEntry::Vacant(cell) => {
                    let slot = keys.len();
                    keys.push(key);
                    t.fill_vacant(cell, h, slot);
                }
                RawEntry::Occupied(_) => panic!("unexpected dup"),
            }
        }
        assert_eq!(t.len(), 3);
        let k = keys.clone();
        assert!(t.contains(t.hash("b"), |s| k[s] == "b"));
        assert_eq!(t.remove(t.hash("b"), |s| k[s] == "b"), Some(1));
        assert_eq!(t.len(), 2);
        let k = keys.clone();
        assert!(!t.contains(t.hash("b"), |s| k[s] == "b"));
        assert!(t.contains(t.hash("a"), |s| k[s] == "a"));
        assert!(t.contains(t.hash("c"), |s| k[s] == "c"));
    }

    #[test]
    fn grows_and_preserves_all_after_resize() {
        let mut keys: Vec<i32> = Vec::new();
        let mut t: IndexTable = IndexTable::new();
        for key in 0..1000 {
            let h = t.hash(&key);
            let k = keys.clone();
            if let RawEntry::Vacant(cell) = t.probe(h, |s| k[s] == key) {
                let slot = keys.len();
                keys.push(key);
                t.fill_vacant(cell, h, slot);
            }
        }
        assert_eq!(t.len(), 1000);
        for key in 0..1000 {
            let k = keys.clone();
            let slot = t.find(t.hash(&key), |s| k[s] == key);
            assert_eq!(slot, Some(key as usize));
        }
    }
}
