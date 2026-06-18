// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! [`RangeSet`] — a mutable, auto-coalescing set of cut-regions over a
//! totally-ordered `T` (v1 ships the `i32` specialisation).
//!
//! A `RangeSet` stores a collection of **disjoint, non-empty, pairwise
//! non-connected** [`Range`]s (the *normal form*). It auto-coalesces on
//! [`add`](RangeSet::add): two ranges merge iff they are
//! [`is_connected`](Range::is_connected) — which is **broader** than mere
//! overlap, because an *abutment* (a cut-touch, e.g. `[1, 3)` & `[3, 5)`) is
//! also connected. Every coalescing / split / complement / ordering decision
//! reduces to the side-aware cut comparisons of [`crate::range`]; there is no
//! `(value, inclusive)` boolean reasoning and **no `±1` endpoint arithmetic**
//! (the `INT_MIN`/`INT_MAX` overflow trap).
//!
//! ## Cut-region, not integer-value-set
//!
//! Because Phase 0 has no `DiscreteDomain`, a `RangeSet` models *cut-regions*,
//! not the set of `i32` values they happen to contain. `add(open(1, 2))` over
//! `i32` produces a **non-empty** set whose single stored range `(1, 2)` is
//! cut-non-empty even though [`contains`](RangeSet::contains) is false for
//! every `i32`. So `{}` and `{(1, 2)}` are **distinct** RangeSets. Every
//! set-level predicate ([`is_empty`](RangeSet::is_empty), canonicality,
//! [`complement`](RangeSet::complement), [`intersects`](RangeSet::intersects),
//! [`span`](RangeSet::span)) is defined on the stored cut-regions; only the
//! point queries ([`contains`](RangeSet::contains) /
//! [`range_containing`](RangeSet::range_containing)) ask about an actual
//! `i32`.
//!
//! ## Backing
//!
//! The backing is a flat `Vec<Range<T>>` kept in the normal form (non-empty,
//! pairwise non-connected, ascending by lower cut). The order is unobservable
//! beyond [`as_ranges`](RangeSet::as_ranges); a tree keyed by lower cut would
//! give identical results. The flat vector is the cleanest match for the
//! `i32` validation universe and keeps the cut algebra in one place.

use crate::range::{Cut, Range};
use std::cmp::Ordering;

/// A mutable, auto-coalescing set of disjoint cut-regions over `T`.
///
/// See the [module docs](crate::range_set) for the cut-region semantics and
/// the normal-form invariant.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RangeSet<T> {
    /// Normal form: non-empty, pairwise non-connected, ascending by lower cut.
    ranges: Vec<Range<T>>,
}

impl<T: Ord + Copy> RangeSet<T> {
    /// An empty range set.
    pub fn new() -> Self {
        RangeSet { ranges: Vec::new() }
    }

    /// Union `range` in, coalescing **all connected** stored ranges. A
    /// **cut-empty** `range` (e.g. `closed_open(5, 5)`) is a **no-op**, decided
    /// by [`Range::is_empty`] (cut-empty), never by discrete cardinality —
    /// `add(open(1, 2))` over `i32` **stores** the range. The merged range
    /// keeps the **outer** cuts of every connected member (the cut `min`/`max`,
    /// no `±1` math).
    pub fn add(&mut self, range: Range<T>) {
        // Empty-range no-op (cut-empty), per the normative empty-range rule.
        if range.is_empty() {
            return;
        }
        // Merge `range` with every connected stored range, spanning all of
        // them. Connectivity (overlap OR abutment) is the coalescing predicate.
        let mut merged = range;
        let mut out: Vec<Range<T>> = Vec::with_capacity(self.ranges.len() + 1);
        for r in self.ranges.drain(..) {
            if r.is_connected(&merged) {
                merged = r.span(&merged);
            } else {
                out.push(r);
            }
        }
        // Insert `merged` at its ascending-by-lower-cut position.
        let pos = out
            .iter()
            .position(|r| cmp_lower(r, &merged) == Ordering::Greater)
            .unwrap_or(out.len());
        out.insert(pos, merged);
        self.ranges = out;
    }

