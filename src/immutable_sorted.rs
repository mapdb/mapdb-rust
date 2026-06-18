// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Compact immutable sorted map / set (`sorted-table-map`).
//!
//! A purpose-built, **pointerless** immutable sorted collection: keys (and,
//! for a map, the matching values) are packed into contiguous parallel
//! [`Vec`]s and queried by binary search. The on-heap analogue of MapDB 3's
//! `SortedTableMap` — we port the observable behaviour and the packed-array +
//! binary-search mechanism, not the off-heap `Volume`/byte-offset machinery.
//!
//! This is **distinct** from the frozen-copy [`crate::ImmutableHashMap`] /
//! [`crate::immutable::ImmutableList`] wrappers (those seal a live structure's
//! per-entry layout against mutation) and from [`crate::Interval`] (a *virtual*
//! arithmetic progression with no stored elements).
//!
//! ## Layout — flat single sorted array (the reference default)
//!
//! The reference port stores **one flat ascending array pair** (`keys` +
//! parallel `values` for a map, a single `elems` array for a set). MapDB 3
//! paged the arrays with a per-page key directory; **paging is a legal but
//! unobservable implementation choice** (lookup, iteration, range, `size`/
//! `is_empty` results are identical regardless). A flat array is the simplest
//! representation that is trivially paging-invariant, so it is the reference.
//!
//! ## Construction is the only way in — built once from sorted input
//!
//! [`ImmutableSortedMap::from_sorted`] / [`ImmutableSortedSet::from_sorted`]
//! take a **strictly ascending** snapshot. Construction **traps** (panics —
//! the family's bad-input posture, like [`crate::Range`]'s out-of-order
//! constructor and [`crate::Interval`]'s minimum step) unless every adjacent
//! input pair satisfies `keys[i-1] < keys[i]` strictly:
//!
//! * out-of-order input (`keys[i] < keys[i-1]`) panics;
//! * a duplicate key (`keys[i] == keys[i-1]`) panics — **no last-wins / dedup**;
//! * (map) a `keys`/`values` length mismatch panics.
//!
//! Empty input (`from_sorted(&[], &[])`) is valid and builds an empty
//! collection; single-element input is valid. Construction **copies** the
//! input, so the built collection is a snapshot independent of the caller's
//! source slices (mutating them afterwards never affects the collection).
//!
//! ## Immutable — no mutators
//!
//! The types expose **no** `insert`/`put`/`add`/`remove`/`clear`/`set`. There
//! is nothing to trap on a mutator: the methods simply do not exist.
//!
//! ## Iterators: materialized snapshots
//!
//! The ascending/descending and range iterators return **materialized [`Vec`]
//! snapshots** (`Vec<K>` / `Vec<(K, V)>`), matching the convention the
//! [`crate::object::treemap::TreeMap`] range/descending methods already use
//! (`navigable-map.md`'s slice/iterator equivalence). A materialized snapshot
//! and a lazy iterator are observably identical; the snapshot keeps the API
//! `Copy`-bounded and trivially independent of `&self`'s lifetime.
//!
//! v1 ships the `i32` surface (the cross-language validation universe). The
//! types stay generic over `K/T: Ord + Copy` so the float / wider-integer
//! matrix widens later exactly as [`crate::Interval`] and [`crate::Range`] did;
//! ordering goes through [`Ord`] (binary search / `Ord::cmp`), never a bare
//! `<` on a generic, so float keys will widen by supplying a total-order
//! wrapper ([`crate::HashableF32`]) with no algorithm change.

use crate::range::Range;

/// Panic message helper for the strictly-ascending construction check.
#[cold]
#[inline(never)]
fn trap_not_ascending() -> ! {
    panic!("ImmutableSorted: input must be strictly ascending (no duplicate or out-of-order keys)");
}

/// Verify a slice is strictly ascending under [`Ord`]; trap otherwise. Empty
/// and single-element slices vacuously pass. Comparison goes through
/// `Ord::cmp` (never a bare `<`), so a float-total-order key type widens with
/// no change.
fn assert_strictly_ascending<T: Ord>(xs: &[T]) {
    for pair in xs.windows(2) {
        // `pair[0] < pair[1]` strictly; equal or greater traps.
        if pair[0].cmp(&pair[1]) != std::cmp::Ordering::Less {
            trap_not_ascending();
        }
    }
}

// ===========================================================================
// ImmutableSortedMap<K, V>
// ===========================================================================

/// A compact immutable sorted map backed by packed parallel arrays
/// (`keys[i]` -> `values[i]`), queried by binary search. Built once from
/// strictly-ascending input via [`from_sorted`](Self::from_sorted); thereafter
/// immutable. See the [module docs](self).
#[derive(Clone, Debug)]
pub struct ImmutableSortedMap<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
}

