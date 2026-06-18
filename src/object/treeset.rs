// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Sorted set backed by a [`TreeMap`] with pluggable [`Comparator`].

use super::strategy::Comparator;
use super::treemap::{TreeMap, TreeMapSink};
use crate::bulk::{BulkError, DuplicatePolicy};
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
    pub fn create(self) -> TreeSet<T> {
        TreeSet {
            tree: self.inner.create(),
        }
    }

    /// Like [`create`](TreeSetSink::create) but returns the poison error
    /// instead of panicking in debug.
    pub fn try_create(self) -> Result<TreeSet<T>, BulkError> {
        Ok(TreeSet {
            tree: self.inner.try_create()?,
        })
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

    #[test]
    fn pump_from_sorted_equals_incremental() {
        let data: Vec<i32> = (0..200).collect();
        let bulk = TreeSet::from_sorted(
            natural_comparator::<i32>(),
            data.clone(),
            DuplicatePolicy::Error,
        )
        .unwrap();
        let mut inc = TreeSet::new(natural_comparator::<i32>());
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
}
