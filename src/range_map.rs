// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! [`RangeMap`] — a mutable piecewise mapping from disjoint non-empty
//! [`Range`]s to values (v1 ships the `i32 -> i32` specialisation).
//!
//! Like [`RangeSet`](crate::range_set::RangeSet), a `RangeMap` is **always
//! maximally merged** — but per value: [`put`](RangeMap::put) is
//! last-writer-wins (it clips/splits every overlapping prior entry) and then
//! **coalesces** the inserted entry with connected neighbours holding an
//! **equal** value. A **different** value is a barrier and is never absorbed or
//! crossed. The normal form therefore carries a global invariant: *no two
//! connected entries hold an equal value*.
//!
//! ## Divergence from Guava
//!
//! `TreeRangeMap::put` does not coalesce; coalescing lives in a separate
//! `putCoalescing`. We fold it into `put` and do **not** expose
//! `put_coalescing`. Guava's split is a compatibility retrofit (`RangeMap` is
//! `@since 14.0`, `putCoalescing` `@since 22.0`, by which point `put`'s
//! behaviour was observable through `asMapOfRanges()` and could not be
//! changed); we have no such constraint. See
//! `spec/features/range-set-map.md` §Coalescing.
//!
//! Every clip / split / merge / ordering decision reduces to the side-aware
//! cut comparisons of [`crate::range`]; there is **no `±1` endpoint
//! arithmetic** (the `INT_MIN`/`INT_MAX` overflow trap).
//!
//! ## Backing
//!
//! A flat `Vec<(Range<T>, V)>` kept in normal form: entry ranges non-empty,
//! pairwise disjoint, each value mapped by at most one point, ascending by
//! lower cut. The order is unobservable beyond
//! [`as_map_of_ranges`](RangeMap::as_map_of_ranges); a tree keyed by lower cut
//! would give identical results.

use crate::range::Range;
use std::cmp::Ordering;

/// A mutable piecewise mapping from disjoint ranges to values.
///
/// See the [module docs](crate::range_map) for the put / coalescing semantics
/// and the normal-form invariant.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RangeMap<T, V> {
    /// Normal form: non-empty, pairwise disjoint, ascending by lower cut.
    entries: Vec<(Range<T>, V)>,
}

impl<T: Ord + Copy, V: Copy + PartialEq> RangeMap<T, V> {
    /// An empty range map.
    pub fn new() -> Self {
        RangeMap {
            entries: Vec::new(),
        }
    }

    /// Assign `value` to **every** point of `range`, **last-writer-wins** over
    /// any prior overlap. Existing entries are clipped to the parts outside
    /// `range` (a straddling entry **splits into two**, both keeping the old
    /// value); the new `(range, value)` is then **coalesced** with any connected
    /// neighbour holding an **equal** value and inserted. A **different** value
    /// is a barrier. A **cut-empty** `range` is a **no-op**, decided before any
    /// clipping.
    ///
    /// `range` may be anything convertible into a [`Range<T>`], so std range
    /// syntax works: `map.put(2..5, v)`, `map.put(2..=5, v)`.
    pub fn put(&mut self, range: impl Into<Range<T>>, value: V) {
        let range = range.into();
        if range.is_empty() {
            return;
        }
        self.clip_out(&range);

        // Coalesce outward from the insertion position. Because the normal form
        // is maintained by every put, AT MOST ONE entry per side is absorbable:
        // if the neighbour is absorbed, the entry beyond it was already either
        // disconnected from it or differently-valued, and stays so against the
        // grown range. Each loop therefore runs at most once. They are loops
        // rather than ifs so a normal form violated by a bug elsewhere degrades
        // into a correct (if slower) result instead of a malformed map.
        let pos = self.insertion_point(&range);
        let mut merged = range;

        let mut lo = pos;
        while lo > 0 {
            let (r, v) = &self.entries[lo - 1];
            if *v != value || !r.is_connected(&merged) {
                break;
            }
            merged = r.span(&merged);
            lo -= 1;
        }

        let mut hi = pos;
        while hi < self.entries.len() {
            let (r, v) = &self.entries[hi];
            if *v != value || !r.is_connected(&merged) {
                break;
            }
            merged = r.span(&merged);
            hi += 1;
        }

        self.entries
            .splice(lo..hi, std::iter::once((merged, value)));
    }

