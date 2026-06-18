// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Open-addressing hash table using grouped Swiss-table probing.
//!
//! `OpenHashMap`/`OpenHashSet` store entries in a flat `Box<[MaybeUninit<…>]>`
//! alongside a control byte array. Each bucket's control byte is one of `EMPTY`
//! (`0xFF`), `DELETED` (`0x80`), or a 7-bit hash tag (`0x00..=0x7F`) marking a
//! FULL slot. Lookups probe by *groups* of [`GROUP_WIDTH`] buckets using a SWAR
//! (SIMD-within-a-register) byte matcher over a little-endian `u64`, then a
//! triangular group sequence that visits every group exactly once. Only FULL
//! slots hold initialized entries; `EMPTY`/`DELETED` slots are uninitialized
//! `MaybeUninit` memory that is never read or dropped as an `Entry`.
//!
//! The maps and sets are generic over the hasher (`S: BuildHasher`), defaulting
//! to [`std::collections::hash_map::RandomState`] for HashDoS resistance — the
//! same default `std::collections::HashMap` uses. Opt into a faster, fixed
//! hasher (FxHash, AHash, …) with [`OpenHashMap::with_hasher`] /
//! [`OpenHashSet::with_hasher`].
//!
//! Generic over any `K: Hash + Eq` and any `V`. For `f32`/`f64` keys, wrap in
//! [`crate::hashable_float::HashableF32`] / [`crate::hashable_float::HashableF64`]
//! to get bit-pattern hashing (NaN-aware, ±0 distinct). Hash iteration order is
//! unspecified.

use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::collections::TryReserveError;
use std::fmt;
use std::hash::{BuildHasher, Hash};
use std::mem::MaybeUninit;

// ---------------------------------------------------------------------------
// Constants and control-byte encoding
// ---------------------------------------------------------------------------

const GROUP_WIDTH: usize = 8;
const MIN_CAPACITY: usize = 16;
const EMPTY: u8 = 0xFF;
const DELETED: u8 = 0x80;

const ONES: u64 = 0x0101_0101_0101_0101;
const HIGHS: u64 = 0x8080_8080_8080_8080;

/// `cap * 7 / 8`.
#[inline]
fn max_load(cap: usize) -> usize {
    cap / 8 * 7 + (cap % 8) * 7 / 8
}

/// Smallest power-of-two `cap >= MIN_CAPACITY` with `n <= max_load(cap)`.
fn capacity_for(n: usize) -> usize {
    let mut cap = MIN_CAPACITY;
    while max_load(cap) < n {
        // cap stays a power of two; guard against overflow.
        cap = cap.checked_mul(2).expect("capacity overflow");
    }
    cap
}

// ---------------------------------------------------------------------------
// SWAR group matcher
// ---------------------------------------------------------------------------

/// A bitmask over the 8 lanes of a group. Lane `i` is "set" iff bit `8*i+7`
/// (the high bit of byte `i`) is 1; all other bits are 0. Produced by the
/// `match_*` helpers below.
#[derive(Clone, Copy)]
struct BitMask(u64);

impl BitMask {
    #[inline]
    fn any(self) -> bool {
        self.0 != 0
    }

    /// Lowest set lane index (0..GROUP_WIDTH), or `None` if empty.
    #[inline]
    fn lowest(self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some((self.0.trailing_zeros() >> 3) as usize)
        }
    }
}

impl Iterator for BitMask {
    type Item = usize;
    #[inline]
    fn next(&mut self) -> Option<usize> {
        if self.0 == 0 {
            return None;
        }
        let lane = (self.0.trailing_zeros() >> 3) as usize;
        self.0 &= self.0 - 1; // clear lowest set bit
        Some(lane)
    }
}

