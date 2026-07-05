// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Bound / Range value model — a pure in-memory value type describing a
//! region `[lo, hi)`, `(-∞, hi]`, `(lo, +∞)`, … with each endpoint
//! independently unbounded / open / closed.
//!
//! This is **not** [`crate::Interval`] (which materialises an arithmetic
//! progression and enumerates elements). A [`Range`] holds no elements; it
//! only describes a region (`contains(x)`) and supports the open/unbounded
//! endpoints `Interval` cannot.
//!
//! The design follows Google Guava's `Range<C>` / `BoundType` / `Cut`. The
//! algebra (`intersection`, `span`, `is_connected`, `encloses`) is total and
//! unambiguous because endpoints are modelled as **cuts between values**
//! rather than `(value, inclusive)` pairs. See
//! `spec/features/bound-range.md` for the normative algorithms; every
//! operation here reduces to a side-aware cut comparison, never to a
//! `(value, inclusive)` boolean.
//!
//! ## Side-aware cut ordering
//!
//! `Unbounded` is *contextual*: as a lower cut it is `-∞`, as an upper cut it
//! is `+∞`. There is therefore no single context-free order on one
//! `Unbounded` value. We avoid that trap by splitting the unbounded state
//! into two distinct sentinels — [`Cut::BelowAll`] (`-∞`) and
//! [`Cut::AboveAll`] (`+∞`). With those two sentinels the four-variant [`Cut`]
//! enum has a *single total order*
//! (`BelowAll < Below(v) < Above(v) < AboveAll`, finite cuts breaking ties by
//! value then `Below < Above`), and the three spec comparators
//! (`compare_lower_cuts`, `compare_upper_cuts`, `compare_lower_to_upper`) all
//! collapse onto it. A lower cut never holds `AboveAll`; an upper cut never
//! holds `BelowAll`; that invariant is established by the factories.
//!
//! v1 ships the `i32` specialisation (matching the cross-language validation
//! universe); the type stays generic over `T: Ord + Copy` so the float /
//! wider-integer matrix widens later exactly as `Interval` did.

use std::cmp::Ordering;
use std::fmt;

/// The kind of a finite endpoint: `Open` (exclusive) or `Closed` (inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundType {
    Open,
    Closed,
}

/// A cut sits *between* values (Guava's `Cut`). The four-variant form carries
/// two distinct unbounded sentinels (`BelowAll` = `-∞`, `AboveAll` = `+∞`) so
/// the enum has a single, total, context-free order — there is no lone
/// `Unbounded` value with an ambiguous position.
///
/// Total order: `BelowAll < Below(v) < Above(v) < AboveAll`. Finite cuts at
/// different values order by value; at the same value `Below(v) < Above(v)`.
///
/// Endpoint meaning:
/// - `Below(v)` — closed **lower** `[v`, or open **upper** `v)`.
/// - `Above(v)` — open **lower** `(v`, or closed **upper** `v]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cut<T> {
    /// `-∞`. Only ever a lower cut.
    BelowAll,
    /// The cut immediately below `v`.
    Below(T),
    /// The cut immediately above `v`.
    Above(T),
    /// `+∞`. Only ever an upper cut.
    AboveAll,
}

impl<T: Ord> Cut<T> {
    /// Total order on cuts (the single source of truth for the algebra). The
    /// three side-aware spec comparators all reduce to this because the two
    /// unbounded states are distinct sentinels rather than one ambiguous
    /// `Unbounded`.
    pub(crate) fn cmp(&self, other: &Cut<T>) -> Ordering {
        fn rank<T>(c: &Cut<T>) -> u8 {
            match c {
                Cut::BelowAll => 0,
                Cut::Below(_) | Cut::Above(_) => 1,
                Cut::AboveAll => 2,
            }
        }
        match (self, other) {
            (Cut::Below(a) | Cut::Above(a), Cut::Below(b) | Cut::Above(b)) => match a.cmp(b) {
                Ordering::Equal => {
                    // Same value: Below(v) < Above(v).
                    let sa = matches!(self, Cut::Above(_));
                    let sb = matches!(other, Cut::Above(_));
                    sa.cmp(&sb)
                }
                ord => ord,
            },
            _ => rank(self).cmp(&rank(other)),
        }
    }

