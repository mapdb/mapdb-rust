// AUTO-GENERATED. DO NOT EDIT.

use crate::hash_table::OpenHashSet;
use std::fmt;

/// Hash set of unique `i8` values.
/// Open-addressing with linear probing and Robin Hood backward-shift deletion.
#[derive(Debug, Clone)]
pub struct I8HashSet {
    inner: OpenHashSet<i8>,
}

impl I8HashSet {
    pub fn new() -> Self {
        I8HashSet {
            inner: OpenHashSet::new(),
        }
    }

    pub fn of(values: &[i8]) -> Self {
        let mut s = Self::new();
        for &v in values {
            s.add(v);
        }
        s
    }

    /// Adds a value. Returns true if it was not already present.
    pub fn add(&mut self, value: i8) -> bool {
        self.inner.add(value)
    }

    /// Removes a value. Returns true if it was present.
    pub fn remove(&mut self, value: i8) -> bool {
        self.inner.remove(value)
    }

    pub fn contains(&self, value: i8) -> bool {
        self.inner.contains(value)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Ensures the set can accept `additional` more entries without a
    /// rehash. Returns `TryReserveError` on allocator failure. See
    /// `docs/rust/error-handling.md`.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.inner.try_reserve(additional)
    }

    pub fn iter(&self) -> impl Iterator<Item = i8> + '_ {
        self.inner.iter()
    }

    pub fn for_each(&self, mut f: impl FnMut(i8)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn select(&self, predicate: impl Fn(i8) -> bool) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if predicate(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn reject(&self, predicate: impl Fn(i8) -> bool) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if !predicate(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn detect(&self, predicate: impl Fn(i8) -> bool) -> Option<i8> {
        self.iter().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(i8) -> bool) -> bool {
        self.iter().any(predicate)
    }

    pub fn all_satisfy(&self, predicate: impl Fn(i8) -> bool) -> bool {
        self.iter().all(predicate)
    }

    pub fn none_satisfy(&self, predicate: impl Fn(i8) -> bool) -> bool {
        !self.iter().any(predicate)
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

    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if !other.contains(v) {
                result.add(v);
            }
        }
        for v in other.iter() {
            if !self.contains(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn to_vec(&self) -> Vec<i8> {
        self.iter().collect()
    }

    pub fn with(mut self, value: i8) -> Self {
        self.add(value);
        self
    }
    pub fn without(mut self, value: i8) -> Self {
        self.remove(value);
        self
    }
}

impl Default for I8HashSet {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for I8HashSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter().all(|v| other.contains(v))
    }
}

impl Eq for I8HashSet {}

impl fmt::Display for I8HashSet {
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
        let mut s = I8HashSet::new();
        s.add(1);
        s.add(2);
        s.add(3);
        assert_eq!(s.len(), 3);
        assert!(s.contains(2));
        assert!(!s.contains(99));
    }

    #[test]
    fn test_add_duplicate() {
        let mut s = I8HashSet::new();
        assert!(s.add(1));
        assert!(!s.add(1));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut s = I8HashSet::of(&[1, 2, 3]);
        assert!(s.remove(2));
        assert!(!s.contains(2));
        assert!(!s.remove(99));
    }

    #[test]
    fn test_clear() {
        let mut s = I8HashSet::of(&[1]);
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_select_reject() {
        let s = I8HashSet::of(&[1, 2, 3, 4, 5]);
        assert_eq!(s.select(|v| v > 3).len(), 2);
        assert_eq!(s.reject(|v| v > 3).len(), 3);
    }

    #[test]
    fn test_union() {
        let a = I8HashSet::of(&[1, 2, 3]);
        let b = I8HashSet::of(&[3, 4, 5]);
        assert_eq!(a.union(&b).len(), 5);
    }

    #[test]
    fn test_intersect() {
        let a = I8HashSet::of(&[1, 2, 3]);
        let b = I8HashSet::of(&[2, 3, 4]);
        assert_eq!(a.intersect(&b).len(), 2);
    }

    #[test]
    fn test_difference() {
        let a = I8HashSet::of(&[1, 2, 3]);
        let b = I8HashSet::of(&[2, 3, 4]);
        assert_eq!(a.difference(&b).len(), 1);
    }

    #[test]
    fn test_equals() {
        let a = I8HashSet::of(&[1, 2]);
        let b = I8HashSet::of(&[2, 1]);
        assert_eq!(a, b);
    }

    #[test]
    fn test_display() {
        let s = I8HashSet::of(&[1]);
        assert!(!s.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut s = I8HashSet::new();
        s.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut s = I8HashSet::new();
        assert!(s.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::i8_collection::I8Collection for I8HashSet {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: i8) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = i8> + '_ {
        self.iter()
    }
}

impl crate::traits::i8_collection::I8MutableCollection for I8HashSet {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::i8_collection::I8Set for I8HashSet {}

impl crate::traits::i8_collection::I8MutableSet for I8HashSet {
    fn add(&mut self, value: i8) -> bool {
        self.add(value)
    }
}