    /// [`add`](RangeSet::add) each range; the final normal form is
    /// order-independent.
    pub fn add_all<I: IntoIterator<Item = Range<T>>>(&mut self, ranges: I) {
        for r in ranges {
            self.add(r);
        }
    }

    /// Subtract `range`, **splitting** any stored range straddling either
    /// boundary. A cut-empty `range` is a **no-op**. The split is pure cut
    /// arithmetic — the boundary cuts flip (`remove([4, 7))` from `[1, 9]`
    /// leaves `[1, 4)` and `[7, 9]`), never `±1`.
    pub fn remove(&mut self, range: Range<T>) {
        if range.is_empty() {
            return;
        }
        let mut out: Vec<Range<T>> = Vec::with_capacity(self.ranges.len() + 1);
        for r in self.ranges.drain(..) {
            // No cut-non-empty overlap -> keep `r` unchanged. Abutment alone
            // (cut-empty intersection) does not split.
            match r.intersection(&range) {
                Some(i) if !i.is_empty() => {
                    // Left fragment: r below the removed range's lower cut.
                    if cmp(r.lower_cut(), range.lower_cut()) == Ordering::Less {
                        out.push(Range::from_cuts_internal(r.lower_cut(), range.lower_cut()));
                    }
                    // Right fragment: r above the removed range's upper cut.
                    if cmp(range.upper_cut(), r.upper_cut()) == Ordering::Less {
                        out.push(Range::from_cuts_internal(range.upper_cut(), r.upper_cut()));
                    }
                }
                _ => out.push(r),
            }
        }
        self.ranges = out;
    }

    /// Whether `value` falls in some stored range. This is the **only**
    /// integer-point predicate — `(1, 2)` correctly contains no `i32`.
    pub fn contains(&self, value: T) -> bool {
        self.ranges.iter().any(|r| r.contains(value))
    }

    /// The stored range containing `value`, or `None`.
    pub fn range_containing(&self, value: T) -> Option<Range<T>> {
        self.ranges.iter().copied().find(|r| r.contains(value))
    }

    /// Whether some **single** stored range encloses `range` (cut-defined
    /// [`Range::encloses`]). A set covering `{[1, 3), [5, 9)}` does **not**
    /// enclose `[2, 6)` — no single stored range does.
    pub fn encloses(&self, range: &Range<T>) -> bool {
        self.ranges.iter().any(|r| r.encloses(range))
    }

    /// Whether [`encloses`](RangeSet::encloses) holds for **every** argument.
    pub fn encloses_all<'a, I: IntoIterator<Item = &'a Range<T>>>(&self, ranges: I) -> bool
    where
        T: 'a,
    {
        ranges.into_iter().all(|r| self.encloses(r))
    }

    /// Whether `range` has a **cut-non-empty intersection** with some stored
    /// range — pure cut algebra. An **abutment** is *not* an intersection
    /// (`intersects([3, 5))` against `[5, 9)` is false); a cut-empty query
    /// never intersects; but a discrete-empty-yet-cut-non-empty overlap **does**
    /// count (`intersects(open(1, 2))` against stored `(1, 2)` is **true**,
    /// though no `i32` lies in it).
    pub fn intersects(&self, range: &Range<T>) -> bool {
        self.ranges
            .iter()
            .filter_map(|r| r.intersection(range))
            .any(|i| !i.is_empty())
    }

    /// The minimum enclosing range `[min lower cut, max upper cut]`; `None` on
    /// an empty set.
    pub fn span(&self) -> Option<Range<T>> {
        let first = self.ranges.first()?;
        let last = self.ranges.last()?;
        Some(Range::from_cuts_internal(
            first.lower_cut(),
            last.upper_cut(),
        ))
    }

