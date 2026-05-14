// AUTO-GENERATED. DO NOT EDIT.

use std::collections::BTreeMap;
use std::fmt;

/// Sorted bag (multiset) of `f64` values with occurrence counting.
/// Elements are kept in sorted order via BTreeMap.
#[derive(Debug, Clone)]
pub struct F64TreeBag {
    counts: BTreeMap<crate::hashable_float::HashableF64, usize>,
    size: usize,
}

impl F64TreeBag {
    pub fn new() -> Self {
        F64TreeBag {
            counts: BTreeMap::new(),
            size: 0,
        }
    }

    pub fn of(values: &[f64]) -> Self {
        let mut b = Self::new();
        for &v in values {
            b.add(v);
        }
        b
    }

    pub fn add(&mut self, value: f64) {
        *self.counts.entry(value.into()).or_insert(0) += 1;
        self.size += 1;
    }

    /// Removes one occurrence. Returns true if the value was present.
    pub fn remove(&mut self, value: f64) -> bool {
        let k = value.into();
        if let Some(count) = self.counts.get_mut(&k) {
            *count -= 1;
            if *count == 0 {
                self.counts.remove(&k);
            }
            self.size -= 1;
            true
        } else {
            false
        }
    }

    /// Removes all occurrences. Returns true if present.
    pub fn remove_all(&mut self, value: f64) -> bool {
        let k = value.into();
        if let Some(count) = self.counts.remove(&k) {
            self.size -= count;
            true
        } else {
            false
        }
    }

    pub fn occurrences_of(&self, value: f64) -> usize {
        self.counts.get(&(value.into())).copied().unwrap_or(0)
    }

    pub fn contains(&self, value: f64) -> bool {
        self.occurrences_of(value) > 0
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn size_distinct(&self) -> usize {
        self.counts.len()
    }
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn clear(&mut self) {
        self.counts.clear();
        self.size = 0;
    }

    /// No-op reservation hook. `BTreeMap` grows node-by-node and does
    /// not expose a fallible pre-reservation API; this method exists for
    /// API uniformity with the hash-backed bags and always returns
    /// `Ok(())`. Callers who need opt-in OOM handling should prefer the
    /// hash-backed `Bag` primitives. See `docs/rust/error-handling.md`.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        let _ = additional;
        Ok(())
    }

    /// Returns the minimum element, or None.
    pub fn min(&self) -> Option<f64> {
        self.counts.keys().next().map(|k| (*k).0)
    }

    /// Returns the maximum element, or None.
    pub fn max(&self) -> Option<f64> {
        self.counts.keys().next_back().map(|k| (*k).0)
    }

    /// Iterates over all elements in sorted order, repeating each value by its count.
    pub fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.counts
            .iter()
            .flat_map(|(&k, &c)| std::iter::repeat_n(k.0, c))
    }

    /// Iterates over distinct values in sorted order.
    pub fn iter_distinct(&self) -> impl Iterator<Item = f64> + '_ {
        self.counts.keys().map(|k| (*k).0)
    }

    pub fn for_each_with_occurrences(&self, mut f: impl FnMut(f64, usize)) {
        for (&k, &c) in &self.counts {
            f(k.0, c);
        }
    }

    pub fn select(&self, predicate: impl Fn(f64) -> bool) -> Self {
        let mut result = Self::new();
        self.for_each_with_occurrences(|v, c| {
            if predicate(v) {
                for _ in 0..c {
                    result.add(v);
                }
            }
        });
        result
    }

    pub fn to_vec(&self) -> Vec<f64> {
        let mut result = Vec::with_capacity(self.size);
        self.for_each_with_occurrences(|v, c| {
            for _ in 0..c {
                result.push(v);
            }
        });
        result
    }

    /// Returns elements as a sorted slice (each element repeated per count).
    pub fn to_sorted_vec(&self) -> Vec<f64> {
        self.to_vec() // already sorted since BTreeMap iterates in order
    }
}

impl Default for F64TreeBag {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for F64TreeBag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        self.for_each_with_occurrences(|v, c| {
            if !first {
                let _ = write!(f, ", ");
            }
            let _ = write!(f, "{}x{}", v, c);
            first = false;
        });
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_occurrences() {
        let mut b = F64TreeBag::new();
        b.add(1.0f64);
        b.add(1.0f64);
        b.add(2.0f64);
        assert_eq!(b.occurrences_of(1.0f64), 2);
        assert_eq!(b.occurrences_of(2.0f64), 1);
        assert_eq!(b.size(), 3);
        assert_eq!(b.size_distinct(), 2);
    }

    #[test]
    fn test_remove() {
        let mut b = F64TreeBag::of(&[1.0f64, 1.0f64, 1.0f64]);
        assert!(b.remove(1.0f64));
        assert_eq!(b.occurrences_of(1.0f64), 2);
    }

    #[test]
    fn test_remove_all() {
        let mut b = F64TreeBag::of(&[1.0f64, 1.0f64, 2.0f64]);
        assert!(b.remove_all(1.0f64));
        assert!(!b.contains(1.0f64));
        assert_eq!(b.size(), 1);
    }

    #[test]
    fn test_sorted_iteration() {
        let b = F64TreeBag::of(&[3.0f64, 1.0f64, 2.0f64]);
        let items = b.to_sorted_vec();
        assert_eq!(items[0], 1.0f64);
        assert_eq!(items[items.len() - 1], 3.0f64);
    }

    #[test]
    fn test_min_max() {
        let b = F64TreeBag::of(&[3.0f64, 1.0f64, 2.0f64]);
        assert_eq!(b.min(), Some(1.0f64));
        assert_eq!(b.max(), Some(3.0f64));
    }

    #[test]
    fn test_clear() {
        let mut b = F64TreeBag::of(&[1.0f64]);
        b.clear();
        assert!(b.is_empty());
    }

    #[test]
    fn test_display() {
        let b = F64TreeBag::of(&[1.0f64]);
        assert!(!b.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_is_noop_happy_path() {
        let mut b = F64TreeBag::new();
        b.try_reserve(100).unwrap();
        b.try_reserve(usize::MAX / 2).unwrap(); // no-op, never fails
    }
}

impl crate::traits::f64_collection::F64Collection for F64TreeBag {
    fn len(&self) -> usize {
        self.size()
    }
    fn contains(&self, value: f64) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.iter()
    }
}

impl crate::traits::f64_collection::F64MutableCollection for F64TreeBag {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::f64_collection::F64Bag for F64TreeBag {
    fn occurrences_of(&self, value: f64) -> usize {
        self.occurrences_of(value)
    }
    fn size_distinct(&self) -> usize {
        self.size_distinct()
    }
}

impl crate::traits::f64_collection::F64MutableBag for F64TreeBag {
    fn add(&mut self, value: f64) {
        self.add(value)
    }
}