    /// Total order on cuts — the single source of truth for the cut algebra,
    /// exposed for the [`RangeSet`](crate::range_set::RangeSet) /
    /// [`RangeMap`](crate::range_map::RangeMap) coalescing, split, complement,
    /// and ascending-by-lower-cut ordering. Because the four-variant `Cut`
    /// carries two distinct unbounded sentinels, this single order already
    /// realises all three spec comparators (`compare_lower_cuts`,
    /// `compare_upper_cuts`, `compare_lower_to_upper`).
    pub fn cmp_cut(&self, other: &Cut<T>) -> Ordering {
        self.cmp(other)
    }
}

/// An ordered region `(lowerCut, upperCut)` over a totally-ordered `T`, with
/// the invariant `lowerCut <= upperCut`. Equality and hashing are structural
/// on the two cuts — `closed_open(v, v)` and `open_closed(v, v)` are distinct
/// (both empty) values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Range<T> {
    lower: Cut<T>,
    upper: Cut<T>,
}

impl<T: Ord + Copy> Range<T> {
    /// Construct from raw cuts after validating `lower <= upper`. **Panics**
    /// if the cuts are out of order (a programming error, like
    /// `Interval::reversed` at the minimum step).
    fn from_cuts(lower: Cut<T>, upper: Cut<T>) -> Self {
        if lower.cmp(&upper) == Ordering::Greater {
            panic!("Range: lower cut must not exceed upper cut");
        }
        Range { lower, upper }
    }

    /// Construct a [`Range`] directly from two [`Cut`]s, validating
    /// `lower <= upper`. Crate-internal so the
    /// [`RangeSet`](crate::range_set::RangeSet) /
    /// [`RangeMap`](crate::range_map::RangeMap) split / complement / clip logic
    /// can re-assemble a range from the cut endpoints it computed (the
    /// boundary-flip of `remove`/`complement`, the clip of `sub_range_set`),
    /// keeping all boundary arithmetic in cut space — never `±1` endpoint math.
    /// **Panics** if `lower > upper`. Deliberately **not** part of the public
    /// API: it does not re-check cut-side legality (a lower cut must never be
    /// `AboveAll`, an upper cut never `BelowAll`), an invariant the public
    /// factories establish; all in-crate callers pass cuts copied from valid
    /// ranges, so the side invariant is preserved by construction.
    pub(crate) fn from_cuts_internal(lower: Cut<T>, upper: Cut<T>) -> Self {
        Self::from_cuts(lower, upper)
    }

    // ---- factories (Guava-parity names) -----------------------------------

    /// `(a, b)` — both endpoints open.
    ///
    /// # Panics
    ///
    /// Panics if `a >= b` (including `open(v, v)`, which is empty-but-invalid as
    /// an open range — the lower cut would exceed the upper).
    pub fn open(a: T, b: T) -> Self {
        Self::from_cuts(Cut::Above(a), Cut::Below(b))
    }

    /// `[a, b]` — both endpoints closed.
    ///
    /// # Panics
    ///
    /// Panics if `a > b` (the lower cut would exceed the upper).
    pub fn closed(a: T, b: T) -> Self {
        Self::from_cuts(Cut::Below(a), Cut::Above(b))
    }

    /// `(a, b]`.
    ///
    /// # Panics
    ///
    /// Panics if `a > b` (the lower cut would exceed the upper).
    pub fn open_closed(a: T, b: T) -> Self {
        Self::from_cuts(Cut::Above(a), Cut::Above(b))
    }

    /// `[a, b)`. `closed_open(v, v)` is the valid empty range
    /// `(Below(v), Below(v))`.
    ///
    /// # Panics
    ///
    /// Panics if `a > b` (the lower cut would exceed the upper).
    pub fn closed_open(a: T, b: T) -> Self {
        Self::from_cuts(Cut::Below(a), Cut::Below(b))
    }

    /// `(a, +∞)`.
    pub fn greater_than(a: T) -> Self {
        Self::from_cuts(Cut::Above(a), Cut::AboveAll)
    }

    /// `[a, +∞)`.
    pub fn at_least(a: T) -> Self {
        Self::from_cuts(Cut::Below(a), Cut::AboveAll)
    }

