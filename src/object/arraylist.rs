// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

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

    /// Bulk-loads a fresh list from `iter` in one allocation pass: reserves the
    /// source's size hint up front, then extends. Order is preserved; no
    /// validation (lists accept any order, any duplicates). This is the list
    /// family's data-pump entry point — a thin, discoverable alias over
    /// `Vec::with_capacity` + `extend`.
    pub fn bulk_load<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut items = Vec::with_capacity(iter.size_hint().0);
        items.extend(iter);
        ArrayList { items }
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

// ---- core + functional API (formerly the trait tower) ----------------------

impl<T: PartialEq> ArrayList<T> {
    /// The number of elements.
    pub fn len(&self) -> usize {
        self.items.len()
    }
    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// Whether `value` is present.
    pub fn contains(&self, value: &T) -> bool {
        self.items.contains(value)
    }
    /// Iterate elements in order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }
    /// The element at `index`, if any.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }
    /// The index of the first occurrence of `value`, if any.
    pub fn index_of(&self, value: &T) -> Option<usize> {
        self.items.iter().position(|v| v == value)
    }
    /// Append `value` to the end.
    pub fn push(&mut self, value: T) {
        self.items.push(value);
    }
    /// Replace the element at `index`, returning the old value.
    pub fn set(&mut self, index: usize, value: T) -> T {
        std::mem::replace(&mut self.items[index], value)
    }
    /// Remove all elements.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Apply `f` to each element.
    pub fn for_each(&self, mut f: impl FnMut(&T)) {
        for v in &self.items {
            f(v);
        }
    }
    /// Whether any element satisfies `predicate`.
    pub fn any_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.items.iter().any(predicate)
    }
    /// Whether every element satisfies `predicate`.
    pub fn all_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.items.iter().all(predicate)
    }
    /// Whether no element satisfies `predicate`.
    pub fn none_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        !self.items.iter().any(predicate)
    }
    /// Count elements matching `predicate`.
    pub fn count_where(&self, predicate: impl Fn(&T) -> bool) -> usize {
        self.items.iter().filter(|v| predicate(v)).count()
    }
    /// The first element matching `predicate`, if any.
    pub fn detect(&self, predicate: impl Fn(&T) -> bool) -> Option<&T> {
        self.items.iter().find(|v| predicate(v))
    }
    /// Fold `f` over the elements starting from `initial`.
    pub fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, &T) -> R) -> R {
        let mut acc = initial;
        for v in &self.items {
            acc = f(acc, v);
        }
        acc
    }
}

impl<T: PartialEq + Clone> ArrayList<T> {
    /// A `Vec` copy of the elements.
    pub fn to_vec(&self) -> Vec<T> {
        self.items.clone()
    }
    /// A `Vec` of the elements matching `predicate`.
    pub fn select(&self, predicate: impl Fn(&T) -> bool) -> Vec<T> {
        self.items
            .iter()
            .filter(|v| predicate(v))
            .cloned()
            .collect()
    }
    /// A `Vec` of the elements *not* matching `predicate`.
    pub fn reject(&self, predicate: impl Fn(&T) -> bool) -> Vec<T> {
        self.items
            .iter()
            .filter(|v| !predicate(v))
            .cloned()
            .collect()
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

impl<'a, T> IntoIterator for &'a ArrayList<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut ArrayList<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

impl<T> FromIterator<T> for ArrayList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        ArrayList {
            items: iter.into_iter().collect(),
        }
    }
}

impl<T> Extend<T> for ArrayList<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.items.extend(iter);
    }
}

impl<T> ArrayList<T> {
    /// Mutable iterator over the backing elements, so `for x in &mut list` and
    /// `list.iter_mut()` both work.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.items.iter_mut()
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
        let list = ArrayList::from_iter(vec![1, 2, 3, 4, 5]);
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
        let mut list = ArrayList::from_iter(vec![3, 1, 2, 1, 3]);
        list.sort();
        assert_eq!(list.to_vec(), vec![1, 1, 2, 3, 3]);
        let rev = list.reversed();
        assert_eq!(rev.to_vec(), vec![3, 3, 2, 1, 1]);
        let dist = list.distinct();
        assert_eq!(dist.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn test_string_type() {
        let list = ArrayList::from_iter(vec!["hello".to_string(), "world".to_string()]);
        assert!(list.contains(&"hello".to_string()));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_set_and_remove() {
        let mut list = ArrayList::from_iter(vec![10, 20, 30]);
        let old = list.set(1, 99);
        assert_eq!(old, 20);
        assert_eq!(list.get(1), Some(&99));
        assert!(list.remove(&10));
        assert!(!list.remove(&10));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_into_iter_ref_and_mut() {
        let mut list = ArrayList::from_iter(vec![1, 2, 3]);
        let sum: i32 = (&list).into_iter().sum();
        assert_eq!(sum, 6);
        for v in &mut list {
            *v *= 10;
        }
        assert_eq!(list.to_vec(), vec![10, 20, 30]);
    }

    #[test]
    fn test_from_iterator_and_extend() {
        let mut list: ArrayList<i32> = (1..=3).collect();
        assert_eq!(list.to_vec(), vec![1, 2, 3]);
        list.extend([4, 5]);
        assert_eq!(list.to_vec(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_partial_eq_order_sensitive() {
        let a = ArrayList::from_iter(vec![1, 2, 3]);
        let b = ArrayList::from_iter(vec![1, 2, 3]);
        let c = ArrayList::from_iter(vec![3, 2, 1]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn bulk_load_preserves_order() {
        let list = ArrayList::bulk_load(vec![3, 1, 2, 1]);
        assert_eq!(list.to_vec(), vec![3, 1, 2, 1]);
        let empty: ArrayList<i32> = ArrayList::bulk_load(Vec::new());
        assert!(empty.is_empty());
    }
}