impl<K: Ord + Copy, V: Copy> ImmutableSortedMap<K, V> {
    /// Build from **strictly ascending** parallel slices: `values[i]` is the
    /// value of `keys[i]`. The input is **copied** (snapshot — independent of
    /// the caller's slices).
    ///
    /// # Panics
    ///
    /// Traps (panics) if `keys.len() != values.len()`, if the keys are not
    /// strictly ascending (out-of-order), or if any key is duplicated. There
    /// is no last-wins/dedup and no silent sort — a caller who wants those
    /// sorts/dedups first. Empty and single-element input are valid.
    pub fn from_sorted(keys: &[K], values: &[V]) -> Self {
        if keys.len() != values.len() {
            panic!(
                "ImmutableSortedMap::from_sorted: keys/values length mismatch ({} != {})",
                keys.len(),
                values.len()
            );
        }
        assert_strictly_ascending(keys);
        Self {
            keys: keys.to_vec(),
            values: values.to_vec(),
        }
    }

    /// Build from an iterator of `(key, value)` pairs that the caller pushes
    /// in **strictly ascending** key order (the data-pump sink shape). The
    /// pairs are materialized and validated exactly like
    /// [`from_sorted`](Self::from_sorted).
    ///
    /// # Panics
    ///
    /// Same traps as [`from_sorted`](Self::from_sorted) (out-of-order or
    /// duplicate keys).
    pub fn from_sorted_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let (keys, values): (Vec<K>, Vec<V>) = iter.into_iter().unzip();
        Self::from_sorted(&keys, &values)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Binary-search index of `key` if present, else the lower-bound insertion
    /// index. The overflow-safe midpoint lives in `slice::binary_search_by`
    /// (`lo + (hi - lo) / 2`), so it is correct at `i32::MIN`/`i32::MAX`.
    fn search(&self, key: &K) -> Result<usize, usize> {
        self.keys.binary_search_by(|probe| probe.cmp(key))
    }

