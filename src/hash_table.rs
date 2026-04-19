// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Open-addressing hash table with linear probing and Robin Hood backward-shift deletion.
//!
//! Uses interleaved Entry structs for cache locality — key, value, and occupied flag
//! sit in the same cache line, minimizing memory loads per probe.

const DEFAULT_CAPACITY: usize = 16;
const LOAD_FACTOR_NUM: usize = 3;
const LOAD_FACTOR_DEN: usize = 4; // 0.75

/// Trait for primitive types that can be used as hash table keys.
pub trait PrimitiveKey: Copy + Default {
    fn hash_code(&self) -> u64;
    fn key_eq(&self, other: &Self) -> bool;
}

macro_rules! impl_primitive_key_int {
    ($($t:ty),*) => {
        $(
            impl PrimitiveKey for $t {
                #[inline]
                fn hash_code(&self) -> u64 {
                    (*self as u64).wrapping_mul(0x9E3779B9)
                }
                #[inline]
                fn key_eq(&self, other: &Self) -> bool {
                    *self == *other
                }
            }
        )*
    };
}

impl_primitive_key_int!(i8, i16, i32, i64);

impl PrimitiveKey for f32 {
    #[inline]
    fn hash_code(&self) -> u64 {
        (self.to_bits() as u64).wrapping_mul(0x9E3779B9)
    }
    #[inline]
    fn key_eq(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl PrimitiveKey for f64 {
    #[inline]
    fn hash_code(&self) -> u64 {
        self.to_bits().wrapping_mul(0x9E3779B9)
    }
    #[inline]
    fn key_eq(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl PrimitiveKey for bool {
    #[inline]
    fn hash_code(&self) -> u64 {
        (if *self { 1u64 } else { 0u64 }).wrapping_mul(0x9E3779B9)
    }
    #[inline]
    fn key_eq(&self, other: &Self) -> bool {
        *self == *other
    }
}

impl PrimitiveKey for char {
    #[inline]
    fn hash_code(&self) -> u64 {
        (*self as u64).wrapping_mul(0x9E3779B9)
    }
    #[inline]
    fn key_eq(&self, other: &Self) -> bool {
        *self == *other
    }
}

// ---------------------------------------------------------------------------
// MapEntry / SetEntry — interleaved for cache locality
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct MapEntry<K: Copy, V: Copy> {
    pub occupied: bool,
    pub key: K,
    pub value: V,
}

impl<K: Copy + Default, V: Copy + Default> Default for MapEntry<K, V> {
    fn default() -> Self {
        MapEntry {
            occupied: false,
            key: K::default(),
            value: V::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SetEntry<K: Copy> {
    pub occupied: bool,
    pub key: K,
}

impl<K: Copy + Default> Default for SetEntry<K> {
    fn default() -> Self {
        SetEntry {
            key: K::default(),
            occupied: false,
        }
    }
}

// ---------------------------------------------------------------------------
// OpenHashMap<K, V>
// ---------------------------------------------------------------------------

/// Open-addressing hash map with interleaved entries for cache locality.
#[derive(Debug, Clone)]
pub struct OpenHashMap<K: PrimitiveKey, V: Copy> {
    entries: Vec<MapEntry<K, V>>,
    size: usize,
}

impl<K: PrimitiveKey, V: Copy + Default> OpenHashMap<K, V> {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(DEFAULT_CAPACITY).next_power_of_two();
        OpenHashMap {
            entries: vec![MapEntry::default(); cap],
            size: 0,
        }
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
    fn needs_resize(&self) -> bool {
        (self.size + 1) * LOAD_FACTOR_DEN > self.cap() * LOAD_FACTOR_NUM
    }

    fn resize(&mut self) {
        // Infallible path: aborts on OOM via `Vec::with_capacity` /
        // `vec![...]`, matching the rest of the panic-on-OOM surface. If the
        // caller wants recoverable OOM they pre-reserve with `try_reserve`.
        self.grow_to_infallible((self.entries.len() * 2).max(DEFAULT_CAPACITY));
    }

    fn grow_to_infallible(&mut self, new_cap: usize) {
        if new_cap <= self.entries.len() {
            return;
        }
        let old = std::mem::replace(&mut self.entries, vec![MapEntry::default(); new_cap]);
        self.size = 0;
        for e in &old {
            if e.occupied {
                self.insert_no_resize(e.key, e.value);
            }
        }
    }

    /// Grows the backing buffer fallibly. Returns `TryReserveError` if the
    /// allocator cannot satisfy the request. Caller guarantees `new_cap`
    /// is a power of two and fits every live entry.
    fn grow_to(&mut self, new_cap: usize) -> Result<(), std::collections::TryReserveError> {
        if new_cap <= self.entries.len() {
            return Ok(());
        }
        let mut new_entries: Vec<MapEntry<K, V>> = Vec::new();
        new_entries.try_reserve_exact(new_cap)?;
        new_entries.resize(new_cap, MapEntry::default());
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for e in &old {
            if e.occupied {
                self.insert_no_resize(e.key, e.value);
            }
        }
        Ok(())
    }

    fn insert_no_resize(&mut self, key: K, value: V) {
        let mask = self.mask();
        let mut idx = key.hash_code() as usize & mask;
        loop {
            let e = &mut self.entries[idx];
            if !e.occupied {
                *e = MapEntry {
                    key,
                    value,
                    occupied: true,
                };
                self.size += 1;
                return;
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Ensures the table can accept `additional` more entries without a
    /// rehash. Returns `TryReserveError` if the allocator cannot satisfy
    /// the request. Idempotent when capacity is already sufficient.
    ///
    /// This is the hook the generated wrappers expose as `try_reserve`,
    /// enabling a "reserve fallibly, then put infallibly" usage pattern
    /// (see `docs/rust/error-handling.md`).
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        // Use saturating arithmetic so absurdly large requests don't panic;
        // they flow through to `Vec::try_reserve_exact`, which turns them
        // into a `CapacityOverflow` error.
        let needed = self.size.saturating_add(additional);
        let required = needed.saturating_mul(4) / 3 + 1;
        if required <= self.entries.len() {
            return Ok(());
        }
        let floor = required.max(DEFAULT_CAPACITY);
        let new_cap = floor.checked_next_power_of_two().unwrap_or(usize::MAX);
        self.grow_to(new_cap)
    }

    fn rehash_from(&mut self, deleted: usize) {
        let mask = self.mask();
        let _cap = self.cap();
        let mut gap = deleted;
        let mut idx = (deleted + 1) & mask;
        while self.entries[idx].occupied {
            let ideal = self.entries[idx].key.hash_code() as usize & mask;
            let dist_current = (idx.wrapping_sub(ideal)) & mask;
            let dist_gap = (gap.wrapping_sub(ideal)) & mask;
            if dist_current > dist_gap {
                self.entries[gap] = self.entries[idx];
                self.entries[idx] = MapEntry::default();
                gap = idx;
            }
            idx = (idx + 1) & mask;
            if idx == deleted {
                break;
            }
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.mask();
        let mut idx = key.hash_code() as usize & mask;
        loop {
            let e = &mut self.entries[idx];
            if !e.occupied {
                *e = MapEntry {
                    key,
                    value,
                    occupied: true,
                };
                self.size += 1;
                return None;
            }
            if e.key.key_eq(&key) {
                let old = e.value;
                e.value = value;
                return Some(old);
            }
            idx = (idx + 1) & mask;
        }
    }

    pub fn get(&self, key: K) -> Option<V> {
        if self.size == 0 {
            return None;
        }
        let mask = self.mask();
        let mut idx = key.hash_code() as usize & mask;
        loop {
            let e = &self.entries[idx];
            if !e.occupied {
                return None;
            }
            if e.key.key_eq(&key) {
                return Some(e.value);
            }
            idx = (idx + 1) & mask;
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        if self.size == 0 {
            return None;
        }
        let mask = self.mask();
        let mut idx = key.hash_code() as usize & mask;
        loop {
            let e = &self.entries[idx];
            if !e.occupied {
                return None;
            }
            if e.key.key_eq(&key) {
                return Some(&mut self.entries[idx].value);
            }
            idx = (idx + 1) & mask;
        }
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        if self.size == 0 {
            return None;
        }
        let mask = self.mask();
        let mut idx = key.hash_code() as usize & mask;
        loop {
            if !self.entries[idx].occupied {
                return None;
            }
            if self.entries[idx].key.key_eq(&key) {
                let old = self.entries[idx].value;
                self.entries[idx] = MapEntry::default();
                self.size -= 1;
                self.rehash_from(idx);
                return Some(old);
            }
            idx = (idx + 1) & mask;
        }
    }

    pub fn get_or_insert(&mut self, key: K, default: V) -> V {
        if let Some(v) = self.get(key) {
            return v;
        }
        self.insert(key, default);
        default
    }

    pub fn get_or_insert_with(&mut self, key: K, f: impl FnOnce() -> V) -> V {
        if let Some(v) = self.get(key) {
            return v;
        }
        let val = f();
        self.insert(key, val);
        val
    }

    pub fn contains_key(&self, key: K) -> bool {
        self.get(key).is_some()
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
        self.entries.fill(MapEntry::default());
        self.size = 0;
    }

    pub fn iter(&self) -> OpenHashMapIter<'_, K, V> {
        OpenHashMapIter {
            entries: &self.entries,
            pos: 0,
        }
    }
}

pub struct OpenHashMapIter<'a, K: Copy, V: Copy> {
    entries: &'a [MapEntry<K, V>],
    pos: usize,
}

impl<'a, K: Copy, V: Copy> Iterator for OpenHashMapIter<'a, K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            if self.entries[i].occupied {
                return Some((self.entries[i].key, self.entries[i].value));
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// OpenHashSet<K>
// ---------------------------------------------------------------------------

/// Open-addressing hash set with interleaved entries for cache locality.
#[derive(Debug, Clone)]
pub struct OpenHashSet<K: PrimitiveKey> {
    entries: Vec<SetEntry<K>>,
    size: usize,
}

impl<K: PrimitiveKey> OpenHashSet<K> {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(DEFAULT_CAPACITY).next_power_of_two();
        OpenHashSet {
            entries: vec![SetEntry::default(); cap],
            size: 0,
        }
    }

    #[inline]
    fn mask(&self) -> usize {
        self.entries.len() - 1
    }

    #[inline]
    fn needs_resize(&self) -> bool {
        (self.size + 1) * LOAD_FACTOR_DEN > self.entries.len() * LOAD_FACTOR_NUM
    }

    fn resize(&mut self) {
        self.grow_to_infallible((self.entries.len() * 2).max(DEFAULT_CAPACITY));
    }

    fn grow_to_infallible(&mut self, new_cap: usize) {
        if new_cap <= self.entries.len() {
            return;
        }
        let old = std::mem::replace(&mut self.entries, vec![SetEntry::default(); new_cap]);
        self.size = 0;
        for e in &old {
            if e.occupied {
                self.insert_no_resize(e.key);
            }
        }
    }

    fn grow_to(&mut self, new_cap: usize) -> Result<(), std::collections::TryReserveError> {
        if new_cap <= self.entries.len() {
            return Ok(());
        }
        let mut new_entries: Vec<SetEntry<K>> = Vec::new();
        new_entries.try_reserve_exact(new_cap)?;
        new_entries.resize(new_cap, SetEntry::default());
        let old = std::mem::replace(&mut self.entries, new_entries);
        self.size = 0;
        for e in &old {
            if e.occupied {
                self.insert_no_resize(e.key);
            }
        }
        Ok(())
    }

    fn insert_no_resize(&mut self, value: K) {
        let mask = self.mask();
        let mut idx = value.hash_code() as usize & mask;
        loop {
            if !self.entries[idx].occupied {
                self.entries[idx] = SetEntry {
                    key: value,
                    occupied: true,
                };
                self.size += 1;
                return;
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Ensures the table can accept `additional` more entries without a
    /// rehash. Returns `TryReserveError` if the allocator cannot satisfy
    /// the request. Idempotent when capacity is already sufficient.
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

    fn rehash_from(&mut self, deleted: usize) {
        let mask = self.mask();
        let mut gap = deleted;
        let mut idx = (deleted + 1) & mask;
        while self.entries[idx].occupied {
            let ideal = self.entries[idx].key.hash_code() as usize & mask;
            let dist_current = (idx.wrapping_sub(ideal)) & mask;
            let dist_gap = (gap.wrapping_sub(ideal)) & mask;
            if dist_current > dist_gap {
                self.entries[gap] = self.entries[idx];
                self.entries[idx] = SetEntry::default();
                gap = idx;
            }
            idx = (idx + 1) & mask;
            if idx == deleted {
                break;
            }
        }
    }

    pub fn add(&mut self, value: K) -> bool {
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.mask();
        let mut idx = value.hash_code() as usize & mask;
        loop {
            if !self.entries[idx].occupied {
                self.entries[idx] = SetEntry {
                    key: value,
                    occupied: true,
                };
                self.size += 1;
                return true;
            }
            if self.entries[idx].key.key_eq(&value) {
                return false;
            }
            idx = (idx + 1) & mask;
        }
    }

    pub fn remove(&mut self, value: K) -> bool {
        if self.size == 0 {
            return false;
        }
        let mask = self.mask();
        let mut idx = value.hash_code() as usize & mask;
        loop {
            if !self.entries[idx].occupied {
                return false;
            }
            if self.entries[idx].key.key_eq(&value) {
                self.entries[idx] = SetEntry::default();
                self.size -= 1;
                self.rehash_from(idx);
                return true;
            }
            idx = (idx + 1) & mask;
        }
    }

    pub fn contains(&self, value: K) -> bool {
        if self.size == 0 {
            return false;
        }
        let mask = self.mask();
        let mut idx = value.hash_code() as usize & mask;
        loop {
            if !self.entries[idx].occupied {
                return false;
            }
            if self.entries[idx].key.key_eq(&value) {
                return true;
            }
            idx = (idx + 1) & mask;
        }
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
        self.entries.fill(SetEntry::default());
        self.size = 0;
    }

    pub fn iter(&self) -> OpenHashSetIter<'_, K> {
        OpenHashSetIter {
            entries: &self.entries,
            pos: 0,
        }
    }
}

pub struct OpenHashSetIter<'a, K: Copy> {
    entries: &'a [SetEntry<K>],
    pos: usize,
}

impl<'a, K: Copy> Iterator for OpenHashSetIter<'a, K> {
    type Item = K;
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            if self.entries[i].occupied {
                return Some(self.entries[i].key);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_insert_get_remove() {
        let mut m = OpenHashMap::<i32, i32>::new();
        assert_eq!(m.insert(1, 10), None);
        assert_eq!(m.insert(2, 20), None);
        assert_eq!(m.insert(1, 99), Some(10));
        assert_eq!(m.get(1), Some(99));
        assert_eq!(m.get(2), Some(20));
        assert_eq!(m.get(3), None);
        assert_eq!(m.remove(1), Some(99));
        assert_eq!(m.get(1), None);
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
            assert_eq!(m.get(i), Some(i * 10));
        }
    }

    #[test]
    fn map_robin_hood_deletion() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..50 {
            m.insert(i, i);
        }
        for i in (0..50).step_by(2) {
            m.remove(i);
        }
        assert_eq!(m.len(), 25);
        for i in (1..50).step_by(2) {
            assert_eq!(m.get(i), Some(i));
        }
    }

    #[test]
    fn map_delete_heavy() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..50_000 {
            m.insert(i, i);
        }
        for i in (0..50_000).step_by(2) {
            m.remove(i);
        }
        for i in 50_000..75_000 {
            m.insert(i, i);
        }
        for i in 0..75_000 {
            m.remove(i);
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
        assert_eq!(m.get(1), None);
    }

    #[test]
    fn map_iter() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.insert(1, 10);
        m.insert(2, 20);
        let mut pairs: Vec<_> = m.iter().collect();
        pairs.sort();
        assert_eq!(pairs, vec![(1, 10), (2, 20)]);
    }

    #[test]
    fn map_float_keys() {
        let mut m = OpenHashMap::<f32, i32>::new();
        m.insert(1.5, 10);
        m.insert(2.5, 20);
        assert_eq!(m.get(1.5), Some(10));
        assert_eq!(m.get(3.5), None);
        m.insert(f32::NAN, 99);
        assert_eq!(m.get(f32::NAN), Some(99));
    }

    #[test]
    fn map_bool_keys() {
        let mut m = OpenHashMap::<bool, i32>::new();
        m.insert(true, 1);
        m.insert(false, 0);
        assert_eq!(m.get(true), Some(1));
        assert_eq!(m.get(false), Some(0));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn set_add_remove_contains() {
        let mut s = OpenHashSet::<i32>::new();
        assert!(s.add(1));
        assert!(s.add(2));
        assert!(!s.add(1));
        assert_eq!(s.len(), 2);
        assert!(s.contains(1));
        assert!(s.remove(1));
        assert!(!s.contains(1));
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
            assert!(s.contains(i));
        }
    }

    #[test]
    fn set_robin_hood_deletion() {
        let mut s = OpenHashSet::<i32>::new();
        for i in 0..50 {
            s.add(i);
        }
        for i in (0..50).step_by(2) {
            s.remove(i);
        }
        assert_eq!(s.len(), 25);
        for i in (1..50).step_by(2) {
            assert!(s.contains(i));
        }
    }

    #[test]
    fn set_float_values() {
        let mut s = OpenHashSet::<f64>::new();
        s.add(1.5);
        s.add(2.5);
        s.add(f64::NAN);
        assert!(s.contains(1.5));
        assert!(s.contains(f64::NAN));
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn set_iter() {
        let mut s = OpenHashSet::<i32>::new();
        s.add(3);
        s.add(1);
        s.add(2);
        let mut vals: Vec<_> = s.iter().collect();
        vals.sort();
        assert_eq!(vals, vec![1, 2, 3]);
    }

    #[test]
    fn map_get_mut() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.insert(1, 10);
        if let Some(v) = m.get_mut(1) {
            *v += 5;
        }
        assert_eq!(m.get(1), Some(15));
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
        // Reservation was sufficient — no further rehashing happened.
        assert_eq!(reserved, m.entries.len());
        assert_eq!(m.len(), 1000);
        for i in 0..1000 {
            assert_eq!(m.get(i), Some(i * 2));
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
        // `isize::MAX / mem::size_of::<MapEntry>()` is the documented ceiling
        // for `Vec::try_reserve_exact` — anything larger is guaranteed to
        // return `TryReserveError::CapacityOverflow`.
        let result = m.try_reserve(usize::MAX / 2);
        assert!(
            result.is_err(),
            "expected allocator failure for absurd reserve size"
        );
    }

    #[test]
    fn map_try_reserve_preserves_entries() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..10 {
            m.insert(i, i);
        }
        m.try_reserve(500).unwrap();
        assert!(m.entries.len() >= 500);
        assert_eq!(m.len(), 10);
        for i in 0..10 {
            assert_eq!(m.get(i), Some(i));
        }
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

    #[test]
    fn set_try_reserve_propagates_allocation_error() {
        let mut s = OpenHashSet::<i32>::new();
        let result = s.try_reserve(usize::MAX / 2);
        assert!(
            result.is_err(),
            "expected allocator failure for absurd reserve size"
        );
    }
}
