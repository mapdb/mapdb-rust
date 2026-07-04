// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Sorted set backed by a [`TreeMap`] with pluggable [`Comparator`].

use super::strategy::{Comparator, Compare, Natural};
use super::treemap::{TreeMap, TreeMapSink};
use crate::bulk::{BulkError, DuplicatePolicy};
use crate::range::Range;
use std::fmt;

/// A sorted set backed by a [`TreeMap`] with a pluggable comparator `C` (the
/// [`Compare`] type parameter). `C` defaults to [`Natural`], so `TreeSet<T>`
/// orders by the element's natural [`Ord`] (built with the no-arg
/// [`new`](TreeSet::new)); use [`with_comparator`](TreeSet::with_comparator) or
/// the [`DynTreeSet`] alias for a runtime comparator.
pub struct TreeSet<T, C = Natural> {
    tree: TreeMap<T, (), C>,
}

/// A [`TreeSet`] whose order is a runtime [`Comparator`].
pub type DynTreeSet<T> = TreeSet<T, Comparator<T>>;

impl<T: fmt::Debug, C: Compare<T>> fmt::Debug for TreeSet<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Ord> TreeSet<T, Natural> {
    /// Creates an empty `TreeSet` ordered by the element's natural [`Ord`]
    /// (zero-sized comparator; comparisons inline). For a runtime comparator
    /// use [`with_comparator`](TreeSet::with_comparator).
    pub fn new() -> Self {
        Self::natural()
    }

    /// Creates an empty `TreeSet` ordered by the element's natural [`Ord`]
    /// (zero-sized comparator; comparisons inline). Alias of the no-arg
    /// [`new`](TreeSet::new).
    pub fn natural() -> Self {
        TreeSet {
            tree: TreeMap::natural(),
        }
    }
}

impl<T, C: Compare<T>> TreeSet<T, C> {
    /// Creates an empty `TreeSet` using the [`Compare`] value `cmp`.
    pub fn with_comparator(cmp: C) -> Self {
        TreeSet {
            tree: TreeMap::with_comparator(cmp),
        }
    }

    /// Inserts a value into the set. Returns `true` if the value was newly
    /// inserted, `false` if it was already present.
    pub fn insert(&mut self, value: T) -> bool {
        self.tree.insert(value, ()).is_none()
    }

    /// Removes a value from the set. Returns `true` if the value was found
    /// and removed.
    pub fn remove(&mut self, value: &T) -> bool {
        self.tree.remove(value).is_some()
    }

    /// Returns `true` if the set contains the given value.
    pub fn contains(&self, value: &T) -> bool {
        self.tree.contains_key(value)
    }

