// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Open-addressing hash table with linear probing and Robin Hood backward-shift deletion.
//!
//! Ported from Eclipse Collections' primitive hash tables. The probe array is a
//! `Vec<Slot<…>>` where `Slot` is a two-variant enum (`Empty` / `Occupied`).
//! The occupancy flag is the enum discriminant (no separate `bool`), and the
//! key/value are stored inline rather than behind `Option`, so probing reads a
//! single packed slot and never unwraps an invariant `Option`. Backward-shift
//! deletion keeps the table tombstone-free, so two variants are sufficient.
//!
//! The maps and sets are generic over the hasher (`S: BuildHasher`), defaulting
//! to [`std::collections::hash_map::RandomState`] for HashDoS resistance — the
//! same default `std::collections::HashMap` uses. Opt into a faster, fixed
//! hasher (FxHash, AHash, …) with [`OpenHashMap::with_hasher`] /
//! [`OpenHashSet::with_hasher`].
//!
//! Generic over any `K: Hash + Eq` and any `V`. For `f32`/`f64` keys, wrap in
//! [`crate::hashable_float::HashableF32`] / [`crate::hashable_float::HashableF64`]
//! to get bit-pattern hashing (NaN-aware, ±0 distinct).

use crate::bulk::{BulkError, DuplicatePolicy};
use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};

const DEFAULT_CAPACITY: usize = 16;
const LOAD_FACTOR_NUM: usize = 3;
const LOAD_FACTOR_DEN: usize = 4; // 0.75

// ---------------------------------------------------------------------------
// Slot — niche-packed, tombstone-free entry storage
// ---------------------------------------------------------------------------

/// A single probe slot. The discriminant *is* the occupancy flag, and the
/// key/value live inline with no `Option` wrapper, so a probe loads one packed
/// slot and never unwraps an invariant `Option`. Backward-shift deletion keeps
/// the table tombstone-free, so two variants suffice.
#[derive(Debug, Clone)]
enum MapSlot<K, V> {
    Empty,
    Occupied { key: K, value: V },
}

#[derive(Debug, Clone)]
enum SetSlot<K> {
    Empty,
    Occupied { key: K },
}

// ---------------------------------------------------------------------------
// OpenHashMap<K, V, S>
// ---------------------------------------------------------------------------

/// Open-addressing hash map with niche-packed slots and a pluggable hasher.
///
/// Accepts any `K: Hash + Eq` (including object types like `String`, not just
/// primitives) and any `V` (including non-`Copy` types like `String`, `Vec`,
/// or user structs). The hasher `S` defaults to [`RandomState`]; use
/// [`OpenHashMap::with_hasher`] for a fixed/faster hasher. For `f32`/`f64`
/// keys, wrap them in [`crate::hashable_float::HashableF32`] or
/// [`crate::hashable_float::HashableF64`].
#[derive(Debug, Clone)]
pub struct OpenHashMap<K, V, S = RandomState> {
    entries: Vec<MapSlot<K, V>>,
    size: usize,
    hasher: S,
}