    /// The value mapped at `value`, or `None` if uncovered.
    pub fn get(&self, value: T) -> Option<&V> {
        self.entries
            .iter()
            .find(|(r, _)| r.contains(value))
            .map(|(_, v)| v)
    }

    /// The `(range, value)` entry covering `value`, or `None`.
    pub fn get_entry(&self, value: T) -> Option<(Range<T>, &V)> {
        self.entries
            .iter()
            .find(|(r, _)| r.contains(value))
            .map(|(r, v)| (*r, v))
    }

    /// Unmap `range`, **splitting** any entry straddling either boundary (both
    /// fragments keep the old value). A cut-empty `range` is a **no-op**.
    ///
    /// `range` may be anything convertible into a [`Range<T>`] (`map.remove(2..5)`).
    pub fn remove(&mut self, range: impl Into<Range<T>>) {
        let range = range.into();
        if range.is_empty() {
            return;
        }
        self.clip_out(&range);
    }

    /// The minimum range enclosing all entry ranges; `None` on an empty map.
    pub fn span(&self) -> Option<Range<T>> {
        let first = self.entries.first()?;
        let last = self.entries.last()?;
        Some(Range::from_cuts_internal(
            first.0.lower_cut(),
            last.0.upper_cut(),
        ))
    }

    /// A **new** independent **SNAPSHOT** `RangeMap` restricted to `view` (each
    /// entry range clipped to `view`, values preserved).
    ///
    /// This is a **materialized copy, not a live write-through view** (unlike
    /// Guava's `RangeMap.subRangeMap`): later mutations of the original are
    /// **not** reflected here, and mutating this result does not affect the
    /// original.
    pub fn sub_range_map(&self, view: &Range<T>) -> RangeMap<T, V> {
        let mut out: Vec<(Range<T>, V)> = Vec::new();
        for (r, v) in &self.entries {
            if let Some(i) = r.intersection(view) {
                if !i.is_empty() {
                    out.push((i, *v));
                }
            }
        }
        RangeMap { entries: out }
    }

    /// The canonical disjoint `(range, value)` entries, **ascending by lower
    /// cut**.
    pub fn as_map_of_ranges(&self) -> impl Iterator<Item = (Range<T>, &V)> + '_ {
        self.entries.iter().map(|(r, v)| (*r, v))
    }

    /// Whether the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    // ---- internals --------------------------------------------------------

    /// Clip every entry to the parts **outside** `range` (the `remove` /
    /// overlap-resolution split). A straddling entry becomes two fragments;
    /// an entry fully inside `range` is dropped. Pure cut arithmetic — the
    /// boundary cuts flip, never `±1`. Abutment alone (cut-empty intersection)
    /// leaves an entry untouched.
    fn clip_out(&mut self, range: &Range<T>) {
        let mut out: Vec<(Range<T>, V)> = Vec::with_capacity(self.entries.len() + 1);
        for (r, v) in self.entries.drain(..) {
            match r.intersection(range) {
                Some(i) if !i.is_empty() => {
                    // Left fragment below the removed range's lower cut.
                    if cmp_lower_cut(&r, range) == Ordering::Less {
                        out.push((
                            Range::from_cuts_internal(r.lower_cut(), range.lower_cut()),
                            v,
                        ));
                    }
                    // Right fragment above the removed range's upper cut.
                    if cmp_upper_cut(range, &r) == Ordering::Less {
                        out.push((
                            Range::from_cuts_internal(range.upper_cut(), r.upper_cut()),
                            v,
                        ));
                    }
                }
                _ => out.push((r, v)),
            }
        }
        self.entries = out;
    }

    /// The ascending-by-lower-cut index at which `range` belongs: the first
    /// index whose lower cut is above `range`'s. Callers must have already
    /// cleared the overlap (via [`clip_out`]), so `range` is disjoint from every
    /// remaining entry and every entry below the returned index lies strictly to
    /// its left.
    fn insertion_point(&self, range: &Range<T>) -> usize {
        self.entries
            .iter()
            .position(|(r, _)| r.lower_cut().cmp_cut(&range.lower_cut()) == Ordering::Greater)
            .unwrap_or(self.entries.len())
    }
}

