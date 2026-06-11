// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Open-addressing hash table with linear probing and Robin Hood backward-shift deletion.
//!
//! Ported from Eclipse Collections' primitive hash tables. Uses interleaved
//! `MapEntry { occupied, key, value }` structs for cache locality — one cache
//! line covers the occupancy flag, key, and value together, minimizing memory
//! loads per probe.
//!
//! Generic over any `K: Hash + Eq` and any `V`. For `f32`/`f64` keys, wrap in
//! [`crate::hashable_float::HashableF32`] / [`crate::hashable_float::HashableF64`]
//! to get bit-pattern hashing (NaN-aware, ±0 distinct).

use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

const DEFAULT_CAPACITY: usize = 16;
const LOAD_FACTOR_NUM: usize = 3;
const LOAD_FACTOR_DEN: usize = 4; // 0.75

// ---------------------------------------------------------------------------
// MapEntry / SetEntry — interleaved for cache locality
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct MapEntry<K, V> {
    occupied: bool,
    key: Option<K>,
    value: Option<V>,
}

impl<K, V> Default for MapEntry<K, V> {
    fn default() -> Self {
        MapEntry {
            occupied: false,
            key: None,
            value: None,
        }
    }
}

#[derive(Debug, Clone)]
struct SetEntry<K> {
    occupied: bool,
    key: Option<K>,
}

impl<K> Default for SetEntry<K> {
    fn default() -> Self {
        SetEntry {
            occupied: false,
            key: None,
        }
    }
}

// Run the key through std's `DefaultHasher` (SipHash13) — already produces
// well-mixed 64-bit output, so no Fibonacci/spread multiplier is added on
// top. A future optimization would be a faster, primitive-specialised hasher
// (e.g. FxHash) behind a feature flag, but DefaultHasher is the safe default
// and matches what `std::HashMap` uses out of the box.
#[inline]
fn spread<K: Hash + ?Sized>(key: &K) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

// ---------------------------------------------------------------------------
// OpenHashMap<K, V>
// ---------------------------------------------------------------------------

/// Open-addressing hash map with interleaved entries for cache locality.
///
/// Accepts any `K: Hash + Eq` (including object types like `String`, not just
/// primitives) and any `V` (including non-`Copy` types like `String`, `Vec`,
/// or user structs). For `f32`/`f64` keys, wrap them in
/// [`crate::hashable_float::HashableF32`] or
/// [`crate::hashable_float::HashableF64`].
#[derive(Debug, Clone)]
pub struct OpenHashMap<K, V> {
    entries: Vec<MapEntry<K, V>>,
    size: usize,
}

impl<K, V> Default for OpenHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> OpenHashMap<K, V> {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(DEFAULT_CAPACITY).next_power_of_two();
        let mut entries = Vec::with_capacity(cap);
        entries.resize_with(cap, MapEntry::default);
        OpenHashMap { entries, size: 0 }
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
            *e = MapEntry::default();
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
}