    /// A **new** independent `RangeSet` of the cut-region **gaps** between the
    /// stored ranges over the full `(-∞, +∞)` domain. `complement(empty)` =
    /// `{all()}`; `complement({all()})` = `{}`; no spurious `±∞` gap when an end
    /// is already unbounded; the boundary side flips (closed↔open at the same
    /// cut value). `complement ∘ complement == identity`.
    pub fn complement(&self) -> RangeSet<T> {
        let mut out: Vec<Range<T>> = Vec::new();
        // Walking cut: the lower cut of the next gap. Starts at `-∞`.
        let mut cursor: Cut<T> = Cut::BelowAll;
        for r in &self.ranges {
            // Gap from `cursor` up to this range's lower cut, when non-empty.
            if cmp(cursor, r.lower_cut()) == Ordering::Less {
                out.push(Range::from_cuts_internal(cursor, r.lower_cut()));
            }
            // Next gap starts just past this range's upper cut.
            cursor = r.upper_cut();
        }
        // Trailing gap from the last upper cut to `+∞`, when non-empty.
        if cmp(cursor, Cut::AboveAll) == Ordering::Less {
            out.push(Range::from_cuts_internal(cursor, Cut::AboveAll));
        }
        RangeSet { ranges: out }
    }

    /// A **new** independent `RangeSet` = this set **intersected** with `view`
    /// (the in-`view` slice, each stored range clipped to `view`).
    /// `sub_range_set([3, 6))` of `{[1, 5), [8, 9]}` = `{[3, 5)}`.
    pub fn sub_range_set(&self, view: &Range<T>) -> RangeSet<T> {
        let mut out: Vec<Range<T>> = Vec::new();
        for r in &self.ranges {
            if let Some(i) = r.intersection(view) {
                if !i.is_empty() {
                    out.push(i);
                }
            }
        }
        // The stored ranges are ascending and disjoint, so their clipped
        // images stay ascending, disjoint, and non-connected.
        RangeSet { ranges: out }
    }

    /// The canonical disjoint ranges, **ascending by lower cut**.
    pub fn as_ranges(&self) -> impl Iterator<Item = Range<T>> + '_ {
        self.ranges.iter().copied()
    }

    /// Whether the set has **no stored ranges**. A cut-region predicate —
    /// `{(1, 2)}` is **not** empty even though it contains no `i32`.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Remove all ranges.
    pub fn clear(&mut self) {
        self.ranges.clear();
    }
}

/// Total cut comparison (the four-variant `Cut` order; see [`crate::range`]).
fn cmp<T: Ord>(a: Cut<T>, b: Cut<T>) -> Ordering {
    a.cmp_cut(&b)
}