/// Loads the eight control bytes starting at `i` as a little-endian `u64`.
/// `ctrl` must have at least `i + GROUP_WIDTH` bytes (the mirror suffix
/// guarantees this for any `i < cap`).
#[inline]
fn load_group(ctrl: &[u8], i: usize) -> u64 {
    let bytes: [u8; 8] = ctrl[i..i + GROUP_WIDTH].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

#[inline]
fn rep(b: u8) -> u64 {
    ONES.wrapping_mul(b as u64)
}

/// Classic SWAR zero-byte detector. A lane's high bit is set in the result iff
/// the corresponding byte of `x` is `0x00`.
#[inline]
fn haszero(x: u64) -> u64 {
    x.wrapping_sub(ONES) & !x & HIGHS
}

#[inline]
fn match_byte(g: u64, b: u8) -> BitMask {
    BitMask(haszero(g ^ rep(b)))
}

#[inline]
fn match_empty(g: u64) -> BitMask {
    // EMPTY = 0xFF.
    match_byte(g, EMPTY)
}

#[inline]
fn match_deleted(g: u64) -> BitMask {
    // DELETED = 0x80.
    match_byte(g, DELETED)
}

/// Triangular group probe sequence. Yields the starting bucket index of each
/// group; for power-of-two `cap` (a multiple of `GROUP_WIDTH`) it visits every
/// group exactly once before repeating.
struct ProbeSeq {
    group: usize,
    stride: usize,
    mask: usize,
}

impl ProbeSeq {
    #[inline]
    fn new(hash: u64, cap: usize) -> Self {
        let mask = cap - 1;
        let bucket = ((hash >> 7) as usize) & mask;
        let group = bucket & !(GROUP_WIDTH - 1);
        ProbeSeq {
            group,
            stride: 0,
            mask,
        }
    }

    #[inline]
    fn next_group(&mut self) -> usize {
        let g = self.group;
        self.stride += GROUP_WIDTH;
        self.group = (self.group + self.stride) & self.mask;
        g
    }
}

// ---------------------------------------------------------------------------
// Entry payloads
// ---------------------------------------------------------------------------

struct Entry<K, V> {
    key: K,
    value: V,
}

struct SetEntry<K> {
    key: K,
}

// ===========================================================================
// OpenHashMap<K, V, S>
// ===========================================================================

/// Open-addressing hash map with grouped Swiss-table probing and a pluggable
/// hasher.
///
/// Accepts any `K: Hash + Eq` (including object types like `String`, not just
/// primitives) and any `V` (including non-`Copy` types like `String`, `Vec`,
/// or user structs). The hasher `S` defaults to [`RandomState`]; use
/// [`OpenHashMap::with_hasher`] for a fixed/faster hasher. For `f32`/`f64`
/// keys, wrap them in [`crate::hashable_float::HashableF32`] or
/// [`crate::hashable_float::HashableF64`].
pub struct OpenHashMap<K, V, S = RandomState> {
    ctrl: Box<[u8]>,
    entries: Box<[MaybeUninit<Entry<K, V>>]>,
    len: usize,
    growth_left: usize,
    hasher: S,
}

/// Builds the initial empty control array (`cap + GROUP_WIDTH` bytes, all
/// `EMPTY`) for a given capacity.
fn empty_ctrl(cap: usize) -> Box<[u8]> {
    vec![EMPTY; cap + GROUP_WIDTH].into_boxed_slice()
}

fn uninit_entries<T>(cap: usize) -> Box<[MaybeUninit<T>]> {
    let mut v = Vec::with_capacity(cap);
    for _ in 0..cap {
        v.push(MaybeUninit::uninit());
    }
    v.into_boxed_slice()
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
        Self::with_capacity_and_hasher(0, hasher)
    }

    /// Creates an empty map with pre-allocated logical item capacity that will
    /// hash keys with `hasher`.
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        let cap = capacity_for(capacity);
        OpenHashMap {
            ctrl: empty_ctrl(cap),
            entries: uninit_entries(cap),
            len: 0,
            growth_left: max_load(cap),
            hasher,
        }
    }

    /// Returns a reference to the map's [`BuildHasher`].
    pub fn hasher(&self) -> &S {
        &self.hasher
    }

    #[inline]
    #[allow(dead_code)]
    fn cap(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Sets control byte `idx` and mirrors into the suffix when `idx < GROUP_WIDTH`.
    #[inline]
    fn set_ctrl(&mut self, idx: usize, byte: u8) {
        let cap = self.entries.len();
        self.ctrl[idx] = byte;
        if idx < GROUP_WIDTH {
            self.ctrl[cap + idx] = byte;
        }
    }

    pub fn clear(&mut self) {
        let cap = self.entries.len();
        // A guard recomputes accounting (len/growth_left) and the mirror suffix
        // from the surviving control bytes whenever this scope exits — including
        // through a panic in a user value `Drop` — so the table is left
        // internally consistent either way. Each control byte is set EMPTY
        // *before* its entry is dropped, so a panicking Drop never leaves a FULL
        // byte pointing at dropped memory (no double-drop).
        struct ClearGuard<'a, K, V, S>(&'a mut OpenHashMap<K, V, S>);
        impl<K, V, S> Drop for ClearGuard<'_, K, V, S> {
            fn drop(&mut self) {
                let t = &mut *self.0;
                let cap = t.entries.len();
                let (mut full, mut deleted) = (0usize, 0usize);
                for i in 0..cap {
                    match t.ctrl[i] {
                        c if c <= 0x7F => full += 1,
                        DELETED => deleted += 1,
                        _ => {}
                    }
                }
                for i in 0..GROUP_WIDTH {
                    t.ctrl[cap + i] = t.ctrl[i];
                }
                t.len = full;
                t.growth_left = max_load(cap) - full - deleted;
            }
        }
        let guard = ClearGuard(self);
        for i in 0..cap {
            if guard.0.ctrl[i] <= 0x7F {
                guard.0.ctrl[i] = EMPTY;
                // SAFETY: was FULL ⇒ initialized; marked EMPTY first ⇒ never
                // double-dropped even if this drop or a later one panics.
                unsafe {
                    std::ptr::drop_in_place(guard.0.entries[i].as_mut_ptr());
                }
            } else {
                guard.0.ctrl[i] = EMPTY;
            }
        }
    }

    pub fn iter(&self) -> OpenHashMapIter<'_, K, V> {
        OpenHashMapIter {
            ctrl: &self.ctrl,
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
    /// default) already produces well-mixed 64-bit output, so no extra spread
    /// is applied.
    #[inline]
    fn hash(&self, key: &(impl Hash + ?Sized)) -> u64 {
        self.hasher.hash_one(key)
    }

    /// Returns the bucket index of `query` if present.
    fn find_index<Q>(&self, query: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.len == 0 {
            return None;
        }
        let cap = self.entries.len();
        let mask = cap - 1;
        let hash = self.hash(query);
        let tag = (hash & 0x7F) as u8;

        let mut seq = ProbeSeq::new(hash, cap);
        loop {
            let group = seq.next_group();
            let g = load_group(&self.ctrl, group);
            for lane in match_byte(g, tag) {
                let idx = (group + lane) & mask;
                // SAFETY: ctrl[idx]==tag (<=0x7F) ⇒ FULL ⇒ initialized.
                if self.ctrl[idx] == tag {
                    let entry = unsafe { &*self.entries[idx].as_ptr() };
                    if entry.key.borrow() == query {
                        return Some(idx);
                    }
                }
            }
            if match_empty(g).any() {
                return None;
            }
        }
    }

    /// Inserts a key-value pair. Returns the old value if the key was already
    /// present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.growth_left == 0 {
            self.rehash_or_grow_for_insert();
        }
        let cap = self.entries.len();
        let mask = cap - 1;
        let hash = self.hash(&key);
        let tag = (hash & 0x7F) as u8;

        let mut first_deleted: Option<usize> = None;
        let mut seq = ProbeSeq::new(hash, cap);
        loop {
            let group = seq.next_group();
            let g = load_group(&self.ctrl, group);

            for lane in match_byte(g, tag) {
                let idx = (group + lane) & mask;
                if self.ctrl[idx] == tag {
                    // SAFETY: FULL slot is initialized.
                    let entry = unsafe { &mut *self.entries[idx].as_mut_ptr() };
                    if entry.key == key {
                        let old = std::mem::replace(&mut entry.value, value);
                        // `key` is dropped here.
                        return Some(old);
                    }
                }
            }

            if first_deleted.is_none() {
                if let Some(lane) = match_deleted(g).lowest() {
                    first_deleted = Some((group + lane) & mask);
                }
            }

            if let Some(lane) = match_empty(g).lowest() {
                let empty_idx = (group + lane) & mask;
                let idx = first_deleted.unwrap_or(empty_idx);
                let was_empty = self.ctrl[idx] == EMPTY;
                // Write entry before publishing the control byte (panic safety).
                self.entries[idx].write(Entry { key, value });
                if was_empty {
                    self.growth_left -= 1;
                }
                self.set_ctrl(idx, tag);
                self.len += 1;
                return None;
            }
        }
    }

    /// Borrows the value for `key`.
    ///
    /// Accepts any borrowed form `&Q` of the key (`K: Borrow<Q>`), so a
    /// `OpenHashMap<String, _>` can be queried with `&str`.
    pub fn get<'a, Q>(&'a self, key: &Q) -> Option<&'a V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let idx = self.find_index(key)?;
        // SAFETY: find_index returns FULL ⇒ initialized index.
        Some(unsafe { &(*self.entries[idx].as_ptr()).value })
    }

    pub fn get_mut<'a, Q>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let idx = self.find_index(key)?;
        // SAFETY: find_index returns FULL ⇒ initialized index.
        Some(unsafe { &mut (*self.entries[idx].as_mut_ptr()).value })
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.find_index(key).is_some()
    }

    /// Removes the key. Returns the old value if present.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let idx = self.find_index(key)?;
        // Move the entry out first (non-panicking), then publish the control
        // byte, then decrement len. No panicking op runs while ctrl still
        // marks the slot FULL pointing at moved-out memory.
        // SAFETY: idx is FULL ⇒ initialized.
        let entry = unsafe { self.entries[idx].as_ptr().read() };
        // Always mark DELETED. The spec's optional `can_mark_empty`
        // optimization checks physically-adjacent groups, which is unsound for
        // the triangular probe sequence used here (probes jump by growing
        // strides, not to physical neighbors), so it is intentionally skipped.
        self.set_ctrl(idx, DELETED);
        self.len -= 1;
        Some(entry.value)
    }

    /// Insert assuming the table has room and no equal key exists; used only by
    /// rehash into a fresh (tombstone-free) table.
    fn insert_no_grow(&mut self, key: K, value: V) {
        let cap = self.entries.len();
        let mask = cap - 1;
        let hash = self.hash(&key);
        let tag = (hash & 0x7F) as u8;
        let mut seq = ProbeSeq::new(hash, cap);
        loop {
            let group = seq.next_group();
            let g = load_group(&self.ctrl, group);
            if let Some(lane) = match_empty(g).lowest() {
                let idx = (group + lane) & mask;
                self.entries[idx].write(Entry { key, value });
                self.set_ctrl(idx, tag);
                self.len += 1;
                self.growth_left -= 1;
                return;
            }
        }
    }

    /// Called when `growth_left == 0` and one more entry needs inserting.
    fn rehash_or_grow_for_insert(&mut self) {
        let cap = self.entries.len();
        if self.len + 1 <= max_load(cap) {
            // Tombstones are exhausting the empty budget; rebuild same size.
            self.rehash_to(cap);
        } else {
            self.rehash_to(cap * 2);
        }
    }

    /// Rebuilds the table at `new_cap`, moving every full entry. Tombstone-free
    /// afterwards. Infallible variant (used on the insert/grow path).
    fn rehash_to(&mut self, new_cap: usize) {
        self.rehash_to_inner(new_cap)
            .expect("allocation failure during rehash");
    }

    // Panic note: if a user `Hash` impl panics while moving an entry, every
    // entry is still dropped exactly once (moved-out entry on the stack,
    // not-yet-moved entries via the guard, already-moved entries in `self`) and
    // `self` is left a valid — but possibly smaller — table. Entries not yet
    // rehashed are lost. This matches `std`'s contract that a panicking `Hash`
    // leaves the map in an unspecified (but sound) state; it is not double-drop
    // or UB. A fully atomic move-rehash is impossible without `K: Clone`.
    fn rehash_to_inner(&mut self, new_cap: usize) -> Result<(), TryReserveError> {
        debug_assert!(new_cap >= self.len);
        // Try to allocate the new entries array fallibly.
        let mut v: Vec<MaybeUninit<Entry<K, V>>> = Vec::new();
        v.try_reserve_exact(new_cap)?;
        for _ in 0..new_cap {
            v.push(MaybeUninit::uninit());
        }
        let new_entries = v.into_boxed_slice();
        let new_ctrl = empty_ctrl(new_cap);

        // Install the fresh empty table; keep the old arrays to drain.
        let old_ctrl = std::mem::replace(&mut self.ctrl, new_ctrl);
        let old_entries = std::mem::replace(&mut self.entries, new_entries);
        let old_cap = old_entries.len();
        self.len = 0;
        self.growth_left = max_load(new_cap);

        // Drain old full slots. Use a guard so a panic in hashing/move drops
        // every remaining old full entry and the already-moved new entries
        // exactly once (the new entries are dropped by `self`'s own Drop, and
        // the guard drops the un-moved old ones).
        let mut old_entries = old_entries;
        struct Guard<'a, K, V> {
            ctrl: &'a [u8],
            entries: &'a mut [MaybeUninit<Entry<K, V>>],
            start: usize,
        }
        impl<K, V> Drop for Guard<'_, K, V> {
            fn drop(&mut self) {
                for i in self.start..self.entries.len() {
                    if self.ctrl[i] <= 0x7F {
                        // SAFETY: still-FULL old slot ⇒ initialized, not yet moved.
                        unsafe {
                            std::ptr::drop_in_place(self.entries[i].as_mut_ptr());
                        }
                    }
                }
            }
        }
        let mut guard = Guard {
            ctrl: &old_ctrl,
            entries: &mut old_entries,
            start: 0,
        };
        for i in 0..old_cap {
            if old_ctrl[i] <= 0x7F {
                // SAFETY: FULL ⇒ initialized. Read it out; advance guard.start
                // BEFORE re-inserting so the guard never re-drops this slot.
                let entry = unsafe { guard.entries[i].as_ptr().read() };
                guard.start = i + 1;
                self.insert_no_grow(entry.key, entry.value);
            } else {
                guard.start = i + 1;
            }
        }
        std::mem::forget(guard); // all old entries moved out; nothing to drop
        Ok(())
    }

    /// Reserves capacity for at least `additional` more entries to be inserted.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        let needed = self
            .len
            .checked_add(additional)
            .ok_or_else(capacity_overflow_error)?;
        let cap = self.entries.len();
        // capacity_for may overflow for absurd requests; detect that.
        let new_cap = checked_capacity_for(needed)?;
        if new_cap > cap {
            self.rehash_to_inner(new_cap)
        } else if self.growth_left < additional && self.deleted_count() != 0 {
            // Cheap `growth_left` test first; only scan ctrl for tombstones when
            // the empty budget is actually short (avoids an O(cap) scan otherwise).
            self.rehash_to_inner(cap)
        } else {
            Ok(())
        }
    }

    fn deleted_count(&self) -> usize {
        let cap = self.entries.len();
        self.ctrl[0..cap].iter().filter(|&&b| b == DELETED).count()
    }
}

