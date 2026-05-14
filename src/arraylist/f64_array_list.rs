// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Resizable array-backed list of `f64` values.
///
/// Specialized for `f64` — no boxing, contiguous memory layout.
#[derive(Debug, Clone)]
pub struct F64ArrayList {
    items: Vec<f64>,
}

impl F64ArrayList {
    /// Creates a new empty list.
    pub fn new() -> Self {
        F64ArrayList { items: Vec::new() }
    }

    /// Creates a new empty list with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        F64ArrayList {
            items: Vec::with_capacity(capacity),
        }
    }

    /// Creates a list from a slice of values.
    pub fn of(values: &[f64]) -> Self {
        F64ArrayList {
            items: values.to_vec(),
        }
    }

    /// Appends a value to the end of the list.
    pub fn push(&mut self, value: f64) {
        self.items.push(value);
    }

    /// Appends all values from a slice.
    pub fn push_all(&mut self, values: &[f64]) {
        self.items.extend_from_slice(values);
    }

    /// Returns the element at the given index, or None if out of bounds.
    pub fn get(&self, index: usize) -> Option<f64> {
        self.items.get(index).copied()
    }

    /// Sets the element at the given index. Returns the old value.
    ///
    /// # Panics
    /// Panics if index is out of bounds.
    pub fn set(&mut self, index: usize, value: f64) -> f64 {
        let old = self.items[index];
        self.items[index] = value;
        old
    }

    /// Removes and returns the element at the given index.
    ///
    /// # Panics
    /// Panics if index is out of bounds.
    pub fn remove_at_index(&mut self, index: usize) -> f64 {
        self.items.remove(index)
    }

    /// Removes the first occurrence of the value. Returns true if found.
    pub fn remove(&mut self, value: f64) -> bool {
        if let Some(idx) = self.index_of(value) {
            self.items.remove(idx);
            true
        } else {
            false
        }
    }

    /// Returns true if the list contains the given value.
    pub fn contains(&self, value: f64) -> bool {
        self.items.iter().any(|&v| v.to_bits() == value.to_bits())
    }

    /// Returns the index of the first occurrence, or None if not found.
    pub fn index_of(&self, value: f64) -> Option<usize> {
        self.items
            .iter()
            .position(|&v| v.to_bits() == value.to_bits())
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
    pub fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.items.iter().copied()
    }

    /// Calls the given function for each element.
    pub fn for_each(&self, mut f: impl FnMut(f64)) {
        for &v in &self.items {
            f(v);
        }
    }

    /// Returns a new list with only elements satisfying the predicate.
    pub fn select(&self, predicate: impl Fn(f64) -> bool) -> Self {
        F64ArrayList {
            items: self
                .items
                .iter()
                .copied()
                .filter(|&v| predicate(v))
                .collect(),
        }
    }

    /// Returns a new list with elements NOT satisfying the predicate.
    pub fn reject(&self, predicate: impl Fn(f64) -> bool) -> Self {
        F64ArrayList {
            items: self
                .items
                .iter()
                .copied()
                .filter(|&v| !predicate(v))
                .collect(),
        }
    }

    /// Returns the first element satisfying the predicate, or None.
    pub fn detect(&self, predicate: impl Fn(f64) -> bool) -> Option<f64> {
        self.items.iter().copied().find(|&v| predicate(v))
    }

    /// Returns true if any element satisfies the predicate.
    pub fn any_satisfy(&self, predicate: impl Fn(f64) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    /// Returns true if all elements satisfy the predicate.
    pub fn all_satisfy(&self, predicate: impl Fn(f64) -> bool) -> bool {
        self.items.iter().all(|&v| predicate(v))
    }

    /// Returns true if no element satisfies the predicate.
    pub fn none_satisfy(&self, predicate: impl Fn(f64) -> bool) -> bool {
        !self.items.iter().any(|&v| predicate(v))
    }

    /// Returns the count of elements satisfying the predicate.
    pub fn count(&self, predicate: impl Fn(f64) -> bool) -> usize {
        self.items.iter().filter(|&&v| predicate(v)).count()
    }

    /// Returns the sum of all elements.
    pub fn sum(&self) -> f64 {
        self.items.iter().copied().sum()
    }

    /// Returns the minimum element, or None if empty.
    pub fn min(&self) -> Option<f64> {
        self.items.iter().copied().min_by(|a, b| a.total_cmp(&b))
    }

    /// Returns the maximum element, or None if empty.
    pub fn max(&self) -> Option<f64> {
        self.items.iter().copied().max_by(|a, b| a.total_cmp(&b))
    }

    /// Sorts the list in ascending order.
    pub fn sort(&mut self) {
        self.items.sort_by(|a, b| a.total_cmp(&b));
    }

    /// Returns a new list with elements in reverse order.
    pub fn reversed(&self) -> Self {
        let mut items = self.items.clone();
        items.reverse();
        F64ArrayList { items }
    }

    /// Returns a new list with duplicate elements removed (preserving first occurrence order).
    pub fn distinct(&self) -> Self {
        let mut seen = std::collections::HashSet::new();
        let items: Vec<f64> = self
            .items
            .iter()
            .copied()
            .filter(|v| {
                let bits = v.to_bits();
                seen.insert(bits)
            })
            .collect();
        F64ArrayList { items }
    }

    /// Returns the elements as a Vec.
    pub fn to_vec(&self) -> Vec<f64> {
        self.items.clone()
    }

    /// Folds all elements using the given function and initial value.
    pub fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, f64) -> R) -> R {
        let mut acc = initial;
        for &v in &self.items {
            acc = f(acc, v);
        }
        acc
    }

    /// Returns the list after pushing a value (fluent API).
    pub fn with(mut self, value: f64) -> Self {
        self.push(value);
        self
    }

    /// Returns the list after removing a value (fluent API).
    pub fn without(mut self, value: f64) -> Self {
        self.remove(value);
        self
    }
}