impl<K: Hash + Eq, V> OpenHashMap<K, V> {
    /// Inserts a key-value pair. Returns the old value if the key was already
    /// present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.mask();
        let mut idx = (spread(&key) as usize) & mask;
        loop {
            let e = &mut self.entries[idx];
            if !e.occupied {
                e.occupied = true;
                e.key = Some(key);
                e.value = Some(value);
                self.size += 1;
                return None;
            }
            if e.key.as_ref().unwrap() == &key {
                return e.value.replace(value);
            }
            idx = (idx + 1) & mask;
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
        let mut idx = (spread(key) as usize) & mask;
        loop {
            let e = &self.entries[idx];
            if !e.occupied {
                return None;
            }
            if e.key.as_ref().unwrap().borrow() == key {
                return e.value.as_ref();
            }
            idx = (idx + 1) & mask;
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
        let mut idx = (spread(key) as usize) & mask;
        loop {
            if !self.entries[idx].occupied {
                return None;
            }
            if self.entries[idx].key.as_ref().unwrap().borrow() == key {
                return self.entries[idx].value.as_mut();
            }
            idx = (idx + 1) & mask;
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
        let mut idx = (spread(key) as usize) & mask;
        loop {
            if !self.entries[idx].occupied {
                return None;
            }
            if self.entries[idx].key.as_ref().unwrap().borrow() == key {
                let mut taken = std::mem::take(&mut self.entries[idx]);
                self.size -= 1;
                self.rehash_from(idx);
                return taken.value.take();
            }
            idx = (idx + 1) & mask;
        }
    }

    fn rehash_from(&mut self, deleted: usize) {
        let mask = self.mask();
        let mut gap = deleted;
        let mut idx = (deleted + 1) & mask;
        while self.entries[idx].occupied {
            let key_ref = self.entries[idx].key.as_ref().unwrap();
            let ideal = (spread(key_ref) as usize) & mask;
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
        let mut new_entries: Vec<MapEntry<K, V>> = Vec::with_capacity(new_cap);
        new_entries.resize_with(new_cap, MapEntry::default);
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for mut e in old.into_iter() {
            if e.occupied {
                let k = e.key.take().unwrap();
                let v = e.value.take().unwrap();
                self.insert_no_resize(k, v);
            }
        }
    }

    fn grow_to(&mut self, new_cap: usize) -> Result<(), std::collections::TryReserveError> {
        if new_cap <= self.entries.len() {
            return Ok(());
        }
        let mut new_entries: Vec<MapEntry<K, V>> = Vec::new();
        new_entries.try_reserve_exact(new_cap)?;
        new_entries.resize_with(new_cap, MapEntry::default);
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for mut e in old.into_iter() {
            if e.occupied {
                let k = e.key.take().unwrap();
                let v = e.value.take().unwrap();
                self.insert_no_resize(k, v);
            }
        }
        Ok(())
    }

    fn insert_no_resize(&mut self, key: K, value: V) {
        let mask = self.mask();
        let mut idx = (spread(&key) as usize) & mask;
        loop {
            let e = &mut self.entries[idx];
            if !e.occupied {
                e.occupied = true;
                e.key = Some(key);
                e.value = Some(value);
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

pub struct OpenHashMapIter<'a, K, V> {
    entries: &'a [MapEntry<K, V>],
    pos: usize,
}

impl<'a, K, V> Iterator for OpenHashMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            let e = &self.entries[i];
            if e.occupied {
                return Some((e.key.as_ref().unwrap(), e.value.as_ref().unwrap()));
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// OpenHashSet<K>
// ---------------------------------------------------------------------------

/// Open-addressing hash set with interleaved entries.
#[derive(Debug, Clone)]
pub struct OpenHashSet<K> {
    entries: Vec<SetEntry<K>>,
    size: usize,
}

impl<K> Default for OpenHashSet<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K> OpenHashSet<K> {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(DEFAULT_CAPACITY).next_power_of_two();
        let mut entries = Vec::with_capacity(cap);
        entries.resize_with(cap, SetEntry::default);
        OpenHashSet { entries, size: 0 }
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
            *e = SetEntry::default();
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
}

impl<K: Hash + Eq> OpenHashSet<K> {
    /// Adds a value. Returns `true` if it was newly inserted (not already present).
    pub fn add(&mut self, value: K) -> bool {
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.mask();
        let mut idx = (spread(&value) as usize) & mask;
        loop {
            let e = &mut self.entries[idx];
            if !e.occupied {
                e.occupied = true;
                e.key = Some(value);
                self.size += 1;
                return true;
            }
            if e.key.as_ref().unwrap() == &value {
                return false;
            }
            idx = (idx + 1) & mask;
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
        let mut idx = (spread(value) as usize) & mask;
        loop {
            let e = &self.entries[idx];
            if !e.occupied {
                return false;
            }
            if e.key.as_ref().unwrap().borrow() == value {
                return true;
            }
            idx = (idx + 1) & mask;
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
        let mut idx = (spread(value) as usize) & mask;
        loop {
            if !self.entries[idx].occupied {
                return false;
            }
            if self.entries[idx].key.as_ref().unwrap().borrow() == value {
                self.entries[idx] = SetEntry::default();
                self.size -= 1;
                self.rehash_from(idx);
                return true;
            }
            idx = (idx + 1) & mask;
        }
    }

    fn rehash_from(&mut self, deleted: usize) {
        let mask = self.mask();
        let mut gap = deleted;
        let mut idx = (deleted + 1) & mask;
        while self.entries[idx].occupied {
            let key_ref = self.entries[idx].key.as_ref().unwrap();
            let ideal = (spread(key_ref) as usize) & mask;
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
        let mut new_entries: Vec<SetEntry<K>> = Vec::with_capacity(new_cap);
        new_entries.resize_with(new_cap, SetEntry::default);
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for mut e in old.into_iter() {
            if e.occupied {
                let k = e.key.take().unwrap();
                self.insert_no_resize(k);
            }
        }
    }

    fn grow_to(&mut self, new_cap: usize) -> Result<(), std::collections::TryReserveError> {
        if new_cap <= self.entries.len() {
            return Ok(());
        }
        let mut new_entries: Vec<SetEntry<K>> = Vec::new();
        new_entries.try_reserve_exact(new_cap)?;
        new_entries.resize_with(new_cap, SetEntry::default);
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for mut e in old.into_iter() {
            if e.occupied {
                let k = e.key.take().unwrap();
                self.insert_no_resize(k);
            }
        }
        Ok(())
    }

    fn insert_no_resize(&mut self, value: K) {
        let mask = self.mask();
        let mut idx = (spread(&value) as usize) & mask;
        loop {
            let e = &mut self.entries[idx];
            if !e.occupied {
                e.occupied = true;
                e.key = Some(value);
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

    pub fn iter(&self) -> OpenHashSetIter<'_, K> {
        OpenHashSetIter {
            entries: &self.entries,
            pos: 0,
        }
    }
}

pub struct OpenHashSetIter<'a, K> {
    entries: &'a [SetEntry<K>],
    pos: usize,
}

impl<'a, K> Iterator for OpenHashSetIter<'a, K> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            if self.entries[i].occupied {
                return Some(self.entries[i].key.as_ref().unwrap());
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Standard-library trait impls (additive idiom layer)
// ---------------------------------------------------------------------------

impl<'a, K: Hash + Eq, V> IntoIterator for &'a OpenHashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = OpenHashMapIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Owning iterator over an `OpenHashMap`'s `(K, V)` pairs (unspecified order).
pub struct OpenHashMapIntoIter<K, V> {
    inner: std::vec::IntoIter<MapEntry<K, V>>,
}

impl<K, V> Iterator for OpenHashMapIntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        for mut e in self.inner.by_ref() {
            if e.occupied {
                return Some((e.key.take().unwrap(), e.value.take().unwrap()));
            }
        }
        None
    }
}

impl<K, V> IntoIterator for OpenHashMap<K, V> {
    type Item = (K, V);
    type IntoIter = OpenHashMapIntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        OpenHashMapIntoIter {
            inner: self.entries.into_iter(),
        }
    }
}

impl<K: Hash + Eq, V> FromIterator<(K, V)> for OpenHashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut m = OpenHashMap::new();
        for (k, v) in iter {
            m.insert(k, v);
        }
        m
    }
}

impl<K: Hash + Eq, V> Extend<(K, V)> for OpenHashMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

/// Order-insensitive equality: same length and every key maps to an equal value.
impl<K: Hash + Eq, V: PartialEq> PartialEq for OpenHashMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<K: Hash + Eq, V: Eq> Eq for OpenHashMap<K, V> {}

impl<'a, K: Hash + Eq> IntoIterator for &'a OpenHashSet<K> {
    type Item = &'a K;
    type IntoIter = OpenHashSetIter<'a, K>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Owning iterator over an `OpenHashSet`'s elements (unspecified order).
pub struct OpenHashSetIntoIter<K> {
    inner: std::vec::IntoIter<SetEntry<K>>,
}

impl<K> Iterator for OpenHashSetIntoIter<K> {
    type Item = K;
    fn next(&mut self) -> Option<Self::Item> {
        for mut e in self.inner.by_ref() {
            if e.occupied {
                return Some(e.key.take().unwrap());
            }
        }
        None
    }
}

impl<K> IntoIterator for OpenHashSet<K> {
    type Item = K;
    type IntoIter = OpenHashSetIntoIter<K>;
    fn into_iter(self) -> Self::IntoIter {
        OpenHashSetIntoIter {
            inner: self.entries.into_iter(),
        }
    }
}

impl<K: Hash + Eq> FromIterator<K> for OpenHashSet<K> {
    fn from_iter<I: IntoIterator<Item = K>>(iter: I) -> Self {
        let mut s = OpenHashSet::new();
        for k in iter {
            s.add(k);
        }
        s
    }
}

impl<K: Hash + Eq> Extend<K> for OpenHashSet<K> {
    fn extend<I: IntoIterator<Item = K>>(&mut self, iter: I) {
        for k in iter {
            self.add(k);
        }
    }
}

/// Order-insensitive equality: same length and every element is present in both.
impl<K: Hash + Eq> PartialEq for OpenHashSet<K> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|k| other.contains(k))
    }
}

impl<K: Hash + Eq> Eq for OpenHashSet<K> {}

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
        a.add(1);
        a.add(2);
        let mut b = OpenHashSet::<i32>::new();
        b.add(2);
        b.add(1);
        assert_eq!(a, b);
        b.add(3);
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
            s.add(i);
        }
        assert_eq!(s.entries.len(), 16);
        s.add(11); // 12th insert
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
    fn set_add_remove_contains() {
        let mut s = OpenHashSet::<i32>::new();
        assert!(s.add(1));
        assert!(s.add(2));
        assert!(!s.add(1));
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
            s.add(i);
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
            s.add(i);
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
        s.add(HashableF64(1.5));
        s.add(HashableF64(2.5));
        s.add(HashableF64(f64::NAN));
        assert!(s.contains(&HashableF64(1.5)));
        assert!(s.contains(&HashableF64(f64::NAN)));
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn set_iter() {
        let mut s = OpenHashSet::<i32>::new();
        s.add(3);
        s.add(1);
        s.add(2);
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
            s.add(i);
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
        assert!(s.add(HashableF32(f32::NAN)));
        assert!(!s.add(HashableF32(f32::NAN))); // already present
        assert!(s.contains(&HashableF32(f32::NAN)));
    }

    #[test]
    fn set_nan_membership_f64() {
        let mut s = OpenHashSet::<HashableF64>::new();
        assert!(s.add(HashableF64(f64::NAN)));
        assert!(!s.add(HashableF64(f64::NAN)));
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
        s.add("world".to_string());
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
}