/// Produces a `TryReserveError` for a capacity overflow.
fn capacity_overflow_error() -> TryReserveError {
    // The only public way to mint a TryReserveError is via a failing reserve.
    let mut v: Vec<u8> = Vec::new();
    v.try_reserve(usize::MAX).unwrap_err()
}

fn checked_capacity_for(n: usize) -> Result<usize, TryReserveError> {
    let mut cap = MIN_CAPACITY;
    while max_load(cap) < n {
        cap = cap.checked_mul(2).ok_or_else(capacity_overflow_error)?;
    }
    Ok(cap)
}

impl<K, V, S> Drop for OpenHashMap<K, V, S> {
    fn drop(&mut self) {
        let cap = self.entries.len();
        for i in 0..cap {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL ⇒ initialized; each slot dropped once.
                unsafe {
                    std::ptr::drop_in_place(self.entries[i].as_mut_ptr());
                }
            }
        }
    }
}

impl<K: Clone, V: Clone, S: Clone> Clone for OpenHashMap<K, V, S> {
    fn clone(&self) -> Self {
        let cap = self.entries.len();
        let mut entries = uninit_entries::<Entry<K, V>>(cap);
        let ctrl = self.ctrl.clone();

        // Guard drops already-cloned full slots if a clone panics.
        struct Guard<'a, K, V> {
            ctrl: &'a [u8],
            entries: &'a mut [MaybeUninit<Entry<K, V>>],
            filled: usize,
        }
        impl<K, V> Drop for Guard<'_, K, V> {
            fn drop(&mut self) {
                for i in 0..self.filled {
                    if self.ctrl[i] <= 0x7F {
                        // SAFETY: slot i (< filled) was cloned & is FULL.
                        unsafe {
                            std::ptr::drop_in_place(self.entries[i].as_mut_ptr());
                        }
                    }
                }
            }
        }
        let mut guard = Guard {
            ctrl: &self.ctrl,
            entries: &mut entries,
            filled: 0,
        };
        for i in 0..cap {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL ⇒ source initialized.
                let src = unsafe { &*self.entries[i].as_ptr() };
                guard.entries[i].write(Entry {
                    key: src.key.clone(),
                    value: src.value.clone(),
                });
            }
            guard.filled = i + 1;
        }
        // Clone the hasher BEFORE disarming the guard: if `S::clone` panics, the
        // guard must still drop the entries cloned above (otherwise they leak).
        let hasher = self.hasher.clone();
        std::mem::forget(guard);

        OpenHashMap {
            ctrl,
            entries,
            len: self.len,
            growth_left: self.growth_left,
            hasher,
        }
    }
}

pub struct OpenHashMapIter<'a, K, V> {
    ctrl: &'a [u8],
    entries: &'a [MaybeUninit<Entry<K, V>>],
    pos: usize,
}

impl<'a, K, V> Iterator for OpenHashMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL ⇒ initialized; bound to iterator lifetime.
                let entry = unsafe { &*self.entries[i].as_ptr() };
                return Some((&entry.key, &entry.value));
            }
        }
        None
    }
}

// ===========================================================================
// OpenHashSet<K, S>
// ===========================================================================