    /// `(-∞, b)`.
    pub fn less_than(b: T) -> Self {
        Self::from_cuts(Cut::BelowAll, Cut::Below(b))
    }

    /// `(-∞, b]`.
    pub fn at_most(b: T) -> Self {
        Self::from_cuts(Cut::BelowAll, Cut::Above(b))
    }

    /// `(-∞, +∞)`.
    pub fn all() -> Self {
        Range {
            lower: Cut::BelowAll,
            upper: Cut::AboveAll,
        }
    }

    /// `[v, v]`.
    pub fn singleton(v: T) -> Self {
        Self::from_cuts(Cut::Below(v), Cut::Above(v))
    }

    /// Build a [`Range`] from any [`std::ops::RangeBounds`] — the std-syntax
    /// counterpart to the Guava-parity factories above. Each std bound maps
    /// onto a [`Cut`]: `Unbounded` → `BelowAll` (lower) / `AboveAll` (upper),
    /// `Included` → `Below` (lower) / `Above` (upper), `Excluded` → `Above`
    /// (lower) / `Below` (upper). So `a..b` is `closed_open(a, b)`, `a..=b` is
    /// `closed(a, b)`, `a..` is `at_least(a)`, `..b` is `less_than(b)`, `..=b`
    /// is `at_most(b)`, `..` is `all()`, and an explicit
    /// `(Excluded(a), Included(b))` tuple is `open_closed(a, b)`. The six
    /// concrete `From<std::ops::Range*>` impls delegate here.
    ///
    /// # Panics
    ///
    /// Panics if the resulting lower cut exceeds the upper — a reversed range
    /// such as `5..2` or `5..=2`, or the empty-but-invalid open `(v, v)` — the
    /// same trap the panicking factories (`open`/`closed`/…) apply. `a..a`
    /// (`closed_open`) is the valid empty range; `a..=a` is the singleton
    /// `[a, a]`.
    pub fn from_bounds<R: std::ops::RangeBounds<T>>(r: R) -> Self {
        use std::ops::Bound;
        let lower = match r.start_bound() {
            Bound::Unbounded => Cut::BelowAll,
            Bound::Included(&a) => Cut::Below(a),
            Bound::Excluded(&a) => Cut::Above(a),
        };
        let upper = match r.end_bound() {
            Bound::Unbounded => Cut::AboveAll,
            Bound::Included(&b) => Cut::Above(b),
            Bound::Excluded(&b) => Cut::Below(b),
        };
        Self::from_cuts(lower, upper)
    }

    // ---- queries ----------------------------------------------------------

    /// Whether `x` falls within the range (normative `contains`).
    pub fn contains(&self, x: T) -> bool {
        let lower_ok = match self.lower {
            Cut::BelowAll => true,
            Cut::Below(v) => v <= x,
            Cut::Above(v) => v < x,
            Cut::AboveAll => false,
        };
        let upper_ok = match self.upper {
            Cut::AboveAll => true,
            Cut::Below(v) => x < v,
            Cut::Above(v) => x <= v,
            Cut::BelowAll => false,
        };
        lower_ok && upper_ok
    }

    /// **Cut-empty**: `lowerCut == upperCut`. NOT discrete cardinality —
    /// `open(1, 2)` over `i32` is *not* empty (no `DiscreteDomain` in
    /// Phase 0).
    pub fn is_empty(&self) -> bool {
        self.lower.cmp(&self.upper) == Ordering::Equal
    }

    /// `Some(BoundType)` of the lower endpoint; `None` when unbounded.
    pub fn lower_bound_type(&self) -> Option<BoundType> {
        match self.lower {
            Cut::Below(_) => Some(BoundType::Closed),
            Cut::Above(_) => Some(BoundType::Open),
            Cut::BelowAll | Cut::AboveAll => None,
        }
    }

    /// `Some(BoundType)` of the upper endpoint; `None` when unbounded.
    pub fn upper_bound_type(&self) -> Option<BoundType> {
        match self.upper {
            Cut::Below(_) => Some(BoundType::Open),
            Cut::Above(_) => Some(BoundType::Closed),
            Cut::BelowAll | Cut::AboveAll => None,
        }
    }

