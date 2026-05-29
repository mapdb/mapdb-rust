// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use super::traits::*;

/// Generic ordered list backed by a `Vec<T>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayList<T> {
    items: Vec<T>,
}

impl<T> ArrayList<T> {
    pub fn new() -> Self {
        ArrayList { items: Vec::new() }
    }
    pub fn with_capacity(cap: usize) -> Self {
        ArrayList {
            items: Vec::with_capacity(cap),
        }
    }
    pub fn of(values: impl IntoIterator<Item = T>) -> Self {
        ArrayList {
            items: values.into_iter().collect(),
        }
    }

    /// Borrows the backing storage as a contiguous slice.
    ///
    /// This is the bridge to the [`parallel`](crate::parallel) module: the
    /// slice-based `BatchIterable`, `SliceSpliterator`, batch executor, and
    /// `as_parallel` all apply directly to `list.as_slice()`.
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }
}

impl<T: PartialEq> Collection<T> for ArrayList<T> {
    fn len(&self) -> usize {
        self.items.len()
    }
    fn contains(&self, value: &T) -> bool {
        self.items.contains(value)
    }
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(self.items.iter())
    }
}

impl<T: PartialEq> MutableCollection<T> for ArrayList<T> {
    fn clear(&mut self) {
        self.items.clear();
    }
}

impl<T: PartialEq> List<T> for ArrayList<T> {
    fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }
    fn index_of(&self, value: &T) -> Option<usize> {
        self.items.iter().position(|v| v == value)
    }
}

impl<T: PartialEq> MutableList<T> for ArrayList<T> {
    fn push(&mut self, value: T) {
        self.items.push(value);
    }
    fn set(&mut self, index: usize, value: T) -> T {
        std::mem::replace(&mut self.items[index], value)
    }
}

impl<T: PartialEq> ArrayList<T> {
    pub fn remove(&mut self, value: &T) -> bool {
        if let Some(pos) = self.items.iter().position(|v| v == value) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }
}

impl<T: PartialEq + Ord> ArrayList<T> {
    pub fn sort(&mut self) {
        self.items.sort();
    }
}

impl<T: PartialEq + Clone> ArrayList<T> {
    pub fn reversed(&self) -> Self {
        let mut v = self.items.clone();
        v.reverse();
        ArrayList { items: v }
    }
}

impl<T: PartialEq + Eq + std::hash::Hash + Clone> ArrayList<T> {
    pub fn distinct(&self) -> Self {
        let mut seen = std::collections::HashSet::new();
        let items = self
            .items
            .iter()
            .filter(|v| seen.insert((*v).clone()))
            .cloned()
            .collect();
        ArrayList { items }
    }
}

impl<T: std::fmt::Display + PartialEq> std::fmt::Display for ArrayList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.items.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", v)?;
        }
        write!(f, "]")
    }
}

impl<T> IntoIterator for ArrayList<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<T: PartialEq> Default for ArrayList<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_ops() {
        let mut list = ArrayList::new();
        assert!(list.is_empty());
        list.push(10);
        list.push(20);
        list.push(30);
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(1), Some(&20));
        assert_eq!(list.index_of(&30), Some(2));
        assert!(list.contains(&10));
        assert!(!list.contains(&99));
    }

    #[test]
    fn test_functional_ops() {
        let list = ArrayList::of(vec![1, 2, 3, 4, 5]);
        assert!(list.any_satisfy(|v| *v > 3));
        assert!(list.all_satisfy(|v| *v > 0));
        assert!(list.none_satisfy(|v| *v > 10));
        assert_eq!(list.count_where(|v| *v % 2 == 0), 2);
        assert_eq!(list.detect(|v| *v > 3), Some(&4));
        assert_eq!(list.select(|v| *v > 3), vec![4, 5]);
        assert_eq!(list.reject(|v| *v > 3), vec![1, 2, 3]);
        let sum = list.inject_into(0i64, |acc, v| acc + *v as i64);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_sort_reverse_distinct() {
        let mut list = ArrayList::of(vec![3, 1, 2, 1, 3]);
        list.sort();
        assert_eq!(list.to_vec(), vec![1, 1, 2, 3, 3]);
        let rev = list.reversed();
        assert_eq!(rev.to_vec(), vec![3, 3, 2, 1, 1]);
        let dist = list.distinct();
        assert_eq!(dist.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn test_string_type() {
        let list = ArrayList::of(vec!["hello".to_string(), "world".to_string()]);
        assert!(list.contains(&"hello".to_string()));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_set_and_remove() {
        let mut list = ArrayList::of(vec![10, 20, 30]);
        let old = list.set(1, 99);
        assert_eq!(old, 20);
        assert_eq!(list.get(1), Some(&99));
        assert!(list.remove(&10));
        assert!(!list.remove(&10));
        assert_eq!(list.len(), 2);
    }
}
