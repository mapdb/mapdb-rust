// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Insertion-ordered set (`LinkedHashSet`), a thin wrapper over
//! [`LinkedHashMap<T, ()>`](super::LinkedHashMap).
//!
//! Rebuilt for v3 (blueprint doc 14 §5 / M6): the previous standalone `Vec` +
//! `HashMap` implementation is deleted and the set now delegates to the arena-
//! backed map. It inherits the map's wins for free: **O(1) `remove`** (no index
//! fix-ups), **no `T: Clone`** on the core operations, and **`Borrow<Q>`**
//! lookups. Iteration follows insertion order; duplicate adds are no-ops.

use super::linkedhashmap;
use super::LinkedHashMap;
use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::{BuildHasher, Hash};

/// Insertion-ordered set of `T`. The hasher `S` defaults to [`RandomState`].
pub struct LinkedHashSet<T, S = RandomState> {
    map: LinkedHashMap<T, (), S>,
}

impl<T: Eq + Hash> LinkedHashSet<T, RandomState> {
    /// An empty set with the default hasher.
    pub fn new() -> Self {
        LinkedHashSet {
            map: LinkedHashMap::new(),
        }
    }

    /// An empty set with room reserved for `cap` elements.
    pub fn with_capacity(cap: usize) -> Self {
        LinkedHashSet {
            map: LinkedHashMap::with_capacity(cap),
        }
    }
}

impl<T: Eq + Hash, S: BuildHasher> LinkedHashSet<T, S> {
    /// An empty set that hashes elements with `hasher`.
    pub fn with_hasher(hasher: S) -> Self {
        LinkedHashSet {
            map: LinkedHashMap::with_hasher(hasher),
        }
    }

    /// The number of elements.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Add `value`. Returns `true` if it was newly inserted, `false` if it was
    /// already present (a no-op that preserves the original position).
    pub fn insert(&mut self, value: T) -> bool {
        self.map.insert(value, ()).is_none()
    }

    /// Whether `value` is present. Accepts any borrowed form.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(value)
    }

    /// Remove `value`. Returns `true` if it was present. O(1). Accepts any
    /// borrowed form.
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.remove(value).is_some()
    }

    /// Remove all elements.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Iterate elements in insertion order.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            inner: self.map.iter(),
        }
    }
}

// ---- set algebra (build new owned sets; needs Clone) -----------------------

impl<T: Eq + Hash + Clone, S: BuildHasher + Default> LinkedHashSet<T, S> {
    /// Elements in `self` or `other` (self's order first, then other's new ones).
    pub fn union(&self, other: &Self) -> Self {
        let mut result = self.clone_shape();
        for v in self.iter() {
            result.insert(v.clone());
        }
        for v in other.iter() {
            result.insert(v.clone());
        }
        result
    }

    /// Elements in both `self` and `other` (self's order).
    pub fn intersect(&self, other: &Self) -> Self {
        let mut result = self.clone_shape();
        for v in self.iter() {
            if other.contains(v) {
                result.insert(v.clone());
            }
        }
        result
    }

    /// Elements in `self` but not `other` (self's order).
    pub fn difference(&self, other: &Self) -> Self {
        let mut result = self.clone_shape();
        for v in self.iter() {
            if !other.contains(v) {
                result.insert(v.clone());
            }
        }
        result
    }

    /// Elements in exactly one of `self`/`other` (self's order, then other's).
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let mut result = self.clone_shape();
        for v in self.iter() {
            if !other.contains(v) {
                result.insert(v.clone());
            }
        }
        for v in other.iter() {
            if !self.contains(v) {
                result.insert(v.clone());
            }
        }
        result
    }

    /// An empty set with the same hasher configuration as `self`.
    fn clone_shape(&self) -> Self {
        Self::with_hasher(S::default())
    }
}

// ---- functional API (formerly the trait tower) -----------------------------

impl<T: Eq + Hash, S: BuildHasher> LinkedHashSet<T, S> {
    /// Whether any element satisfies `predicate`.
    pub fn any_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.iter().any(predicate)
    }
    /// Whether every element satisfies `predicate`.
    pub fn all_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.iter().all(predicate)
    }
    /// Whether no element satisfies `predicate`.
    pub fn none_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        !self.iter().any(predicate)
    }
    /// Count elements matching `predicate`.
    pub fn count_where(&self, predicate: impl Fn(&T) -> bool) -> usize {
        self.iter().filter(|v| predicate(v)).count()
    }
    /// The first element matching `predicate` (insertion order), if any.
    pub fn detect(&self, predicate: impl Fn(&T) -> bool) -> Option<&T> {
        self.iter().find(|v| predicate(v))
    }
    /// Fold `f` over the elements (insertion order) starting from `initial`.
    pub fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, &T) -> R) -> R {
        let mut acc = initial;
        for v in self.iter() {
            acc = f(acc, v);
        }
        acc
    }
}

impl<T: Eq + Hash + Clone, S: BuildHasher> LinkedHashSet<T, S> {
    /// A `Vec` copy of the elements in insertion order.
    pub fn to_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }
    /// A `Vec` of the elements matching `predicate` (insertion order).
    pub fn select(&self, predicate: impl Fn(&T) -> bool) -> Vec<T> {
        self.iter().filter(|v| predicate(v)).cloned().collect()
    }
    /// A `Vec` of the elements *not* matching `predicate` (insertion order).
    pub fn reject(&self, predicate: impl Fn(&T) -> bool) -> Vec<T> {
        self.iter().filter(|v| !predicate(v)).cloned().collect()
    }
}

// ---- std-style trait impls -------------------------------------------------

impl<T: Eq + Hash, S: BuildHasher + Default> Default for LinkedHashSet<T, S> {
    fn default() -> Self {
        LinkedHashSet {
            map: LinkedHashMap::default(),
        }
    }
}