    /// The lower endpoint value; `None` when unbounded below.
    pub fn lower_endpoint(&self) -> Option<T> {
        match self.lower {
            Cut::Below(v) | Cut::Above(v) => Some(v),
            Cut::BelowAll | Cut::AboveAll => None,
        }
    }

    /// The upper endpoint value; `None` when unbounded above.
    pub fn upper_endpoint(&self) -> Option<T> {
        match self.upper {
            Cut::Below(v) | Cut::Above(v) => Some(v),
            Cut::BelowAll | Cut::AboveAll => None,
        }
    }

    /// Whether the lower endpoint is finite.
    pub fn has_lower_bound(&self) -> bool {
        matches!(self.lower, Cut::Below(_) | Cut::Above(_))
    }

    /// Whether the upper endpoint is finite.
    pub fn has_upper_bound(&self) -> bool {
        matches!(self.upper, Cut::Below(_) | Cut::Above(_))
    }

    /// The lower [`Cut`] of this range (the cut sitting at the lower
    /// endpoint). `BelowAll` when unbounded below. Exposed so a packed
    /// sorted-array collection can bracket a contiguous in-range slice
    /// directly from the cut semantics (`Below(v)`/`Above(v)`/`BelowAll`),
    /// never from `±1` endpoint arithmetic — the overflow trap at
    /// `INT_MIN`/`INT_MAX` the `sorted-table-map` spec guards against.
    pub fn lower_cut(&self) -> Cut<T> {
        self.lower
    }

    /// The upper [`Cut`] of this range. `AboveAll` when unbounded above.
    /// See [`lower_cut`](Self::lower_cut).
    pub fn upper_cut(&self) -> Cut<T> {
        self.upper
    }

    /// Bracket the contiguous `[start, end)` index window of a **strictly
    /// ascending** slice whose elements fall inside this range. Membership
    /// over a sorted slice is contiguous (the range is convex), so two binary
    /// searches suffice: `start` is the first index `i` with
    /// `lower_cut < keys[i]` and `end` is the first index `i` with
    /// `upper_cut < keys[i]` (equivalently `!(keys[i] < upper_cut)`).
    ///
    /// The brackets are derived purely from the cut comparison — `Below(v)`
    /// vs `Above(v)` vs the unbounded sentinels — so open/closed bounds at
    /// `INT_MIN`/`INT_MAX` never compute a predecessor/successor (`v ± 1`)
    /// and never overflow. `start == end` is an empty (possibly cut-empty or
    /// discrete-empty, e.g. `open(1, 2)` over `i32`) result, never an error.
    pub fn bracket(&self, sorted: &[T]) -> (usize, usize) {
        // start: first index whose key is strictly ABOVE the lower cut, i.e.
        // the lower cut sits strictly below keys[i]. Equivalent to the lower
        // bound of the in-range window.
        let start = match self.lower {
            Cut::BelowAll => 0,
            // Closed lower `[v`: include v -> first key >= v.
            Cut::Below(v) => sorted.partition_point(|k| *k < v),
            // Open lower `(v`: exclude v -> first key > v.
            Cut::Above(v) => sorted.partition_point(|k| *k <= v),
            // A lower cut is never AboveAll (factory invariant); treat as empty.
            Cut::AboveAll => sorted.len(),
        };
        // end: first index whose key is NOT below the upper cut (one past the
        // last in-range key).
        let end = match self.upper {
            Cut::AboveAll => sorted.len(),
            // Open upper `v)`: exclude v -> first key >= v.
            Cut::Below(v) => sorted.partition_point(|k| *k < v),
            // Closed upper `v]`: include v -> first key > v.
            Cut::Above(v) => sorted.partition_point(|k| *k <= v),
            // An upper cut is never BelowAll (factory invariant); empty.
            Cut::BelowAll => 0,
        };
        // Clamp: a fully-disjoint range can yield start > end; normalise to
        // an empty window so callers can slice safely.
        if start > end {
            (end, end)
        } else {
            (start, end)
        }
    }

    // ---- algebra (all via cut comparison) ---------------------------------