/// Consuming iterator over a [`RangeMap`]'s canonical `(range, value)` entries,
/// **ascending by lower cut** — the owned counterpart to
/// [`RangeMap::as_map_of_ranges`] (which borrows the values).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct RangeMapIntoIter<T, V> {
    inner: std::vec::IntoIter<(Range<T>, V)>,
}

impl<T, V> Iterator for RangeMapIntoIter<T, V> {
    type Item = (Range<T>, V);
    fn next(&mut self) -> Option<(Range<T>, V)> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T, V> DoubleEndedIterator for RangeMapIntoIter<T, V> {
    fn next_back(&mut self) -> Option<(Range<T>, V)> {
        self.inner.next_back()
    }
}

impl<T, V> ExactSizeIterator for RangeMapIntoIter<T, V> {}
impl<T, V> std::iter::FusedIterator for RangeMapIntoIter<T, V> {}

/// [`put`](RangeMap::put) each `(range, value)` in iterator order
/// (last-writer-wins on overlap; no coalescing across values).
impl<T: Ord + Copy, V: Copy + PartialEq> Extend<(Range<T>, V)> for RangeMap<T, V> {
    fn extend<I: IntoIterator<Item = (Range<T>, V)>>(&mut self, entries: I) {
        for (range, value) in entries {
            self.put(range, value);
        }
    }
}

/// Build a `RangeMap` from `(range, value)` entries (`iter.collect()`),
/// last-writer-wins over overlaps in iterator order.
impl<T: Ord + Copy, V: Copy + PartialEq> FromIterator<(Range<T>, V)> for RangeMap<T, V> {
    fn from_iter<I: IntoIterator<Item = (Range<T>, V)>>(iter: I) -> Self {
        let mut map = RangeMap::new();
        map.extend(iter);
        map
    }
}

/// Consumes the map, yielding its canonical `(range, value)` entries ascending
/// by lower cut (the [normal form](RangeMap)).
impl<T, V> IntoIterator for RangeMap<T, V> {
    type Item = (Range<T>, V);
    type IntoIter = RangeMapIntoIter<T, V>;
    fn into_iter(self) -> Self::IntoIter {
        RangeMapIntoIter {
            inner: self.entries.into_iter(),
        }
    }
}

fn cmp_lower_cut<T: Ord + Copy>(a: &Range<T>, b: &Range<T>) -> Ordering {
    a.lower_cut().cmp_cut(&b.lower_cut())
}

fn cmp_upper_cut<T: Ord + Copy>(a: &Range<T>, b: &Range<T>) -> Ordering {
    a.upper_cut().cmp_cut(&b.upper_cut())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collected(m: &RangeMap<i32, i32>) -> Vec<(Range<i32>, i32)> {
        m.as_map_of_ranges().map(|(r, v)| (r, *v)).collect()
    }

    #[test]
    fn put_basic() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed(8, 9), 200);
        assert_eq!(
            collected(&m),
            vec![(Range::closed_open(1, 5), 100), (Range::closed(8, 9), 200)]
        );
        assert_eq!(m.get(3), Some(&100));
        assert_eq!(m.get(6), None);
        assert_eq!(m.get(8), Some(&200));
    }

    #[test]
    fn accepts_std_range_syntax_via_into() {
        let mut m: RangeMap<i32, i32> = RangeMap::new();
        m.put(1..5, 100); // half-open std literal
        m.put(8..=9, 200); // inclusive
        assert_eq!(
            collected(&m),
            vec![(Range::closed_open(1, 5), 100), (Range::closed(8, 9), 200)]
        );
        assert_eq!(m.get(3), Some(&100));
        assert_eq!(m.get(8), Some(&200));
        // put via std syntax merges equal-valued abutters.
        m.put(5..8, 100);
        m.put(0..1, 100);
        assert_eq!(m.get(6), Some(&100));
        // remove via std syntax.
        m.remove(2..4);
        assert_eq!(m.get(3), None);
        assert_eq!(m.get(1), Some(&100));
        // Explicit `Range<T>` still accepted.
        m.put(Range::closed(20, 22), 999);
        assert_eq!(m.get(21), Some(&999));
    }

    #[test]
    fn into_iter_yields_ascending_normal_form() {
        let mut m = RangeMap::new();
        m.put(Range::closed(8, 9), 200);
        m.put(Range::closed_open(1, 5), 100);
        let borrowed = collected(&m);
        let owned: Vec<(Range<i32>, i32)> = m.into_iter().collect();
        assert_eq!(owned, borrowed);
        assert_eq!(
            owned,
            vec![(Range::closed_open(1, 5), 100), (Range::closed(8, 9), 200)]
        );
    }

    #[test]
    fn into_iter_double_ended_and_exact_size() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 10);
        m.put(Range::closed_open(5, 9), 20);
        m.put(Range::closed_open(9, 12), 30);
        let mut it = m.into_iter();
        assert_eq!(it.len(), 3); // ExactSizeIterator
        assert_eq!(it.next(), Some((Range::closed_open(1, 5), 10)));
        assert_eq!(it.next_back(), Some((Range::closed_open(9, 12), 30)));
        assert_eq!(it.next(), Some((Range::closed_open(5, 9), 20)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn into_iter_empty() {
        let m: RangeMap<i32, i32> = RangeMap::new();
        assert_eq!(m.into_iter().count(), 0);
    }

    #[test]
    fn from_iter_and_extend_last_writer_wins() {
        // collect() applies put in order; the later overlapping entry wins.
        let m: RangeMap<i32, i32> = [
            (Range::closed_open(1, 5), 10),
            (Range::closed_open(3, 9), 20),
        ]
        .into_iter()
        .collect();
        assert_eq!(m.get(2), Some(&10));
        assert_eq!(m.get(4), Some(&20));
        assert_eq!(m.get(8), Some(&20));

        let mut m2: RangeMap<i32, i32> = RangeMap::new();
        m2.extend([(Range::closed(1, 2), 1), (Range::closed(5, 6), 2)]);
        assert_eq!(m2.get(1), Some(&1));
        assert_eq!(m2.get(5), Some(&2));
    }

    #[test]
    fn put_overwrite_clips() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed_open(3, 9), 200);
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(1, 3), 100),
                (Range::closed_open(3, 9), 200)
            ]
        );
        assert_eq!(m.get(2), Some(&100));
        assert_eq!(m.get(4), Some(&200));
        assert_eq!(m.get(8), Some(&200));
    }

    #[test]
    fn put_split_straddle() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 9), 100);
        m.put(Range::closed_open(3, 5), 200);
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(1, 3), 100),
                (Range::closed_open(3, 5), 200),
                (Range::closed_open(5, 9), 100),
            ]
        );
        assert_eq!(m.get(2), Some(&100));
        assert_eq!(m.get(4), Some(&200));
        assert_eq!(m.get(6), Some(&100));
    }

    #[test]
    fn put_coalesces_equal_value_abut() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed_open(5, 9), 100);
        // ONE entry: equal value and abutting, so plain put merges them.
        // Guava's TreeRangeMap leaves two here; this is the divergence.
        assert_eq!(collected(&m), vec![(Range::closed_open(1, 9), 100)]);
        assert_eq!(m.get(5), Some(&100));
    }

    #[test]
    fn put_different_value_no_merge() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed_open(5, 9), 200);
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(1, 5), 100),
                (Range::closed_open(5, 9), 200)
            ]
        );
    }

    #[test]
    fn put_coalesces_both_sides() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed_open(9, 12), 100);
        m.put(Range::closed_open(5, 9), 100);
        assert_eq!(collected(&m), vec![(Range::closed_open(1, 12), 100)]);
    }

    #[test]
    fn put_coalesces_chain_ascending_order() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 2), 7);
        m.put(Range::closed_open(2, 3), 7);
        // A chain never forms: the map is already [1,3) here.
        assert_eq!(collected(&m), vec![(Range::closed_open(1, 3), 7)]);
        m.put(Range::closed_open(3, 4), 7);
        assert_eq!(collected(&m), vec![(Range::closed_open(1, 4), 7)]);
    }

    #[test]
    fn put_coalesces_order_independent() {
        // Mirror of put_coalesces_chain_ascending_order: same three puts,
        // inserted so the existing entries lie to the RIGHT of the last one.
        let mut m = RangeMap::new();
        m.put(Range::closed_open(2, 3), 7);
        m.put(Range::closed_open(3, 4), 7);
        m.put(Range::closed_open(1, 2), 7);
        assert_eq!(collected(&m), vec![(Range::closed_open(1, 4), 7)]);
    }

    #[test]
    fn put_different_value_is_a_hard_barrier() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 2), 7);
        m.put(Range::closed_open(2, 3), 8);
        m.put(Range::closed_open(3, 4), 7);
        // The 8 entry is neither absorbed nor crossed, so the far [1,2) -> 7 is
        // unreachable even though both hold 7.
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(1, 2), 7),
                (Range::closed_open(2, 3), 8),
                (Range::closed_open(3, 4), 7)
            ]
        );
    }

    #[test]
    fn put_split_fragments_do_not_rejoin_across_the_insert() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 9), 100);
        m.put(Range::closed_open(3, 5), 200);
        // The two 100 fragments are separated by the 200 entry, so they are not
        // connected and must not be re-merged by the coalescing step.
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(1, 3), 100),
                (Range::closed_open(3, 5), 200),
                (Range::closed_open(5, 9), 100)
            ]
        );
    }

    #[test]
    fn normal_form_has_no_connected_equal_valued_pair() {
        // The global invariant that the old put/put_coalescing split could not
        // state: after every operation, no two connected entries hold an equal
        // value.
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 2), 7);
        m.put(Range::closed_open(2, 3), 7);
        m.put(Range::closed_open(3, 4), 8);
        m.put(Range::closed_open(4, 5), 8);
        m.put(Range::closed_open(5, 6), 7);
        let v = collected(&m);
        for w in v.windows(2) {
            assert!(
                !(w[0].0.is_connected(&w[1].0) && w[0].1 == w[1].1),
                "connected entries must not hold an equal value"
            );
        }
        assert_eq!(
            v,
            vec![
                (Range::closed_open(1, 3), 7),
                (Range::closed_open(3, 5), 8),
                (Range::closed_open(5, 6), 7)
            ]
        );
    }

    #[test]
    fn remove_splits() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 9), 100);
        m.remove(Range::closed_open(4, 7));
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(1, 4), 100),
                (Range::closed_open(7, 9), 100)
            ]
        );
        assert_eq!(m.get(5), None);
    }

    #[test]
    fn get_entry_lookup() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        assert_eq!(m.get_entry(3), Some((Range::closed_open(1, 5), &100)));
        assert_eq!(m.get_entry(6), None);
    }

    #[test]
    fn span_over_entries() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed(8, 9), 200);
        // span = [lower of first entry, upper of last entry] = [1, 9].
        assert_eq!(m.span(), Some(Range::closed(1, 9)));
    }

    #[test]
    fn empty_put_is_noop() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(5, 5), 100);
        assert!(m.is_empty());
        assert_eq!(collected(&m), vec![]);
    }

    #[test]
    fn sub_range_map_clips_snapshot() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed(8, 9), 200);
        let sub = m.sub_range_map(&Range::closed_open(3, 6));
        assert_eq!(collected(&sub), vec![(Range::closed_open(3, 5), 100)]);
        // snapshot independence: mutate the parent, sub unchanged.
        let mut sub2 = sub.clone();
        m.put(Range::closed(3, 3), 999);
        assert_eq!(collected(&sub2), vec![(Range::closed_open(3, 5), 100)]);
        sub2.put(Range::closed(50, 60), 7);
        // mutating the snapshot does not touch the parent.
        assert_eq!(m.get(55), None);
    }

    #[test]
    fn signed_extremes_no_plus_minus_one() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(i32::MIN, 0), 1);
        m.put(Range::closed(0, i32::MAX), 2);
        assert_eq!(m.get(i32::MIN), Some(&1));
        assert_eq!(m.get(0), Some(&2));
        assert_eq!(m.get(i32::MAX), Some(&2));
    }

    #[test]
    fn normal_form_disjoint_after_sequence() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 10), 1);
        m.put(Range::closed_open(3, 5), 2);
        m.put(Range::closed_open(7, 20), 3);
        m.put(Range::closed_open(20, 25), 3);
        let v = collected(&m);
        for w in v.windows(2) {
            assert_eq!(
                w[0].0.lower_cut().cmp_cut(&w[1].0.lower_cut()),
                Ordering::Less,
                "ascending"
            );
            // disjoint: no cut-non-empty intersection between entries.
            let inter = w[0].0.intersection(&w[1].0);
            assert!(inter.map(|i| i.is_empty()).unwrap_or(true), "disjoint");
        }
        assert!(v.iter().all(|(r, _)| !r.is_empty()), "non-empty");
    }

    #[test]
    fn put_unbounded_chains_on_both_sides_collapse_to_all() {
        // No ±1 endpoint arithmetic: sentinel cuts span straight through.
        // Each unbounded pair merges as it lands, then the middle piece bridges
        // the two tails into all().
        let mut m = RangeMap::new();
        m.put(Range::less_than(-5), 7);
        m.put(Range::closed_open(-5, 0), 7);
        m.put(Range::closed_open(5, 10), 7);
        m.put(Range::at_least(10), 7);
        assert_eq!(
            collected(&m),
            vec![(Range::less_than(0), 7), (Range::at_least(5), 7)]
        );
        m.put(Range::closed_open(0, 5), 7);
        assert_eq!(collected(&m), vec![(Range::all(), 7)]);
        assert_eq!(m.get(i32::MIN), Some(&7));
        assert_eq!(m.get(0), Some(&7));
        assert_eq!(m.get(i32::MAX), Some(&7));
    }

    #[test]
    fn put_rejoins_clipped_fragment_and_chain_beyond_it() {
        // The insert overlaps the tail of an equal-valued entry and the head of
        // a different-valued one: the equal fragment rejoins, the other is
        // clipped and stays a barrier.
        let mut m = RangeMap::new();
        m.put(Range::closed_open(0, 10), 7);
        m.put(Range::closed_open(10, 12), 9);
        m.put(Range::closed_open(6, 11), 7);
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(0, 11), 7),
                (Range::closed_open(11, 12), 9)
            ]
        );
        assert_eq!(m.get(0), Some(&7));
        assert_eq!(m.get(10), Some(&7));
        assert_eq!(m.get(11), Some(&9));
    }

    #[test]
    fn put_rejoins_both_clip_fragments_of_a_straddled_equal_entry() {
        // An insert strictly inside an equal-valued entry splits it into two
        // fragments; both must rejoin so the map is unchanged.
        let mut m = RangeMap::new();
        m.put(Range::closed_open(0, 20), 7);
        m.put(Range::closed_open(6, 14), 7);
        assert_eq!(collected(&m), vec![(Range::closed_open(0, 20), 7)]);
        // Same shape with different-valued neighbours on both sides: they are
        // barriers and stay put.
        let mut m = RangeMap::new();
        m.put(Range::closed_open(0, 2), 1);
        m.put(Range::closed_open(2, 18), 7);
        m.put(Range::closed_open(18, 20), 1);
        let before = collected(&m);
        m.put(Range::closed_open(6, 14), 7);
        assert_eq!(collected(&m), before);
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(0, 2), 1),
                (Range::closed_open(2, 18), 7),
                (Range::closed_open(18, 20), 1)
            ]
        );
    }

    #[test]
    fn put_cut_empty_range_at_an_abutment_is_noop() {
        // The empty check must precede clip_out — otherwise a cut-empty range
        // sitting exactly on the abutment (either form) or strictly inside an
        // entry could still disturb its host.
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed_open(5, 9), 200);
        let before = vec![
            (Range::closed_open(1, 5), 100),
            (Range::closed_open(5, 9), 200),
        ];
        assert_eq!(collected(&m), before);
        m.put(Range::closed_open(5, 5), 100);
        assert_eq!(collected(&m), before, "[5,5) on the abutment");
        m.put(Range::open_closed(5, 5), 200);
        assert_eq!(collected(&m), before, "(5,5] on the abutment");
        m.put(Range::closed_open(3, 3), 999);
        assert_eq!(collected(&m), before, "[3,3) inside an entry");
    }

    #[test]
    fn put_no_integer_range_is_a_stored_barrier() {
        // `(1, 2)` is cut-non-empty but holds no i32. It must still be stored,
        // split the enclosing entry, and act as a barrier between the two
        // equal-valued fragments — no point-level test can see this, which is
        // why the dense oracle needs it pinned exactly.
        let mut m = RangeMap::new();
        m.put(Range::all(), 1);
        m.put(Range::open(1, 2), 2);
        assert_eq!(
            collected(&m),
            vec![
                (Range::at_most(1), 1),
                (Range::open(1, 2), 2),
                (Range::at_least(2), 1)
            ]
        );
        // Removing it leaves the two fragments apart: `(1, 2)` is a real gap.
        m.remove(Range::open(1, 2));
        assert_eq!(
            collected(&m),
            vec![(Range::at_most(1), 1), (Range::at_least(2), 1)]
        );
    }

    #[test]
    fn remove_unbounded_clips_to_exact_sentinel_cut() {
        // The surviving fragment starts at exactly the removed range's upper
        // cut — Below(0) or Above(0) — with no ±1 endpoint arithmetic.
        let mut m = RangeMap::new();
        m.put(Range::all(), 1);
        m.remove(Range::less_than(0));
        assert_eq!(collected(&m), vec![(Range::at_least(0), 1)]);

        let mut m = RangeMap::new();
        m.put(Range::all(), 1);
        m.remove(Range::at_most(0));
        assert_eq!(collected(&m), vec![(Range::greater_than(0), 1)]);
    }

    #[test]
    fn put_remove_random_ops_match_dense_oracle() {
        // Differential test over a small dense domain, checked after EVERY op
        // against a naive per-point oracle: the mapping (get / get_entry), the
        // normal form (ascending, cut-non-empty, pairwise disjoint, no two
        // connected entries with an equal value) and a full reconstruction of
        // the dense array from the entries. Endpoints are drawn from a band
        // narrower than the domain, and the unbounded factories are drawn too,
        // so the BelowAll / AboveAll sentinels are exercised at the edges.
        const LO: i32 = -12;
        const HI: i32 = 12;
        const OPS: usize = 400;
        let points = || LO..=HI;
        let idx = |p: i32| (p - LO) as usize;

        for seed in [0x1234_5678u64, 0xdead_beef, 0x0bad_cafe] {
            let mut m: RangeMap<i32, i32> = RangeMap::new();
            let mut oracle: Vec<Option<i32>> = vec![None; idx(HI) + 1];
            let mut x = seed;
            let mut next = move || {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (x >> 33) as i32
            };
            for step in 0..OPS {
                // Endpoint band -8..8; cut-empty draws are let through and
                // must be no-ops (the oracle sees no point inside them).
                let a = next().rem_euclid(17) - 8;
                let b = next().rem_euclid(17) - 8;
                let (a, b) = (a.min(b), a.max(b));
                let r = match next().rem_euclid(9) {
                    0 => Range::closed(a, b),
                    1 if a < b => Range::open(a, b),
                    1 => Range::closed_open(a, b), // open(v, v) is invalid
                    2 => Range::closed_open(a, b),
                    3 => Range::open_closed(a, b),
                    4 => Range::less_than(a),
                    5 => Range::at_most(a),
                    6 => Range::greater_than(b),
                    7 => Range::at_least(b),
                    _ => Range::all(),
                };
                let v = next().rem_euclid(3) + 1;
                let is_put = next().rem_euclid(10) < 7;
                if is_put {
                    m.put(r, v);
                } else {
                    m.remove(r);
                }
                for p in points() {
                    if r.contains(p) {
                        oracle[idx(p)] = if is_put { Some(v) } else { None };
                    }
                }

                // (a) + (b): pointwise lookup and the entry it comes from.
                for p in points() {
                    let want = oracle[idx(p)];
                    assert_eq!(
                        m.get(p).copied(),
                        want,
                        "seed {seed:#x} step {step} get({p})"
                    );
                    match (m.get_entry(p), want) {
                        (None, None) => {}
                        (Some((er, ev)), Some(w)) => {
                            assert!(
                                er.contains(p),
                                "seed {seed:#x} step {step} entry {er:?} at {p}"
                            );
                            assert_eq!(*ev, w, "seed {seed:#x} step {step} get_entry({p})");
                        }
                        (got, want) => {
                            panic!("seed {seed:#x} step {step} get_entry({p}): {got:?} vs {want:?}")
                        }
                    }
                }

                // (c): normal form over the entry iterator.
                let entries = collected(&m);
                assert!(
                    entries.iter().all(|(r, _)| !r.is_empty()),
                    "seed {seed:#x} step {step} cut-empty entry"
                );
                for w in entries.windows(2) {
                    assert_eq!(
                        w[0].0.lower_cut().cmp_cut(&w[1].0.lower_cut()),
                        Ordering::Less,
                        "seed {seed:#x} step {step} ascending"
                    );
                    assert!(
                        w[0].0
                            .intersection(&w[1].0)
                            .map(|i| i.is_empty())
                            .unwrap_or(true),
                        "seed {seed:#x} step {step} disjoint"
                    );
                    assert!(
                        !(w[0].0.is_connected(&w[1].0) && w[0].1 == w[1].1),
                        "seed {seed:#x} step {step} connected equal-valued pair {:?} / {:?}",
                        w[0],
                        w[1]
                    );
                }

                // (d): the entries reproduce the dense array exactly.
                let mut rebuilt: Vec<Option<i32>> = vec![None; oracle.len()];
                for (er, ev) in &entries {
                    for p in points() {
                        if er.contains(p) {
                            assert!(
                                rebuilt[idx(p)].is_none(),
                                "seed {seed:#x} step {step} point {p} covered twice"
                            );
                            rebuilt[idx(p)] = Some(*ev);
                        }
                    }
                }
                assert_eq!(rebuilt, oracle, "seed {seed:#x} step {step} reconstruction");
            }
        }
    }
}