/// Open-addressing hash set with grouped Swiss-table probing and a pluggable
/// hasher.
///
/// The hasher `S` defaults to [`RandomState`]; use [`OpenHashSet::with_hasher`]
/// for a fixed/faster hasher.
pub struct OpenHashSet<K, S = RandomState> {
    ctrl: Box<[u8]>,
    entries: Box<[MaybeUninit<SetEntry<K>>]>,
    len: usize,
    growth_left: usize,
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
        Self::with_capacity_and_hasher(0, hasher)
    }

    /// Creates an empty set with pre-allocated logical item capacity that will
    /// hash values with `hasher`.
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        let cap = capacity_for(capacity);
        OpenHashSet {
            ctrl: empty_ctrl(cap),
            entries: uninit_entries(cap),
            len: 0,
            growth_left: max_load(cap),
            hasher,
        }
    }

    /// Returns a reference to the set's [`BuildHasher`].
    pub fn hasher(&self) -> &S {
        &self.hasher
    }

    #[inline]
    #[allow(dead_code)]
    fn cap(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn set_ctrl(&mut self, idx: usize, byte: u8) {
        let cap = self.entries.len();
        self.ctrl[idx] = byte;
        if idx < GROUP_WIDTH {
            self.ctrl[cap + idx] = byte;
        }
    }

    pub fn clear(&mut self) {
        let cap = self.entries.len();
        // See OpenHashMap::clear: a guard keeps accounting/mirror consistent even
        // if a user element `Drop` panics mid-clear.
        struct ClearGuard<'a, K, S>(&'a mut OpenHashSet<K, S>);
        impl<K, S> Drop for ClearGuard<'_, K, S> {
            fn drop(&mut self) {
                let t = &mut *self.0;
                let cap = t.entries.len();
                let (mut full, mut deleted) = (0usize, 0usize);
                for i in 0..cap {
                    match t.ctrl[i] {
                        c if c <= 0x7F => full += 1,
                        DELETED => deleted += 1,
                        _ => {}
                    }
                }
                for i in 0..GROUP_WIDTH {
                    t.ctrl[cap + i] = t.ctrl[i];
                }
                t.len = full;
                t.growth_left = max_load(cap) - full - deleted;
            }
        }
        let guard = ClearGuard(self);
        for i in 0..cap {
            if guard.0.ctrl[i] <= 0x7F {
                guard.0.ctrl[i] = EMPTY;
                // SAFETY: was FULL ⇒ initialized; marked EMPTY first ⇒ no double-drop.
                unsafe {
                    std::ptr::drop_in_place(guard.0.entries[i].as_mut_ptr());
                }
            } else {
                guard.0.ctrl[i] = EMPTY;
            }
        }
    }

    pub fn iter(&self) -> OpenHashSetIter<'_, K> {
        OpenHashSetIter {
            ctrl: &self.ctrl,
            entries: &self.entries,
            pos: 0,
        }
    }
}

impl<K: Hash + Eq, S: BuildHasher> OpenHashSet<K, S> {
    #[inline]
    fn hash(&self, key: &(impl Hash + ?Sized)) -> u64 {
        self.hasher.hash_one(key)
    }

    fn find_index<Q>(&self, query: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.len == 0 {
            return None;
        }
        let cap = self.entries.len();
        let mask = cap - 1;
        let hash = self.hash(query);
        let tag = (hash & 0x7F) as u8;

        let mut seq = ProbeSeq::new(hash, cap);
        loop {
            let group = seq.next_group();
            let g = load_group(&self.ctrl, group);
            for lane in match_byte(g, tag) {
                let idx = (group + lane) & mask;
                if self.ctrl[idx] == tag {
                    // SAFETY: FULL ⇒ initialized.
                    let entry = unsafe { &*self.entries[idx].as_ptr() };
                    if entry.key.borrow() == query {
                        return Some(idx);
                    }
                }
            }
            if match_empty(g).any() {
                return None;
            }
        }
    }

    /// Inserts a value. Returns `true` if it was newly inserted (not already present).
    pub fn insert(&mut self, value: K) -> bool {
        if self.growth_left == 0 {
            self.rehash_or_grow_for_insert();
        }
        let cap = self.entries.len();
        let mask = cap - 1;
        let hash = self.hash(&value);
        let tag = (hash & 0x7F) as u8;

        let mut first_deleted: Option<usize> = None;
        let mut seq = ProbeSeq::new(hash, cap);
        loop {
            let group = seq.next_group();
            let g = load_group(&self.ctrl, group);

            for lane in match_byte(g, tag) {
                let idx = (group + lane) & mask;
                if self.ctrl[idx] == tag {
                    // SAFETY: FULL ⇒ initialized.
                    let entry = unsafe { &*self.entries[idx].as_ptr() };
                    if entry.key == value {
                        return false;
                    }
                }
            }

            if first_deleted.is_none() {
                if let Some(lane) = match_deleted(g).lowest() {
                    first_deleted = Some((group + lane) & mask);
                }
            }

            if let Some(lane) = match_empty(g).lowest() {
                let empty_idx = (group + lane) & mask;
                let idx = first_deleted.unwrap_or(empty_idx);
                let was_empty = self.ctrl[idx] == EMPTY;
                self.entries[idx].write(SetEntry { key: value });
                if was_empty {
                    self.growth_left -= 1;
                }
                self.set_ctrl(idx, tag);
                self.len += 1;
                return true;
            }
        }
    }

    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.find_index(value).is_some()
    }

    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let idx = match self.find_index(value) {
            Some(i) => i,
            None => return false,
        };
        // SAFETY: idx FULL ⇒ initialized. Move out, publish ctrl, dec len.
        let entry = unsafe { self.entries[idx].as_ptr().read() };
        // Always mark DELETED. The spec's optional `can_mark_empty`
        // optimization checks physically-adjacent groups, which is unsound for
        // the triangular probe sequence used here (probes jump by growing
        // strides, not to physical neighbors), so it is intentionally skipped.
        self.set_ctrl(idx, DELETED);
        self.len -= 1;
        drop(entry.key);
        true
    }

    fn insert_no_grow(&mut self, value: K) {
        let cap = self.entries.len();
        let mask = cap - 1;
        let hash = self.hash(&value);
        let tag = (hash & 0x7F) as u8;
        let mut seq = ProbeSeq::new(hash, cap);
        loop {
            let group = seq.next_group();
            let g = load_group(&self.ctrl, group);
            if let Some(lane) = match_empty(g).lowest() {
                let idx = (group + lane) & mask;
                self.entries[idx].write(SetEntry { key: value });
                self.set_ctrl(idx, tag);
                self.len += 1;
                self.growth_left -= 1;
                return;
            }
        }
    }

    fn rehash_or_grow_for_insert(&mut self) {
        let cap = self.entries.len();
        if self.len + 1 <= max_load(cap) {
            self.rehash_to(cap);
        } else {
            self.rehash_to(cap * 2);
        }
    }

    fn rehash_to(&mut self, new_cap: usize) {
        self.rehash_to_inner(new_cap)
            .expect("allocation failure during rehash");
    }

    // Panic note: if a user `Hash` impl panics while moving an entry, every
    // entry is still dropped exactly once (moved-out entry on the stack,
    // not-yet-moved entries via the guard, already-moved entries in `self`) and
    // `self` is left a valid — but possibly smaller — table. Entries not yet
    // rehashed are lost. This matches `std`'s contract that a panicking `Hash`
    // leaves the map in an unspecified (but sound) state; it is not double-drop
    // or UB. A fully atomic move-rehash is impossible without `K: Clone`.
    fn rehash_to_inner(&mut self, new_cap: usize) -> Result<(), TryReserveError> {
        debug_assert!(new_cap >= self.len);
        let mut v: Vec<MaybeUninit<SetEntry<K>>> = Vec::new();
        v.try_reserve_exact(new_cap)?;
        for _ in 0..new_cap {
            v.push(MaybeUninit::uninit());
        }
        let new_entries = v.into_boxed_slice();
        let new_ctrl = empty_ctrl(new_cap);

        let old_ctrl = std::mem::replace(&mut self.ctrl, new_ctrl);
        let old_entries = std::mem::replace(&mut self.entries, new_entries);
        let old_cap = old_entries.len();
        self.len = 0;
        self.growth_left = max_load(new_cap);

        let mut old_entries = old_entries;
        struct Guard<'a, K> {
            ctrl: &'a [u8],
            entries: &'a mut [MaybeUninit<SetEntry<K>>],
            start: usize,
        }
        impl<K> Drop for Guard<'_, K> {
            fn drop(&mut self) {
                for i in self.start..self.entries.len() {
                    if self.ctrl[i] <= 0x7F {
                        // SAFETY: still-FULL old slot ⇒ initialized.
                        unsafe {
                            std::ptr::drop_in_place(self.entries[i].as_mut_ptr());
                        }
                    }
                }
            }
        }
        let mut guard = Guard {
            ctrl: &old_ctrl,
            entries: &mut old_entries,
            start: 0,
        };
        for i in 0..old_cap {
            if old_ctrl[i] <= 0x7F {
                // SAFETY: FULL ⇒ initialized.
                let entry = unsafe { guard.entries[i].as_ptr().read() };
                guard.start = i + 1;
                self.insert_no_grow(entry.key);
            } else {
                guard.start = i + 1;
            }
        }
        std::mem::forget(guard);
        Ok(())
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        let needed = self
            .len
            .checked_add(additional)
            .ok_or_else(capacity_overflow_error)?;
        let cap = self.entries.len();
        let new_cap = checked_capacity_for(needed)?;
        if new_cap > cap {
            self.rehash_to_inner(new_cap)
        } else if self.growth_left < additional && self.deleted_count() != 0 {
            // Cheap `growth_left` test first; only scan ctrl for tombstones when
            // the empty budget is actually short (avoids an O(cap) scan otherwise).
            self.rehash_to_inner(cap)
        } else {
            Ok(())
        }
    }

    fn deleted_count(&self) -> usize {
        let cap = self.entries.len();
        self.ctrl[0..cap].iter().filter(|&&b| b == DELETED).count()
    }
}