    /// Retains only the elements for which `keep(&elem)` returns `true`,
    /// visiting them in ascending comparator order; rejected elements are
    /// dropped. If `keep` panics, the set is left holding exactly the elements
    /// visited before the panic (a valid set with a correct
    /// [`len`](Self::len)); every not-yet-visited element is dropped.
    pub fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.tree.retain(|t, ()| keep(t));
    }

    /// Returns the minimum element, or `None` if empty.
    pub fn min(&self) -> Option<&T> {
        self.tree.min().map(|(k, _)| k)
    }

    /// Returns the maximum element, or `None` if empty.
    pub fn max(&self) -> Option<&T> {
        self.tree.max().map(|(k, _)| k)
    }

    // ── Point navigation (NavigableSet surface) ─────────────────────

    /// Greatest element `<= x`, or `None`.
    pub fn floor(&self, x: &T) -> Option<&T> {
        self.tree.floor_key(x)
    }

    /// Least element `>= x`, or `None`.
    pub fn ceiling(&self, x: &T) -> Option<&T> {
        self.tree.ceiling_key(x)
    }

    /// Greatest element `< x` (strict), or `None`.
    pub fn lower(&self, x: &T) -> Option<&T> {
        self.tree.lower_key(x)
    }

    /// Least element `> x` (strict), or `None`.
    pub fn higher(&self, x: &T) -> Option<&T> {
        self.tree.higher_key(x)
    }

    /// Minimum element, or `None`. Alias for [`min`](Self::min).
    pub fn first(&self) -> Option<&T> {
        self.min()
    }

    /// Maximum element, or `None`. Alias for [`max`](Self::max).
    pub fn last(&self) -> Option<&T> {
        self.max()
    }

    // ── Order statistics (rank / select) ────────────────────────────

    /// Returns the number of elements strictly less than `x` under the set's
    /// comparator — the 0-based lower-bound index `x` occupies (if present)
    /// or would occupy (if absent). Result is in `0..=len()`. Pure query.
    pub fn rank(&self, x: &T) -> usize {
        self.tree.rank(x)
    }

    /// Returns the `i`-th smallest element (0-based), or `None` if
    /// `i >= len()` (no trap on an empty set). Round-trips with
    /// [`rank`](Self::rank): `select(rank(x)) == Some(x)` for present `x`,
    /// and `rank(select(i)) == i` for every `0 <= i < len()`.
    pub fn select(&self, i: usize) -> Option<&T> {
        self.tree.select_key(i)
    }

    /// Returns the number of elements.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// Returns `true` if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Removes all elements.
    pub fn clear(&mut self) {
        self.tree.clear();
    }

    /// Returns an iterator over elements in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.tree.keys()
    }

    /// A lazy, double-ended, exact-size iterator over the elements in `range`,
    /// ascending (`.rev()` for descending). Bounds are compared through the
    /// set's **own comparator**, so selection can never disagree with the set
    /// order. See [`TreeMap::range`] for the inverted-bounds policy.
    pub fn range<R: std::ops::RangeBounds<T>>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator + std::iter::FusedIterator + '_
    {
        self.tree.range(range).map(|(k, _)| k)
    }

    /// Collects all elements into a `Vec` in sorted order.
    pub fn to_vec(&self) -> Vec<&T> {
        self.iter().collect()
    }

    /// Calls `f` for each element in sorted order.
    pub fn for_each(&self, mut f: impl FnMut(&T)) {
        self.tree.for_each(|k, _| f(k));
    }

    /// Returns elements matching the predicate as a `Vec` of references.
    ///
    /// Named `select_where` (not `select`) so the bare `select` name is
    /// reserved for the order-statistic [`select`](Self::select) (i-th
    /// smallest by 0-based rank), per `spec/features/rank-select.md`.
    pub fn select_where(&self, predicate: impl Fn(&T) -> bool) -> Vec<&T> {
        self.iter().filter(|v| predicate(v)).collect()
    }

    /// Returns elements not matching the predicate as a `Vec` of references.
    pub fn reject(&self, predicate: impl Fn(&T) -> bool) -> Vec<&T> {
        self.iter().filter(|v| !predicate(v)).collect()
    }
}

impl<T> TreeSet<T, Comparator<T>> {
    /// Builds a fresh `TreeSet` from already-sorted input in a single O(n)
    /// pass. Input must be strictly ascending under `cmp`; see
    /// [`TreeMap::from_sorted`] for the order/duplicate contract.
    pub fn from_sorted<I: IntoIterator<Item = T>>(
        cmp: Comparator<T>,
        iter: I,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError> {
        let tree = TreeMap::from_sorted(cmp, iter.into_iter().map(|t| (t, ())), dup)?;
        Ok(TreeSet { tree })
    }
}

/// Streaming bulk builder for [`TreeSet`], wrapping [`TreeMapSink`]. Accepts
/// strictly-ascending elements via [`put`](TreeSetSink::put) /
/// [`put_all`](TreeSetSink::put_all); [`create`](TreeSetSink::create) finishes
/// the build. Poisoned after an error; `create` is once-only.
pub struct TreeSetSink<T> {
    inner: TreeMapSink<T, ()>,
}

impl<T> TreeSetSink<T> {
    /// Starts a fresh sorted bulk build under `cmp` with duplicate policy `dup`.
    pub fn new(cmp: Comparator<T>, dup: DuplicatePolicy) -> Self {
        TreeSetSink {
            inner: TreeMapSink::new(cmp, dup),
        }
    }

    /// Appends one prepared element (must be strictly greater than the last).
    pub fn put(&mut self, value: T) -> Result<(), BulkError> {
        self.inner.put(value, ())
    }

    /// Convenience: `put` every element of `iter`.
    pub fn put_all<I: IntoIterator<Item = T>>(&mut self, iter: I) -> Result<(), BulkError> {
        for v in iter {
            self.put(v)?;
        }
        Ok(())
    }