impl Default for F64ArrayList {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for F64ArrayList {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        self.items
            .iter()
            .zip(other.items.iter())
            .all(|(a, b)| a.to_bits() == b.to_bits())
    }
}

impl fmt::Display for F64ArrayList {
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

impl IntoIterator for F64ArrayList {
    type Item = f64;
    type IntoIter = std::vec::IntoIter<f64>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a F64ArrayList {
    type Item = &'a f64;
    type IntoIter = std::slice::Iter<'a, f64>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl std::ops::Index<usize> for F64ArrayList {
    type Output = f64;
    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_get() {
        let mut l = F64ArrayList::new();
        l.push(1.0f64);
        l.push(2.0f64);
        l.push(3.0f64);
        assert_eq!(l.len(), 3);
        assert_eq!(l.get(0), Some(1.0f64));
        assert_eq!(l.get(2), Some(3.0f64));
        assert_eq!(l.get(99), None);
    }

    #[test]
    fn test_of() {
        let l = F64ArrayList::of(&[1.0f64, 2.0f64, 3.0f64]);
        assert_eq!(l.len(), 3);
    }

    #[test]
    fn test_set() {
        let mut l = F64ArrayList::of(&[1.0f64, 2.0f64]);
        let old = l.set(0, 3.0f64);
        assert_eq!(old, 1.0f64);
        assert_eq!(l.get(0), Some(3.0f64));
    }

    #[test]
    fn test_remove_at_index() {
        let mut l = F64ArrayList::of(&[1.0f64, 2.0f64, 3.0f64]);
        let removed = l.remove_at_index(1);
        assert_eq!(removed, 2.0f64);
        assert_eq!(l.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut l = F64ArrayList::of(&[1.0f64, 2.0f64, 3.0f64]);
        assert!(l.remove(2.0f64));
        assert!(!l.contains(2.0f64));
        assert!(!l.remove(99.0f64));
    }

    #[test]
    fn test_contains() {
        let l = F64ArrayList::of(&[1.0f64, 2.0f64]);
        assert!(l.contains(1.0f64));
        assert!(!l.contains(99.0f64));
    }

    #[test]
    fn test_is_empty_and_clear() {
        let mut l = F64ArrayList::new();
        assert!(l.is_empty());
        l.push(1.0f64);
        assert!(!l.is_empty());
        l.clear();
        assert!(l.is_empty());
    }

    #[test]
    fn test_select_reject() {
        let l = F64ArrayList::of(&[1.0f64, 2.0f64, 3.0f64, 4.0f64, 5.0f64]);
        let sel = l.select(|v| v > 3.0f64);
        assert_eq!(sel.len(), 2);
        let rej = l.reject(|v| v > 3.0f64);
        assert_eq!(rej.len(), 3);
    }

    #[test]
    fn test_detect() {
        let l = F64ArrayList::of(&[1.0f64, 2.0f64]);
        assert_eq!(l.detect(|v| v == 2.0f64), Some(2.0f64));
        assert_eq!(l.detect(|v| v == 99.0f64), None);
    }

    #[test]
    fn test_any_all_none_satisfy() {
        let l = F64ArrayList::of(&[1.0f64, 2.0f64, 3.0f64]);
        assert!(l.any_satisfy(|v| v == 2.0f64));
        assert!(!l.any_satisfy(|v| v == 99.0f64));
        assert!(l.all_satisfy(|v| v > 0.0));
        assert!(l.none_satisfy(|v| v == 99.0f64));
    }

    #[test]
    fn test_sum_min_max() {
        let l = F64ArrayList::of(&[3.0f64, 1.0f64, 2.0f64]);
        assert_eq!(l.min(), Some(1.0f64));
        assert_eq!(l.max(), Some(3.0f64));
    }

    #[test]
    fn test_sort() {
        let mut l = F64ArrayList::of(&[3.0f64, 1.0f64, 2.0f64]);
        l.sort();
        assert_eq!(l.to_vec(), vec![1.0f64, 2.0f64, 3.0f64]);
    }

    #[test]
    fn test_reversed() {
        let l = F64ArrayList::of(&[1.0f64, 2.0f64, 3.0f64]);
        let r = l.reversed();
        assert_eq!(r.get(0), Some(3.0f64));
        assert_eq!(r.get(2), Some(1.0f64));
    }

    #[test]
    fn test_equals() {
        let l1 = F64ArrayList::of(&[1.0f64, 2.0f64]);
        let l2 = F64ArrayList::of(&[1.0f64, 2.0f64]);
        let l3 = F64ArrayList::of(&[1.0f64]);
        assert_eq!(l1, l2);
        assert_ne!(l1, l3);
    }

    #[test]
    fn test_display() {
        let l = F64ArrayList::of(&[1.0f64]);
        assert!(!l.to_string().is_empty());
    }

    #[test]
    fn test_index() {
        let l = F64ArrayList::of(&[1.0f64, 2.0f64]);
        assert_eq!(l[0], 1.0f64);
        assert_eq!(l[1], 2.0f64);
    }

    #[test]
    fn test_into_iter() {
        let l = F64ArrayList::of(&[1.0f64, 2.0f64, 3.0f64]);
        let sum: f64 = l.into_iter().map(|v| v).sum();
        assert!(sum > 0.0);
    }

    #[test]
    fn test_resize() {
        let mut l = F64ArrayList::new();
        for i in 0..100 {
            l.push(i as f64);
        }
        assert_eq!(l.len(), 100);
    }

    #[test]
    fn test_try_reserve_grows_and_avoids_subsequent_realloc() {
        let mut l = F64ArrayList::new();
        l.try_reserve(100).unwrap();
        let reserved = l.items.capacity();
        assert!(reserved >= 100);
        l.push(1.0f64);
        l.push(2.0f64);
        l.push(3.0f64);
        assert_eq!(reserved, l.items.capacity());
    }

    #[test]
    fn test_try_reserve_exact_sets_minimum_capacity() {
        let mut l = F64ArrayList::new();
        l.try_reserve_exact(64).unwrap();
        assert!(l.items.capacity() >= 64);
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut l = F64ArrayList::new();
        let result = l.try_reserve(usize::MAX / 2);
        assert!(result.is_err());
    }
}

impl crate::traits::f64_collection::F64Collection for F64ArrayList {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: f64) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.iter()
    }
}

impl crate::traits::f64_collection::F64MutableCollection for F64ArrayList {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::f64_collection::F64List for F64ArrayList {
    fn get(&self, index: usize) -> Option<f64> {
        self.get(index)
    }
    fn index_of(&self, value: f64) -> Option<usize> {
        self.index_of(value)
    }
}

impl crate::traits::f64_collection::F64MutableList for F64ArrayList {
    fn push(&mut self, value: f64) {
        self.push(value)
    }
    fn set(&mut self, index: usize, value: f64) -> f64 {
        self.set(index, value)
    }
}
