// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use super::traits::*;
use std::collections::HashMap as StdHashMap;
use std::hash::Hash;

/// Insertion-ordered set backed by a `Vec` of elements and a hash index.
/// Iteration follows insertion order. Duplicate adds are no-ops.
#[derive(Debug, Clone)]
pub struct LinkedHashSet<T: Eq + Hash + Clone> {
    entries: Vec<T>,
    index: StdHashMap<T, usize>,
}

impl<T: Eq + Hash + Clone> LinkedHashSet<T> {
    pub fn new() -> Self {
        LinkedHashSet {
            entries: Vec::new(),
            index: StdHashMap::new(),
        }
    }

    pub fn of(values: impl IntoIterator<Item = T>) -> Self {
        let mut s = Self::new();
        for v in values {
            s.add(v);
        }
        s
    }
}

impl<T: Eq + Hash + Clone + PartialEq> Collection<T> for LinkedHashSet<T> {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn contains(&self, value: &T) -> bool {
        self.index.contains_key(value)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(self.entries.iter())
    }
}

impl<T: Eq + Hash + Clone + PartialEq> MutableCollection<T> for LinkedHashSet<T> {
    fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }
}

impl<T: Eq + Hash + Clone + PartialEq> Set<T> for LinkedHashSet<T> {}

impl<T: Eq + Hash + Clone + PartialEq> MutableSet<T> for LinkedHashSet<T> {
    fn add(&mut self, value: T) -> bool {
        if self.index.contains_key(&value) {
            return false;
        }
        let idx = self.entries.len();
        self.index.insert(value.clone(), idx);
        self.entries.push(value);
        true
    }

    fn remove(&mut self, value: &T) -> bool {
        if let Some(idx) = self.index.remove(value) {
            self.entries.remove(idx);
            // Fix indices for entries that shifted down
            for (_, i) in self.index.iter_mut() {
                if *i > idx {
                    *i -= 1;
                }
            }
            true
        } else {
            false
        }
    }
}

impl<T: Eq + Hash + Clone> LinkedHashSet<T> {
    pub fn union(&self, other: &Self) -> Self {
        let mut result = Self::of(self.entries.iter().cloned());
        for v in &other.entries {
            result.add(v.clone());
        }
        result
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for v in &self.entries {
            if other.index.contains_key(v) {
                result.add(v.clone());
            }
        }
        result
    }

    pub fn difference(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for v in &self.entries {
            if !other.index.contains_key(v) {
                result.add(v.clone());
            }
        }
        result
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for v in &self.entries {
            if !other.index.contains_key(v) {
                result.add(v.clone());
            }
        }
        for v in &other.entries {
            if !self.index.contains_key(v) {
                result.add(v.clone());
            }
        }
        result
    }
}

impl<T: Eq + Hash + Clone> Default for LinkedHashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut s = LinkedHashSet::new();
        assert!(s.add(1));
        assert!(s.add(2));
        assert!(!s.add(1));
        assert_eq!(s.len(), 2);
        assert!(s.contains(&1));
        assert!(s.remove(&1));
        assert!(!s.contains(&1));
    }

    #[test]
    fn test_insertion_order() {
        let s = LinkedHashSet::of(vec![3, 1, 4, 1, 5, 9]);
        let v: Vec<&i32> = s.iter().collect();
        assert_eq!(v, vec![&3, &1, &4, &5, &9]);
    }

    #[test]
    fn test_remove_preserves_order() {
        let mut s = LinkedHashSet::of(vec![1, 2, 3, 4]);
        s.remove(&2);
        let v: Vec<&i32> = s.iter().collect();
        assert_eq!(v, vec![&1, &3, &4]);
    }

    #[test]
    fn test_set_operations() {
        let a = LinkedHashSet::of(vec![1, 2, 3]);
        let b = LinkedHashSet::of(vec![2, 3, 4]);
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
        let s = LinkedHashSet::of(vec![1, 2, 3, 4, 5]);
        assert!(s.any_satisfy(|v| *v > 4));
        assert!(s.all_satisfy(|v| *v > 0));
        assert_eq!(s.count_where(|v| *v % 2 == 0), 2);
    }

    #[test]
    fn test_clear() {
        let mut s = LinkedHashSet::of(vec![1, 2]);
        s.clear();
        assert!(s.is_empty());
    }
}