impl<K, S> Drop for OpenHashSet<K, S> {
    fn drop(&mut self) {
        let cap = self.entries.len();
        for i in 0..cap {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL ⇒ initialized; dropped once.
                unsafe {
                    std::ptr::drop_in_place(self.entries[i].as_mut_ptr());
                }
            }
        }
    }
}

impl<K: Clone, S: Clone> Clone for OpenHashSet<K, S> {
    fn clone(&self) -> Self {
        let cap = self.entries.len();
        let mut entries = uninit_entries::<SetEntry<K>>(cap);
        let ctrl = self.ctrl.clone();

        struct Guard<'a, K> {
            ctrl: &'a [u8],
            entries: &'a mut [MaybeUninit<SetEntry<K>>],
            filled: usize,
        }
        impl<K> Drop for Guard<'_, K> {
            fn drop(&mut self) {
                for i in 0..self.filled {
                    if self.ctrl[i] <= 0x7F {
                        // SAFETY: cloned & FULL.
                        unsafe {
                            std::ptr::drop_in_place(self.entries[i].as_mut_ptr());
                        }
                    }
                }
            }
        }
        let mut guard = Guard {
            ctrl: &self.ctrl,
            entries: &mut entries,
            filled: 0,
        };
        for i in 0..cap {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL ⇒ source initialized.
                let src = unsafe { &*self.entries[i].as_ptr() };
                guard.entries[i].write(SetEntry {
                    key: src.key.clone(),
                });
            }
            guard.filled = i + 1;
        }
        // Clone the hasher BEFORE disarming the guard (see OpenHashMap::clone).
        let hasher = self.hasher.clone();
        std::mem::forget(guard);

        OpenHashSet {
            ctrl,
            entries,
            len: self.len,
            growth_left: self.growth_left,
            hasher,
        }
    }
}

pub struct OpenHashSetIter<'a, K> {
    ctrl: &'a [u8],
    entries: &'a [MaybeUninit<SetEntry<K>>],
    pos: usize,
}

impl<'a, K> Iterator for OpenHashSetIter<'a, K> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL ⇒ initialized.
                let entry = unsafe { &*self.entries[i].as_ptr() };
                return Some(&entry.key);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Debug
// ---------------------------------------------------------------------------

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for OpenHashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: fmt::Debug, S> fmt::Debug for OpenHashSet<K, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
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
    ctrl: Box<[u8]>,
    entries: Box<[MaybeUninit<Entry<K, V>>]>,
    pos: usize,
}

impl<K, V> Iterator for OpenHashMapIntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            if self.ctrl[i] <= 0x7F {
                // Mark consumed so the destructor won't drop it again.
                self.ctrl[i] = EMPTY;
                // SAFETY: FULL ⇒ initialized; moved out exactly once.
                let entry = unsafe { self.entries[i].as_ptr().read() };
                return Some((entry.key, entry.value));
            }
        }
        None
    }
}

impl<K, V> Drop for OpenHashMapIntoIter<K, V> {
    fn drop(&mut self) {
        for i in self.pos..self.entries.len() {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: still-FULL ⇒ not yet yielded ⇒ initialized.
                unsafe {
                    std::ptr::drop_in_place(self.entries[i].as_mut_ptr());
                }
            }
        }
    }
}

impl<K, V, S> IntoIterator for OpenHashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = OpenHashMapIntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        // Move the arrays out of `self` without running `self`'s Drop (which
        // would drop all full entries). Take ownership of ctrl/entries.
        let this = std::mem::ManuallyDrop::new(self);
        // SAFETY: read each field out of the ManuallyDrop wrapper exactly once;
        // `this` is never used again and its Drop does not run.
        let ctrl = unsafe { std::ptr::read(&this.ctrl) };
        let entries = unsafe { std::ptr::read(&this.entries) };
        let hasher = unsafe { std::ptr::read(&this.hasher) };
        // Construct the owning iterator FIRST, then drop the hasher: if a user
        // hasher's `Drop` panics, `iter` is a live local whose `Drop` runs during
        // unwinding and frees the remaining entries (otherwise they would leak).
        let iter = OpenHashMapIntoIter {
            ctrl,
            entries,
            pos: 0,
        };
        drop(hasher);
        iter
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
    ctrl: Box<[u8]>,
    entries: Box<[MaybeUninit<SetEntry<K>>]>,
    pos: usize,
}

impl<K> Iterator for OpenHashSetIntoIter<K> {
    type Item = K;
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.entries.len() {
            let i = self.pos;
            self.pos += 1;
            if self.ctrl[i] <= 0x7F {
                self.ctrl[i] = EMPTY;
                // SAFETY: FULL ⇒ initialized; moved out once.
                let entry = unsafe { self.entries[i].as_ptr().read() };
                return Some(entry.key);
            }
        }
        None
    }
}

impl<K> Drop for OpenHashSetIntoIter<K> {
    fn drop(&mut self) {
        for i in self.pos..self.entries.len() {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: still-FULL ⇒ initialized.
                unsafe {
                    std::ptr::drop_in_place(self.entries[i].as_mut_ptr());
                }
            }
        }
    }
}