    /// Cut-defined containment: `self.lower <= other.lower` and
    /// `self.upper >= other.upper`. NOT `∀ value ∈ other: contains(value)` —
    /// `[1, 5)` *encloses* the empty `[5, 5)` though `5 ∉ [1, 5)`.
    pub fn encloses(&self, other: &Range<T>) -> bool {
        self.lower.cmp(&other.lower) != Ordering::Greater
            && self.upper.cmp(&other.upper) != Ordering::Less
    }

    /// Whether there is a (possibly empty) range enclosed by both. Cut-equal
    /// endpoints count as connected (empty overlap).
    pub fn is_connected(&self, other: &Range<T>) -> bool {
        self.lower.cmp(&other.upper) != Ordering::Greater
            && other.lower.cmp(&self.upper) != Ordering::Greater
    }

    /// The overlap. `None` **only** when disconnected; abutting operands
    /// return a *present* cut-empty range at the touch point.
    pub fn intersection(&self, other: &Range<T>) -> Option<Range<T>> {
        if !self.is_connected(other) {
            return None;
        }
        let lower = max_cut(self.lower, other.lower);
        let upper = min_cut(self.upper, other.upper);
        Some(Range { lower, upper })
    }

    /// The smallest range enclosing both. No cross-shape canonicalisation.
    pub fn span(&self, other: &Range<T>) -> Range<T> {
        let lower = min_cut(self.lower, other.lower);
        let upper = max_cut(self.upper, other.upper);
        Range { lower, upper }
    }
}

fn max_cut<T: Ord + Copy>(a: Cut<T>, b: Cut<T>) -> Cut<T> {
    if a.cmp(&b) == Ordering::Less {
        b
    } else {
        a
    }
}

fn min_cut<T: Ord + Copy>(a: Cut<T>, b: Cut<T>) -> Cut<T> {
    if a.cmp(&b) == Ordering::Greater {
        b
    } else {
        a
    }
}

impl<T: Ord + Copy + fmt::Display> fmt::Display for Range<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.lower {
            Cut::BelowAll => write!(f, "(-\u{221e}")?,
            Cut::Below(v) => write!(f, "[{}", v)?,
            Cut::Above(v) => write!(f, "({}", v)?,
            Cut::AboveAll => write!(f, "(+\u{221e}")?,
        }
        write!(f, ", ")?;
        match self.upper {
            Cut::AboveAll => write!(f, "+\u{221e})"),
            Cut::Below(v) => write!(f, "{})", v),
            Cut::Above(v) => write!(f, "{}]", v),
            Cut::BelowAll => write!(f, "-\u{221e})"),
        }
    }
}

// ---- std range-syntax interop (T4) ---------------------------------------
//
// Each concrete `std::ops::Range*` literal converts to the equivalent Guava
// `Range<T>` via `from_bounds`, so callers can write `Range::from(2..5)` /
// `(2..5).into()` instead of `Range::closed_open(2, 5)`. Reversed two-bounded
// inputs panic (see `from_bounds`), matching the panicking factories.

impl<T: Ord + Copy> From<std::ops::Range<T>> for Range<T> {
    /// `a..b` → `closed_open(a, b)`. **Panics** if `a > b`.
    fn from(r: std::ops::Range<T>) -> Self {
        Self::from_bounds(r)
    }
}

impl<T: Ord + Copy> From<std::ops::RangeInclusive<T>> for Range<T> {
    /// `a..=b` → `closed(a, b)`. **Panics** if `a > b`.
    fn from(r: std::ops::RangeInclusive<T>) -> Self {
        Self::from_bounds(r)
    }
}

impl<T: Ord + Copy> From<std::ops::RangeFrom<T>> for Range<T> {
    /// `a..` → `at_least(a)`.
    fn from(r: std::ops::RangeFrom<T>) -> Self {
        Self::from_bounds(r)
    }
}

impl<T: Ord + Copy> From<std::ops::RangeTo<T>> for Range<T> {
    /// `..b` → `less_than(b)`.
    fn from(r: std::ops::RangeTo<T>) -> Self {
        Self::from_bounds(r)
    }
}

impl<T: Ord + Copy> From<std::ops::RangeToInclusive<T>> for Range<T> {
    /// `..=b` → `at_most(b)`.
    fn from(r: std::ops::RangeToInclusive<T>) -> Self {
        Self::from_bounds(r)
    }
}

