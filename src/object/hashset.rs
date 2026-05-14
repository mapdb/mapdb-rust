// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use super::traits::*;
use crate::hash_table::OpenHashSet;

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
    pub fn of(values: impl IntoIterator<Item = T>) -> Self {
        let mut s = HashSet::new();
        for v in values {
            s.inner.add(v);
        }
        s
    }
}

impl<T: Eq + std::hash::Hash> Collection<T> for HashSet<T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
    fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
    }
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(self.inner.iter())
    }
}

impl<T: Eq + std::hash::Hash> MutableCollection<T> for HashSet<T> {
    fn clear(&mut self) {
        self.inner.clear();
    }
}

impl<T: Eq + std::hash::Hash> Set<T> for HashSet<T> {}

impl<T: Eq + std::hash::Hash> MutableSet<T> for HashSet<T> {
    fn add(&mut self, value: T) -> bool {
        self.inner.add(value)
    }
    fn remove(&mut self, value: &T) -> bool {
        self.inner.remove(value)
    }
}

impl<T: Eq + std::hash::Hash + Clone> HashSet<T> {
    pub fn union(&self, other: &Self) -> Self {
        let mut out = self.clone();
        for v in other.inner.iter() {
            out.inner.add(v.clone());
        }
        out
    }
    pub fn intersect(&self, other: &Self) -> Self {
        let mut out = HashSet::new();
        for v in self.inner.iter() {
            if other.inner.contains(v) {
                out.inner.add(v.clone());
            }
        }
        out
    }
    pub fn difference(&self, other: &Self) -> Self {
        let mut out = HashSet::new();
        for v in self.inner.iter() {
            if !other.inner.contains(v) {
                out.inner.add(v.clone());
            }
        }
        out
    }
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let mut out = self.difference(other);
        let rev = other.difference(self);
        for v in rev.inner.iter() {
            out.inner.add(v.clone());
        }
        out
    }
}

impl<T: Eq + std::hash::Hash> Default for HashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut s = HashSet::new();
        assert!(s.add(1));
        assert!(s.add(2));
        assert!(!s.add(1));
        assert_eq!(s.len(), 2);
        assert!(s.contains(&1));
        assert!(s.remove(&1));
        assert!(!s.contains(&1));
    }

    #[test]
    fn test_set_operations() {
        let a = HashSet::of(vec![1, 2, 3]);
        let b = HashSet::of(vec![2, 3, 4]);
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
        let s = HashSet::of(vec![1, 2, 3, 4, 5]);
        assert!(s.any_satisfy(|v| *v > 4));
        assert!(s.all_satisfy(|v| *v > 0));
        assert_eq!(s.count_where(|v| *v % 2 == 0), 2);
    }

    #[test]
    fn test_string_type() {
        let s = HashSet::of(vec!["a".to_string(), "b".to_string()]);
        assert!(s.contains(&"a".to_string()));
        assert_eq!(s.len(), 2);
    }
}