/// Compare two ranges by their lower cut.
fn cmp_lower<T: Ord + Copy>(a: &Range<T>, b: &Range<T>) -> Ordering {
    cmp(a.lower_cut(), b.lower_cut())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::range::BoundType;

    fn rs(ranges: &[Range<i32>]) -> RangeSet<i32> {
        let mut s = RangeSet::new();
        s.add_all(ranges.iter().copied());
        s
    }

    fn collected(s: &RangeSet<i32>) -> Vec<Range<i32>> {
        s.as_ranges().collect()
    }

    #[test]
    fn coalesce_overlap() {
        let s = rs(&[Range::closed(1, 5), Range::closed(3, 9)]);
        assert_eq!(collected(&s), vec![Range::closed(1, 9)]);
        assert!(s.contains(4));
        assert!(!s.contains(10));
        assert_eq!(s.span(), Some(Range::closed(1, 9)));
    }

    #[test]
    fn coalesce_abut_cut_touch() {
        // [1,3) & [3,5) touch at Below(3) -> single [1,5).
        let s = rs(&[Range::closed_open(1, 3), Range::closed_open(3, 5)]);
        assert_eq!(collected(&s), vec![Range::closed_open(1, 5)]);
        assert!(s.contains(3));
        assert!(!s.contains(5));
        // The merged upper cut is Below(5) (open), the OUTER cut survives.
        assert_eq!(
            s.as_ranges().next().unwrap().upper_bound_type(),
            Some(BoundType::Open)
        );
    }

    #[test]
    fn open_gap_no_merge() {
        // (1,3) & (3,5): value 3 is the gap -> TWO ranges.
        let s = rs(&[Range::open(1, 3), Range::open(3, 5)]);
        assert_eq!(collected(&s), vec![Range::open(1, 3), Range::open(3, 5)]);
        assert!(!s.contains(3));
    }

    #[test]
    fn adjacent_closed_no_integer_adjacency_merge() {
        // [1,3] & [4,5]: cut model has no integer adjacency (Below(4) > Above(3)).
        let s = rs(&[Range::closed(1, 3), Range::closed(4, 5)]);
        assert_eq!(
            collected(&s),
            vec![Range::closed(1, 3), Range::closed(4, 5)]
        );
    }

    #[test]
    fn add_empty_is_noop() {
        let mut s = RangeSet::new();
        s.add(Range::closed_open(5, 5));
        assert!(s.is_empty());
        s.add(Range::open_closed(5, 5));
        assert!(s.is_empty());
    }

    #[test]
    fn add_open_no_integer_stores() {
        // open(1,2) is cut-non-empty -> stored, though it contains no i32.
        let s = rs(&[Range::open(1, 2)]);
        assert!(!s.is_empty());
        assert_eq!(collected(&s), vec![Range::open(1, 2)]);
        assert!(!s.contains(1));
        assert!(!s.contains(2));
    }

    #[test]
    fn remove_splits() {
        let mut s = rs(&[Range::closed(1, 9)]);
        s.remove(Range::closed_open(4, 7));
        assert_eq!(
            collected(&s),
            vec![Range::closed_open(1, 4), Range::closed(7, 9)]
        );
    }

    #[test]
    fn remove_empty_is_noop() {
        let mut s = rs(&[Range::closed(1, 9)]);
        s.remove(Range::closed_open(5, 5));
        assert_eq!(collected(&s), vec![Range::closed(1, 9)]);
    }

    #[test]
    fn remove_abutment_does_not_split() {
        // remove([5,9)) abuts [1,5) at Below(5) -> no change to [1,5).
        let mut s = rs(&[Range::closed_open(1, 5)]);
        s.remove(Range::closed_open(5, 9));
        assert_eq!(collected(&s), vec![Range::closed_open(1, 5)]);
    }

    #[test]
    fn contains_and_range_containing() {
        let s = rs(&[Range::closed_open(1, 5), Range::closed(8, 9)]);
        assert!(s.contains(3));
        assert!(!s.contains(6));
        assert_eq!(s.range_containing(3), Some(Range::closed_open(1, 5)));
        assert_eq!(s.range_containing(6), None);
    }

    #[test]
    fn encloses_single_range() {
        let s = rs(&[Range::closed_open(1, 3), Range::closed_open(5, 9)]);
        // No single stored range encloses [2,6).
        assert!(!s.encloses(&Range::closed_open(2, 6)));
        assert!(s.encloses(&Range::closed_open(1, 2)));
        assert!(s.encloses_all([Range::closed_open(1, 2), Range::closed_open(5, 8)].iter()));
        assert!(!s.encloses_all([Range::closed_open(1, 2), Range::closed_open(2, 6)].iter()));
    }

    #[test]
    fn intersects_cut_algebra() {
        let s = rs(&[Range::closed_open(1, 3), Range::closed_open(5, 9)]);
        // cut-non-empty overlap with both -> true.
        assert!(s.intersects(&Range::closed_open(2, 6)));
        // cut-empty query -> false.
        assert!(!s.intersects(&Range::closed_open(5, 5)));
        // abutment -> false ([3,5) abuts [5,9) at Below(5)).
        let s2 = rs(&[Range::closed_open(5, 9)]);
        assert!(!s2.intersects(&Range::closed_open(3, 5)));
    }

    #[test]
    fn intersects_open_cut_non_empty_no_integer() {
        // intersects(open(1,2)) vs stored (1,2) is TRUE (cut-non-empty), even
        // though no i32 lies in it.
        let s = rs(&[Range::open(1, 2)]);
        assert!(s.intersects(&Range::open(1, 2)));
    }

    #[test]
    fn complement_basic() {
        let s = rs(&[Range::closed(1, 5)]);
        let c = s.complement();
        assert_eq!(
            collected(&c),
            vec![Range::less_than(1), Range::greater_than(5)]
        );
    }

    #[test]
    fn complement_all_is_empty() {
        let s = rs(&[Range::all()]);
        assert!(s.complement().is_empty());
    }

    #[test]
    fn complement_empty_is_all() {
        let s: RangeSet<i32> = RangeSet::new();
        assert_eq!(collected(&s.complement()), vec![Range::all()]);
    }

    #[test]
    fn complement_unbounded_no_spurious_gap() {
        // complement(lessThan(10)) = {[10,+inf)}, no leading (-inf,..) gap.
        let s = rs(&[Range::less_than(10)]);
        assert_eq!(collected(&s.complement()), vec![Range::at_least(10)]);
    }

    #[test]
    fn complement_involution() {
        for ranges in [
            vec![Range::closed(1, 5)],
            vec![Range::open(1, 3), Range::open(3, 5)],
            vec![Range::less_than(10)],
            vec![Range::closed(i32::MIN, 0), Range::open_closed(0, i32::MAX)],
            vec![],
            vec![Range::all()],
        ] {
            let s = rs(&ranges);
            let cc = s.complement().complement();
            assert_eq!(
                collected(&s),
                collected(&cc),
                "involution failed for {ranges:?}"
            );
        }
    }

    #[test]
    fn sub_range_set_clips() {
        let s = rs(&[Range::closed_open(1, 5), Range::closed(8, 9)]);
        let sub = s.sub_range_set(&Range::closed_open(3, 6));
        assert_eq!(collected(&sub), vec![Range::closed_open(3, 5)]);
    }

    #[test]
    fn sub_range_set_independent_snapshot() {
        let s = rs(&[Range::closed_open(1, 5)]);
        let mut sub = s.sub_range_set(&Range::closed_open(2, 4));
        sub.add(Range::closed(100, 200));
        // mutating the snapshot must not touch the parent.
        assert_eq!(collected(&s), vec![Range::closed_open(1, 5)]);
    }

    #[test]
    fn signed_extremes_no_plus_minus_one() {
        let mut s = RangeSet::new();
        s.add(Range::closed(i32::MIN, 0));
        s.add(Range::open_closed(0, i32::MAX));
        // [MIN,0] and (0,MAX] abut at Above(0) -> coalesce to [MIN, MAX].
        assert_eq!(collected(&s), vec![Range::closed(i32::MIN, i32::MAX)]);
        assert!(s.contains(i32::MIN));
        assert!(s.contains(i32::MAX));
        assert_eq!(s.span(), Some(Range::closed(i32::MIN, i32::MAX)));
        // [MIN, MAX] is NOT all() (its cuts are Below(MIN)/Above(MAX), not the
        // unbounded sentinels), so its complement is the two flanking gaps
        // (-inf, MIN) and (MAX, +inf) — computed in cut space, no overflow.
        let c = s.complement();
        assert_eq!(
            collected(&c),
            vec![Range::less_than(i32::MIN), Range::greater_than(i32::MAX)]
        );

        // all() over the whole domain DOES complement to empty, no overflow.
        let mut whole: RangeSet<i32> = RangeSet::new();
        whole.add(Range::all());
        assert!(whole.complement().is_empty());
    }

    #[test]
    fn clear_empties() {
        let mut s = rs(&[Range::closed(1, 9)]);
        s.clear();
        assert!(s.is_empty());
        assert_eq!(collected(&s), vec![]);
    }

    #[test]
    fn normal_form_after_random_sequence() {
        // A deterministic op sequence: the invariant (non-empty, pairwise
        // non-connected, ascending) must hold afterwards.
        let mut s = RangeSet::new();
        let ops = [
            Range::closed(1, 5),
            Range::closed_open(10, 12),
            Range::closed_open(12, 15),
            Range::open(20, 25),
            Range::closed(4, 11),
        ];
        for r in ops {
            s.add(r);
        }
        let v = collected(&s);
        for w in v.windows(2) {
            assert_eq!(cmp_lower(&w[0], &w[1]), Ordering::Less, "ascending");
            assert!(!w[0].is_connected(&w[1]), "pairwise non-connected");
        }
        assert!(v.iter().all(|r| !r.is_empty()), "non-empty");
    }
}