impl<T, S> Clone for LinkedHashSet<T, S>
where
    T: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        LinkedHashSet {
            map: self.map.clone(),
        }
    }
}

impl<T: fmt::Debug, S> fmt::Debug for LinkedHashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Eq + Hash, S: BuildHasher + Default> FromIterator<T> for LinkedHashSet<T, S> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut s = Self::default();
        s.extend(iter);
        s
    }
}

impl<T: Eq + Hash, S: BuildHasher> Extend<T> for LinkedHashSet<T, S> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for v in iter {
            self.insert(v);
        }
    }
}

/// Order-insensitive set equality (same element set, ordering ignored).
impl<T: Eq + Hash, S: BuildHasher> PartialEq for LinkedHashSet<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|v| other.contains(v))
    }
}

impl<T: Eq + Hash, S: BuildHasher> Eq for LinkedHashSet<T, S> {}

// ---- iterators -------------------------------------------------------------

/// Shared-reference iterator over elements in insertion order.
pub struct Iter<'a, T> {
    inner: linkedhashmap::Iter<'a, T, ()>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.inner.next().map(|(k, _)| k)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}
impl<T> std::iter::FusedIterator for Iter<'_, T> {}

/// Owned iterator over elements in insertion order.
pub struct IntoIter<T> {
    inner: linkedhashmap::IntoIter<T, ()>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.next().map(|(k, _)| k)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}
impl<T> std::iter::FusedIterator for IntoIter<T> {}

impl<'a, T: Eq + Hash, S: BuildHasher> IntoIterator for &'a LinkedHashSet<T, S> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T, S> IntoIterator for LinkedHashSet<T, S> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.map.into_iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut s = LinkedHashSet::new();
        assert!(s.insert(1));
        assert!(s.insert(2));
        assert!(!s.insert(1));
        assert_eq!(s.len(), 2);
        assert!(s.contains(&1));
        assert!(s.remove(&1));
        assert!(!s.contains(&1));
    }

    #[test]
    fn test_insertion_order() {
        let s: LinkedHashSet<i32> = LinkedHashSet::from_iter([3, 1, 4, 1, 5, 9]);
        let v: Vec<&i32> = s.iter().collect();
        assert_eq!(v, vec![&3, &1, &4, &5, &9]);
    }

    #[test]
    fn test_remove_preserves_order() {
        let mut s: LinkedHashSet<i32> = LinkedHashSet::from_iter([1, 2, 3, 4]);
        s.remove(&2);
        let v: Vec<&i32> = s.iter().collect();
        assert_eq!(v, vec![&1, &3, &4]);
    }

    #[test]
    fn test_set_operations() {
        let a: LinkedHashSet<i32> = LinkedHashSet::from_iter([1, 2, 3]);
        let b: LinkedHashSet<i32> = LinkedHashSet::from_iter([2, 3, 4]);
        let union = a.union(&b);
        assert_eq!(union.len(), 4);
        let v: Vec<&i32> = union.iter().collect();
        assert_eq!(v, vec![&1, &2, &3, &4]);
        let inter = a.intersect(&b);
        assert_eq!(inter.len(), 2);
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 1);
        assert!(diff.contains(&1));
        let sym = a.symmetric_difference(&b);
        assert_eq!(sym.len(), 2);
    }

    #[test]
    fn test_functional() {
        let s: LinkedHashSet<i32> = LinkedHashSet::from_iter([1, 2, 3, 4, 5]);
        assert!(s.any_satisfy(|v| *v > 4));
        assert!(s.all_satisfy(|v| *v > 0));
        assert_eq!(s.count_where(|v| *v % 2 == 0), 2);
    }

    #[test]
    fn test_clear() {
        let mut s: LinkedHashSet<i32> = LinkedHashSet::from_iter([1, 2]);
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_into_iter_insertion_order() {
        let s: LinkedHashSet<i32> = LinkedHashSet::from_iter([3, 1, 2]);
        let borrowed: Vec<i32> = (&s).into_iter().copied().collect();
        assert_eq!(borrowed, vec![3, 1, 2]);
        let owned: Vec<i32> = s.into_iter().collect();
        assert_eq!(owned, vec![3, 1, 2]);
    }

    #[test]
    fn test_from_iterator_and_extend() {
        let mut s: LinkedHashSet<i32> = [1, 2, 3].into_iter().collect();
        assert_eq!(s.len(), 3);
        s.extend([3, 4]);
        let v: Vec<&i32> = s.iter().collect();
        assert_eq!(v, vec![&1, &2, &3, &4]);
    }

    #[test]
    fn test_partial_eq_order_insensitive() {
        let a: LinkedHashSet<i32> = LinkedHashSet::from_iter([1, 2, 3]);
        let b: LinkedHashSet<i32> = LinkedHashSet::from_iter([3, 2, 1]);
        assert_eq!(a, b);
        let c: LinkedHashSet<i32> = LinkedHashSet::from_iter([1, 2, 4]);
        assert_ne!(a, c);
    }

    // ---- v3 additions --------------------------------------------------

    #[test]
    fn borrow_lookup_with_str() {
        let mut s: LinkedHashSet<String> = LinkedHashSet::new();
        s.insert("alpha".to_string());
        s.insert("beta".to_string());
        assert!(s.contains("alpha"));
        assert!(s.remove("alpha"));
        assert!(!s.contains("alpha"));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn clone_is_independent() {
        let a: LinkedHashSet<i32> = LinkedHashSet::from_iter([1, 2, 3]);
        let mut b = a.clone();
        b.insert(4);
        assert_eq!(a.len(), 3);
        assert_eq!(b.len(), 4);
        assert_eq!(b.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3, 4]);
    }
}
