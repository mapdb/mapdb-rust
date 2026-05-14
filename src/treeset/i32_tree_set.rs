// AUTO-GENERATED. DO NOT EDIT.

use std::collections::BTreeSet;
use std::fmt;

/// Sorted set of unique `i32` values.
#[derive(Debug, Clone)]
pub struct I32TreeSet {
    inner: BTreeSet<i32>,
}

impl I32TreeSet {
    pub fn new() -> Self {
        I32TreeSet {
            inner: BTreeSet::new(),
        }
    }

    pub fn of(values: &[i32]) -> Self {
        let mut s = Self::new();
        for &v in values {
            s.add(v);
        }
        s
    }

    pub fn add(&mut self, value: i32) -> bool {
        self.inner.insert(value)
    }

    pub fn remove(&mut self, value: i32) -> bool {
        self.inner.remove(&value)
    }

    pub fn contains(&self, value: i32) -> bool {
        self.inner.contains(&value)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Ensures that `additional` more entries can be added without a
    /// reallocation on the backing store. Returns `TryReserveError` on
    /// allocator failure. For `BTreeSet`-backed variants this is a no-op
    /// — see the `try_reserve` doc on the matching TreeMap and
    /// `docs/rust/error-handling.md`.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        let _ = additional;
        Ok(())
    }

    pub fn min(&self) -> Option<i32> {
        self.inner.iter().next().copied()
    }

    pub fn max(&self) -> Option<i32> {
        self.inner.iter().next_back().copied()
    }

    /// Iterates in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        self.inner.iter().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(i32)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn select(&self, predicate: impl Fn(i32) -> bool) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if predicate(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn any_satisfy(&self, predicate: impl Fn(i32) -> bool) -> bool {
        self.iter().any(predicate)
    }

    pub fn all_satisfy(&self, predicate: impl Fn(i32) -> bool) -> bool {
        self.iter().all(predicate)
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for v in other.iter() {
            result.add(v);
        }
        result
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if other.contains(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn difference(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if !other.contains(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn to_vec(&self) -> Vec<i32> {
        self.iter().collect()
    }
    pub fn with(mut self, value: i32) -> Self {
        self.add(value);
        self
    }
    pub fn without(mut self, value: i32) -> Self {
        self.remove(value);
        self
    }
}

impl Default for I32TreeSet {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for I32TreeSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl fmt::Display for I32TreeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_contains() {
        let mut s = I32TreeSet::new();
        s.add(1);
        s.add(2);
        s.add(3);
        assert_eq!(s.len(), 3);
        assert!(s.contains(2));
        assert!(!s.contains(99));
    }

    #[test]
    fn test_duplicate() {
        let mut s = I32TreeSet::new();
        assert!(s.add(1));
        assert!(!s.add(1));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_sorted_iteration() {
        let s = I32TreeSet::of(&[3, 1, 2]);
        let vals: Vec<_> = s.iter().collect();
        assert_eq!(vals, vec![1, 2, 3]);
    }

    #[test]
    fn test_min_max() {
        let s = I32TreeSet::of(&[3, 1, 2]);
        assert_eq!(s.min(), Some(1));
        assert_eq!(s.max(), Some(3));
    }

    #[test]
    fn test_remove() {
        let mut s = I32TreeSet::of(&[1, 2]);
        assert!(s.remove(1));
        assert!(!s.contains(1));
    }

    #[test]
    fn test_union() {
        let a = I32TreeSet::of(&[1, 2]);
        let b = I32TreeSet::of(&[2, 3]);
        assert_eq!(a.union(&b).len(), 3);
    }

    #[test]
    fn test_intersect() {
        let a = I32TreeSet::of(&[1, 2]);
        let b = I32TreeSet::of(&[2, 3]);
        assert_eq!(a.intersect(&b).len(), 1);
    }

    #[test]
    fn test_display() {
        let s = I32TreeSet::of(&[1]);
        assert!(!s.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut s = I32TreeSet::new();
        s.try_reserve(100).unwrap();
    }
}

impl crate::traits::i32_collection::I32Collection for I32TreeSet {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: i32) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        self.iter()
    }
}

impl crate::traits::i32_collection::I32MutableCollection for I32TreeSet {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::i32_collection::I32Set for I32TreeSet {}

impl crate::traits::i32_collection::I32MutableSet for I32TreeSet {
    fn add(&mut self, value: i32) -> bool {
        self.add(value)
    }
}
