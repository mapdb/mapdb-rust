// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use crate::bulk::{BulkError, DuplicatePolicy};
use crate::hash_table::OpenHashSet;
use std::borrow::Borrow;
use std::hash::Hash;

/// Generic unordered set backed by [`crate::hash_table::OpenHashSet`] — the
/// project's port of Eclipse Collections' open-addressing hash set. (Not
/// `std::HashSet`.)
#[derive(Debug, Clone)]
pub struct HashSet<T: Eq + std::hash::Hash> {
    inner: OpenHashSet<T>,
}

impl<T: Eq + std::hash::Hash> HashSet<T> {
    pub fn new() -> Self {
        HashSet {
            inner: OpenHashSet::new(),
        }
    }

    /// Bulk-loads a fresh set (size-hint path; may rehash). See
    /// [`OpenHashSet::bulk_load`].
    pub fn bulk_load<I: IntoIterator<Item = T>>(
        iter: I,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError> {
        Ok(HashSet {
            inner: OpenHashSet::bulk_load(iter, dup)?,
        })
    }

    /// Zero-rehash bulk load for an exactly-`n`-element source. See
    /// [`OpenHashSet::bulk_load_exact`].
    pub fn bulk_load_exact<I: IntoIterator<Item = T>>(
        iter: I,
        n: usize,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError> {
        Ok(HashSet {
            inner: OpenHashSet::bulk_load_exact(iter, n, dup)?,
        })
    }
}

// ---- core + functional API (formerly the trait tower) ----------------------

impl<T: Eq + std::hash::Hash> HashSet<T> {
    /// The number of elements.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    /// Add `value`; returns `true` if it was newly inserted.
    pub fn insert(&mut self, value: T) -> bool {
        self.inner.insert(value)
    }
    /// Remove all elements.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Retain only the elements for which `keep(&t)` returns `true`.
    /// O(n), no `T: Clone` (see [`OpenHashSet::retain`]).
    pub fn retain<F>(&mut self, keep: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.inner.retain(keep);
    }

    /// Whether any element satisfies `predicate`.
    pub fn any_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.inner.iter().any(predicate)
    }
    /// Whether every element satisfies `predicate`.
    pub fn all_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.inner.iter().all(predicate)
    }
    /// Whether no element satisfies `predicate`.
    pub fn none_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        !self.inner.iter().any(predicate)
    }
    /// Count elements matching `predicate`.
    pub fn count_where(&self, predicate: impl Fn(&T) -> bool) -> usize {
        self.inner.iter().filter(|v| predicate(v)).count()
    }
    /// The first element matching `predicate` (iteration order), if any.
    pub fn detect(&self, predicate: impl Fn(&T) -> bool) -> Option<&T> {
        self.inner.iter().find(|v| predicate(v))
    }
}

impl<T: Eq + std::hash::Hash + Clone> HashSet<T> {
    /// A `Vec` copy of the elements.
    pub fn to_vec(&self) -> Vec<T> {
        self.inner.iter().cloned().collect()
    }
    /// A `Vec` of the elements matching `predicate`.
    pub fn select(&self, predicate: impl Fn(&T) -> bool) -> Vec<T> {
        self.inner
            .iter()
            .filter(|v| predicate(v))
            .cloned()
            .collect()
    }
    /// A `Vec` of the elements *not* matching `predicate`.
    pub fn reject(&self, predicate: impl Fn(&T) -> bool) -> Vec<T> {
        self.inner
            .iter()
            .filter(|v| !predicate(v))
            .cloned()
            .collect()
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut out = self.clone();
        for v in other.inner.iter() {
            out.inner.insert(v.clone());
        }
        out
    }
    pub fn intersect(&self, other: &Self) -> Self {
        let mut out = HashSet::new();
        for v in self.inner.iter() {
            if other.inner.contains(v) {
                out.inner.insert(v.clone());
            }
        }
        out
    }
    pub fn difference(&self, other: &Self) -> Self {
        let mut out = HashSet::new();
        for v in self.inner.iter() {
            if !other.inner.contains(v) {
                out.inner.insert(v.clone());
            }
        }
        out
    }
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let mut out = self.difference(other);
        let rev = other.difference(self);
        for v in rev.inner.iter() {
            out.inner.insert(v.clone());
        }
        out
    }
}

impl<T: Eq + std::hash::Hash> Default for HashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ---- idiomatic std-style additions ----------------------------------------

