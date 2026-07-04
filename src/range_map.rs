// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! [`RangeMap`] — a mutable piecewise mapping from disjoint non-empty
//! [`Range`]s to values (v1 ships the `i32 -> i32` specialisation).
//!
//! **Unlike [`RangeSet`](crate::range_set::RangeSet), a `RangeMap` does NOT
//! coalesce across different values.** [`put`](RangeMap::put) is
//! last-writer-wins: it clips/splits every overlapping prior entry and inserts
//! the new `(range, value)`, but leaves adjacent equal-valued entries
//! **distinct**. [`put_coalescing`](RangeMap::put_coalescing) is the variant
//! that merges connected neighbours holding an **equal** value.
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
/// See the [module docs](crate::range_map) for the put / put-coalescing
/// semantics and the normal-form invariant.
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
    /// value); the new `(range, value)` is then inserted. A **cut-empty**
    /// `range` is a **no-op**. `put` does **not** coalesce — an adjacent equal
    /// value stays a distinct entry.
    pub fn put(&mut self, range: Range<T>, value: V) {
        if range.is_empty() {
            return;
        }
        self.clip_out(&range);
        self.insert_entry(range, value);
    }

    /// Like [`put`](RangeMap::put), then **merge** the inserted entry with any
    /// **connected** (overlapping *or* abutting) neighbour whose value
    /// **equals** `value`, producing one entry spanning the union. Neighbours
    /// with a different value are left untouched (clipped by the `put` step as
    /// usual).
    pub fn put_coalescing(&mut self, range: Range<T>, value: V) {
        if range.is_empty() {
            return;
        }
        self.clip_out(&range);
        // Span over every connected entry with an EQUAL value, dropping them.
        let mut merged = range;
        let mut out: Vec<(Range<T>, V)> = Vec::with_capacity(self.entries.len() + 1);
        for (r, v) in self.entries.drain(..) {
            if v == value && r.is_connected(&merged) {
                merged = r.span(&merged);
                // Growing `merged` rightward can bridge an equal-valued entry
                // already emitted to `out` on the left (entries are sorted, so
                // only the tail can newly connect). A single forward pass would
                // otherwise leave `put([0,5),v); put([5,10),v);
                // put_coalescing([10,15),v)` as two entries instead of one
                // `[0,15)`. Pull back the connected-equal tail.
                while out
                    .last()
                    .is_some_and(|(lr, lv)| *lv == value && lr.is_connected(&merged))
                {
                    let (lr, _) = out.pop().unwrap();
                    merged = lr.span(&merged);
                }
            } else {
                out.push((r, v));
            }
        }
        self.entries = out;
        self.insert_entry(merged, value);
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
    pub fn remove(&mut self, range: Range<T>) {
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

    /// Insert `(range, value)` at its ascending-by-lower-cut position. Callers
    /// must have already cleared the overlap (via [`clip_out`]); `range` is
    /// disjoint from every remaining entry.
    fn insert_entry(&mut self, range: Range<T>, value: V) {
        let pos = self
            .entries
            .iter()
            .position(|(r, _)| r.lower_cut().cmp_cut(&range.lower_cut()) == Ordering::Greater)
            .unwrap_or(self.entries.len());
        self.entries.insert(pos, (range, value));
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
    fn put_does_not_coalesce() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed_open(5, 9), 100);
        // TWO entries even though value equal and they abut.
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(1, 5), 100),
                (Range::closed_open(5, 9), 100)
            ]
        );
        assert_eq!(m.get(5), Some(&100));
    }

    #[test]
    fn put_coalescing_equal_value_abut() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put_coalescing(Range::closed_open(5, 9), 100);
        assert_eq!(collected(&m), vec![(Range::closed_open(1, 9), 100)]);
    }

    #[test]
    fn put_coalescing_different_value_no_merge() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put_coalescing(Range::closed_open(5, 9), 200);
        assert_eq!(
            collected(&m),
            vec![
                (Range::closed_open(1, 5), 100),
                (Range::closed_open(5, 9), 200)
            ]
        );
    }

    #[test]
    fn put_coalescing_both_sides() {
        let mut m = RangeMap::new();
        m.put(Range::closed_open(1, 5), 100);
        m.put(Range::closed_open(9, 12), 100);
        m.put_coalescing(Range::closed_open(5, 9), 100);
        assert_eq!(collected(&m), vec![(Range::closed_open(1, 12), 100)]);
    }

    #[test]
    fn put_coalescing_bridges_already_emitted_left_entry() {
        // Regression: a single forward drain emitted the left entry before a
        // later entry grew `merged` enough to bridge it. `put`+`put`+
        // `put_coalescing` on abutting equal values must yield ONE entry.
        let mut m = RangeMap::new();
        m.put(Range::closed_open(0, 5), 7);
        m.put(Range::closed_open(5, 10), 7);
        m.put_coalescing(Range::closed_open(10, 15), 7);
        assert_eq!(collected(&m), vec![(Range::closed_open(0, 15), 7)]);
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
        m.put_coalescing(Range::closed_open(20, 25), 3);
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
}