impl<K, V, S: BuildHasher + Default> Default for OpenHashMap<K, V, S> {
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K, V> OpenHashMap<K, V, RandomState> {
    pub fn new() -> Self {
        Self::with_hasher(RandomState::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<K, V, S> OpenHashMap<K, V, S> {
    /// Creates an empty map that will hash keys with `hasher`.
    pub fn with_hasher(hasher: S) -> Self {
        Self::with_capacity_and_hasher(DEFAULT_CAPACITY, hasher)
    }

    /// Creates an empty map with pre-allocated capacity that will hash keys
    /// with `hasher`.
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        let cap = capacity.max(DEFAULT_CAPACITY).next_power_of_two();
        let mut entries = Vec::with_capacity(cap);
        entries.resize_with(cap, || MapSlot::Empty);
        OpenHashMap {
            entries,
            size: 0,
            hasher,
        }
    }

    /// Returns a reference to the map's [`BuildHasher`].
    pub fn hasher(&self) -> &S {
        &self.hasher
    }

    #[inline]
    fn cap(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    fn mask(&self) -> usize {
        self.entries.len() - 1
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn clear(&mut self) {
        for e in &mut self.entries {
            *e = MapSlot::Empty;
        }
        self.size = 0;
    }

    #[inline]
    fn needs_resize(&self) -> bool {
        // Grow strictly *below* the 0.75 load factor: `>=` (not `>`) so
        // the table grows when `cap*3 == (size+1)*4` exactly (e.g. the
        // 12th insert into a capacity-16 table). Matches the
        // `needed*4/3 + 1` form used by `try_reserve`.
        (self.size + 1) * LOAD_FACTOR_DEN >= self.cap() * LOAD_FACTOR_NUM
    }

    pub fn iter(&self) -> OpenHashMapIter<'_, K, V> {
        OpenHashMapIter {
            entries: &self.entries,
            pos: 0,
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.iter().map(|(_, v)| v)
    }

    /// Iterate `(&K, &mut V)` over the live entries in unspecified order. Keys are
    /// handed out as shared `&K` (mutating a key would desync its hash slot);
    /// only values are mutable.
    pub fn iter_mut(&mut self) -> OpenHashMapIterMut<'_, K, V> {
        OpenHashMapIterMut {
            entries: self.entries.iter_mut(),
        }
    }

    /// Iterate `&mut V` over the live values in unspecified order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> + '_ {
        self.iter_mut().map(|(_, v)| v)
    }

    /// Remove all entries, yielding them as owned `(K, V)` pairs while
    /// **retaining the table's capacity** for reuse — the reuse-friendly
    /// counterpart to `into_iter` (which consumes the map). The map is emptied
    /// *immediately*: a fresh empty table of the same capacity is swapped in
    /// before the first item is yielded, so the map is left valid and empty even
    /// if the returned iterator is only partially consumed, dropped early, or
    /// leaked (no drop guard needed).
    pub fn drain(&mut self) -> OpenHashMapDrain<'_, K, V> {
        let cap = self.entries.len();
        let mut fresh: Vec<MapSlot<K, V>> = Vec::with_capacity(cap);
        fresh.resize_with(cap, || MapSlot::Empty);
        let old = std::mem::replace(&mut self.entries, fresh);
        self.size = 0;
        OpenHashMapDrain {
            inner: old.into_iter(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> OpenHashMap<K, V, S> {
    /// Hashes `key` through the table's `BuildHasher`. `RandomState` (the
    /// default) already produces well-mixed 64-bit output, so no
    /// Fibonacci/spread multiplier is layered on top.
    #[inline]
    fn hash(&self, key: &(impl Hash + ?Sized)) -> u64 {
        self.hasher.hash_one(key)
    }

    /// Inserts a key-value pair. Returns the old value if the key was already
    /// present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.mask();
        let mut idx = (self.hash(&key) as usize) & mask;
        loop {
            match &mut self.entries[idx] {
                MapSlot::Empty => {
                    self.entries[idx] = MapSlot::Occupied { key, value };
                    self.size += 1;
                    return None;
                }
                MapSlot::Occupied { key: k, value: v } if *k == key => {
                    return Some(std::mem::replace(v, value));
                }
                MapSlot::Occupied { .. } => {
                    idx = (idx + 1) & mask;
                }
            }
        }
    }

    /// Gets the [`Entry`] for `key` for in-place insert-or-update in a single
    /// probe.
    ///
    /// ```
    /// use mapdb_collections::OpenHashMap;
    /// let mut counts: OpenHashMap<&str, i32> = OpenHashMap::new();
    /// for w in ["a", "b", "a"] {
    ///     *counts.entry(w).or_insert(0) += 1;
    /// }
    /// assert_eq!(counts.get(&"a"), Some(&2));
    /// ```
    ///
    /// # Growth
    /// Like [`insert`](OpenHashMap::insert), `entry` resolves a pending resize
    /// **before** probing (so the returned slot index stays valid for the
    /// entry's lifetime). A consequence — matching `std` and this map's own
    /// `insert` — is that calling `entry` at the load-factor threshold grows the
    /// table **even if the key already exists and you only read it**. An
    /// `and_modify`-only use may therefore reallocate.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, S> {
        // Resolve resize BEFORE computing the index (same order as `insert`), so
        // the slot the entry captures cannot be invalidated by a later grow and
        // the resulting table is byte-identical to the `insert`-built one.
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.mask();
        let mut idx = (self.hash(&key) as usize) & mask;
        // Probe to either the matching key or the first empty slot, recording
        // which. `idx` is a plain `usize`, so the transient borrow of `entries`
        // ends before we move `self` into the entry.
        let occupied = loop {
            match &self.entries[idx] {
                MapSlot::Empty => break false,
                MapSlot::Occupied { key: k, .. } if *k == key => break true,
                MapSlot::Occupied { .. } => idx = (idx + 1) & mask,
            }
        };
        if occupied {
            Entry::Occupied(OccupiedEntry { map: self, idx })
        } else {
            Entry::Vacant(VacantEntry {
                map: self,
                key,
                idx,
            })
        }
    }

    /// Borrows the value for `key`.
    ///
    /// Accepts any borrowed form `&Q` of the key (`K: Borrow<Q>`), so a
    /// `OpenHashMap<String, _>` can be queried with `&str`. Existing `&K`
    /// callers continue to work because `K: Borrow<K>` always holds.
    pub fn get<'a, Q>(&'a self, key: &Q) -> Option<&'a V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.size == 0 {
            return None;
        }
        let mask = self.mask();
        let mut idx = (self.hash(key) as usize) & mask;
        loop {
            match &self.entries[idx] {
                MapSlot::Empty => return None,
                MapSlot::Occupied { key: k, value } if k.borrow() == key => {
                    return Some(value);
                }
                MapSlot::Occupied { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    pub fn get_mut<'a, Q>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.size == 0 {
            return None;
        }
        let mask = self.mask();
        let mut idx = (self.hash(key) as usize) & mask;
        loop {
            match &self.entries[idx] {
                MapSlot::Empty => return None,
                MapSlot::Occupied { key: k, .. } if k.borrow() == key => {
                    match &mut self.entries[idx] {
                        MapSlot::Occupied { value, .. } => return Some(value),
                        MapSlot::Empty => unreachable!(),
                    }
                }
                MapSlot::Occupied { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Removes the key. Returns the old value if present.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.size == 0 {
            return None;
        }
        let mask = self.mask();
        let mut idx = (self.hash(key) as usize) & mask;
        loop {
            match &self.entries[idx] {
                MapSlot::Empty => return None,
                MapSlot::Occupied { key: k, .. } if k.borrow() == key => {
                    let taken = std::mem::replace(&mut self.entries[idx], MapSlot::Empty);
                    self.size -= 1;
                    self.rehash_from(idx);
                    match taken {
                        MapSlot::Occupied { value, .. } => return Some(value),
                        MapSlot::Empty => unreachable!(),
                    }
                }
                MapSlot::Occupied { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    /// Retains only the entries for which `keep(&k, &mut v)` returns `true`.
    ///
    /// Implemented by rebuilding the table in place (take entries, clear,
    /// re-insert the survivors at the **same capacity**) rather than an
    /// in-place scan — over the backward-shift kernel a live scan index can be
    /// invalidated when a surviving key is relocated into an already-visited
    /// slot, so a rebuild is the correct primitive. O(n), no `K: Clone`.
    pub fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let cap = self.entries.len();
        let mut fresh: Vec<MapSlot<K, V>> = Vec::with_capacity(cap);
        fresh.resize_with(cap, || MapSlot::Empty);
        let old = std::mem::replace(&mut self.entries, fresh);
        self.size = 0;
        for slot in old {
            if let MapSlot::Occupied { key, mut value } = slot {
                if keep(&key, &mut value) {
                    self.insert_no_resize(key, value);
                }
            }
        }
    }

    fn rehash_from(&mut self, deleted: usize) {
        let mask = self.mask();
        let mut gap = deleted;
        let mut idx = (deleted + 1) & mask;
        while let MapSlot::Occupied { key, .. } = &self.entries[idx] {
            let ideal = (self.hash(key) as usize) & mask;
            let dist_current = idx.wrapping_sub(ideal) & mask;
            let dist_gap = gap.wrapping_sub(ideal) & mask;
            if dist_current > dist_gap {
                self.entries.swap(gap, idx);
                gap = idx;
            }
            idx = (idx + 1) & mask;
            if idx == deleted {
                break;
            }
        }
    }

    fn resize(&mut self) {
        let new_cap = (self.entries.len() * 2).max(DEFAULT_CAPACITY);
        self.grow_to_infallible(new_cap);
    }

    fn grow_to_infallible(&mut self, new_cap: usize) {
        if new_cap <= self.entries.len() {
            return;
        }
        let mut new_entries: Vec<MapSlot<K, V>> = Vec::with_capacity(new_cap);
        new_entries.resize_with(new_cap, || MapSlot::Empty);
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for e in old.into_iter() {
            if let MapSlot::Occupied { key, value } = e {
                self.insert_no_resize(key, value);
            }
        }
    }

    fn grow_to(&mut self, new_cap: usize) -> Result<(), std::collections::TryReserveError> {
        if new_cap <= self.entries.len() {
            return Ok(());
        }
        let mut new_entries: Vec<MapSlot<K, V>> = Vec::new();
        new_entries.try_reserve_exact(new_cap)?;
        new_entries.resize_with(new_cap, || MapSlot::Empty);
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for e in old.into_iter() {
            if let MapSlot::Occupied { key, value } = e {
                self.insert_no_resize(key, value);
            }
        }
        Ok(())
    }

    fn insert_no_resize(&mut self, key: K, value: V) {
        let mask = self.mask();
        let mut idx = (self.hash(&key) as usize) & mask;
        loop {
            if let MapSlot::Empty = &self.entries[idx] {
                self.entries[idx] = MapSlot::Occupied { key, value };
                self.size += 1;
                return;
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Reserves capacity for at least `additional` more entries to be inserted.
    /// Returns `TryReserveError` if the allocator cannot satisfy the request.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        let needed = self.size.saturating_add(additional);
        let required = needed.saturating_mul(4) / 3 + 1;
        if required <= self.entries.len() {
            return Ok(());
        }
        let floor = required.max(DEFAULT_CAPACITY);
        let new_cap = floor.checked_next_power_of_two().unwrap_or(usize::MAX);
        self.grow_to(new_cap)
    }

    /// Inserts into a table sized for the whole load, never resizing. Returns
    /// `Err(index)` (the caller's running index) when `key` is a duplicate and
    /// the policy is [`DuplicatePolicy::Error`]; `Ok(true)` when newly inserted,
    /// `Ok(false)` when an ignored duplicate.
    #[inline]
    fn bulk_insert(
        &mut self,
        key: K,
        value: V,
        dup: DuplicatePolicy,
        index: usize,
    ) -> Result<bool, BulkError> {
        let mask = self.mask();
        let mut idx = (self.hash(&key) as usize) & mask;
        loop {
            match &mut self.entries[idx] {
                MapSlot::Empty => {
                    self.entries[idx] = MapSlot::Occupied { key, value };
                    self.size += 1;
                    return Ok(true);
                }
                MapSlot::Occupied { key: k, .. } if *k == key => match dup {
                    DuplicatePolicy::Error => return Err(BulkError::Duplicate { index }),
                    DuplicatePolicy::IgnoreDuplicates => return Ok(false),
                },
                MapSlot::Occupied { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    /// Sets the table to an exact power-of-two capacity for `n` items in a
    /// single allocation, replacing the (empty) backing store. The caller must
    /// ensure the map is empty.
    fn bulk_presize(&mut self, n: usize) -> Result<(), BulkError> {
        debug_assert_eq!(self.size, 0, "bulk_presize requires an empty table");
        let cap = crate::bulk::open_addressing_capacity(n, DEFAULT_CAPACITY);
        let mut entries: Vec<MapSlot<K, V>> = Vec::new();
        entries.try_reserve_exact(cap)?;
        entries.resize_with(cap, || MapSlot::Empty);
        self.entries = entries;
        Ok(())
    }

    /// Bulk-load a fresh map from `iter`, pre-sizing the table once for the
    /// source's length so the load triggers **no** mid-load rehash when the
    /// length is exact (see [`OpenHashMap::bulk_load_exact`]). For an unsized or
    /// untrusted source this uses the iterator's size hint; it is correct but
    /// does not claim the zero-rehash guarantee.
    ///
    /// Duplicate keys follow `dup`. Single pass, O(n).
    pub fn bulk_load<I: IntoIterator<Item = (K, V)>>(
        iter: I,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError>
    where
        S: Default,
    {
        let iter = iter.into_iter();
        let hint = iter.size_hint().0;
        let mut map = Self::with_hasher(S::default());
        map.bulk_presize(hint)?;
        for (index, (k, v)) in iter.enumerate() {
            // Grow if the size hint under-counted; this is the "hint" path and
            // is allowed to rehash.
            if map.needs_resize() {
                map.resize();
            }
            map.bulk_insert(k, v, dup, index)?;
        }
        Ok(map)
    }

    /// Bulk-load a fresh map from a source declared to hold exactly `n` items.
    /// Pre-sizes for `n` in one allocation and **never grows**: the source
    /// producing more than `n` items is a [`BulkError::ExactSizeExceeded`].
    /// This is the zero-rehash path (tested at `n = 3·2^k`).
    pub fn bulk_load_exact<I: IntoIterator<Item = (K, V)>>(
        iter: I,
        n: usize,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError>
    where
        S: Default,
    {
        let mut map = Self::with_hasher(S::default());
        map.bulk_presize(n)?;
        for (index, (k, v)) in iter.into_iter().enumerate() {
            // Enforce the limit on *consumed* source length, not unique inserted
            // cardinality: an overlong source whose extras are duplicates must
            // still error under IgnoreDuplicates (it consumed > n items).
            if index >= n {
                return Err(BulkError::ExactSizeExceeded { expected: n });
            }
            map.bulk_insert(k, v, dup, index)?;
        }
        Ok(map)
    }
}

// ---------------------------------------------------------------------------
// Entry API
// ---------------------------------------------------------------------------

/// A view into a single map slot, obtained from [`OpenHashMap::entry`].
///
/// This is safe on the backward-shift kernel without generation counters: the
/// entry borrows the map `&mut`, so the borrow checker makes any intervening
/// mutation (the only thing that could invalidate the captured slot index)
/// unrepresentable — a resize was already resolved by `entry`, and the one
/// operation that shifts slots, [`OccupiedEntry::remove`], consumes the entry.
#[must_use]
pub enum Entry<'a, K, V, S> {
    /// The key is present.
    Occupied(OccupiedEntry<'a, K, V, S>),
    /// The key is absent; the captured slot is empty and ready to receive it.
    Vacant(VacantEntry<'a, K, V, S>),
}

/// An occupied [`Entry`].
pub struct OccupiedEntry<'a, K, V, S> {
    map: &'a mut OpenHashMap<K, V, S>,
    idx: usize,
}

/// A vacant [`Entry`].
pub struct VacantEntry<'a, K, V, S> {
    map: &'a mut OpenHashMap<K, V, S>,
    key: K,
    idx: usize,
}

impl<'a, K: Hash + Eq, V, S: BuildHasher> OccupiedEntry<'a, K, V, S> {
    #[inline]
    fn slot(&self) -> (&K, &V) {
        match &self.map.entries[self.idx] {
            MapSlot::Occupied { key, value } => (key, value),
            MapSlot::Empty => unreachable!("OccupiedEntry over an empty slot"),
        }
    }

    /// The key in this entry.
    pub fn key(&self) -> &K {
        self.slot().0
    }

    /// Borrows the value.
    pub fn get(&self) -> &V {
        self.slot().1
    }

    /// Mutably borrows the value.
    pub fn get_mut(&mut self) -> &mut V {
        match &mut self.map.entries[self.idx] {
            MapSlot::Occupied { value, .. } => value,
            MapSlot::Empty => unreachable!("OccupiedEntry over an empty slot"),
        }
    }

    /// Converts into a mutable reference to the value with the map's lifetime.
    pub fn into_mut(self) -> &'a mut V {
        match &mut self.map.entries[self.idx] {
            MapSlot::Occupied { value, .. } => value,
            MapSlot::Empty => unreachable!("OccupiedEntry over an empty slot"),
        }
    }

    /// Replaces the value, returning the old one.
    pub fn insert(&mut self, value: V) -> V {
        std::mem::replace(self.get_mut(), value)
    }

    /// Removes the entry and returns its `(key, value)`. Runs the same
    /// backward-shift as [`OpenHashMap::remove`], keeping the table
    /// tombstone-free.
    pub fn remove_entry(self) -> (K, V) {
        let taken = std::mem::replace(&mut self.map.entries[self.idx], MapSlot::Empty);
        self.map.size -= 1;
        self.map.rehash_from(self.idx);
        match taken {
            MapSlot::Occupied { key, value } => (key, value),
            MapSlot::Empty => unreachable!("OccupiedEntry over an empty slot"),
        }
    }

    /// Removes the entry and returns its value.
    pub fn remove(self) -> V {
        self.remove_entry().1
    }
}

impl<'a, K: Hash + Eq, V, S: BuildHasher> VacantEntry<'a, K, V, S> {
    /// The key that would be inserted.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Takes back ownership of the key.
    pub fn into_key(self) -> K {
        self.key
    }

    /// Inserts `value` into the captured empty slot and returns a mutable
    /// reference to it. No re-probe or resize: `entry` already resolved growth
    /// and located the empty slot, and the `&mut` borrow guaranteed nothing
    /// changed since.
    pub fn insert(self, value: V) -> &'a mut V {
        debug_assert!(matches!(self.map.entries[self.idx], MapSlot::Empty));
        self.map.entries[self.idx] = MapSlot::Occupied {
            key: self.key,
            value,
        };
        self.map.size += 1;
        match &mut self.map.entries[self.idx] {
            MapSlot::Occupied { value, .. } => value,
            MapSlot::Empty => unreachable!(),
        }
    }
}

impl<'a, K: Hash + Eq, V, S: BuildHasher> Entry<'a, K, V, S> {
    /// The key for this entry (present or to-be-inserted).
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => e.key(),
        }
    }

    /// Ensures a value is present, inserting `default` if vacant; returns a
    /// mutable reference to the value.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default),
        }
    }

    /// Like [`or_insert`](Entry::or_insert) but computes the default lazily.
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default()),
        }
    }

    /// Like [`or_insert_with`](Entry::or_insert_with) but the closure receives
    /// the key.
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let value = default(&e.key);
                e.insert(value)
            }
        }
    }

    /// Runs `f` on the value if the entry is occupied, then returns the entry
    /// for chaining (e.g. `.and_modify(|v| *v += 1).or_insert(1)`).
    pub fn and_modify<F: FnOnce(&mut V)>(mut self, f: F) -> Self {
        if let Entry::Occupied(e) = &mut self {
            f(e.get_mut());
        }
        self
    }
}

impl<'a, K: Hash + Eq, V: Default, S: BuildHasher> Entry<'a, K, V, S> {
    /// Ensures a value is present, inserting `V::default()` if vacant.
    pub fn or_default(self) -> &'a mut V {
        self.or_insert_with(V::default)
    }
}

pub struct OpenHashMapIter<'a, K, V> {
    entries: &'a [MapSlot<K, V>],
    pos: usize,
}

impl<'a, K, V> Iterator for OpenHashMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            if let MapSlot::Occupied { key, value } = &self.entries[i] {
                return Some((key, value));
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// OpenHashSet<K, S>
// ---------------------------------------------------------------------------

/// Open-addressing hash set with niche-packed slots and a pluggable hasher.
///
/// The hasher `S` defaults to [`RandomState`]; use [`OpenHashSet::with_hasher`]
/// for a fixed/faster hasher.
#[derive(Debug, Clone)]
pub struct OpenHashSet<K, S = RandomState> {
    entries: Vec<SetSlot<K>>,
    size: usize,
    hasher: S,
}

impl<K, S: BuildHasher + Default> Default for OpenHashSet<K, S> {
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K> OpenHashSet<K, RandomState> {
    pub fn new() -> Self {
        Self::with_hasher(RandomState::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<K, S> OpenHashSet<K, S> {
    /// Creates an empty set that will hash values with `hasher`.
    pub fn with_hasher(hasher: S) -> Self {
        Self::with_capacity_and_hasher(DEFAULT_CAPACITY, hasher)
    }

    /// Creates an empty set with pre-allocated capacity that will hash values
    /// with `hasher`.
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        let cap = capacity.max(DEFAULT_CAPACITY).next_power_of_two();
        let mut entries = Vec::with_capacity(cap);
        entries.resize_with(cap, || SetSlot::Empty);
        OpenHashSet {
            entries,
            size: 0,
            hasher,
        }
    }

    /// Returns a reference to the set's [`BuildHasher`].
    pub fn hasher(&self) -> &S {
        &self.hasher
    }

    #[inline]
    fn mask(&self) -> usize {
        self.entries.len() - 1
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn clear(&mut self) {
        for e in &mut self.entries {
            *e = SetSlot::Empty;
        }
        self.size = 0;
    }

    #[inline]
    fn needs_resize(&self) -> bool {
        // Grow strictly *below* the 0.75 load factor: `>=` (not `>`) so
        // the table grows when `cap*3 == (size+1)*4` exactly (e.g. the
        // 12th insert into a capacity-16 table). Matches the
        // `needed*4/3 + 1` form used by `try_reserve`.
        (self.size + 1) * LOAD_FACTOR_DEN >= self.entries.len() * LOAD_FACTOR_NUM
    }

    pub fn iter(&self) -> OpenHashSetIter<'_, K> {
        OpenHashSetIter {
            entries: &self.entries,
            pos: 0,
        }
    }

    /// Remove all elements, yielding them as owned `K` while **retaining the
    /// table's capacity** for reuse — the reuse-friendly counterpart to
    /// `into_iter`. Emptied immediately (a fresh same-capacity table is swapped
    /// in before the first item), so it is leak/early-drop/panic safe with no
    /// drop guard.
    pub fn drain(&mut self) -> OpenHashSetDrain<'_, K> {
        let cap = self.entries.len();
        let mut fresh: Vec<SetSlot<K>> = Vec::with_capacity(cap);
        fresh.resize_with(cap, || SetSlot::Empty);
        let old = std::mem::replace(&mut self.entries, fresh);
        self.size = 0;
        OpenHashSetDrain {
            inner: old.into_iter(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<K: Hash + Eq, S: BuildHasher> OpenHashSet<K, S> {
    /// Hashes `key` through the set's `BuildHasher` (see [`OpenHashMap`] for the
    /// no-extra-spread rationale).
    #[inline]
    fn hash(&self, key: &(impl Hash + ?Sized)) -> u64 {
        self.hasher.hash_one(key)
    }

    /// Inserts a value. Returns `true` if it was newly inserted (not already present).
    pub fn insert(&mut self, value: K) -> bool {
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.mask();
        let mut idx = (self.hash(&value) as usize) & mask;
        loop {
            match &self.entries[idx] {
                SetSlot::Empty => {
                    self.entries[idx] = SetSlot::Occupied { key: value };
                    self.size += 1;
                    return true;
                }
                SetSlot::Occupied { key } if *key == value => return false,
                SetSlot::Occupied { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.size == 0 {
            return false;
        }
        let mask = self.mask();
        let mut idx = (self.hash(value) as usize) & mask;
        loop {
            match &self.entries[idx] {
                SetSlot::Empty => return false,
                SetSlot::Occupied { key } if key.borrow() == value => return true,
                SetSlot::Occupied { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.size == 0 {
            return false;
        }
        let mask = self.mask();
        let mut idx = (self.hash(value) as usize) & mask;
        loop {
            match &self.entries[idx] {
                SetSlot::Empty => return false,
                SetSlot::Occupied { key } if key.borrow() == value => {
                    self.entries[idx] = SetSlot::Empty;
                    self.size -= 1;
                    self.rehash_from(idx);
                    return true;
                }
                SetSlot::Occupied { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    /// Retains only the elements for which `keep(&k)` returns `true`. Rebuilds
    /// the table in place at the same capacity (see [`OpenHashMap::retain`] for
    /// why a rebuild rather than an in-place scan). O(n), no `K: Clone`.
    pub fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(&K) -> bool,
    {
        let cap = self.entries.len();
        let mut fresh: Vec<SetSlot<K>> = Vec::with_capacity(cap);
        fresh.resize_with(cap, || SetSlot::Empty);
        let old = std::mem::replace(&mut self.entries, fresh);
        self.size = 0;
        for slot in old {
            if let SetSlot::Occupied { key } = slot {
                if keep(&key) {
                    self.insert_no_resize(key);
                }
            }
        }
    }

    // ── Set algebra ─────────────────────────────────────────────────
    //
    // The four combining operations return an **owned** new set (eager, like
    // the sibling `object::HashSet`, not std's lazy iterators) seeded with a
    // clone of `self`'s hasher, so the result hashes identically to `self`.

    /// `true` if every element of `self` is also in `other` (`self ⊆ other`).
    /// The empty set is a subset of everything.
    pub fn is_subset(&self, other: &Self) -> bool {
        self.len() <= other.len() && self.iter().all(|k| other.contains(k))
    }

    /// `true` if every element of `other` is also in `self` (`self ⊇ other`).
    pub fn is_superset(&self, other: &Self) -> bool {
        other.is_subset(self)
    }

    /// `true` if `self` and `other` share no element. Iterates the smaller set.
    pub fn is_disjoint(&self, other: &Self) -> bool {
        let (small, big) = if self.len() <= other.len() {
            (self, other)
        } else {
            (other, self)
        };
        small.iter().all(|k| !big.contains(k))
    }

    /// The union `self ∪ other` (every element of either set).
    pub fn union(&self, other: &Self) -> Self
    where
        K: Clone,
        S: Clone,
    {
        let mut out =
            Self::with_capacity_and_hasher(self.len() + other.len(), self.hasher().clone());
        for k in self.iter().chain(other.iter()) {
            out.insert(k.clone());
        }
        out
    }

    /// The intersection `self ∩ other` (elements in both sets).
    pub fn intersection(&self, other: &Self) -> Self
    where
        K: Clone,
        S: Clone,
    {
        // Iterate the smaller set, probe the larger — fewer lookups.
        let (small, big) = if self.len() <= other.len() {
            (self, other)
        } else {
            (other, self)
        };
        let mut out = Self::with_capacity_and_hasher(small.len(), self.hasher().clone());
        for k in small.iter() {
            if big.contains(k) {
                out.insert(k.clone());
            }
        }
        out
    }

    /// The difference `self \ other` (elements in `self` but not `other`).
    pub fn difference(&self, other: &Self) -> Self
    where
        K: Clone,
        S: Clone,
    {
        let mut out = Self::with_capacity_and_hasher(self.len(), self.hasher().clone());
        for k in self.iter() {
            if !other.contains(k) {
                out.insert(k.clone());
            }
        }
        out
    }

    /// The symmetric difference `self △ other` (elements in exactly one set).
    pub fn symmetric_difference(&self, other: &Self) -> Self
    where
        K: Clone,
        S: Clone,
    {
        let mut out = self.difference(other);
        for k in other.iter() {
            if !self.contains(k) {
                out.insert(k.clone());
            }
        }
        out
    }

    fn rehash_from(&mut self, deleted: usize) {
        let mask = self.mask();
        let mut gap = deleted;
        let mut idx = (deleted + 1) & mask;
        while let SetSlot::Occupied { key } = &self.entries[idx] {
            let ideal = (self.hash(key) as usize) & mask;
            let dist_current = idx.wrapping_sub(ideal) & mask;
            let dist_gap = gap.wrapping_sub(ideal) & mask;
            if dist_current > dist_gap {
                self.entries.swap(gap, idx);
                gap = idx;
            }
            idx = (idx + 1) & mask;
            if idx == deleted {
                break;
            }
        }
    }

    fn resize(&mut self) {
        let new_cap = (self.entries.len() * 2).max(DEFAULT_CAPACITY);
        self.grow_to_infallible(new_cap);
    }

    fn grow_to_infallible(&mut self, new_cap: usize) {
        if new_cap <= self.entries.len() {
            return;
        }
        let mut new_entries: Vec<SetSlot<K>> = Vec::with_capacity(new_cap);
        new_entries.resize_with(new_cap, || SetSlot::Empty);
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for e in old.into_iter() {
            if let SetSlot::Occupied { key } = e {
                self.insert_no_resize(key);
            }
        }
    }

    fn grow_to(&mut self, new_cap: usize) -> Result<(), std::collections::TryReserveError> {
        if new_cap <= self.entries.len() {
            return Ok(());
        }
        let mut new_entries: Vec<SetSlot<K>> = Vec::new();
        new_entries.try_reserve_exact(new_cap)?;
        new_entries.resize_with(new_cap, || SetSlot::Empty);
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for e in old.into_iter() {
            if let SetSlot::Occupied { key } = e {
                self.insert_no_resize(key);
            }
        }
        Ok(())
    }

    fn insert_no_resize(&mut self, value: K) {
        let mask = self.mask();
        let mut idx = (self.hash(&value) as usize) & mask;
        loop {
            if let SetSlot::Empty = &self.entries[idx] {
                self.entries[idx] = SetSlot::Occupied { key: value };
                self.size += 1;
                return;
            }
            idx = (idx + 1) & mask;
        }
    }

    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        let needed = self.size.saturating_add(additional);
        let required = needed.saturating_mul(4) / 3 + 1;
        if required <= self.entries.len() {
            return Ok(());
        }
        let floor = required.max(DEFAULT_CAPACITY);
        let new_cap = floor.checked_next_power_of_two().unwrap_or(usize::MAX);
        self.grow_to(new_cap)
    }

    /// No-resize bulk insert with duplicate detection (see the `OpenHashMap`
    /// twin for semantics).
    #[inline]
    fn bulk_insert(
        &mut self,
        value: K,
        dup: DuplicatePolicy,
        index: usize,
    ) -> Result<bool, BulkError> {
        let mask = self.mask();
        let mut idx = (self.hash(&value) as usize) & mask;
        loop {
            match &self.entries[idx] {
                SetSlot::Empty => {
                    self.entries[idx] = SetSlot::Occupied { key: value };
                    self.size += 1;
                    return Ok(true);
                }
                SetSlot::Occupied { key } if *key == value => match dup {
                    DuplicatePolicy::Error => return Err(BulkError::Duplicate { index }),
                    DuplicatePolicy::IgnoreDuplicates => return Ok(false),
                },
                SetSlot::Occupied { .. } => idx = (idx + 1) & mask,
            }
        }
    }

    fn bulk_presize(&mut self, n: usize) -> Result<(), BulkError> {
        debug_assert_eq!(self.size, 0, "bulk_presize requires an empty table");
        let cap = crate::bulk::open_addressing_capacity(n, DEFAULT_CAPACITY);
        let mut entries: Vec<SetSlot<K>> = Vec::new();
        entries.try_reserve_exact(cap)?;
        entries.resize_with(cap, || SetSlot::Empty);
        self.entries = entries;
        Ok(())
    }

    /// Bulk-load a fresh set; size hint path (may rehash). See
    /// [`OpenHashMap::bulk_load`].
    pub fn bulk_load<I: IntoIterator<Item = K>>(
        iter: I,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError>
    where
        S: Default,
    {
        let iter = iter.into_iter();
        let hint = iter.size_hint().0;
        let mut set = Self::with_hasher(S::default());
        set.bulk_presize(hint)?;
        for (index, k) in iter.enumerate() {
            if set.needs_resize() {
                set.resize();
            }
            set.bulk_insert(k, dup, index)?;
        }
        Ok(set)
    }

    /// Zero-rehash bulk load for an exactly-`n`-element source. See
    /// [`OpenHashMap::bulk_load_exact`].
    pub fn bulk_load_exact<I: IntoIterator<Item = K>>(
        iter: I,
        n: usize,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError>
    where
        S: Default,
    {
        let mut set = Self::with_hasher(S::default());
        set.bulk_presize(n)?;
        for (index, k) in iter.into_iter().enumerate() {
            // Enforce the limit on *consumed* source length, not unique inserted
            // cardinality: an overlong source whose extras are duplicates must
            // still error under IgnoreDuplicates (it consumed > n items).
            if index >= n {
                return Err(BulkError::ExactSizeExceeded { expected: n });
            }
            set.bulk_insert(k, dup, index)?;
        }
        Ok(set)
    }
}

pub struct OpenHashSetIter<'a, K> {
    entries: &'a [SetSlot<K>],
    pos: usize,
}

impl<'a, K> Iterator for OpenHashSetIter<'a, K> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            if let SetSlot::Occupied { key } = &self.entries[i] {
                return Some(key);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Standard-library trait impls (additive idiom layer)
// ---------------------------------------------------------------------------

impl<'a, K, V, S> IntoIterator for &'a OpenHashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = OpenHashMapIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Mutable-value iterator over an `OpenHashMap`'s live entries (unspecified
/// order): yields `(&K, &mut V)`. Built by a disjoint-borrow walk of the slot
/// array (no `unsafe`); the key is shared so the hash slot can't be desynced.
pub struct OpenHashMapIterMut<'a, K, V> {
    entries: std::slice::IterMut<'a, MapSlot<K, V>>,
}

impl<'a, K, V> Iterator for OpenHashMapIterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        for slot in self.entries.by_ref() {
            if let MapSlot::Occupied { key, value } = slot {
                return Some((&*key, value));
            }
        }
        None
    }
}

impl<K, V> std::iter::FusedIterator for OpenHashMapIterMut<'_, K, V> {}

impl<'a, K, V, S> IntoIterator for &'a mut OpenHashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = OpenHashMapIterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Owning iterator over an `OpenHashMap`'s `(K, V)` pairs (unspecified order).
pub struct OpenHashMapIntoIter<K, V> {
    inner: std::vec::IntoIter<MapSlot<K, V>>,
}

impl<K, V> Iterator for OpenHashMapIntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        for e in self.inner.by_ref() {
            if let MapSlot::Occupied { key, value } = e {
                return Some((key, value));
            }
        }
        None
    }
}

impl<K, V, S> IntoIterator for OpenHashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = OpenHashMapIntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        OpenHashMapIntoIter {
            inner: self.entries.into_iter(),
        }
    }
}

/// Draining iterator over an `OpenHashMap`'s `(K, V)` pairs (unspecified order),
/// returned by [`OpenHashMap::drain`]. Holds a `&mut` borrow of the map for its
/// lifetime; the map was already emptied when this was created.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct OpenHashMapDrain<'a, K, V> {
    inner: std::vec::IntoIter<MapSlot<K, V>>,
    _marker: std::marker::PhantomData<&'a mut ()>,
}

impl<K, V> Iterator for OpenHashMapDrain<'_, K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        for e in self.inner.by_ref() {
            if let MapSlot::Occupied { key, value } = e {
                return Some((key, value));
            }
        }
        None
    }
}

impl<K, V> std::iter::FusedIterator for OpenHashMapDrain<'_, K, V> {}

impl<K: Hash + Eq, V, S: BuildHasher + Default> FromIterator<(K, V)> for OpenHashMap<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut m = OpenHashMap::with_hasher(S::default());
        for (k, v) in iter {
            m.insert(k, v);
        }
        m
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Extend<(K, V)> for OpenHashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

/// Order-insensitive equality: same length and every key maps to an equal value.
impl<K: Hash + Eq, V: PartialEq, S: BuildHasher> PartialEq for OpenHashMap<K, V, S> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<K: Hash + Eq, V: Eq, S: BuildHasher> Eq for OpenHashMap<K, V, S> {}

impl<'a, K, S> IntoIterator for &'a OpenHashSet<K, S> {
    type Item = &'a K;
    type IntoIter = OpenHashSetIter<'a, K>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Owning iterator over an `OpenHashSet`'s elements (unspecified order).
pub struct OpenHashSetIntoIter<K> {
    inner: std::vec::IntoIter<SetSlot<K>>,
}

impl<K> Iterator for OpenHashSetIntoIter<K> {
    type Item = K;
    fn next(&mut self) -> Option<Self::Item> {
        for e in self.inner.by_ref() {
            if let SetSlot::Occupied { key } = e {
                return Some(key);
            }
        }
        None
    }
}

impl<K, S> IntoIterator for OpenHashSet<K, S> {
    type Item = K;
    type IntoIter = OpenHashSetIntoIter<K>;
    fn into_iter(self) -> Self::IntoIter {
        OpenHashSetIntoIter {
            inner: self.entries.into_iter(),
        }
    }
}

/// Draining iterator over an `OpenHashSet`'s elements (unspecified order),
/// returned by [`OpenHashSet::drain`]. Holds a `&mut` borrow for its lifetime;
/// the set was already emptied when this was created.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct OpenHashSetDrain<'a, K> {
    inner: std::vec::IntoIter<SetSlot<K>>,
    _marker: std::marker::PhantomData<&'a mut ()>,
}

impl<K> Iterator for OpenHashSetDrain<'_, K> {
    type Item = K;
    fn next(&mut self) -> Option<Self::Item> {
        for e in self.inner.by_ref() {
            if let SetSlot::Occupied { key } = e {
                return Some(key);
            }
        }
        None
    }
}

impl<K> std::iter::FusedIterator for OpenHashSetDrain<'_, K> {}

impl<K: Hash + Eq, S: BuildHasher + Default> FromIterator<K> for OpenHashSet<K, S> {
    fn from_iter<I: IntoIterator<Item = K>>(iter: I) -> Self {
        let mut s = OpenHashSet::with_hasher(S::default());
        for k in iter {
            s.insert(k);
        }
        s
    }
}

impl<K: Hash + Eq, S: BuildHasher> Extend<K> for OpenHashSet<K, S> {
    fn extend<I: IntoIterator<Item = K>>(&mut self, iter: I) {
        for k in iter {
            self.insert(k);
        }
    }
}

/// Order-insensitive equality: same length and every element is present in both.
impl<K: Hash + Eq, S: BuildHasher> PartialEq for OpenHashSet<K, S> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|k| other.contains(k))
    }
}

impl<K: Hash + Eq, S: BuildHasher> Eq for OpenHashSet<K, S> {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashable_float::{HashableF32, HashableF64};

    #[test]
    fn drain_empties_map_and_yields_all() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..30 {
            m.insert(i, i * 10);
        }
        let mut drained: Vec<(i32, i32)> = m.drain().collect();
        drained.sort_unstable();
        assert_eq!(drained.len(), 30);
        assert_eq!(drained[5], (5, 50));
        // Map is empty afterwards and reusable.
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
        assert_eq!(m.get(&5), None);
        m.insert(99, 1);
        assert_eq!(m.get(&99), Some(&1));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn drain_early_drop_and_leak_leave_empty_map() {
        // Partial consume then drop: map already empty (eager drain).
        let mut m = OpenHashMap::<i32, i32>::from_iter((0..10).map(|i| (i, i)));
        {
            let mut d = m.drain();
            assert!(d.next().is_some());
        } // dropped early
        assert!(m.is_empty());
        assert_eq!(m.get(&0), None);

        // Leaking the drain also leaves the map empty (swap happened up front).
        let mut m2 = OpenHashMap::<i32, i32>::from_iter((0..10).map(|i| (i, i)));
        std::mem::forget(m2.drain());
        assert!(m2.is_empty());
        assert_eq!(m2.get(&3), None);
    }

    #[test]
    fn drain_set_empties_and_yields() {
        let mut s = OpenHashSet::<i32>::from_iter(0..20);
        let mut drained: Vec<i32> = s.drain().collect();
        drained.sort_unstable();
        assert_eq!(drained, (0..20).collect::<Vec<_>>());
        assert!(s.is_empty());
        s.insert(7);
        assert!(s.contains(&7) && !s.contains(&0));
    }

    #[test]
    fn iter_mut_and_values_mut_mutate_in_place() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..50 {
            m.insert(i, i);
        }
        for (k, v) in m.iter_mut() {
            *v += *k * 100; // key is &K (shared) — read-only; value mutable
        }
        for i in 0..50 {
            assert_eq!(m.get(&i), Some(&(i + i * 100)));
        }
        for v in m.values_mut() {
            *v = 0;
        }
        assert!(m.values().all(|&v| v == 0));
        assert_eq!(m.len(), 50);
    }

    #[test]
    fn iter_mut_via_mut_ref_into_iter() {
        let mut m = OpenHashMap::<&str, i32>::new();
        m.insert("a", 1);
        m.insert("b", 2);
        for (_k, v) in &mut m {
            *v *= 10;
        }
        let mut got: Vec<(&str, i32)> = m.iter().map(|(&k, &v)| (k, v)).collect();
        got.sort_unstable();
        assert_eq!(got, vec![("a", 10), ("b", 20)]);
    }

    #[test]
    fn iter_mut_empty_and_fused() {
        let mut m = OpenHashMap::<i32, i32>::new();
        let mut it = m.iter_mut();
        assert!(it.next().is_none());
        assert!(it.next().is_none()); // FusedIterator
    }

    #[test]
    fn open_hash_map_partial_eq_order_insensitive() {
        let mut a = OpenHashMap::<i32, i32>::new();
        a.insert(1, 10);
        a.insert(2, 20);
        let mut b = OpenHashMap::<i32, i32>::new();
        b.insert(2, 20);
        b.insert(1, 10);
        assert_eq!(a, b);
        b.insert(2, 99);
        assert_ne!(a, b);
    }

    #[test]
    fn open_hash_set_partial_eq_order_insensitive() {
        let mut a = OpenHashSet::<i32>::new();
        a.insert(1);
        a.insert(2);
        let mut b = OpenHashSet::<i32>::new();
        b.insert(2);
        b.insert(1);
        assert_eq!(a, b);
        b.insert(3);
        assert_ne!(a, b);
    }

    #[test]
    fn map_insert_get_remove() {
        let mut m = OpenHashMap::<i32, i32>::new();
        assert_eq!(m.insert(1, 10), None);
        assert_eq!(m.insert(2, 20), None);
        assert_eq!(m.insert(1, 99), Some(10));
        assert_eq!(m.get(&1), Some(&99));
        assert_eq!(m.get(&2), Some(&20));
        assert_eq!(m.get(&3), None);
        assert_eq!(m.remove(&1), Some(99));
        assert_eq!(m.get(&1), None);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn map_resize() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..200 {
            m.insert(i, i * 10);
        }
        assert_eq!(m.len(), 200);
        for i in 0..200 {
            assert_eq!(m.get(&i), Some(&(i * 10)));
        }
    }

    // Spec: load factor must stay strictly below 0.75. With a
    // capacity-16 table, the 12th distinct entry (size would reach
    // 12 == 16*0.75) must trigger a grow *before* it is stored.
    #[test]
    fn map_load_factor_strictly_below_three_quarters() {
        let mut m = OpenHashMap::<i32, i32>::new();
        assert_eq!(m.cap(), 16);
        for i in 0..11 {
            m.insert(i, i);
        }
        assert_eq!(m.cap(), 16);
        m.insert(11, 11); // 12th insert
        assert_eq!(m.cap(), 32);
        assert_eq!(m.len(), 12);
    }

    #[test]
    fn set_load_factor_strictly_below_three_quarters() {
        let mut s = OpenHashSet::<i32>::new();
        assert_eq!(s.entries.len(), 16);
        for i in 0..11 {
            s.insert(i);
        }
        assert_eq!(s.entries.len(), 16);
        s.insert(11); // 12th insert
        assert_eq!(s.entries.len(), 32);
        assert_eq!(s.len(), 12);
    }

    #[test]
    fn map_robin_hood_deletion() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..50 {
            m.insert(i, i);
        }
        for i in (0..50).step_by(2) {
            m.remove(&i);
        }
        assert_eq!(m.len(), 25);
        for i in (1..50).step_by(2) {
            assert_eq!(m.get(&i), Some(&i));
        }
    }

    #[test]
    fn map_delete_heavy() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..50_000 {
            m.insert(i, i);
        }
        for i in (0..50_000).step_by(2) {
            m.remove(&i);
        }
        for i in 50_000..75_000 {
            m.insert(i, i);
        }
        for i in 0..75_000 {
            m.remove(&i);
        }
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn map_clear() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.insert(1, 1);
        m.insert(2, 2);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.get(&1), None);
    }

    #[test]
    fn map_iter() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.insert(1, 10);
        m.insert(2, 20);
        let mut pairs: Vec<_> = m.iter().map(|(k, v)| (*k, *v)).collect();
        pairs.sort();
        assert_eq!(pairs, vec![(1, 10), (2, 20)]);
    }

    #[test]
    fn map_float_keys_via_hashable_newtype() {
        let mut m = OpenHashMap::<HashableF32, i32>::new();
        m.insert(HashableF32(1.5), 10);
        m.insert(HashableF32(2.5), 20);
        assert_eq!(m.get(&HashableF32(1.5)), Some(&10));
        assert_eq!(m.get(&HashableF32(3.5)), None);
        m.insert(HashableF32(f32::NAN), 99);
        assert_eq!(m.get(&HashableF32(f32::NAN)), Some(&99));
    }

    #[test]
    fn map_bool_keys() {
        let mut m = OpenHashMap::<bool, i32>::new();
        m.insert(true, 1);
        m.insert(false, 0);
        assert_eq!(m.get(&true), Some(&1));
        assert_eq!(m.get(&false), Some(&0));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn map_string_values() {
        // Phase 1 win: V no longer needs to be Copy.
        let mut m = OpenHashMap::<i32, String>::new();
        m.insert(1, "one".to_string());
        m.insert(2, "two".to_string());
        assert_eq!(m.get(&1), Some(&"one".to_string()));
        let popped = m.remove(&1);
        assert_eq!(popped.as_deref(), Some("one"));
    }

    #[test]
    fn map_vec_values() {
        let mut m = OpenHashMap::<i32, Vec<u8>>::new();
        m.insert(7, vec![1, 2, 3]);
        assert_eq!(m.get(&7).map(|v| v.len()), Some(3));
    }

    #[test]
    fn set_insert_remove_contains() {
        let mut s = OpenHashSet::<i32>::new();
        assert!(s.insert(1));
        assert!(s.insert(2));
        assert!(!s.insert(1));
        assert_eq!(s.len(), 2);
        assert!(s.contains(&1));
        assert!(s.remove(&1));
        assert!(!s.contains(&1));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn set_resize() {
        let mut s = OpenHashSet::<i32>::new();
        for i in 0..200 {
            s.insert(i);
        }
        assert_eq!(s.len(), 200);
        for i in 0..200 {
            assert!(s.contains(&i));
        }
    }

    #[test]
    fn set_robin_hood_deletion() {
        let mut s = OpenHashSet::<i32>::new();
        for i in 0..50 {
            s.insert(i);
        }
        for i in (0..50).step_by(2) {
            s.remove(&i);
        }
        assert_eq!(s.len(), 25);
        for i in (1..50).step_by(2) {
            assert!(s.contains(&i));
        }
    }

    #[test]
    fn set_float_via_hashable_newtype() {
        let mut s = OpenHashSet::<HashableF64>::new();
        s.insert(HashableF64(1.5));
        s.insert(HashableF64(2.5));
        s.insert(HashableF64(f64::NAN));
        assert!(s.contains(&HashableF64(1.5)));
        assert!(s.contains(&HashableF64(f64::NAN)));
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn set_iter() {
        let mut s = OpenHashSet::<i32>::new();
        s.insert(3);
        s.insert(1);
        s.insert(2);
        let mut vals: Vec<_> = s.iter().copied().collect();
        vals.sort();
        assert_eq!(vals, vec![1, 2, 3]);
    }

    #[test]
    fn map_get_mut() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.insert(1, 10);
        if let Some(v) = m.get_mut(&1) {
            *v += 5;
        }
        assert_eq!(m.get(&1), Some(&15));
    }

    #[test]
    fn map_try_reserve_grows_and_avoids_subsequent_resize() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.try_reserve(1000).unwrap();
        let reserved = m.entries.len();
        assert!(reserved >= 1000);
        for i in 0..1000 {
            m.insert(i, i * 2);
        }
        assert_eq!(reserved, m.entries.len());
        assert_eq!(m.len(), 1000);
        for i in 0..1000 {
            assert_eq!(m.get(&i), Some(&(i * 2)));
        }
    }

    #[test]
    fn map_try_reserve_is_idempotent() {
        let mut m = OpenHashMap::<i32, i32>::new();
        let before = m.entries.len();
        m.try_reserve(1).unwrap();
        assert_eq!(before, m.entries.len());
    }

    #[test]
    fn map_try_reserve_propagates_allocation_error() {
        let mut m = OpenHashMap::<i32, i32>::new();
        let result = m.try_reserve(usize::MAX / 2);
        assert!(result.is_err());
    }

    #[test]
    fn set_try_reserve_grows_and_avoids_subsequent_resize() {
        let mut s = OpenHashSet::<i32>::new();
        s.try_reserve(500).unwrap();
        let reserved = s.entries.len();
        assert!(reserved >= 500);
        for i in 0..500 {
            s.insert(i);
        }
        assert_eq!(reserved, s.entries.len());
        assert_eq!(s.len(), 500);
    }

    // ---- NaN tests, parity with the Go port ----

    #[test]
    fn map_nan_key_roundtrip_f32() {
        let mut m = OpenHashMap::<HashableF32, &'static str>::new();
        m.insert(HashableF32(f32::NAN), "nan");
        assert_eq!(m.get(&HashableF32(f32::NAN)), Some(&"nan"));
        assert!(m.contains_key(&HashableF32(f32::NAN)));
    }

    #[test]
    fn map_nan_key_roundtrip_f64() {
        let mut m = OpenHashMap::<HashableF64, &'static str>::new();
        m.insert(HashableF64(f64::NAN), "nan");
        assert_eq!(m.get(&HashableF64(f64::NAN)), Some(&"nan"));
        assert!(m.contains_key(&HashableF64(f64::NAN)));
    }

    #[test]
    fn set_nan_membership_f32() {
        let mut s = OpenHashSet::<HashableF32>::new();
        assert!(s.insert(HashableF32(f32::NAN)));
        assert!(!s.insert(HashableF32(f32::NAN))); // already present
        assert!(s.contains(&HashableF32(f32::NAN)));
    }

    #[test]
    fn set_nan_membership_f64() {
        let mut s = OpenHashSet::<HashableF64>::new();
        assert!(s.insert(HashableF64(f64::NAN)));
        assert!(!s.insert(HashableF64(f64::NAN)));
        assert!(s.contains(&HashableF64(f64::NAN)));
    }

    #[test]
    fn map_signed_zero_keys_are_distinct_f32() {
        // ±0.0 have different bit patterns; with bit-aware hashing they must
        // remain distinct keys (matches Go's math.Float32bits behavior).
        let mut m = OpenHashMap::<HashableF32, &'static str>::new();
        m.insert(HashableF32(0.0_f32), "pos");
        m.insert(HashableF32(-0.0_f32), "neg");
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&HashableF32(0.0_f32)), Some(&"pos"));
        assert_eq!(m.get(&HashableF32(-0.0_f32)), Some(&"neg"));
    }

    #[test]
    fn map_signed_zero_keys_are_distinct_f64() {
        let mut m = OpenHashMap::<HashableF64, &'static str>::new();
        m.insert(HashableF64(0.0_f64), "pos");
        m.insert(HashableF64(-0.0_f64), "neg");
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&HashableF64(0.0_f64)), Some(&"pos"));
        assert_eq!(m.get(&HashableF64(-0.0_f64)), Some(&"neg"));
    }

    #[test]
    fn map_inf_keys_f64() {
        let mut m = OpenHashMap::<HashableF64, i32>::new();
        m.insert(HashableF64(f64::INFINITY), 1);
        m.insert(HashableF64(f64::NEG_INFINITY), -1);
        assert_eq!(m.get(&HashableF64(f64::INFINITY)), Some(&1));
        assert_eq!(m.get(&HashableF64(f64::NEG_INFINITY)), Some(&-1));
    }

    #[test]
    fn map_nan_payload_distinct() {
        // Two NaNs with different bit payloads are distinct keys under the
        // to_bits()-based Hash/Eq contract. (Same canonicalisation choice
        // as the Go port.)
        let nan1 = f32::from_bits(0x7fc0_0001);
        let nan2 = f32::from_bits(0x7fc0_0002);
        assert!(nan1.is_nan() && nan2.is_nan());
        let mut m = OpenHashMap::<HashableF32, i32>::new();
        m.insert(HashableF32(nan1), 1);
        m.insert(HashableF32(nan2), 2);
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&HashableF32(nan1)), Some(&1));
        assert_eq!(m.get(&HashableF32(nan2)), Some(&2));
    }

    #[test]
    fn borrow_lookup_string_key() {
        let mut m = OpenHashMap::<String, i32>::new();
        m.insert("hello".to_string(), 1);
        // Query a String-keyed map with &str via Borrow.
        assert_eq!(m.get("hello"), Some(&1));
        assert!(m.contains_key("hello"));
        assert_eq!(m.remove("hello"), Some(1));

        let mut s = OpenHashSet::<String>::new();
        s.insert("world".to_string());
        assert!(s.contains("world"));
        assert!(s.remove("world"));
    }

    #[test]
    fn into_iter_from_iter_extend() {
        let mut m: OpenHashMap<i32, i32> = [(1, 10), (2, 20)].into_iter().collect();
        let borrowed: i32 = (&m).into_iter().map(|(_, v)| *v).sum();
        assert_eq!(borrowed, 30);
        m.extend([(3, 30)]);
        assert_eq!(m.len(), 3);
        let mut owned: Vec<(i32, i32)> = m.into_iter().collect();
        owned.sort();
        assert_eq!(owned, vec![(1, 10), (2, 20), (3, 30)]);

        let s: OpenHashSet<i32> = [1, 2, 3].into_iter().collect();
        let set_sum: i32 = (&s).into_iter().sum();
        assert_eq!(set_sum, 6);
        let mut set_owned: Vec<i32> = s.into_iter().collect();
        set_owned.sort();
        assert_eq!(set_owned, vec![1, 2, 3]);
    }

    // ---- Data pump (bulk_load / bulk_load_exact) ----

    use crate::bulk::{BulkError, DuplicatePolicy};

    // Zero mid-load rehash for bulk_load_exact at n = 3·2^k (3, 6, 12, 24, 48).
    #[test]
    fn map_bulk_load_exact_zero_rehash_at_3_times_pow2() {
        for &n in &[3usize, 6, 12, 24, 48] {
            let data: Vec<(i32, i32)> = (0..n as i32).map(|i| (i, i * 10)).collect();
            let m =
                OpenHashMap::<i32, i32>::bulk_load_exact(data, n, DuplicatePolicy::Error).unwrap();
            let cap_after = m.entries.len();
            assert_eq!(m.len(), n);
            // The capacity must already satisfy the strict growth predicate for
            // the full load: inserting the (n)th element never grows.
            assert!(
                !m.needs_resize() || n == 0,
                "table at n={n} would resize on the next insert (cap={cap_after})"
            );
            // Predicted capacity matches the documented formula.
            let expected_cap = crate::bulk::open_addressing_capacity(n, DEFAULT_CAPACITY);
            assert_eq!(cap_after, expected_cap, "capacity mismatch at n={n}");
            for i in 0..n as i32 {
                assert_eq!(m.get(&i), Some(&(i * 10)));
            }
        }
    }

    #[test]
    fn set_bulk_load_exact_zero_rehash_at_3_times_pow2() {
        for &n in &[3usize, 6, 12, 24, 48] {
            let data: Vec<i32> = (0..n as i32).collect();
            let s = OpenHashSet::<i32>::bulk_load_exact(data, n, DuplicatePolicy::Error).unwrap();
            assert_eq!(s.len(), n);
            assert!(!s.needs_resize() || n == 0);
            assert_eq!(
                s.entries.len(),
                crate::bulk::open_addressing_capacity(n, DEFAULT_CAPACITY)
            );
        }
    }

    // Bulk-built table must be byte-identical to a pre-sized incremental put
    // loop at the same final capacity. Naturally grown insertion may have a
    // different resize history and is not the byte-identity contract.
    #[test]
    fn map_bulk_load_byte_identical_to_presized_incremental() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::BuildHasherDefault;
        type Fixed = BuildHasherDefault<DefaultHasher>;
        let n = 100usize;
        let data: Vec<(i32, i32)> = (0..n as i32).map(|i| (i * 7, i)).collect();

        let bulk: OpenHashMap<i32, i32, Fixed> =
            OpenHashMap::bulk_load_exact(data.clone(), n, DuplicatePolicy::Error).unwrap();

        // Incremental: pre-reserve to the same final capacity, then insert.
        let mut inc: OpenHashMap<i32, i32, Fixed> = OpenHashMap::with_hasher(Fixed::default());
        inc.bulk_presize(n).unwrap();
        for (k, v) in &data {
            inc.insert(*k, *v);
        }
        assert_eq!(bulk.entries.len(), inc.entries.len());
        // Compare slot-by-slot.
        for (a, b) in bulk.entries.iter().zip(inc.entries.iter()) {
            match (a, b) {
                (MapSlot::Empty, MapSlot::Empty) => {}
                (
                    MapSlot::Occupied { key: ka, value: va },
                    MapSlot::Occupied { key: kb, value: vb },
                ) => {
                    assert_eq!((ka, va), (kb, vb));
                }
                _ => panic!("slot layout differs between bulk and incremental"),
            }
        }
    }

    #[test]
    fn entry_built_byte_identical_to_insert_built() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::BuildHasherDefault;
        type Fixed = BuildHasherDefault<DefaultHasher>;
        let data: Vec<(i32, i32)> = (0..200i32).map(|i| (i * 7 + 1, i)).collect();

        let mut via_insert: OpenHashMap<i32, i32, Fixed> =
            OpenHashMap::with_hasher(Fixed::default());
        for (k, v) in &data {
            via_insert.insert(*k, *v);
        }
        let mut via_entry: OpenHashMap<i32, i32, Fixed> =
            OpenHashMap::with_hasher(Fixed::default());
        for (k, v) in &data {
            via_entry.entry(*k).or_insert(*v);
        }

        assert_eq!(via_insert.len(), via_entry.len());
        assert_eq!(via_insert.entries.len(), via_entry.entries.len());
        for (a, b) in via_insert.entries.iter().zip(via_entry.entries.iter()) {
            match (a, b) {
                (MapSlot::Empty, MapSlot::Empty) => {}
                (
                    MapSlot::Occupied { key: ka, value: va },
                    MapSlot::Occupied { key: kb, value: vb },
                ) => assert_eq!((ka, va), (kb, vb)),
                _ => panic!("entry-built and insert-built slot layouts differ"),
            }
        }
    }

    #[test]
    fn entry_or_insert_and_and_modify() {
        let mut m: OpenHashMap<&str, i32> = OpenHashMap::new();
        for w in ["a", "b", "a", "a", "b", "c"] {
            m.entry(w).and_modify(|c| *c += 1).or_insert(1);
        }
        assert_eq!(m.get(&"a"), Some(&3));
        assert_eq!(m.get(&"b"), Some(&2));
        assert_eq!(m.get(&"c"), Some(&1));
        // or_insert on an existing key does not overwrite.
        assert_eq!(*m.entry("a").or_insert(999), 3);
        // or_default.
        *m.entry("d").or_default() += 5;
        assert_eq!(m.get(&"d"), Some(&5));
    }

    #[test]
    fn entry_remove_matches_remove() {
        // An OccupiedEntry::remove must backward-shift exactly like remove(),
        // leaving the table probe-consistent for the displaced keys.
        let mut via_entry: OpenHashMap<i32, i32> = (0..50).map(|i| (i, i * 2)).collect();
        let mut via_remove: OpenHashMap<i32, i32> = (0..50).map(|i| (i, i * 2)).collect();
        for k in [7, 13, 0, 49, 25] {
            let removed = match via_entry.entry(k) {
                Entry::Occupied(e) => Some(e.remove()),
                Entry::Vacant(_) => None,
            };
            assert_eq!(removed, via_remove.remove(&k));
        }
        // Both maps must still answer every surviving key correctly.
        for k in 0..50 {
            assert_eq!(via_entry.get(&k), via_remove.get(&k), "mismatch at {k}");
        }
        assert_eq!(via_entry.len(), via_remove.len());
    }

    #[test]
    fn entry_at_load_threshold_grows_even_for_existing_key() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::BuildHasherDefault;
        type Fixed = BuildHasherDefault<DefaultHasher>;
        let mut m: OpenHashMap<i32, i32, Fixed> = OpenHashMap::with_hasher(Fixed::default());
        let mut k = 0;
        while !m.needs_resize() {
            m.insert(k, k);
            k += 1;
        }
        let existing = k - 1; // last inserted key is present
        let cap_before = m.entries.len();
        // Reading an existing key through `entry` at the threshold still grows,
        // matching insert()'s resolve-resize-first contract (documented).
        let v = *m.entry(existing).or_insert(-1);
        assert!(
            m.entries.len() > cap_before,
            "entry at the load threshold must grow the table like insert"
        );
        assert_eq!(
            v, existing,
            "or_insert must not overwrite an existing value"
        );
    }

    #[test]
    fn map_retain_keeps_survivors_probe_consistent() {
        let mut m: OpenHashMap<i32, i32> = (0..100).map(|i| (i, i)).collect();
        m.retain(|k, v| {
            *v += 1000; // retain may mutate the value
            k % 3 == 0
        });
        assert_eq!(m.len(), 34); // 0,3,..,99
        for k in 0..100 {
            if k % 3 == 0 {
                assert_eq!(m.get(&k), Some(&(k + 1000)), "kept {k}");
            } else {
                assert_eq!(m.get(&k), None, "dropped {k}");
            }
        }
    }

    #[test]
    fn set_retain_keeps_survivors() {
        let mut s: OpenHashSet<i32> = (0..100).collect();
        s.retain(|k| k % 7 == 0);
        for k in 0..100 {
            assert_eq!(s.contains(&k), k % 7 == 0, "at {k}");
        }
    }

    #[test]
    fn map_bulk_load_duplicate_error_reports_index() {
        // Duplicate at the middle position.
        let data = vec![(1, 1), (2, 2), (2, 99), (3, 3)];
        let err = OpenHashMap::<i32, i32>::bulk_load(data, DuplicatePolicy::Error).unwrap_err();
        match err {
            BulkError::Duplicate { index } => assert_eq!(index, 2),
            other => panic!("expected Duplicate, got {other:?}"),
        }
    }

    #[test]
    fn map_bulk_load_ignore_duplicates_keeps_first() {
        let data = vec![(1, 10), (1, 20), (2, 30)];
        let m =
            OpenHashMap::<i32, i32>::bulk_load(data, DuplicatePolicy::IgnoreDuplicates).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&1), Some(&10)); // first wins
        assert_eq!(m.get(&2), Some(&30));
    }

    #[test]
    fn map_bulk_load_exact_size_exceeded() {
        // Source yields more than the declared n.
        let data = vec![(1, 1), (2, 2), (3, 3)];
        let err =
            OpenHashMap::<i32, i32>::bulk_load_exact(data, 2, DuplicatePolicy::Error).unwrap_err();
        assert!(matches!(err, BulkError::ExactSizeExceeded { expected: 2 }));
    }

    #[test]
    fn set_bulk_load_exact_overlong_duplicates_still_errors() {
        // Consumed source length is 3 > n = 2, even though the duplicate `1`
        // would be ignored and only 2 unique items inserted. The exact path must
        // enforce *consumed* length, not unique cardinality.
        let err = OpenHashSet::<i32>::bulk_load_exact(
            vec![1, 1, 2],
            2,
            DuplicatePolicy::IgnoreDuplicates,
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::ExactSizeExceeded { expected: 2 }));
    }

    #[test]
    fn map_bulk_load_exact_overlong_duplicates_still_errors() {
        let err = OpenHashMap::<i32, i32>::bulk_load_exact(
            vec![(1, 10), (1, 20), (2, 30)],
            2,
            DuplicatePolicy::IgnoreDuplicates,
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::ExactSizeExceeded { expected: 2 }));
    }

    #[test]
    fn map_bulk_load_empty() {
        let m = OpenHashMap::<i32, i32>::bulk_load_exact(Vec::new(), 0, DuplicatePolicy::Error)
            .unwrap();
        assert!(m.is_empty());
        let m2 = OpenHashMap::<i32, i32>::bulk_load(Vec::new(), DuplicatePolicy::Error).unwrap();
        assert!(m2.is_empty());
    }

    #[test]
    fn map_bulk_load_equals_incremental_observably() {
        let data: Vec<(i32, i32)> = (0..500).map(|i| (i, i * 3)).collect();
        let bulk = OpenHashMap::<i32, i32>::bulk_load_exact(
            data.clone(),
            data.len(),
            DuplicatePolicy::Error,
        )
        .unwrap();
        let mut inc = OpenHashMap::<i32, i32>::new();
        for (k, v) in &data {
            inc.insert(*k, *v);
        }
        assert_eq!(bulk, inc);
    }

    // NaN / ±0 / Inf flow through the pump path via HashableF32/F64.
    #[test]
    fn map_bulk_load_float_edge_cases() {
        let data = vec![
            (HashableF64(f64::NAN), 1),
            (HashableF64(0.0_f64), 2),
            (HashableF64(-0.0_f64), 3),
            (HashableF64(f64::INFINITY), 4),
            (HashableF64(f64::NEG_INFINITY), 5),
        ];
        let n = data.len();
        let m = OpenHashMap::<HashableF64, i32>::bulk_load_exact(data, n, DuplicatePolicy::Error)
            .unwrap();
        assert_eq!(m.len(), 5); // ±0 distinct
        assert_eq!(m.get(&HashableF64(f64::NAN)), Some(&1));
        assert_eq!(m.get(&HashableF64(0.0_f64)), Some(&2));
        assert_eq!(m.get(&HashableF64(-0.0_f64)), Some(&3));
        assert_eq!(m.get(&HashableF64(f64::INFINITY)), Some(&4));
        assert_eq!(m.get(&HashableF64(f64::NEG_INFINITY)), Some(&5));
    }

    #[test]
    fn set_bulk_load_dup_and_ignore() {
        let err = OpenHashSet::<i32>::bulk_load(vec![1, 2, 2], DuplicatePolicy::Error).unwrap_err();
        assert!(matches!(err, BulkError::Duplicate { index: 2 }));
        let s = OpenHashSet::<i32>::bulk_load(vec![1, 2, 2, 3], DuplicatePolicy::IgnoreDuplicates)
            .unwrap();
        assert_eq!(s.len(), 3);
    }

    // No leak on mid-load allocation failure: bulk_presize fails before any
    // element is consumed, so the (empty) map is dropped cleanly.
    #[test]
    fn map_bulk_load_alloc_failure_no_leak() {
        let huge = usize::MAX / 2;
        let data = std::iter::empty::<(i32, i32)>();
        let err = OpenHashMap::<i32, i32>::bulk_load_exact(data, huge, DuplicatePolicy::Error)
            .unwrap_err();
        assert!(matches!(err, BulkError::Alloc(_)));
    }

    #[test]
    fn map_with_fixed_hasher() {
        // Opt into a deterministic hasher via with_hasher.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::BuildHasherDefault;
        type Fixed = BuildHasherDefault<DefaultHasher>;
        let mut m: OpenHashMap<i32, i32, Fixed> = OpenHashMap::with_hasher(Fixed::default());
        m.insert(1, 10);
        m.insert(2, 20);
        assert_eq!(m.get(&1), Some(&10));
        assert_eq!(m.get(&2), Some(&20));
        // hasher() exposes the chosen BuildHasher.
        let _: &Fixed = m.hasher();

        // FromIterator/collect also honours a Default hasher type param.
        let s: OpenHashSet<i32, Fixed> = [1, 2, 3].into_iter().collect();
        assert_eq!(s.len(), 3);
        let _: &Fixed = s.hasher();
    }

    #[test]
    fn openhashset_set_algebra() {
        let sorted = |s: &OpenHashSet<i32>| {
            let mut v: Vec<i32> = s.iter().copied().collect();
            v.sort_unstable();
            v
        };
        let a: OpenHashSet<i32> = [1, 2, 3, 4].into_iter().collect();
        let b: OpenHashSet<i32> = [3, 4, 5, 6].into_iter().collect();

        assert_eq!(sorted(&a.union(&b)), vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(sorted(&a.intersection(&b)), vec![3, 4]);
        assert_eq!(sorted(&a.difference(&b)), vec![1, 2]);
        assert_eq!(sorted(&b.difference(&a)), vec![5, 6]);
        assert_eq!(sorted(&a.symmetric_difference(&b)), vec![1, 2, 5, 6]);

        // Intersection is order-independent (iterates the smaller set).
        assert_eq!(sorted(&b.intersection(&a)), vec![3, 4]);
    }

    #[test]
    fn openhashset_relational_predicates() {
        let a: OpenHashSet<i32> = [1, 2, 3].into_iter().collect();
        let sub: OpenHashSet<i32> = [2, 3].into_iter().collect();
        let disjoint: OpenHashSet<i32> = [7, 8].into_iter().collect();
        let empty: OpenHashSet<i32> = OpenHashSet::new();

        assert!(sub.is_subset(&a));
        assert!(!a.is_subset(&sub));
        assert!(a.is_superset(&sub));
        assert!(a.is_disjoint(&disjoint));
        assert!(!a.is_disjoint(&sub));
        // Empty set edge cases.
        assert!(empty.is_subset(&a));
        assert!(a.is_superset(&empty));
        assert!(empty.is_disjoint(&a));
    }
}