impl<T: Eq + Hash> HashSet<T> {
    /// Borrowed `&T` iterator, so `for x in &set` and `set.iter()` both work.
    pub fn iter(&self) -> crate::hash_table::OpenHashSetIter<'_, T> {
        self.inner.iter()
    }

    /// Membership test by any borrowed form of the element (`T: Borrow<Q>`),
    /// e.g. `set.contains("str")` on a `HashSet<String>`.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains(value)
    }

    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(value)
    }
}

impl<'a, T: Eq + Hash> IntoIterator for &'a HashSet<T> {
    type Item = &'a T;
    type IntoIter = crate::hash_table::OpenHashSetIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<T: Eq + Hash> IntoIterator for HashSet<T> {
    type Item = T;
    type IntoIter = crate::hash_table::OpenHashSetIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<T: Eq + Hash> FromIterator<T> for HashSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        HashSet {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T: Eq + Hash> Extend<T> for HashSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for v in iter {
            self.inner.insert(v);
        }
    }
}

/// Order-insensitive set equality.
impl<T: Eq + Hash> PartialEq for HashSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.len() == other.inner.len() && self.inner.iter().all(|v| other.inner.contains(v))
    }
}

impl<T: Eq + Hash> Eq for HashSet<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut s = HashSet::new();
        assert!(s.insert(1));
        assert!(s.insert(2));
        assert!(!s.insert(1));
        assert_eq!(s.len(), 2);
        assert!(s.contains(&1));
        assert!(s.remove(&1));
        assert!(!s.contains(&1));
    }

    #[test]
    fn test_set_operations() {
        let a = HashSet::from_iter([1, 2, 3]);
        let b = HashSet::from_iter([2, 3, 4]);
        let union = a.union(&b);
        assert_eq!(union.len(), 4);
        let inter = a.intersect(&b);
        assert_eq!(inter.len(), 2);
        assert!(inter.contains(&2) && inter.contains(&3));
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 1);
        assert!(diff.contains(&1));
        let sym = a.symmetric_difference(&b);
        assert_eq!(sym.len(), 2);
    }

    #[test]
    fn test_functional() {
        let s = HashSet::from_iter([1, 2, 3, 4, 5]);
        assert!(s.any_satisfy(|v| *v > 4));
        assert!(s.all_satisfy(|v| *v > 0));
        assert_eq!(s.count_where(|v| *v % 2 == 0), 2);
    }

    #[test]
    fn test_string_type() {
        let s = HashSet::from_iter(["a".to_string(), "b".to_string()]);
        assert!(s.contains(&"a".to_string()));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_into_iter_borrowing_and_owned() {
        let s = HashSet::from_iter([1, 2, 3]);
        let mut sum = 0;
        for v in &s {
            sum += *v;
        }
        assert_eq!(sum, 6);
        let mut owned: Vec<i32> = s.into_iter().collect();
        owned.sort();
        assert_eq!(owned, vec![1, 2, 3]);
    }

    #[test]
    fn test_from_iterator_and_extend() {
        let mut s: HashSet<i32> = [1, 2, 3].into_iter().collect();
        assert_eq!(s.len(), 3);
        s.extend([3, 4, 5]);
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn test_partial_eq_order_insensitive() {
        let a: HashSet<i32> = [1, 2, 3].into_iter().collect();
        let b: HashSet<i32> = [3, 2, 1].into_iter().collect();
        assert_eq!(a, b);
        let c: HashSet<i32> = [1, 2, 4].into_iter().collect();
        assert_ne!(a, c);
    }

    #[test]
    fn test_borrow_contains_str() {
        let mut s: HashSet<String> = HashSet::new();
        s.insert("hello".to_string());
        assert!(s.contains("hello"));
        assert!(s.remove("hello"));
        assert!(!s.contains("hello"));
    }

    #[test]
    fn bulk_load_equals_incremental() {
        use crate::bulk::DuplicatePolicy;
        let data: Vec<i32> = (0..50).collect();
        let bulk =
            HashSet::bulk_load_exact(data.clone(), data.len(), DuplicatePolicy::Error).unwrap();
        let mut inc = HashSet::new();
        for v in &data {
            inc.insert(*v);
        }
        assert_eq!(bulk, inc);
    }

    #[test]
    fn retain_keeps_matching() {
        let mut s: HashSet<i32> = (0..10).collect();
        s.retain(|v| v % 3 == 0);
        assert_eq!(s.len(), 4); // 0,3,6,9
        assert!(s.contains(&0) && s.contains(&9));
        assert!(!s.contains(&1));
    }
}
