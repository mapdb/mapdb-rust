// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Sorted set of unique `f32` values.
#[derive(Debug, Clone)]
pub struct F32TreeSet {
    items: Vec<f32>,
}

impl F32TreeSet {
    pub fn new() -> Self {
        F32TreeSet { items: Vec::new() }
    }

    pub fn of(values: &[f32]) -> Self {
        let mut s = Self::new();
        for &v in values {
            s.add(v);
        }
        s
    }

    pub fn add(&mut self, value: f32) -> bool {
        if self.contains(value) {
            return false;
        }
        let pos = self.items.partition_point(|v| v.total_cmp(&&value).is_lt());
        self.items.insert(pos, value);
        true
    }

    pub fn remove(&mut self, value: f32) -> bool {
        if let Some(pos) = self
            .items
            .iter()
            .position(|&v| v.to_bits() == value.to_bits())
        {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn contains(&self, value: f32) -> bool {
        self.items.iter().any(|&v| v.to_bits() == value.to_bits())
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn clear(&mut self) {
        self.items.clear();
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
        self.items.try_reserve(additional)
    }

    pub fn min(&self) -> Option<f32> {
        self.items.first().copied()
    }

    pub fn max(&self) -> Option<f32> {
        self.items.last().copied()
    }

    /// Iterates in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        self.items.iter().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(f32)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn select(&self, predicate: impl Fn(f32) -> bool) -> Self {
        let mut result = Self::new();
        for v in self.iter() {
            if predicate(v) {
                result.add(v);
            }
        }
        result
    }

    pub fn any_satisfy(&self, predicate: impl Fn(f32) -> bool) -> bool {
        self.iter().any(predicate)
    }

    pub fn all_satisfy(&self, predicate: impl Fn(f32) -> bool) -> bool {
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

    pub fn to_vec(&self) -> Vec<f32> {
        self.iter().collect()
    }
    pub fn with(mut self, value: f32) -> Self {
        self.add(value);
        self
    }
    pub fn without(mut self, value: f32) -> Self {
        self.remove(value);
        self
    }
}

impl Default for F32TreeSet {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for F32TreeSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .zip(other.iter())
            .all(|(a, b)| a.to_bits() == b.to_bits())
    }
}

impl fmt::Display for F32TreeSet {
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
        let mut s = F32TreeSet::new();
        s.add(1.0f32);
        s.add(2.0f32);
        s.add(3.0f32);
        assert_eq!(s.len(), 3);
        assert!(s.contains(2.0f32));
        assert!(!s.contains(99.0f32));
    }

    #[test]
    fn test_duplicate() {
        let mut s = F32TreeSet::new();
        assert!(s.add(1.0f32));
        assert!(!s.add(1.0f32));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_sorted_iteration() {
        let s = F32TreeSet::of(&[3.0f32, 1.0f32, 2.0f32]);
        let vals: Vec<_> = s.iter().collect();
        assert_eq!(vals, vec![1.0f32, 2.0f32, 3.0f32]);
    }

    #[test]
    fn test_min_max() {
        let s = F32TreeSet::of(&[3.0f32, 1.0f32, 2.0f32]);
        assert_eq!(s.min(), Some(1.0f32));
        assert_eq!(s.max(), Some(3.0f32));
    }

    #[test]
    fn test_remove() {
        let mut s = F32TreeSet::of(&[1.0f32, 2.0f32]);
        assert!(s.remove(1.0f32));
        assert!(!s.contains(1.0f32));
    }

    #[test]
    fn test_union() {
        let a = F32TreeSet::of(&[1.0f32, 2.0f32]);
        let b = F32TreeSet::of(&[2.0f32, 3.0f32]);
        assert_eq!(a.union(&b).len(), 3);
    }

    #[test]
    fn test_intersect() {
        let a = F32TreeSet::of(&[1.0f32, 2.0f32]);
        let b = F32TreeSet::of(&[2.0f32, 3.0f32]);
        assert_eq!(a.intersect(&b).len(), 1);
    }

    #[test]
    fn test_display() {
        let s = F32TreeSet::of(&[1.0f32]);
        assert!(!s.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut s = F32TreeSet::new();
        s.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut s = F32TreeSet::new();
        assert!(s.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::f32_collection::F32Collection for F32TreeSet {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: f32) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        self.iter()
    }
}

impl crate::traits::f32_collection::F32MutableCollection for F32TreeSet {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::f32_collection::F32Set for F32TreeSet {}

impl crate::traits::f32_collection::F32MutableSet for F32TreeSet {
    fn add(&mut self, value: f32) -> bool {
        self.add(value)
    }
}