    /// The value for `key`, or `None` if absent.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.search(key).ok().map(|i| &self.values[i])
    }

    /// Whether `key` is present.
    pub fn contains_key(&self, key: &K) -> bool {
        self.search(key).is_ok()
    }

    /// Minimum key, or `None` if empty.
    pub fn first_key(&self) -> Option<&K> {
        self.keys.first()
    }

    /// Maximum key, or `None` if empty.
    pub fn last_key(&self) -> Option<&K> {
        self.keys.last()
    }

    /// Minimum `(key, value)` entry, or `None`.
    pub fn first_entry(&self) -> Option<(&K, &V)> {
        Some((self.keys.first()?, self.values.first()?))
    }

    /// Maximum `(key, value)` entry, or `None`.
    pub fn last_entry(&self) -> Option<(&K, &V)> {
        Some((self.keys.last()?, self.values.last()?))
    }

    // ── Point navigation (NavigableMap surface, reused verbatim) ─────
    //
    // floor `<= k`, ceiling `>= k`, lower `< k` (strict), higher `> k`
    // (strict). All resolve to a single binary search over the packed key
    // array; the index arithmetic never computes a `k ± 1`, so it is
    // overflow-safe at the signed extremes.

    /// Greatest key `<= k`, or `None`.
    pub fn floor_key(&self, k: &K) -> Option<&K> {
        self.floor_index(k).map(|i| &self.keys[i])
    }

    /// Greatest key `<= k` and its value, or `None`.
    pub fn floor_entry(&self, k: &K) -> Option<(&K, &V)> {
        self.floor_index(k)
            .map(|i| (&self.keys[i], &self.values[i]))
    }

    /// Least key `>= k`, or `None`.
    pub fn ceiling_key(&self, k: &K) -> Option<&K> {
        self.ceiling_index(k).map(|i| &self.keys[i])
    }

    /// Least key `>= k` and its value, or `None`.
    pub fn ceiling_entry(&self, k: &K) -> Option<(&K, &V)> {
        self.ceiling_index(k)
            .map(|i| (&self.keys[i], &self.values[i]))
    }

    /// Greatest key `< k` (strict), or `None`.
    pub fn lower_key(&self, k: &K) -> Option<&K> {
        self.lower_index(k).map(|i| &self.keys[i])
    }

    /// Greatest key `< k` (strict) and its value, or `None`.
    pub fn lower_entry(&self, k: &K) -> Option<(&K, &V)> {
        self.lower_index(k)
            .map(|i| (&self.keys[i], &self.values[i]))
    }

    /// Least key `> k` (strict), or `None`.
    pub fn higher_key(&self, k: &K) -> Option<&K> {
        self.higher_index(k).map(|i| &self.keys[i])
    }

    /// Least key `> k` (strict) and its value, or `None`.
    pub fn higher_entry(&self, k: &K) -> Option<(&K, &V)> {
        self.higher_index(k)
            .map(|i| (&self.keys[i], &self.values[i]))
    }

    /// Index of the greatest key `<= k`. With `search` returning `Ok(i)` (hit
    /// at `i`) or `Err(i)` (lower-bound insertion point), floor is the hit
    /// itself, or `i - 1` for an absent key.
    fn floor_index(&self, k: &K) -> Option<usize> {
        match self.search(k) {
            Ok(i) => Some(i),
            Err(i) => i.checked_sub(1),
        }
    }

    /// Index of the greatest key `< k` (strict): the entry just below the
    /// lower-bound insertion point, never the key itself.
    fn lower_index(&self, k: &K) -> Option<usize> {
        let i = match self.search(k) {
            Ok(i) | Err(i) => i,
        };
        i.checked_sub(1)
    }

    /// Index of the least key `>= k`: the lower-bound insertion point (a hit
    /// returns its own index).
    fn ceiling_index(&self, k: &K) -> Option<usize> {
        let i = match self.search(k) {
            Ok(i) | Err(i) => i,
        };
        (i < self.keys.len()).then_some(i)
    }

    /// Index of the least key `> k` (strict): one past a hit, or the
    /// lower-bound insertion point for an absent key.
    fn higher_index(&self, k: &K) -> Option<usize> {
        let i = match self.search(k) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        (i < self.keys.len()).then_some(i)
    }

    // ── Order statistics (rank / select) ────────────────────────────
    //
    // On a flat ascending array `rank` IS the lower-bound binary-search index
    // and `select(i)` IS `keys[i]`, so they are trivially consistent with the
    // iteration order — no subtree-size augmentation needed.

    /// Number of keys **strictly less than** `key` — the 0-based lower-bound
    /// index `key` occupies (if present) or would occupy (if absent). In
    /// `0..=len()`. Defined for present and absent keys.
    pub fn rank(&self, key: &K) -> usize {
        match self.search(key) {
            Ok(i) | Err(i) => i,
        }
    }

    /// The `i`-th smallest key (0-based), or `None` if `i >= len()`. Round-trips
    /// with [`rank`](Self::rank): `select_key(rank(k)) == Some(k)` for present
    /// `k`.
    pub fn select_key(&self, i: usize) -> Option<&K> {
        self.keys.get(i)
    }

    /// The `i`-th smallest `(key, value)` entry (0-based), or `None`.
    pub fn select_entry(&self, i: usize) -> Option<(&K, &V)> {
        Some((self.keys.get(i)?, self.values.get(i)?))
    }

    // ── Iteration (ascending) ───────────────────────────────────────

    /// Keys in ascending order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.keys.iter()
    }

    /// Values in **ascending-key order** (paired with [`keys`](Self::keys)),
    /// NOT sorted by value.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.values.iter()
    }

    /// `(key, value)` entries in ascending key order.
    pub fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.values.iter())
    }

    // ── Iteration (descending) — required, not optional ─────────────

    /// All keys, descending.
    pub fn descending_keys(&self) -> Vec<K> {
        self.keys.iter().rev().copied().collect()
    }

    /// All `(key, value)` entries, descending.
    pub fn descending_entries(&self) -> Vec<(K, V)> {
        self.keys
            .iter()
            .rev()
            .zip(self.values.iter().rev())
            .map(|(k, v)| (*k, *v))
            .collect()
    }

    // ── Range queries (consume `Range<K>`; membership == range.contains) ──
    //
    // The in-range entries form a CONTIGUOUS slice of the packed array (the
    // range is convex), bracketed by two binary searches via
    // `Range::bracket`. The brackets come from the range's CUT semantics
    // (`Below(v)`/`Above(v)`/unbounded), never from `v ± 1` arithmetic, so
    // open/closed bounds at `INT_MIN`/`INT_MAX` do not overflow. `open(1, 2)`
    // over i32 yields an empty slice (membership is `contains`, never inferred
    // cut-emptiness).

    /// Keys whose key ∈ `range`, ascending.
    pub fn range_keys(&self, range: Range<K>) -> Vec<K> {
        let (lo, hi) = range.bracket(&self.keys);
        self.keys[lo..hi].to_vec()
    }

    /// `(key, value)` entries whose key ∈ `range`, ascending.
    pub fn range_entries(&self, range: Range<K>) -> Vec<(K, V)> {
        let (lo, hi) = range.bracket(&self.keys);
        self.keys[lo..hi]
            .iter()
            .zip(self.values[lo..hi].iter())
            .map(|(k, v)| (*k, *v))
            .collect()
    }

    /// Keys whose key ∈ `range`, descending.
    pub fn descending_range_keys(&self, range: Range<K>) -> Vec<K> {
        let mut v = self.range_keys(range);
        v.reverse();
        v
    }

    /// `(key, value)` entries whose key ∈ `range`, descending.
    pub fn descending_range_entries(&self, range: Range<K>) -> Vec<(K, V)> {
        let mut v = self.range_entries(range);
        v.reverse();
        v
    }
}

