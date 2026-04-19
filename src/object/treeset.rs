// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Sorted set backed by a [`TreeMap`] with pluggable [`Comparator`].

use super::strategy::Comparator;
use super::treemap::TreeMap;
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

    /// Adds a value to the set. Returns `true` if the value was newly added,
    /// `false` if it was already present.
    pub fn add(&mut self, value: T) -> bool {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::strategy::*;

    #[test]
    fn test_basic() {
        let mut s = TreeSet::new(natural_comparator::<i32>());
        assert!(s.add(3));
        assert!(s.add(1));
        assert!(s.add(2));
        assert!(!s.add(1)); // duplicate

        assert_eq!(s.len(), 3);
        let items: Vec<&i32> = s.to_vec();
        assert_eq!(items, vec![&1, &2, &3]);
    }

    #[test]
    fn test_min_max() {
        let mut s = TreeSet::new(natural_comparator::<String>());
        s.add("banana".to_string());
        s.add("apple".to_string());
        s.add("cherry".to_string());

        assert_eq!(s.min(), Some(&"apple".to_string()));
        assert_eq!(s.max(), Some(&"cherry".to_string()));
    }

    #[test]
    fn test_remove() {
        let mut s = TreeSet::new(natural_comparator::<i32>());
        for i in 0..50 {
            s.add(i);
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
            s.add(i);
        }
        let evens = s.select(|v| *v % 2 == 0);
        assert_eq!(evens, vec![&2, &4]);

        let odds = s.reject(|v| *v % 2 == 0);
        assert_eq!(odds, vec![&1, &3, &5]);
    }

    #[test]
    fn test_clear() {
        let mut s = TreeSet::new(natural_comparator::<i32>());
        s.add(1);
        s.add(2);
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_stress() {
        let mut s = TreeSet::new(natural_comparator::<i32>());
        for i in (0..1000).rev() {
            s.add(i);
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
        s.add(Person {
            name: "Charlie".into(),
            age: 30,
        });
        s.add(Person {
            name: "Alice".into(),
            age: 30,
        });
        s.add(Person {
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
        s.add(1);
        s.add(3);
        s.add(2);
        let items: Vec<&i32> = s.to_vec();
        assert_eq!(items, vec![&3, &2, &1]);
    }

    #[test]
    fn test_empty_min_max() {
        let s = TreeSet::new(natural_comparator::<i32>());
        assert_eq!(s.min(), None);
        assert_eq!(s.max(), None);
    }
}
