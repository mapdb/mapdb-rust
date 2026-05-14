// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Resizable array-backed list of `bool` values.
///
/// Specialized for `bool` — no boxing, contiguous memory layout.
#[derive(Debug, Clone)]
pub struct BoolArrayList {
    items: Vec<bool>,
}

impl BoolArrayList {
    /// Creates a new empty list.
    pub fn new() -> Self {
        BoolArrayList { items: Vec::new() }
    }

    /// Creates a new empty list with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        BoolArrayList {
            items: Vec::with_capacity(capacity),
        }
    }

    /// Creates a list from a slice of values.
    pub fn of(values: &[bool]) -> Self {
        BoolArrayList {
            items: values.to_vec(),
        }
    }

    /// Appends a value to the end of the list.
    pub fn push(&mut self, value: bool) {
        self.items.push(value);
    }

    /// Appends all values from a slice.
    pub fn push_all(&mut self, values: &[bool]) {
        self.items.extend_from_slice(values);
    }

    /// Returns the element at the given index, or None if out of bounds.
    pub fn get(&self, index: usize) -> Option<bool> {
        self.items.get(index).copied()
    }

    /// Sets the element at the given index. Returns the old value.
    ///
    /// # Panics
    /// Panics if index is out of bounds.
    pub fn set(&mut self, index: usize, value: bool) -> bool {
        let old = self.items[index];
        self.items[index] = value;
        old
    }

    /// Removes and returns the element at the given index.
    ///
    /// # Panics
    /// Panics if index is out of bounds.
    pub fn remove_at_index(&mut self, index: usize) -> bool {
        self.items.remove(index)
    }

    /// Removes the first occurrence of the value. Returns true if found.
    pub fn remove(&mut self, value: bool) -> bool {
        if let Some(idx) = self.index_of(value) {
            self.items.remove(idx);
            true
        } else {
            false
        }
    }

    /// Returns true if the list contains the given value.
    pub fn contains(&self, value: bool) -> bool {
        self.items.contains(&value)
    }

    /// Returns the index of the first occurrence, or None if not found.
    pub fn index_of(&self, value: bool) -> Option<usize> {
        self.items.iter().position(|&v| v == value)
    }

    /// Returns the number of elements.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Removes all elements.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    // ---- Fallible capacity reservation ----

    /// Ensures that `additional` more items can be pushed without a
    /// reallocation. Delegates to [`Vec::try_reserve`]; returns
    /// `TryReserveError` on allocator failure. Pair this with the
    /// infallible [`push`] / [`with`] methods to get an opt-in
    /// allocation-failure handling path. See
    /// `docs/rust/error-handling.md`.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.items.try_reserve(additional)
    }

    /// Ensures that `additional` more items can be pushed without a
    /// reallocation, rounding to the exact requested amount (no
    /// amortization slack). See [`Vec::try_reserve_exact`].
    pub fn try_reserve_exact(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.items.try_reserve_exact(additional)
    }

    /// Returns an iterator over the elements.
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.items.iter().copied()
    }

    /// Calls the given function for each element.
    pub fn for_each(&self, mut f: impl FnMut(bool)) {
        for &v in &self.items {
            f(v);
        }
    }

    /// Returns a new list with only elements satisfying the predicate.
    pub fn select(&self, predicate: impl Fn(bool) -> bool) -> Self {
        BoolArrayList {
            items: self
                .items
                .iter()
                .copied()
                .filter(|&v| predicate(v))
                .collect(),
        }
    }

    /// Returns a new list with elements NOT satisfying the predicate.
    pub fn reject(&self, predicate: impl Fn(bool) -> bool) -> Self {
        BoolArrayList {
            items: self
                .items
                .iter()
                .copied()
                .filter(|&v| !predicate(v))
                .collect(),
        }
    }

    /// Returns the first element satisfying the predicate, or None.
    pub fn detect(&self, predicate: impl Fn(bool) -> bool) -> Option<bool> {
        self.items.iter().copied().find(|&v| predicate(v))
    }

    /// Returns true if any element satisfies the predicate.
    pub fn any_satisfy(&self, predicate: impl Fn(bool) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    /// Returns true if all elements satisfy the predicate.
    pub fn all_satisfy(&self, predicate: impl Fn(bool) -> bool) -> bool {
        self.items.iter().all(|&v| predicate(v))
    }

    /// Returns true if no element satisfies the predicate.
    pub fn none_satisfy(&self, predicate: impl Fn(bool) -> bool) -> bool {
        !self.items.iter().any(|&v| predicate(v))
    }

    /// Returns the count of elements satisfying the predicate.
    pub fn count(&self, predicate: impl Fn(bool) -> bool) -> usize {
        self.items.iter().filter(|&&v| predicate(v)).count()
    }

    /// Returns the sum of all elements.
    pub fn sum(&self) -> i64 {
        self.items.iter().copied().map(|v| v as i64).sum()
    }

    /// Returns the minimum element, or None if empty.
    pub fn min(&self) -> Option<bool> {
        self.items.iter().copied().min()
    }

    /// Returns the maximum element, or None if empty.
    pub fn max(&self) -> Option<bool> {
        self.items.iter().copied().max()
    }

    /// Sorts the list in ascending order.
    pub fn sort(&mut self) {
        self.items.sort();
    }

    /// Returns a new list with elements in reverse order.
    pub fn reversed(&self) -> Self {
        let mut items = self.items.clone();
        items.reverse();
        BoolArrayList { items }
    }

    /// Returns a new list with duplicate elements removed (preserving first occurrence order).
    pub fn distinct(&self) -> Self {
        let mut seen = std::collections::HashSet::new();
        let items: Vec<bool> = self
            .items
            .iter()
            .copied()
            .filter(|v| seen.insert(*v))
            .collect();
        BoolArrayList { items }
    }

    /// Returns the elements as a Vec.
    pub fn to_vec(&self) -> Vec<bool> {
        self.items.clone()
    }

    /// Folds all elements using the given function and initial value.
    pub fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, bool) -> R) -> R {
        let mut acc = initial;
        for &v in &self.items {
            acc = f(acc, v);
        }
        acc
    }

    /// Returns the list after pushing a value (fluent API).
    pub fn with(mut self, value: bool) -> Self {
        self.push(value);
        self
    }

    /// Returns the list after removing a value (fluent API).
    pub fn without(mut self, value: bool) -> Self {
        self.remove(value);
        self
    }
}