// ===========================================================================
// ImmutableSortedSet<T>
// ===========================================================================

/// A compact immutable sorted set backed by a single packed ascending array,
/// queried by binary search. The element analogue of [`ImmutableSortedMap`].
#[derive(Clone, Debug)]
pub struct ImmutableSortedSet<T> {
    elems: Vec<T>,
}

impl<T: Ord + Copy> ImmutableSortedSet<T> {
    /// Build from a **strictly ascending** element slice (copied — snapshot).
    ///
    /// # Panics
    ///
    /// Traps (panics) if the elements are not strictly ascending or contain a
    /// duplicate. Empty and single-element input are valid.
    pub fn from_sorted(elements: &[T]) -> Self {
        assert_strictly_ascending(elements);
        Self {
            elems: elements.to_vec(),
        }
    }

    /// Build from an iterator of elements the caller pushes in **strictly
    /// ascending** order (the sink shape).
    ///
    /// # Panics
    ///
    /// Same traps as [`from_sorted`](Self::from_sorted).
    pub fn from_sorted_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let elems: Vec<T> = iter.into_iter().collect();
        Self::from_sorted(&elems)
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.elems.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    fn search(&self, elem: &T) -> Result<usize, usize> {
        self.elems.binary_search_by(|probe| probe.cmp(elem))
    }

    /// Whether `elem` is present.
    pub fn contains(&self, elem: &T) -> bool {
        self.search(elem).is_ok()
    }

    /// Minimum element, or `None`.
    pub fn first(&self) -> Option<&T> {
        self.elems.first()
    }

    /// Maximum element, or `None`.
    pub fn last(&self) -> Option<&T> {
        self.elems.last()
    }

    /// Greatest element `<= k`, or `None`.
    pub fn floor(&self, k: &T) -> Option<&T> {
        match self.search(k) {
            Ok(i) => Some(&self.elems[i]),
            Err(i) => i.checked_sub(1).map(|j| &self.elems[j]),
        }
    }

    /// Least element `>= k`, or `None`.
    pub fn ceiling(&self, k: &T) -> Option<&T> {
        let i = match self.search(k) {
            Ok(i) | Err(i) => i,
        };
        self.elems.get(i)
    }

    /// Greatest element `< k` (strict), or `None`.
    pub fn lower(&self, k: &T) -> Option<&T> {
        let i = match self.search(k) {
            Ok(i) | Err(i) => i,
        };
        i.checked_sub(1).map(|j| &self.elems[j])
    }

    /// Least element `> k` (strict), or `None`.
    pub fn higher(&self, k: &T) -> Option<&T> {
        let i = match self.search(k) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        self.elems.get(i)
    }

    /// Number of elements **strictly less than** `elem` (lower-bound index).
    pub fn rank(&self, elem: &T) -> usize {
        match self.search(elem) {
            Ok(i) | Err(i) => i,
        }
    }

    /// The `i`-th smallest element (0-based), or `None` if `i >= len()`.
    pub fn select(&self, i: usize) -> Option<&T> {
        self.elems.get(i)
    }

    /// Elements in ascending order.
    pub fn elements(&self) -> impl Iterator<Item = &T> {
        self.elems.iter()
    }

    /// All elements, descending.
    pub fn descending_elements(&self) -> Vec<T> {
        self.elems.iter().rev().copied().collect()
    }

    /// Elements ∈ `range`, ascending. Bracketed by two binary searches from
    /// the range's cut semantics (overflow-safe at the signed extremes).
    pub fn range_elements(&self, range: Range<T>) -> Vec<T> {
        let (lo, hi) = range.bracket(&self.elems);
        self.elems[lo..hi].to_vec()
    }

    /// Elements ∈ `range`, descending.
    pub fn descending_range_elements(&self, range: Range<T>) -> Vec<T> {
        let mut v = self.range_elements(range);
        v.reverse();
        v
    }
}

#[cfg(test)]
mod tests;
