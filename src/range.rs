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
    fn cmp(&self, other: &Cut<T>) -> Ordering {
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

    // ---- factories (Guava-parity names) -----------------------------------

    /// `(a, b)` — both endpoints open. Panics if `a >= b` (incl. `open(v, v)`,
    /// which is empty-but-invalid-as-open).
    pub fn open(a: T, b: T) -> Self {
        Self::from_cuts(Cut::Above(a), Cut::Below(b))
    }

    /// `[a, b]` — both endpoints closed. Panics if `a > b`.
    pub fn closed(a: T, b: T) -> Self {
        Self::from_cuts(Cut::Below(a), Cut::Above(b))
    }

    /// `(a, b]`. Panics if `a > b`.
    pub fn open_closed(a: T, b: T) -> Self {
        Self::from_cuts(Cut::Above(a), Cut::Above(b))
    }

    /// `[a, b)`. Panics if `a > b`. `closed_open(v, v)` is the valid empty
    /// range `(Below(v), Below(v))`.
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
}