impl Default for BoolArrayList {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for BoolArrayList {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        self.items == other.items
    }
}

impl Eq for BoolArrayList {}

impl fmt::Display for BoolArrayList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl IntoIterator for BoolArrayList {
    type Item = bool;
    type IntoIter = std::vec::IntoIter<bool>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a BoolArrayList {
    type Item = &'a bool;
    type IntoIter = std::slice::Iter<'a, bool>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl std::ops::Index<usize> for BoolArrayList {
    type Output = bool;
    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_get() {
        let mut l = BoolArrayList::new();
        l.push(true);
        l.push(false);
        l.push(true);
        assert_eq!(l.len(), 3);
        assert_eq!(l.get(0), Some(true));
        assert_eq!(l.get(2), Some(true));
        assert_eq!(l.get(99), None);
    }

    #[test]
    fn test_of() {
        let l = BoolArrayList::of(&[true, false, true]);
        assert_eq!(l.len(), 3);
    }

    #[test]
    fn test_set() {
        let mut l = BoolArrayList::of(&[true, false]);
        let old = l.set(0, true);
        assert_eq!(old, true);
        assert_eq!(l.get(0), Some(true));
    }

    #[test]
    fn test_remove_at_index() {
        let mut l = BoolArrayList::of(&[true, false, true]);
        let removed = l.remove_at_index(1);
        assert_eq!(removed, false);
        assert_eq!(l.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut l = BoolArrayList::of(&[true, false, true]);
        assert!(l.remove(false));
        assert!(!l.contains(false));
        assert!(!l.remove(false));
    }

    #[test]
    fn test_contains() {
        let l = BoolArrayList::of(&[true, false]);
        assert!(l.contains(true));
    }

    #[test]
    fn test_is_empty_and_clear() {
        let mut l = BoolArrayList::new();
        assert!(l.is_empty());
        l.push(true);
        assert!(!l.is_empty());
        l.clear();
        assert!(l.is_empty());
    }

    #[test]
    fn test_select_reject() {
        let l = BoolArrayList::of(&[true, false, true]);
        let sel = l.select(|v| v);
        assert_eq!(sel.len(), 2);
        let rej = l.reject(|v| v);
        assert_eq!(rej.len(), 1);
    }

    #[test]
    fn test_detect() {
        let l = BoolArrayList::of(&[true, false]);
        assert_eq!(l.detect(|v| v == false), Some(false));
    }

    #[test]
    fn test_any_all_none_satisfy() {
        let l = BoolArrayList::of(&[true, false]);
        assert!(l.any_satisfy(|v| v == false));
        assert!(l.any_satisfy(|v| v == true));
        assert!(l.any_satisfy(|v| v == false));
    }

    #[test]
    fn test_min_max() {
        let l = BoolArrayList::of(&[true, false]);
        assert_eq!(l.min(), Some(false));
        assert_eq!(l.max(), Some(true));
    }

    #[test]
    fn test_sort() {
        let mut l = BoolArrayList::of(&[true, false, true]);
        l.sort();
        assert_eq!(l.get(0), Some(false));
    }

    #[test]
    fn test_reversed() {
        let l = BoolArrayList::of(&[true, false, true]);
        let r = l.reversed();
        assert_eq!(r.get(0), Some(true));
        assert_eq!(r.get(2), Some(true));
    }

    #[test]
    fn test_equals() {
        let l1 = BoolArrayList::of(&[true, false]);
        let l2 = BoolArrayList::of(&[true, false]);
        let l3 = BoolArrayList::of(&[true]);
        assert_eq!(l1, l2);
        assert_ne!(l1, l3);
    }

    #[test]
    fn test_display() {
        let l = BoolArrayList::of(&[true]);
        assert!(!l.to_string().is_empty());
    }

    #[test]
    fn test_index() {
        let l = BoolArrayList::of(&[true, false]);
        assert_eq!(l[0], true);
        assert_eq!(l[1], false);
    }

    #[test]
    fn test_into_iter() {
        let l = BoolArrayList::of(&[true, false, true]);
        let sum: i64 = l.into_iter().map(|v| v as i64).sum();
        assert!(sum > false as i64);
    }

    #[test]
    fn test_resize() {
        let mut l = BoolArrayList::new();
        for i in 0..100usize {
            l.push(i % 2 == 0);
        }
        assert_eq!(l.len(), 100);
    }

    #[test]
    fn test_try_reserve_grows_and_avoids_subsequent_realloc() {
        let mut l = BoolArrayList::new();
        l.try_reserve(100).unwrap();
        let reserved = l.items.capacity();
        assert!(reserved >= 100);
        l.push(true);
        l.push(false);
        l.push(true);
        assert_eq!(reserved, l.items.capacity());
    }

    #[test]
    fn test_try_reserve_exact_sets_minimum_capacity() {
        let mut l = BoolArrayList::new();
        l.try_reserve_exact(64).unwrap();
        assert!(l.items.capacity() >= 64);
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut l = BoolArrayList::new();
        let result = l.try_reserve(usize::MAX / 2);
        assert!(result.is_err());
    }
}

impl crate::traits::bool_collection::BoolCollection for BoolArrayList {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: bool) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter()
    }
}

impl crate::traits::bool_collection::BoolMutableCollection for BoolArrayList {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::bool_collection::BoolList for BoolArrayList {
    fn get(&self, index: usize) -> Option<bool> {
        self.get(index)
    }
    fn index_of(&self, value: bool) -> Option<usize> {
        self.index_of(value)
    }
}

impl crate::traits::bool_collection::BoolMutableList for BoolArrayList {
    fn push(&mut self, value: bool) {
        self.push(value)
    }
    fn set(&mut self, index: usize, value: bool) -> bool {
        self.set(index, value)
    }
}
