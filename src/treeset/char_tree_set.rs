// AUTO-GENERATED. DO NOT EDIT.

use std::collections::BTreeSet;
use std::fmt;

/// Sorted set of unique `char` values.
#[derive(Debug, Clone)]
pub struct CharTreeSet {
    inner: BTreeSet<char>,
}

impl CharTreeSet {
    pub fn new() -> Self {
        CharTreeSet {
            inner: BTreeSet::new(),
        }
    }

    pub fn of(values: &[char]) -> Self {
        let mut s = Self::new();
        for &v in values {
            s.add(v);
        }
        s
    }

    pub fn add(&mut self, value: char) -> bool {
        self.inner.insert(value)
    }

    pub fn remove(&mut self, value: char) -> bool {
        self.inner.remove(&value)
    }

    pub fn contains(&self, value: char) -> bool {
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

    pub fn min(&self) -> Option<char> {
        self.inner.iter().next().copied()
    }

    pub fn max(&self) -> Option<char> {
        self.inner.iter().next_back().copied()
    }

    /// Iterates in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.inner.iter().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(char)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn select(&self, predicate: impl Fn(char) -> bool) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if predicate(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn any_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
        self.iter().any(predicate)
    }

    pub fn all_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
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

    pub fn to_vec(&self) -> Vec<char> {
        self.iter().collect()
    }
    pub fn with(mut self, value: char) -> Self {
        self.add(value);
        self
    }
    pub fn without(mut self, value: char) -> Self {
        self.remove(value);
        self
    }
}

impl Default for CharTreeSet {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for CharTreeSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl fmt::Display for CharTreeSet {
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
        let mut s = CharTreeSet::new();
        s.add('a');
        s.add('b');
        s.add('c');
        assert_eq!(s.len(), 3);
        assert!(s.contains('b'));
        assert!(!s.contains('z'));
    }

    #[test]
    fn test_duplicate() {
        let mut s = CharTreeSet::new();
        assert!(s.add('a'));
        assert!(!s.add('a'));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_sorted_iteration() {
        let s = CharTreeSet::of(&['c', 'a', 'b']);
        let vals: Vec<_> = s.iter().collect();
        assert_eq!(vals, vec!['a', 'b', 'c']);
    }

    #[test]
    fn test_min_max() {
        let s = CharTreeSet::of(&['c', 'a', 'b']);
        assert_eq!(s.min(), Some('a'));
        assert_eq!(s.max(), Some('c'));
    }

    #[test]
    fn test_remove() {
        let mut s = CharTreeSet::of(&['a', 'b']);
        assert!(s.remove('a'));
        assert!(!s.contains('a'));
    }

    #[test]
    fn test_union() {
        let a = CharTreeSet::of(&['a', 'b']);
        let b = CharTreeSet::of(&['b', 'c']);
        assert_eq!(a.union(&b).len(), 3);
    }

    #[test]
    fn test_intersect() {
        let a = CharTreeSet::of(&['a', 'b']);
        let b = CharTreeSet::of(&['b', 'c']);
        assert_eq!(a.intersect(&b).len(), 1);
    }

    #[test]
    fn test_display() {
        let s = CharTreeSet::of(&['a']);
        assert!(!s.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut s = CharTreeSet::new();
        s.try_reserve(100).unwrap();
    }
}

impl crate::traits::char_collection::CharCollection for CharTreeSet {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: char) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.iter()
    }
}

impl crate::traits::char_collection::CharMutableCollection for CharTreeSet {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::char_collection::CharSet for CharTreeSet {}

impl crate::traits::char_collection::CharMutableSet for CharTreeSet {
    fn add(&mut self, value: char) -> bool {
        self.add(value)
    }
}
