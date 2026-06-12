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
}