    /// Finishes the build, returning the constructed `TreeSet`.
    ///
    /// # Panics
    ///
    /// Panics if the sink is **poisoned** (a prior `put` error was swallowed) —
    /// see [`TreeMapSink::create`]. Use [`try_create`](TreeSetSink::try_create)
    /// for the fallible form.
    pub fn create(self) -> TreeSet<T, Comparator<T>> {
        TreeSet {
            tree: self.inner.create(),
        }
    }

    /// Like [`create`](TreeSetSink::create) but returns the poison error
    /// instead of panicking.
    pub fn try_create(self) -> Result<TreeSet<T, Comparator<T>>, BulkError> {
        Ok(TreeSet {
            tree: self.inner.try_create()?,
        })
    }
}

impl<T: Clone, C: Compare<T>> TreeSet<T, C> {
    // ── Poll (positional removal) ───────────────────────────────────

    /// Removes and returns the minimum element, or `None` if empty. Does
    /// not trap on an empty set.
    pub fn poll_first(&mut self) -> Option<T> {
        self.tree.poll_first_entry().map(|(k, _)| k)
    }

    /// Removes and returns the maximum element, or `None` if empty.
    pub fn poll_last(&mut self) -> Option<T> {
        self.tree.poll_last_entry().map(|(k, _)| k)
    }
}

impl<T: Ord + Copy> TreeSet<T> {
    // ── Natural-order range snapshot & removal (consume `Range<T>`) ──
    //
    // Range membership is EXACTLY `range.contains(element)`, selected by the
    // element's natural `Ord`. Use [`TreeSet::range`] for a lazy,
    // comparator-correct range query.

    /// A **new independent** set of the elements ∈ `range` (materialized
    /// snapshot; mutating it never affects the original and vice versa). Both
    /// the source and the snapshot order by the element's natural [`Ord`]; use
    /// [`range`](TreeSet::range) for comparator-correct slices.
    pub fn sub_set(&self, range: Range<T>) -> TreeSet<T> {
        let mut out = TreeSet::new();
        for x in self.iter() {
            if range.contains(*x) {
                out.insert(*x);
            }
        }
        out
    }

    /// Removes every element ∈ `range`; returns the count removed. A range
    /// that matches nothing is a no-op returning `0`.
    pub fn remove_range(&mut self, range: Range<T>) -> usize {
        self.tree.remove_range(range)
    }
}

/// Sorted-order iterator over a `TreeSet`'s elements.
pub struct TreeSetIter<'a, T> {
    inner: super::treemap::TreeMapIter<'a, T, ()>,
}

impl<'a, T> Iterator for TreeSetIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.inner.next().map(|(k, _)| k)
    }
}

/// Borrowing iteration in sorted order: `for x in &set`.
impl<'a, T, C: Compare<T>> IntoIterator for &'a TreeSet<T, C> {
    type Item = &'a T;
    type IntoIter = TreeSetIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        TreeSetIter {
            inner: self.tree.iter(),
        }
    }
}

/// Owning iterator over a `TreeSet`'s elements in ascending order.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct TreeSetIntoIter<T> {
    inner: super::treemap::TreeMapIntoIter<T, ()>,
}

impl<T> Iterator for TreeSetIntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.next().map(|(k, _)| k)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<T> DoubleEndedIterator for TreeSetIntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.next_back().map(|(k, _)| k)
    }
}
impl<T> ExactSizeIterator for TreeSetIntoIter<T> {}
impl<T> std::iter::FusedIterator for TreeSetIntoIter<T> {}

impl<T, C> TreeSet<T, C> {
    /// Removes all elements and returns them as an iterator in ascending
    /// comparator order, keeping the emptied set (and its comparator) for reuse
    /// — the reuse-friendly counterpart to [`into_iter`](Self::into_iter). The
    /// set is emptied immediately, before the first item is yielded, so it stays
    /// valid and empty even if the iterator is dropped early or a consuming loop
    /// panics. Borrows the set mutably for the iterator's lifetime. Needs no
    /// comparator (teardown does not compare), matching the bound-free
    /// [`IntoIterator`].
    pub fn drain(&mut self) -> TreeSetDrain<'_, T> {
        TreeSetDrain {
            inner: self.tree.drain(),
        }
    }
}

/// Draining iterator over a `TreeSet`'s elements in ascending order, from
/// [`TreeSet::drain`]. The set is emptied when `drain` is called (before the
/// first item is yielded). Borrows the set mutably for its lifetime.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct TreeSetDrain<'a, T> {
    inner: super::treemap::TreeMapDrain<'a, T, ()>,
}

