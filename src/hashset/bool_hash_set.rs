// AUTO-GENERATED. DO NOT EDIT.

use crate::hash_table::OpenHashSet;
use std::fmt;

/// Hash set of unique `bool` values.
/// Open-addressing with linear probing and Robin Hood backward-shift deletion.
#[derive(Debug, Clone)]
pub struct BoolHashSet {
    inner: OpenHashSet<bool>,
}

impl BoolHashSet {
    pub fn new() -> Self {
        BoolHashSet {
            inner: OpenHashSet::new(),
        }
    }

    pub fn of(values: &[bool]) -> Self {
        let mut s = Self::new();
        for &v in values {
            s.add(v);
        }
        s
    }

    /// Adds a value. Returns true if it was not already present.
    pub fn add(&mut self, value: bool) -> bool {
        self.inner.add(value)
    }

    /// Removes a value. Returns true if it was present.
    pub fn remove(&mut self, value: bool) -> bool {
        self.inner.remove(value)
    }

    pub fn contains(&self, value: bool) -> bool {
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

    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.iter()
    }

    pub fn for_each(&self, mut f: impl FnMut(bool)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn select(&self, predicate: impl Fn(bool) -> bool) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if predicate(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn reject(&self, predicate: impl Fn(bool) -> bool) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if !predicate(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn detect(&self, predicate: impl Fn(bool) -> bool) -> Option<bool> {
        self.iter().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(bool) -> bool) -> bool {
        self.iter().any(predicate)
    }

    pub fn all_satisfy(&self, predicate: impl Fn(bool) -> bool) -> bool {
        self.iter().all(predicate)
    }

    pub fn none_satisfy(&self, predicate: impl Fn(bool) -> bool) -> bool {
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

    pub fn to_vec(&self) -> Vec<bool> {
        self.iter().collect()
    }

    pub fn with(mut self, value: bool) -> Self {
        self.add(value);
        self
    }
    pub fn without(mut self, value: bool) -> Self {
        self.remove(value);
        self
    }
}

impl Default for BoolHashSet {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for BoolHashSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter().all(|v| other.contains(v))
    }
}

impl Eq for BoolHashSet {}

impl fmt::Display for BoolHashSet {
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
        let mut s = BoolHashSet::new();
        s.add(true);
        s.add(false);
        assert_eq!(s.len(), 2);
        assert!(s.contains(false));
    }

    #[test]
    fn test_add_duplicate() {
        let mut s = BoolHashSet::new();
        assert!(s.add(true));
        assert!(!s.add(true));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut s = BoolHashSet::of(&[true, false, true]);
        assert!(s.remove(false));
        assert!(!s.contains(false));
        assert!(!s.remove(false));
    }

    #[test]
    fn test_clear() {
        let mut s = BoolHashSet::of(&[true]);
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_select_reject() {
        let s = BoolHashSet::of(&[true, false]);
        assert_eq!(s.select(|v| v).len(), 1);
        assert_eq!(s.reject(|v| v).len(), 1);
    }

    #[test]
    fn test_union() {
        let a = BoolHashSet::of(&[true]);
        let b = BoolHashSet::of(&[false]);
        assert_eq!(a.union(&b).len(), 2);
    }

    #[test]
    fn test_intersect() {
        let a = BoolHashSet::of(&[true, false]);
        let b = BoolHashSet::of(&[false]);
        assert_eq!(a.intersect(&b).len(), 1);
    }

    #[test]
    fn test_difference() {
        let a = BoolHashSet::of(&[true, false]);
        let b = BoolHashSet::of(&[false]);
        assert_eq!(a.difference(&b).len(), 1);
    }

    #[test]
    fn test_equals() {
        let a = BoolHashSet::of(&[true, false]);
        let b = BoolHashSet::of(&[false, true]);
        assert_eq!(a, b);
    }

    #[test]
    fn test_display() {
        let s = BoolHashSet::of(&[true]);
        assert!(!s.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut s = BoolHashSet::new();
        s.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut s = BoolHashSet::new();
        assert!(s.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::bool_collection::BoolCollection for BoolHashSet {
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

impl crate::traits::bool_collection::BoolMutableCollection for BoolHashSet {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::bool_collection::BoolSet for BoolHashSet {}

impl crate::traits::bool_collection::BoolMutableSet for BoolHashSet {
    fn add(&mut self, value: bool) -> bool {
        self.add(value)
    }
}