impl<K, S> IntoIterator for OpenHashSet<K, S> {
    type Item = K;
    type IntoIter = OpenHashSetIntoIter<K>;
    fn into_iter(self) -> Self::IntoIter {
        let this = std::mem::ManuallyDrop::new(self);
        // SAFETY: read each field out exactly once; `this`'s Drop never runs.
        let ctrl = unsafe { std::ptr::read(&this.ctrl) };
        let entries = unsafe { std::ptr::read(&this.entries) };
        let hasher = unsafe { std::ptr::read(&this.hasher) };
        // Construct the owning iterator FIRST, then drop the hasher (see
        // OpenHashMap::into_iter) so a panicking hasher Drop can't leak entries.
        let iter = OpenHashSetIntoIter {
            ctrl,
            entries,
            pos: 0,
        };
        drop(hasher);
        iter
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
// Structural invariant verification
// ---------------------------------------------------------------------------

impl<K: Hash + Eq, V, S: BuildHasher> OpenHashMap<K, V, S> {
    /// Panics if any structural invariant from the design doc is violated.
    /// O(cap); usable from tests and verification examples in this crate.
    pub fn assert_invariants(&self) {
        let cap = self.entries.len();
        // 1. Capacity shape.
        assert_eq!(self.ctrl.len(), cap + GROUP_WIDTH, "ctrl length");
        assert!(cap.is_power_of_two(), "cap power of two");
        assert!(cap >= MIN_CAPACITY, "cap >= MIN_CAPACITY");
        assert_eq!(cap % GROUP_WIDTH, 0, "cap multiple of GROUP_WIDTH");

        // 2. Control encoding.
        for i in 0..cap {
            let b = self.ctrl[i];
            assert!(
                b == EMPTY || b == DELETED || b <= 0x7F,
                "ctrl[{i}] = {b:#x} invalid"
            );
        }

        // 3. Mirror suffix.
        for i in 0..GROUP_WIDTH {
            assert_eq!(self.ctrl[cap + i], self.ctrl[i], "mirror suffix at {i}");
        }

        // 4(partial: tag) + 5. Length/growth accounting.
        let mut full = 0usize;
        let mut deleted = 0usize;
        for i in 0..cap {
            let b = self.ctrl[i];
            if b <= 0x7F {
                full += 1;
                // SAFETY: FULL ⇒ initialized.
                let entry = unsafe { &*self.entries[i].as_ptr() };
                let tag = (self.hash(&entry.key) & 0x7F) as u8;
                assert_eq!(b, tag, "ctrl[{i}] tag mismatch");
            } else if b == DELETED {
                deleted += 1;
            }
        }
        assert_eq!(full, self.len, "len == full_count");
        assert!(self.len <= max_load(cap), "len <= max_load");
        assert_eq!(
            self.growth_left,
            max_load(cap) - self.len - deleted,
            "growth_left accounting"
        );

        // 6. Probe reachability + 8. lookup consistency.
        for i in 0..cap {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL.
                let entry = unsafe { &*self.entries[i].as_ptr() };
                assert_eq!(
                    self.find_index(&entry.key),
                    Some(i),
                    "find_index must return own slot {i}"
                );
            }
        }

        // 7. No duplicate keys (O(n) via a hash set so this stays usable on
        // multi-million-entry tables; a pairwise O(n²) scan would not).
        let mut seen: std::collections::HashSet<&K> =
            std::collections::HashSet::with_capacity(self.len);
        for i in 0..cap {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL.
                let entry = unsafe { &*self.entries[i].as_ptr() };
                assert!(seen.insert(&entry.key), "duplicate key found");
            }
        }
    }
}

impl<K: Hash + Eq, S: BuildHasher> OpenHashSet<K, S> {
    /// Panics if any structural invariant from the design doc is violated.
    pub fn assert_invariants(&self) {
        let cap = self.entries.len();
        assert_eq!(self.ctrl.len(), cap + GROUP_WIDTH, "ctrl length");
        assert!(cap.is_power_of_two(), "cap power of two");
        assert!(cap >= MIN_CAPACITY, "cap >= MIN_CAPACITY");
        assert_eq!(cap % GROUP_WIDTH, 0, "cap multiple of GROUP_WIDTH");

        for i in 0..cap {
            let b = self.ctrl[i];
            assert!(
                b == EMPTY || b == DELETED || b <= 0x7F,
                "ctrl[{i}] = {b:#x} invalid"
            );
        }
        for i in 0..GROUP_WIDTH {
            assert_eq!(self.ctrl[cap + i], self.ctrl[i], "mirror suffix at {i}");
        }

        let mut full = 0usize;
        let mut deleted = 0usize;
        for i in 0..cap {
            let b = self.ctrl[i];
            if b <= 0x7F {
                full += 1;
                // SAFETY: FULL.
                let entry = unsafe { &*self.entries[i].as_ptr() };
                let tag = (self.hash(&entry.key) & 0x7F) as u8;
                assert_eq!(b, tag, "ctrl[{i}] tag mismatch");
            } else if b == DELETED {
                deleted += 1;
            }
        }
        assert_eq!(full, self.len, "len == full_count");
        assert!(self.len <= max_load(cap), "len <= max_load");
        assert_eq!(
            self.growth_left,
            max_load(cap) - self.len - deleted,
            "growth_left accounting"
        );

        for i in 0..cap {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL.
                let entry = unsafe { &*self.entries[i].as_ptr() };
                assert_eq!(
                    self.find_index(&entry.key),
                    Some(i),
                    "find_index must return own slot {i}"
                );
            }
        }