impl<T: Ord + Copy> From<std::ops::RangeFull> for Range<T> {
    /// `..` → `all()`.
    fn from(_: std::ops::RangeFull) -> Self {
        Self::all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_closed() {
        let r: Range<i32> = Range::closed(10, 20);
        assert!(r.contains(10));
        assert!(r.contains(15));
        assert!(r.contains(20));
        assert!(!r.contains(9));
        assert!(!r.contains(21));
    }

    #[test]
    fn contains_open() {
        let r: Range<i32> = Range::open(10, 20);
        assert!(!r.contains(10));
        assert!(!r.contains(20));
        assert!(r.contains(11));
        assert!(r.contains(19));
    }

    #[test]
    fn contains_half_open() {
        let co: Range<i32> = Range::closed_open(10, 20);
        assert!(co.contains(10));
        assert!(!co.contains(20));
        let oc: Range<i32> = Range::open_closed(10, 20);
        assert!(!oc.contains(10));
        assert!(oc.contains(20));
    }

    #[test]
    fn contains_unbounded() {
        let all: Range<i32> = Range::all();
        assert!(all.contains(i32::MIN));
        assert!(all.contains(0));
        assert!(all.contains(i32::MAX));

        let al: Range<i32> = Range::at_least(10);
        assert!(al.contains(10));
        assert!(!al.contains(9));
        assert!(al.contains(i32::MAX));

        let gt: Range<i32> = Range::greater_than(10);
        assert!(!gt.contains(10));
        assert!(gt.contains(11));

        let lt: Range<i32> = Range::less_than(5);
        assert!(lt.contains(4));
        assert!(!lt.contains(5));

        let am: Range<i32> = Range::at_most(5);
        assert!(am.contains(5));
        assert!(!am.contains(6));
    }

    #[test]
    fn bound_types_and_endpoints() {
        let r: Range<i32> = Range::closed_open(10, 20);
        assert_eq!(r.lower_bound_type(), Some(BoundType::Closed));
        assert_eq!(r.upper_bound_type(), Some(BoundType::Open));
        assert_eq!(r.lower_endpoint(), Some(10));
        assert_eq!(r.upper_endpoint(), Some(20));
        assert!(r.has_lower_bound());
        assert!(r.has_upper_bound());

        let all: Range<i32> = Range::all();
        assert_eq!(all.lower_bound_type(), None);
        assert_eq!(all.upper_bound_type(), None);
        assert_eq!(all.lower_endpoint(), None);
        assert_eq!(all.upper_endpoint(), None);
        assert!(!all.has_lower_bound());
        assert!(!all.has_upper_bound());
    }

    #[test]
    fn empty_cut_semantics() {
        // open(1,2) is NOT cut-empty (no DiscreteDomain), even though no i32
        // is contained.
        let o: Range<i32> = Range::open(1, 2);
        assert!(!o.is_empty());
        assert!(!o.contains(1));
        assert!(!o.contains(2));

        // closedOpen(v,v) and openClosed(v,v) are both empty but DISTINCT.
        let co: Range<i32> = Range::closed_open(5, 5);
        let oc: Range<i32> = Range::open_closed(5, 5);
        assert!(co.is_empty());
        assert!(oc.is_empty());
        assert_ne!(co, oc);
        assert!(!co.contains(5));
        assert!(!oc.contains(5));
        // bound types are preserved per the cuts.
        assert_eq!(co.lower_bound_type(), Some(BoundType::Closed));
        assert_eq!(co.upper_bound_type(), Some(BoundType::Open));
        assert_eq!(oc.lower_bound_type(), Some(BoundType::Open));
        assert_eq!(oc.upper_bound_type(), Some(BoundType::Closed));
        // empties at different positions are unequal.
        assert_ne!(co, Range::<i32>::closed_open(6, 6));
    }

    #[test]
    fn singleton_is_not_empty() {
        let s: Range<i32> = Range::singleton(5);
        assert!(!s.is_empty());
        assert!(s.contains(5));
        assert!(!s.contains(4));
        assert!(!s.contains(6));
    }

    #[test]
    fn encloses_cut_defined() {
        let big: Range<i32> = Range::closed(10, 30);
        assert!(big.encloses(&Range::closed(15, 25)));
        assert!(!big.encloses(&Range::closed(5, 25)));
        // [10,30] encloses empty@20.
        assert!(big.encloses(&Range::closed_open(20, 20)));
        // [1,5) encloses empty@5 (cut-defined; 5 NOT contained).
        let half: Range<i32> = Range::closed_open(1, 5);
        assert!(half.encloses(&Range::closed_open(5, 5)));
        assert!(!half.contains(5));
    }

    #[test]
    fn connected_overlap() {
        let a: Range<i32> = Range::closed(10, 20);
        let b: Range<i32> = Range::closed(15, 25);
        assert!(a.is_connected(&b));
        let i = a.intersection(&b).expect("connected -> present");
        assert!(!i.is_empty());
        assert_eq!(i, Range::closed(15, 20));
    }

    #[test]
    fn connected_abut_present_empty() {
        // [10,20) & [20,30) -> connected, present cut-empty at (Below20,Below20).
        let a: Range<i32> = Range::closed_open(10, 20);
        let b: Range<i32> = Range::closed_open(20, 30);
        assert!(a.is_connected(&b));
        let i = a.intersection(&b).expect("abut -> present");
        assert!(i.is_empty());
        assert_eq!(i, Range::closed_open(20, 20));
        assert_eq!(i.lower_endpoint(), Some(20));
        assert_eq!(i.upper_endpoint(), Some(20));
        assert_eq!(i.lower_bound_type(), Some(BoundType::Closed));
        assert_eq!(i.upper_bound_type(), Some(BoundType::Open));
    }

    #[test]
    fn abut_open_closed_present_empty() {
        // [10,20] & (20,30) -> connected, present cut-empty at (Above20,Above20).
        let a: Range<i32> = Range::closed(10, 20);
        let b: Range<i32> = Range::open(20, 30);
        assert!(a.is_connected(&b));
        let i = a.intersection(&b).expect("abut -> present");
        assert!(i.is_empty());
        assert_eq!(i, Range::open_closed(20, 20));
        assert_eq!(i.lower_bound_type(), Some(BoundType::Open));
        assert_eq!(i.upper_bound_type(), Some(BoundType::Closed));
    }

    #[test]
    fn disjoint_is_none() {
        let a: Range<i32> = Range::closed_open(10, 15);
        let b: Range<i32> = Range::closed_open(20, 25);
        assert!(!a.is_connected(&b));
        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn connected_unbounded_abut() {
        // lessThan(5) & atLeast(5) -> connected, present empty (Below5,Below5).
        let a: Range<i32> = Range::less_than(5);
        let b: Range<i32> = Range::at_least(5);
        assert!(a.is_connected(&b));
        let i = a.intersection(&b).expect("abut -> present");
        assert!(i.is_empty());
        assert_eq!(i, Range::closed_open(5, 5));
        assert_eq!(i.lower_bound_type(), Some(BoundType::Closed));
        assert_eq!(i.upper_bound_type(), Some(BoundType::Open));
    }

    #[test]
    fn disjoint_unbounded_is_none() {
        // lessThan(5) & greaterThan(5) -> DISCONNECTED (5 is the gap).
        let a: Range<i32> = Range::less_than(5);
        let b: Range<i32> = Range::greater_than(5);
        assert!(!a.is_connected(&b));
        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn span_basic() {
        let a: Range<i32> = Range::closed(10, 15);
        let b: Range<i32> = Range::closed(20, 25);
        let s = a.span(&b);
        assert_eq!(s, Range::closed(10, 25));
        assert_eq!(s.lower_endpoint(), Some(10));
        assert_eq!(s.upper_endpoint(), Some(25));
        assert_eq!(s.lower_bound_type(), Some(BoundType::Closed));
        assert_eq!(s.upper_bound_type(), Some(BoundType::Closed));
    }

    #[test]
    fn span_unbounded() {
        let a: Range<i32> = Range::at_least(10);
        let b: Range<i32> = Range::closed(0, 5);
        let s = a.span(&b);
        assert_eq!(s, Range::at_least(0));
        assert_eq!(s.lower_endpoint(), Some(0));
        assert_eq!(s.upper_endpoint(), None);
        assert_eq!(s.lower_bound_type(), Some(BoundType::Closed));
        assert_eq!(s.upper_bound_type(), None);
    }

    #[test]
    #[should_panic(expected = "lower cut must not exceed upper cut")]
    fn closed_bad_order_panics() {
        let _: Range<i32> = Range::closed(5, 1);
    }

    #[test]
    #[should_panic(expected = "lower cut must not exceed upper cut")]
    fn open_equal_panics() {
        // open(3,3) = (Above(3), Below(3)) is lower > upper.
        let _: Range<i32> = Range::open(3, 3);
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", Range::<i32>::closed(1, 5)), "[1, 5]");
        assert_eq!(format!("{}", Range::<i32>::open(1, 5)), "(1, 5)");
        assert_eq!(format!("{}", Range::<i32>::closed_open(1, 5)), "[1, 5)");
        assert_eq!(format!("{}", Range::<i32>::at_least(1)), "[1, +\u{221e})");
        assert_eq!(format!("{}", Range::<i32>::less_than(5)), "(-\u{221e}, 5)");
        assert_eq!(format!("{}", Range::<i32>::all()), "(-\u{221e}, +\u{221e})");
    }

    // ---- std range-syntax interop (T4) ----

    #[test]
    fn from_std_range_shapes_map_to_guava_factories() {
        assert_eq!(Range::<i32>::from(2..5), Range::closed_open(2, 5));
        assert_eq!(Range::<i32>::from(2..=5), Range::closed(2, 5));
        assert_eq!(Range::<i32>::from(2..), Range::at_least(2));
        assert_eq!(Range::<i32>::from(..5), Range::less_than(5));
        assert_eq!(Range::<i32>::from(..=5), Range::at_most(5));
        assert_eq!(Range::<i32>::from(..), Range::all());
        // `.into()` in argument position picks the right impl.
        let r: Range<i32> = (2..5).into();
        assert_eq!(r, Range::closed_open(2, 5));
    }

    #[test]
    fn from_bounds_covers_excluded_lower_and_tuples() {
        use std::ops::Bound::{Excluded, Included, Unbounded};
        // Owned tuple bounds (avoid the &-bound E0283 ambiguity).
        assert_eq!(
            Range::<i32>::from_bounds((Excluded(2), Included(5))),
            Range::open_closed(2, 5)
        );
        assert_eq!(
            Range::<i32>::from_bounds((Excluded(2), Excluded(5))),
            Range::open(2, 5)
        );
        assert_eq!(
            Range::<i32>::from_bounds((Included(2), Unbounded)),
            Range::at_least(2)
        );
        assert_eq!(
            Range::<i32>::from_bounds((Excluded(2), Unbounded)),
            Range::greater_than(2)
        );
        assert_eq!(
            Range::<i32>::from_bounds::<std::ops::RangeFull>(..),
            Range::all()
        );
    }

    #[test]
    fn from_std_range_edge_and_extremes() {
        // `a..a` half-open is the valid empty range (does not panic).
        let empty: Range<i32> = (5..5).into();
        assert_eq!(empty, Range::closed_open(5, 5));
        assert!(!empty.contains(5));
        // `a..=a` is the singleton.
        assert_eq!(Range::<i32>::from(5..=5), Range::singleton(5));
        // Signed extremes: no `±1` arithmetic, so no overflow.
        assert!(Range::<i32>::from(i32::MIN..i32::MAX).contains(0));
        assert!(Range::<i32>::from(i32::MIN..=i32::MAX).contains(i32::MAX));
    }

    #[test]
    #[should_panic(expected = "lower cut must not exceed upper cut")]
    #[allow(clippy::reversed_empty_ranges)] // intentionally reversed — must trap
    fn from_reversed_half_open_panics() {
        let _: Range<i32> = (5..2).into();
    }

    #[test]
    #[should_panic(expected = "lower cut must not exceed upper cut")]
    #[allow(clippy::reversed_empty_ranges)] // intentionally reversed — must trap
    fn from_reversed_inclusive_panics() {
        let _: Range<i32> = Range::from(5..=2);
    }
}