impl<T> Iterator for TreeSetDrain<'_, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.next().map(|(k, _)| k)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<T> DoubleEndedIterator for TreeSetDrain<'_, T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.next_back().map(|(k, _)| k)
    }
}
impl<T> ExactSizeIterator for TreeSetDrain<'_, T> {}
impl<T> std::iter::FusedIterator for TreeSetDrain<'_, T> {}

/// Owned iteration in sorted order: `for x in set`, yielding `T` by value.
impl<T, C> IntoIterator for TreeSet<T, C> {
    type Item = T;
    type IntoIter = TreeSetIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        TreeSetIntoIter {
            inner: self.tree.into_iter(),
        }
    }
}

impl<T: Ord> Default for TreeSet<T, Natural> {
    /// An empty set ordered by natural [`Ord`].
    fn default() -> Self {
        Self::natural()
    }
}

impl<T: Ord> FromIterator<T> for TreeSet<T, Natural> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = TreeSet::natural();
        for v in iter {
            set.insert(v);
        }
        set
    }
}

impl<T: Ord> Extend<T> for TreeSet<T, Natural> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for v in iter {
            self.insert(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::strategy::*;

    #[test]
    fn range_bounds_double_ended() {
        let s: TreeSet<i32, Natural> = (0..10).collect();
        assert_eq!(s.range(3..7).copied().collect::<Vec<_>>(), vec![3, 4, 5, 6]);
        assert_eq!(
            s.range(3..=7).copied().collect::<Vec<_>>(),
            vec![3, 4, 5, 6, 7]
        );
        assert_eq!(s.range(..).len(), 10);
        assert_eq!(
            s.range(2..8).rev().copied().collect::<Vec<_>>(),
            vec![7, 6, 5, 4, 3, 2]
        );
    }

    #[test]
    fn natural_and_reverse_type_params() {
        use crate::object::strategy::Reverse;
        // Natural: collect, Default.
        let s: TreeSet<i32, Natural> = [3, 1, 2, 1].into_iter().collect();
        assert_eq!(s.len(), 3);
        assert_eq!(s.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert!(TreeSet::<i32, Natural>::default().is_empty());

        // Reverse type parameter descends.
        let mut r: TreeSet<i32, Reverse> = TreeSet::with_comparator(Reverse(Natural));
        for x in [1, 3, 2] {
            r.insert(x);
        }
        assert_eq!(r.iter().copied().collect::<Vec<_>>(), vec![3, 2, 1]);
    }

    #[test]
    fn test_sub_set_natural_snapshot_and_independence() {
        // sub_set is a natural-order materialized snapshot (comparator-correct
        // slices are `range`'s job). Membership ∈ range, ascending natural order.
        let mut s = TreeSet::new();
        for k in [10, 20, 30, 40, 50] {
            s.insert(k);
        }
        assert_eq!(s.to_vec(), vec![&10, &20, &30, &40, &50]);
        let sub = s.sub_set(Range::closed_open(20, 50)); // {20,30,40}
        assert_eq!(sub.to_vec(), vec![&20, &30, &40]);
        // Independence: mutating the snapshot does not touch the original.
        let mut sub2 = sub;
        sub2.remove(&30);
        assert!(s.contains(&30));
    }

    #[test]
    fn test_basic() {
        let mut s = TreeSet::with_comparator(natural_comparator::<i32>());
        assert!(s.insert(3));
        assert!(s.insert(1));
        assert!(s.insert(2));
        assert!(!s.insert(1)); // duplicate

        assert_eq!(s.len(), 3);
        let items: Vec<&i32> = s.to_vec();
        assert_eq!(items, vec![&1, &2, &3]);
    }

    #[test]
    fn test_min_max() {
        let mut s = TreeSet::with_comparator(natural_comparator::<String>());
        s.insert("banana".to_string());
        s.insert("apple".to_string());
        s.insert("cherry".to_string());

        assert_eq!(s.min(), Some(&"apple".to_string()));
        assert_eq!(s.max(), Some(&"cherry".to_string()));
    }

    #[test]
    fn test_remove() {
        let mut s = TreeSet::with_comparator(natural_comparator::<i32>());
        for i in 0..50 {
            s.insert(i);
        }
        for i in (0..50).step_by(2) {
            assert!(s.remove(&i));
        }
        assert_eq!(s.len(), 25);
        assert!(!s.contains(&0));
        assert!(!s.contains(&2));
        assert!(s.contains(&1));
        assert!(s.contains(&3));
    }

    #[test]
    fn test_select_reject() {
        let mut s = TreeSet::with_comparator(natural_comparator::<i32>());
        for i in 1..=5 {
            s.insert(i);
        }
        let evens = s.select_where(|v| *v % 2 == 0);
        assert_eq!(evens, vec![&2, &4]);

        let odds = s.reject(|v| *v % 2 == 0);
        assert_eq!(odds, vec![&1, &3, &5]);
    }

    #[test]
    fn test_clear() {
        let mut s = TreeSet::with_comparator(natural_comparator::<i32>());
        s.insert(1);
        s.insert(2);
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_stress() {
        let mut s = TreeSet::with_comparator(natural_comparator::<i32>());
        for i in (0..1000).rev() {
            s.insert(i);
        }
        assert_eq!(s.len(), 1000);

        // Verify sorted order.
        let mut prev = -1;
        for v in s.iter() {
            assert!(*v > prev, "not sorted: {} after {}", v, prev);
            prev = *v;
        }

        // Remove all.
        for i in 0..1000 {
            assert!(s.remove(&i));
        }
        assert!(s.is_empty());
    }

    #[derive(Debug, Clone)]
    struct Person {
        name: String,
        age: i32,
    }

    #[test]
    fn test_then_comparing() {
        let by_age = comparator_by_field(|p: &Person| p.age);
        let by_name = comparator_by_field(|p: &Person| p.name.clone());
        let cmp = then_comparing(by_age, by_name);

        let mut s = TreeSet::with_comparator(cmp);
        s.insert(Person {
            name: "Charlie".into(),
            age: 30,
        });
        s.insert(Person {
            name: "Alice".into(),
            age: 30,
        });
        s.insert(Person {
            name: "Bob".into(),
            age: 25,
        });

        let names: Vec<&str> = s.iter().map(|p| p.name.as_str()).collect();
        // Bob(25) < Alice(30) < Charlie(30) — age first, then name
        assert_eq!(names, vec!["Bob", "Alice", "Charlie"]);
    }

    #[test]
    fn test_reverse_order() {
        let mut s = TreeSet::with_comparator(reverse_comparator::<i32>());
        s.insert(1);
        s.insert(3);
        s.insert(2);
        let items: Vec<&i32> = s.to_vec();
        assert_eq!(items, vec![&3, &2, &1]);
    }

    #[test]
    fn test_empty_min_max() {
        let s = TreeSet::with_comparator(natural_comparator::<i32>());
        assert_eq!(s.min(), None);
        assert_eq!(s.max(), None);
    }

    #[test]
    fn test_into_iter_borrowing_sorted() {
        let mut s = TreeSet::with_comparator(natural_comparator::<i32>());
        s.insert(3);
        s.insert(1);
        s.insert(2);
        let v: Vec<i32> = (&s).into_iter().copied().collect();
        assert_eq!(v, vec![1, 2, 3]);
    }

    // ── NavigableSet surface ────────────────────────────────────────

    use crate::range::Range;

    fn set_of(elems: &[i32]) -> TreeSet<i32> {
        let mut s = TreeSet::new();
        for &e in elems {
            s.insert(e);
        }
        s
    }

    #[test]
    fn test_floor_ceiling_lower_higher() {
        let s = set_of(&[10, 20, 30]);
        assert_eq!(s.floor(&25), Some(&20));
        assert_eq!(s.ceiling(&25), Some(&30));
        assert_eq!(s.floor(&10), Some(&10));
        assert_eq!(s.lower(&10), None);
        assert_eq!(s.higher(&30), None);
        assert_eq!(s.ceiling(&5), Some(&10));
        assert_eq!(s.first(), Some(&10));
        assert_eq!(s.last(), Some(&30));
    }

    #[test]
    fn test_nav_empty() {
        let s: TreeSet<i32> = set_of(&[]);
        assert_eq!(s.floor(&5), None);
        assert_eq!(s.ceiling(&5), None);
        assert_eq!(s.lower(&5), None);
        assert_eq!(s.higher(&5), None);
        assert_eq!(s.first(), None);
        assert_eq!(s.last(), None);
    }

    #[test]
    fn test_nav_signed_extremes() {
        let s = set_of(&[i32::MIN, -1, 0, 1, i32::MAX]);
        assert_eq!(s.floor(&i32::MIN), Some(&i32::MIN));
        assert_eq!(s.lower(&i32::MIN), None);
        assert_eq!(s.higher(&-1), Some(&0));
        assert_eq!(s.ceiling(&i32::MAX), Some(&i32::MAX));
        assert_eq!(s.higher(&i32::MAX), None);
        // Descending order via the lazy `range` iterator (`.rev()`).
        assert_eq!(
            s.range(..).rev().copied().collect::<Vec<_>>(),
            vec![i32::MAX, 1, 0, -1, i32::MIN]
        );
    }

    #[test]
    fn test_poll_first_last_then_empty() {
        let mut s = set_of(&[10, 20, 30]);
        assert_eq!(s.poll_first(), Some(10));
        assert_eq!(s.poll_last(), Some(30));
        assert_eq!(s.poll_first(), Some(20));
        assert_eq!(s.poll_first(), None);
        assert_eq!(s.poll_last(), None);
    }

    #[test]
    fn test_range_and_descending() {
        use std::ops::Bound;
        let s = set_of(&[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        assert_eq!(
            s.range(30..70).copied().collect::<Vec<_>>(),
            vec![30, 40, 50, 60]
        );
        assert_eq!(
            s.range(30..70).rev().copied().collect::<Vec<_>>(),
            vec![60, 50, 40, 30]
        );
        assert_eq!(
            s.range((Bound::Excluded(30), Bound::Included(70)))
                .copied()
                .collect::<Vec<_>>(),
            vec![40, 50, 60, 70]
        );
        assert_eq!(
            s.range(80..).copied().collect::<Vec<_>>(),
            vec![80, 90, 100]
        );
    }

    #[test]
    fn test_range_open_no_integer_is_empty() {
        use std::ops::Bound;
        let s = set_of(&[1, 2]);
        assert_eq!(
            s.range((Bound::Excluded(1), Bound::Excluded(2)))
                .copied()
                .collect::<Vec<i32>>(),
            Vec::<i32>::new()
        );
    }

    #[test]
    fn test_remove_range_count_and_noop() {
        let mut s = set_of(&[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        assert_eq!(s.remove_range(Range::closed_open(30, 70)), 4);
        assert_eq!(s.remove_range(Range::closed_open(30, 70)), 0);
        let v: Vec<i32> = s.iter().copied().collect();
        assert_eq!(v, vec![10, 20, 70, 80, 90, 100]);
    }

    // ── Order statistics (rank / select) ────────────────────────────

    #[test]
    fn test_rank_select_basic() {
        let s = set_of(&[10, 20, 30, 40, 50]);
        assert_eq!(s.rank(&10), 0);
        assert_eq!(s.rank(&30), 2);
        assert_eq!(s.rank(&50), 4);
        assert_eq!(s.rank(&5), 0); // before min
        assert_eq!(s.rank(&25), 2); // absent → lower bound
        assert_eq!(s.rank(&55), 5); // past max → size
        assert_eq!(s.select(0), Some(&10));
        assert_eq!(s.select(2), Some(&30));
        assert_eq!(s.select(4), Some(&50));
        assert_eq!(s.select(5), None); // == size
    }

    #[test]
    fn test_rank_select_empty_single() {
        let empty = set_of(&[]);
        assert_eq!(empty.rank(&5), 0);
        assert_eq!(empty.select(0), None);

        let s = set_of(&[7]);
        assert_eq!(s.rank(&6), 0);
        assert_eq!(s.rank(&7), 0);
        assert_eq!(s.rank(&8), 1);
        assert_eq!(s.select(0), Some(&7));
        assert_eq!(s.select(1), None);
    }

    #[test]
    fn test_rank_select_signed() {
        let s = set_of(&[i32::MIN, -1, 0, 1, i32::MAX]);
        assert_eq!(s.rank(&i32::MIN), 0);
        assert_eq!(s.rank(&0), 2);
        assert_eq!(s.rank(&i32::MAX), 4);
        assert_eq!(s.select(0), Some(&i32::MIN));
        assert_eq!(s.select(4), Some(&i32::MAX));
        assert_eq!(s.select(5), None);
    }

    #[test]
    fn test_rank_select_after_remove_round_trip() {
        let mut s = set_of(&[10, 20, 30, 40, 50]);
        assert!(s.remove(&30));
        assert_eq!(s.rank(&40), 2);
        assert_eq!(s.rank(&35), 2);
        assert_eq!(s.select(2), Some(&40));
        assert_eq!(s.select(4), None);
        // round-trip identity over the live set
        for x in s.iter().copied().collect::<Vec<_>>() {
            assert_eq!(s.select(s.rank(&x)), Some(&x));
        }
        for i in 0..s.len() {
            let x = s.select(i).copied().unwrap();
            assert_eq!(s.rank(&x), i);
        }
    }

    #[test]
    fn test_sub_set_independence() {
        let mut s = set_of(&[10, 20, 30, 40, 50]);
        let mut snap = s.sub_set(Range::closed(20, 40));
        let snap_v: Vec<i32> = snap.iter().copied().collect();
        assert_eq!(snap_v, vec![20, 30, 40]);
        snap.insert(99);
        snap.remove(&20);
        assert!(s.contains(&20));
        assert!(!s.contains(&99));
        s.remove(&30);
        assert!(snap.contains(&30));
    }

    #[test]
    fn pump_from_sorted_equals_incremental() {
        let data: Vec<i32> = (0..200).collect();
        let bulk = TreeSet::from_sorted(
            natural_comparator::<i32>(),
            data.clone(),
            DuplicatePolicy::Error,
        )
        .unwrap();
        let mut inc = TreeSet::with_comparator(natural_comparator::<i32>());
        for i in (0..200).rev() {
            inc.insert(i);
        }
        let b: Vec<i32> = bulk.iter().copied().collect();
        let i: Vec<i32> = inc.iter().copied().collect();
        assert_eq!(b, i);
    }

    #[test]
    fn pump_sink_and_errors() {
        let mut sink = TreeSetSink::new(natural_comparator::<i32>(), DuplicatePolicy::Error);
        sink.put(1).unwrap();
        sink.put(2).unwrap();
        let err = sink.put(2).unwrap_err(); // duplicate
        assert!(matches!(err, BulkError::Duplicate { index: 2 }));
        // poisoned now.
        assert!(sink.try_create().is_err());

        let s = TreeSet::from_sorted(
            natural_comparator::<i32>(),
            vec![1, 2, 2, 3],
            DuplicatePolicy::IgnoreDuplicates,
        )
        .unwrap();
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn pump_sink_post_build_mutation_stays_valid() {
        // Build via sink, then mutate and confirm it still behaves like a set.
        let mut sink = TreeSetSink::new(natural_comparator::<i32>(), DuplicatePolicy::Error);
        sink.put_all(0..100).unwrap();
        let mut s = sink.create();
        assert_eq!(s.len(), 100);
        // remove & re-add a spread of keys, then verify membership.
        for i in (0..100).step_by(3) {
            assert!(s.remove(&i));
        }
        for i in (0..100).step_by(3) {
            assert!(s.insert(i));
        }
        assert_eq!(s.len(), 100);
        let v: Vec<i32> = s.iter().copied().collect();
        assert_eq!(v, (0..100).collect::<Vec<_>>());
    }

    #[test]
    fn retain_keeps_matching_in_order() {
        let mut s: TreeSet<i32> = (0..20).collect();
        s.retain(|x| x % 3 == 0);
        let v: Vec<i32> = s.iter().copied().collect();
        assert_eq!(v, (0..20).filter(|x| x % 3 == 0).collect::<Vec<_>>());
        assert_eq!(s.len(), v.len());
    }

    #[test]
    fn drain_yields_sorted_and_empties() {
        let mut s: TreeSet<i32> = (0..20).rev().collect();
        let drained: Vec<i32> = s.drain().collect();
        assert_eq!(drained, (0..20).collect::<Vec<_>>());
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
        assert_eq!(s.iter().count(), 0);
        // The emptied set is reusable.
        s.insert(42);
        assert!(s.contains(&42));
    }

    #[test]
    fn drain_dropped_early_still_empties() {
        let mut s: TreeSet<i32> = (0..10).collect();
        {
            let mut d = s.drain();
            assert_eq!(d.next(), Some(0));
            assert_eq!(d.next_back(), Some(9));
        }
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
    }
}
