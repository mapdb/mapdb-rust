// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Sorted set backed by a [`TreeMap`] with pluggable [`Comparator`].

use super::strategy::Comparator;
use super::treemap::TreeMap;
use crate::range::Range;
use std::fmt;

/// A sorted set backed by a red-black tree with a pluggable [`Comparator`].
/// Elements are maintained in the order defined by the comparator.
pub struct TreeSet<T> {
    tree: TreeMap<T, ()>,
}

impl<T: fmt::Debug> fmt::Debug for TreeSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T> TreeSet<T> {
    /// Creates an empty `TreeSet` using the given comparator.
    pub fn new(cmp: Comparator<T>) -> Self {
        TreeSet {
            tree: TreeMap::new(cmp),
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

    /// Collects all elements into a `Vec` in sorted order.
    pub fn to_vec(&self) -> Vec<&T> {
        self.iter().collect()
    }

    /// Calls `f` for each element in sorted order.
    pub fn for_each(&self, mut f: impl FnMut(&T)) {
        self.tree.for_each(|k, _| f(k));
    }

    /// Returns elements matching the predicate as a `Vec` of references.
    pub fn select(&self, predicate: impl Fn(&T) -> bool) -> Vec<&T> {
        self.iter().filter(|v| predicate(v)).collect()
    }

    /// Returns elements not matching the predicate as a `Vec` of references.
    pub fn reject(&self, predicate: impl Fn(&T) -> bool) -> Vec<&T> {
        self.iter().filter(|v| !predicate(v)).collect()
    }
}

impl<T: Clone> TreeSet<T> {
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
    // ── Range slice & descending iteration (consume `Range<T>`) ──────
    //
    // Range membership is EXACTLY `range.contains(element)`.

    /// Elements in `range`, ascending. Snapshot at call time; read-only.
    pub fn range_elements(&self, range: Range<T>) -> Vec<T> {
        self.tree.range_keys(range)
    }

    /// Elements in `range`, descending.
    pub fn descending_range_elements(&self, range: Range<T>) -> Vec<T> {
        self.tree.descending_range_keys(range)
    }

    /// All elements, descending.
    pub fn descending(&self) -> Vec<T> {
        self.tree.descending_keys()
    }

    /// A **new independent** set of the elements ∈ `range` (materialized
    /// snapshot; mutating it never affects the original and vice versa).
    pub fn sub_set(&self, range: Range<T>) -> TreeSet<T>
    where
        T: 'static,
    {
        let mut out = TreeSet::new(crate::object::natural_comparator::<T>());
        for x in self.range_elements(range) {
            out.insert(x);
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
///
/// Owned iteration / `FromIterator` are intentionally not provided: a
/// `TreeSet` needs a [`Comparator`] that an iterator alone cannot supply.
impl<'a, T> IntoIterator for &'a TreeSet<T> {
    type Item = &'a T;
    type IntoIter = TreeSetIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        TreeSetIter {
            inner: self.tree.iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::strategy::*;

    #[test]
    fn test_basic() {
        let mut s = TreeSet::new(natural_comparator::<i32>());
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
        let mut s = TreeSet::new(natural_comparator::<String>());
        s.insert("banana".to_string());
        s.insert("apple".to_string());
        s.insert("cherry".to_string());

        assert_eq!(s.min(), Some(&"apple".to_string()));
        assert_eq!(s.max(), Some(&"cherry".to_string()));
    }

    #[test]
    fn test_remove() {
        let mut s = TreeSet::new(natural_comparator::<i32>());
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
        let mut s = TreeSet::new(natural_comparator::<i32>());
        for i in 1..=5 {
            s.insert(i);
        }
        let evens = s.select(|v| *v % 2 == 0);
        assert_eq!(evens, vec![&2, &4]);

        let odds = s.reject(|v| *v % 2 == 0);
        assert_eq!(odds, vec![&1, &3, &5]);
    }

    #[test]
    fn test_clear() {
        let mut s = TreeSet::new(natural_comparator::<i32>());
        s.insert(1);
        s.insert(2);
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_stress() {
        let mut s = TreeSet::new(natural_comparator::<i32>());
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

        let mut s = TreeSet::new(cmp);
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
        let mut s = TreeSet::new(reverse_comparator::<i32>());
        s.insert(1);
        s.insert(3);
        s.insert(2);
        let items: Vec<&i32> = s.to_vec();
        assert_eq!(items, vec![&3, &2, &1]);
    }

    #[test]
    fn test_empty_min_max() {
        let s = TreeSet::new(natural_comparator::<i32>());
        assert_eq!(s.min(), None);
        assert_eq!(s.max(), None);
    }

    #[test]
    fn test_into_iter_borrowing_sorted() {
        let mut s = TreeSet::new(natural_comparator::<i32>());
        s.insert(3);
        s.insert(1);
        s.insert(2);
        let v: Vec<i32> = (&s).into_iter().copied().collect();
        assert_eq!(v, vec![1, 2, 3]);
    }

    // ── NavigableSet surface ────────────────────────────────────────

    use crate::range::Range;

    fn set_of(elems: &[i32]) -> TreeSet<i32> {
        let mut s = TreeSet::new(natural_comparator::<i32>());
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
        assert_eq!(s.descending(), vec![i32::MAX, 1, 0, -1, i32::MIN]);
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
        let s = set_of(&[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        assert_eq!(
            s.range_elements(Range::closed_open(30, 70)),
            vec![30, 40, 50, 60]
        );
        assert_eq!(
            s.descending_range_elements(Range::closed_open(30, 70)),
            vec![60, 50, 40, 30]
        );
        assert_eq!(
            s.range_elements(Range::open_closed(30, 70)),
            vec![40, 50, 60, 70]
        );
        assert_eq!(s.range_elements(Range::at_least(80)), vec![80, 90, 100]);
    }

    #[test]
    fn test_range_open_no_integer_is_empty() {
        let s = set_of(&[1, 2]);
        assert_eq!(s.range_elements(Range::open(1, 2)), Vec::<i32>::new());
    }

    #[test]
    fn test_remove_range_count_and_noop() {
        let mut s = set_of(&[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        assert_eq!(s.remove_range(Range::closed_open(30, 70)), 4);
        assert_eq!(s.remove_range(Range::closed_open(30, 70)), 0);
        let v: Vec<i32> = s.iter().copied().collect();
        assert_eq!(v, vec![10, 20, 70, 80, 90, 100]);
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
}