        // 7. No duplicate keys (O(n) via a hash set; see OpenHashMap).
        let mut seen: std::collections::HashSet<&K> =
            std::collections::HashSet::with_capacity(self.len);
        for i in 0..cap {
            if self.ctrl[i] <= 0x7F {
                // SAFETY: FULL.
                let entry = unsafe { &*self.entries[i].as_ptr() };
                assert!(seen.insert(&entry.key), "duplicate key found");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashable_float::{HashableF32, HashableF64};

    #[test]
    fn swar_match_byte_basics() {
        // bytes [0x05, 0xFF, 0x80, 0x05, 0x00, 0x7F, 0x42, 0x05]
        let ctrl = [0x05u8, 0xFF, 0x80, 0x05, 0x00, 0x7F, 0x42, 0x05];
        let g = u64::from_le_bytes(ctrl);
        let lanes: Vec<usize> = match_byte(g, 0x05).collect();
        assert_eq!(lanes, vec![0, 3, 7]);
        let empties: Vec<usize> = match_empty(g).collect();
        assert_eq!(empties, vec![1]);
        let deleted: Vec<usize> = match_deleted(g).collect();
        assert_eq!(deleted, vec![2]);
        // Searching for tag 0x00 must match lane 4 only (the FULL 0x00),
        // never EMPTY(0xFF) or DELETED(0x80).
        let zeros: Vec<usize> = match_byte(g, 0x00).collect();
        assert_eq!(zeros, vec![4]);
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
        m.assert_invariants();
    }

    #[test]
    fn map_partial_eq_order_insensitive() {
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
    fn set_partial_eq_order_insensitive() {
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
    fn map_resize() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..200 {
            m.insert(i, i * 10);
        }
        assert_eq!(m.len(), 200);
        for i in 0..200 {
            assert_eq!(m.get(&i), Some(&(i * 10)));
        }
        m.assert_invariants();
    }

    #[test]
    fn map_resize_many_growths() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..5000 {
            m.insert(i, i);
            if i % 137 == 0 {
                m.assert_invariants();
            }
        }
        assert_eq!(m.len(), 5000);
        for i in 0..5000 {
            assert_eq!(m.get(&i), Some(&i));
        }
        m.assert_invariants();
    }

    #[test]
    fn map_tombstone_reuse() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..10 {
            m.insert(i, i);
        }
        assert_eq!(m.remove(&5), Some(5));
        m.assert_invariants();
        // Re-insert the same key; it should reuse a freed/empty slot.
        assert_eq!(m.insert(5, 555), None);
        assert_eq!(m.get(&5), Some(&555));
        m.assert_invariants();
    }

    #[test]
    fn map_forced_rehash_clears_tombstones() {
        // Many remove+insert cycles at a fixed logical size must not grow the
        // table unboundedly; same-capacity rehash should reclaim tombstones.
        let mut m = OpenHashMap::<i32, i32>::with_capacity(64);
        let cap0 = m.cap();
        for i in 0..40 {
            m.insert(i, i);
        }
        let cap1 = m.cap();
        for round in 0..2000 {
            let k = 1000 + round;
            m.insert(k, k);
            assert_eq!(m.remove(&k), Some(k));
        }
        // The live set never exceeded ~41, so capacity must remain bounded.
        assert!(m.cap() <= cap1 * 2, "table grew unexpectedly");
        let _ = cap0;
        assert_eq!(m.len(), 40);
        for i in 0..40 {
            assert_eq!(m.get(&i), Some(&i));
        }
        m.assert_invariants();
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
        m.assert_invariants();
    }

    #[test]
    fn map_clear() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.insert(1, 1);
        m.insert(2, 2);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.get(&1), None);
        m.assert_invariants();
        // reuse after clear
        m.insert(7, 7);
        assert_eq!(m.get(&7), Some(&7));
        m.assert_invariants();
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
    fn map_get_mut() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.insert(1, 10);
        if let Some(v) = m.get_mut(&1) {
            *v += 5;
        }
        assert_eq!(m.get(&1), Some(&15));
        // Mutate after several inserts/removes.
        for i in 2..50 {
            m.insert(i, i);
        }
        for i in 2..50 {
            *m.get_mut(&i).unwrap() *= 2;
        }
        for i in 2..50 {
            assert_eq!(m.get(&i), Some(&(i * 2)));
        }
        m.assert_invariants();
    }

    #[test]
    fn map_clone_equality() {
        let mut m = OpenHashMap::<i32, String>::new();
        for i in 0..100 {
            m.insert(i, format!("v{i}"));
        }
        m.remove(&50);
        m.remove(&51);
        let c = m.clone();
        assert_eq!(c, m);
        assert_eq!(c.len(), m.len());
        c.assert_invariants();
        for i in 0..100 {
            if i == 50 || i == 51 {
                assert_eq!(c.get(&i), None);
            } else {
                assert_eq!(c.get(&i), Some(&format!("v{i}")));
            }
        }
    }

    #[test]
    fn map_into_iter_completeness() {
        let mut m = OpenHashMap::<i32, i32>::new();
        for i in 0..200 {
            m.insert(i, i * 3);
        }
        let mut owned: Vec<(i32, i32)> = m.into_iter().collect();
        owned.sort();
        let expected: Vec<(i32, i32)> = (0..200).map(|i| (i, i * 3)).collect();
        assert_eq!(owned, expected);
    }

    #[test]
    fn map_into_iter_partial_drops_rest() {
        // Partially consuming into_iter must drop the remaining entries exactly
        // once (no leak, no double-drop). Use Rc to detect leaks.
        use std::rc::Rc;
        let shared = Rc::new(());
        let mut m = OpenHashMap::<i32, Rc<()>>::new();
        for i in 0..100 {
            m.insert(i, shared.clone());
        }
        assert_eq!(Rc::strong_count(&shared), 101);
        let mut it = m.into_iter();
        let _ = it.next();
        let _ = it.next();
        drop(it);
        assert_eq!(Rc::strong_count(&shared), 1);
    }

    #[test]
    fn map_borrowed_lookup_string() {
        let mut m = OpenHashMap::<String, i32>::new();
        m.insert("hello".to_string(), 1);
        m.insert("world".to_string(), 2);
        assert_eq!(m.get("hello"), Some(&1));
        assert!(m.contains_key("world"));
        assert_eq!(m.get_mut("hello").map(|v| *v), Some(1));
        assert_eq!(m.remove("hello"), Some(1));
        assert_eq!(m.get("hello"), None);
        m.assert_invariants();
    }

    #[test]
    fn map_load_factor_grows_below_seven_eighths() {
        // cap-16 table: max_load = 14. The 15th insert must grow.
        let mut m = OpenHashMap::<i32, i32>::new();
        assert_eq!(m.cap(), 16);
        for i in 0..14 {
            m.insert(i, i);
        }
        assert_eq!(m.cap(), 16);
        m.insert(14, 14);
        assert_eq!(m.cap(), 32);
        assert_eq!(m.len(), 15);
        m.assert_invariants();
    }

    #[test]
    fn map_try_reserve_grows_and_avoids_subsequent_resize() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.try_reserve(1000).unwrap();
        let reserved = m.cap();
        assert!(max_load(reserved) >= 1000);
        for i in 0..1000 {
            m.insert(i, i * 2);
        }
        assert_eq!(reserved, m.cap());
        assert_eq!(m.len(), 1000);
        for i in 0..1000 {
            assert_eq!(m.get(&i), Some(&(i * 2)));
        }
        m.assert_invariants();
    }

    #[test]
    fn map_try_reserve_is_idempotent() {
        let mut m = OpenHashMap::<i32, i32>::new();
        let before = m.cap();
        m.try_reserve(1).unwrap();
        assert_eq!(before, m.cap());
    }

    #[test]
    fn map_try_reserve_propagates_allocation_error() {
        let mut m = OpenHashMap::<i32, i32>::new();
        let result = m.try_reserve(usize::MAX / 2);
        assert!(result.is_err());
    }

    #[test]
    fn map_try_reserve_clears_tombstones() {
        // Fill the table close to max_load, then delete almost everything so
        // that tombstones dominate and growth_left is small. A reserve whose
        // `additional` exceeds growth_left (without needing a bigger capacity)
        // must rebuild in place, clearing all tombstones (invariant 9).
        let mut m = OpenHashMap::<i32, i32>::new();
        let cap = m.cap();
        let load = max_load(cap);
        for i in 0..load as i32 {
            m.insert(i, i);
        }
        // Remove all but one: creates `load-1` tombstones, growth_left == 0.
        for i in 0..(load as i32 - 1) {
            m.remove(&i);
        }
        assert!(m.deleted_count() > 0);
        assert_eq!(m.cap(), cap, "must not have grown yet");
        // additional small enough that needed (1 + additional) still fits cap,
        // but larger than growth_left (== 0 here) -> in-place rebuild.
        m.try_reserve(2).unwrap();
        assert_eq!(m.cap(), cap, "reserve should rebuild in place, not grow");
        m.assert_invariants();
        assert_eq!(m.deleted_count(), 0);
    }

    // ---- set ----

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
        s.assert_invariants();
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
        s.assert_invariants();
    }

    #[test]
    fn set_delete_heavy() {
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
        s.assert_invariants();
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
    fn set_clone_equality() {
        let mut s = OpenHashSet::<String>::new();
        for i in 0..50 {
            s.insert(format!("k{i}"));
        }
        s.remove("k10");
        let c = s.clone();
        assert_eq!(c, s);
        c.assert_invariants();
    }

    #[test]
    fn set_into_iter_completeness() {
        let mut s = OpenHashSet::<i32>::new();
        for i in 0..200 {
            s.insert(i);
        }
        let mut v: Vec<i32> = s.into_iter().collect();
        v.sort();
        assert_eq!(v, (0..200).collect::<Vec<_>>());
    }

    #[test]
    fn set_try_reserve_grows_and_avoids_subsequent_resize() {
        let mut s = OpenHashSet::<i32>::new();
        s.try_reserve(500).unwrap();
        let reserved = s.cap();
        assert!(max_load(reserved) >= 500);
        for i in 0..500 {
            s.insert(i);
        }
        assert_eq!(reserved, s.cap());
        assert_eq!(s.len(), 500);
        s.assert_invariants();
    }

    // ---- NaN / float parity ----

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
        assert!(!s.insert(HashableF32(f32::NAN)));
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
        use std::collections::hash_map::DefaultHasher;
        use std::hash::BuildHasherDefault;
        type Fixed = BuildHasherDefault<DefaultHasher>;
        let mut m: OpenHashMap<i32, i32, Fixed> = OpenHashMap::with_hasher(Fixed::default());
        m.insert(1, 10);
        m.insert(2, 20);
        assert_eq!(m.get(&1), Some(&10));
        assert_eq!(m.get(&2), Some(&20));
        let _: &Fixed = m.hasher();

        let s: OpenHashSet<i32, Fixed> = [1, 2, 3].into_iter().collect();
        assert_eq!(s.len(), 3);
        let _: &Fixed = s.hasher();
    }

    #[test]
    fn map_debug_format() {
        let mut m = OpenHashMap::<i32, i32>::new();
        m.insert(1, 2);
        let s = format!("{m:?}");
        assert!(s.contains("1") && s.contains("2"));
    }

    // ---- randomized invariant fuzz (deterministic xorshift) ----

    struct XorShift(u64);
    impl XorShift {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
    }

    #[test]
    fn map_randomized_invariants() {
        let mut rng = XorShift(0x9E3779B97F4A7C15);
        let mut m = OpenHashMap::<i64, i64>::new();
        let mut model = std::collections::HashMap::<i64, i64>::new();
        for step in 0..20_000u64 {
            let k = (rng.next() % 500) as i64;
            let op = rng.next() % 3;
            match op {
                0 => {
                    let v = rng.next() as i64;
                    assert_eq!(m.insert(k, v), model.insert(k, v));
                }
                1 => {
                    assert_eq!(m.remove(&k), model.remove(&k));
                }
                _ => {
                    assert_eq!(m.get(&k), model.get(&k));
                }
            }
            if step % 250 == 0 {
                m.assert_invariants();
                assert_eq!(m.len(), model.len());
            }
        }
        m.assert_invariants();
        assert_eq!(m.len(), model.len());
        for (k, v) in &model {
            assert_eq!(m.get(k), Some(v));
        }
    }

    #[test]
    fn set_randomized_invariants() {
        let mut rng = XorShift(0xDEADBEEFCAFEBABE);
        let mut s = OpenHashSet::<i64>::new();
        let mut model = std::collections::HashSet::<i64>::new();
        for step in 0..20_000u64 {
            let k = (rng.next() % 400) as i64;
            let op = rng.next() % 3;
            match op {
                0 => assert_eq!(s.insert(k), model.insert(k)),
                1 => assert_eq!(s.remove(&k), model.remove(&k)),
                _ => assert_eq!(s.contains(&k), model.contains(&k)),
            }
            if step % 250 == 0 {
                s.assert_invariants();
                assert_eq!(s.len(), model.len());
            }
        }
        s.assert_invariants();
        assert_eq!(s.len(), model.len());
    }
}


#[cfg(test)]
mod panic_safety_tests {
    //! Regression tests for the panic/unwind-safety fixes (codex review round 2):
    //! a panicking user `Hash`/`Clone`/`Drop` must not leak entries or leave the
    //! table internally inconsistent.
    use super::*;
    use std::cell::Cell;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    thread_local! {
        // Net live `Tracked` instances (created - dropped). Must return to 0.
        static LIVE: Cell<i64> = const { Cell::new(0) };
    }

    #[derive(PartialEq, Eq)]
    struct Tracked {
        v: u64,
        panic_on_drop: bool,
    }
    impl Tracked {
        fn new(v: u64) -> Self {
            LIVE.with(|c| c.set(c.get() + 1));
            Tracked { v, panic_on_drop: false }
        }
    }
    impl Clone for Tracked {
        fn clone(&self) -> Self {
            Tracked::new(self.v)
        }
    }
    impl Hash for Tracked {
        fn hash<H: Hasher>(&self, h: &mut H) {
            self.v.hash(h);
        }
    }
    impl Drop for Tracked {
        fn drop(&mut self) {
            if self.panic_on_drop {
                self.panic_on_drop = false; // avoid abort on double-panic
                panic!("Tracked drop panic");
            }
            LIVE.with(|c| c.set(c.get() - 1));
        }
    }

    /// Deterministic, controllable hasher: can be told to panic on clone or drop.
    struct TestHasher {
        panic_clone: bool,
        panic_drop: bool,
    }
    impl BuildHasher for TestHasher {
        type Hasher = DefaultHasher;
        fn build_hasher(&self) -> DefaultHasher {
            DefaultHasher::new()
        }
    }
    impl Clone for TestHasher {
        fn clone(&self) -> Self {
            if self.panic_clone {
                panic!("hasher clone panic");
            }
            TestHasher {
                panic_clone: self.panic_clone,
                panic_drop: self.panic_drop,
            }
        }
    }
    impl Drop for TestHasher {
        fn drop(&mut self) {
            if self.panic_drop {
                self.panic_drop = false;
                panic!("hasher drop panic");
            }
        }
    }

    #[test]
    fn clone_does_not_leak_when_hasher_clone_panics() {
        LIVE.with(|c| c.set(0));
        {
            let mut m: OpenHashMap<u64, Tracked, TestHasher> = OpenHashMap::with_hasher(TestHasher {
                panic_clone: true,
                panic_drop: false,
            });
            for i in 0..50u64 {
                m.insert(i, Tracked::new(i));
            }
            assert_eq!(LIVE.with(|c| c.get()), 50);
            // clone() clones all 50 entries (LIVE -> 100), then hasher.clone()
            // panics; the guard must drop the 50 cloned entries (LIVE -> 50).
            let r = catch_unwind(AssertUnwindSafe(|| m.clone()));
            assert!(r.is_err(), "expected hasher clone panic");
            assert_eq!(LIVE.with(|c| c.get()), 50, "cloned entries leaked");
            // dropping the original (its hasher has panic_clone, not panic_drop).
        }
        assert_eq!(LIVE.with(|c| c.get()), 0, "original entries leaked");
    }

    #[test]
    fn into_iter_does_not_leak_when_hasher_drop_panics() {
        LIVE.with(|c| c.set(0));
        let mut m: OpenHashMap<u64, Tracked, TestHasher> = OpenHashMap::with_hasher(TestHasher {
            panic_clone: false,
            panic_drop: true,
        });
        for i in 0..50u64 {
            m.insert(i, Tracked::new(i));
        }
        assert_eq!(LIVE.with(|c| c.get()), 50);
        // into_iter reads ctrl/entries/hasher out, then drops the hasher which
        // panics; the just-constructed IntoIter must drop the 50 entries.
        let r = catch_unwind(AssertUnwindSafe(|| {
            let _it = m.into_iter();
            // unreachable: into_iter panics while dropping the hasher.
        }));
        assert!(r.is_err(), "expected hasher drop panic");
        assert_eq!(LIVE.with(|c| c.get()), 0, "entries leaked on into_iter hasher panic");
    }

    #[test]
    fn clear_stays_consistent_when_value_drop_panics() {
        LIVE.with(|c| c.set(0));
        let mut m: OpenHashMap<u64, Tracked> = OpenHashMap::new();
        for i in 0..100u64 {
            m.insert(i, Tracked::new(i));
        }
        // Flag one value to panic when dropped during clear().
        m.get_mut(&40).unwrap().panic_on_drop = true;
        let r = catch_unwind(AssertUnwindSafe(|| m.clear()));
        assert!(r.is_err(), "expected value drop panic");
        // The fix: accounting + invariants must be consistent after the panic.
        // `assert_invariants` checks `len == count(FULL slots)`, which is the
        // property the pre-fix `clear` violated (stale `len`/`growth_left`).
        m.assert_invariants();
        // The table is still usable after the caught panic.
        let _ = m.insert(1000, Tracked::new(1000));
        assert!(m.contains_key(&1000));
        m.assert_invariants();
        drop(m);
    }
}
