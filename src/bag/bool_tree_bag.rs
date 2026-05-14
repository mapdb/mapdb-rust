// AUTO-GENERATED. DO NOT EDIT.

use std::collections::BTreeMap;
use std::fmt;

/// Sorted bag (multiset) of `bool` values with occurrence counting.
/// Elements are kept in sorted order via BTreeMap.
#[derive(Debug, Clone)]
pub struct BoolTreeBag {
    counts: BTreeMap<bool, usize>,
    size: usize,
}

impl BoolTreeBag {
    pub fn new() -> Self {
        BoolTreeBag {
            counts: BTreeMap::new(),
            size: 0,
        }
    }

    pub fn of(values: &[bool]) -> Self {
        let mut b = Self::new();
        for &v in values {
            b.add(v);
        }
        b
    }

    pub fn add(&mut self, value: bool) {
        *self.counts.entry(value).or_insert(0) += 1;
        self.size += 1;
    }

    /// Removes one occurrence. Returns true if the value was present.
    pub fn remove(&mut self, value: bool) -> bool {
        let k = value;
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
    pub fn remove_all(&mut self, value: bool) -> bool {
        let k = value;
        if let Some(count) = self.counts.remove(&k) {
            self.size -= count;
            true
        } else {
            false
        }
    }

    pub fn occurrences_of(&self, value: bool) -> usize {
        self.counts.get(&(value)).copied().unwrap_or(0)
    }

    pub fn contains(&self, value: bool) -> bool {
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
    pub fn min(&self) -> Option<bool> {
        self.counts.keys().next().copied()
    }

    /// Returns the maximum element, or None.
    pub fn max(&self) -> Option<bool> {
        self.counts.keys().next_back().copied()
    }

    /// Iterates over all elements in sorted order, repeating each value by its count.
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.counts
            .iter()
            .flat_map(|(&k, &c)| std::iter::repeat_n(k, c))
    }

    /// Iterates over distinct values in sorted order.
    pub fn iter_distinct(&self) -> impl Iterator<Item = bool> + '_ {
        self.counts.keys().copied()
    }

    pub fn for_each_with_occurrences(&self, mut f: impl FnMut(bool, usize)) {
        for (&k, &c) in &self.counts {
            f(k, c);
        }
    }

    pub fn select(&self, predicate: impl Fn(bool) -> bool) -> Self {
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

    pub fn to_vec(&self) -> Vec<bool> {
        let mut result = Vec::with_capacity(self.size);
        self.for_each_with_occurrences(|v, c| {
            for _ in 0..c {
                result.push(v);
            }
        });
        result
    }

    /// Returns elements as a sorted slice (each element repeated per count).
    pub fn to_sorted_vec(&self) -> Vec<bool> {
        self.to_vec() // already sorted since BTreeMap iterates in order
    }
}

impl Default for BoolTreeBag {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BoolTreeBag {
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
        let mut b = BoolTreeBag::new();
        b.add(true);
        b.add(true);
        b.add(false);
        assert_eq!(b.occurrences_of(true), 2);
        assert_eq!(b.occurrences_of(false), 1);
        assert_eq!(b.size(), 3);
        assert_eq!(b.size_distinct(), 2);
    }

    #[test]
    fn test_remove() {
        let mut b = BoolTreeBag::of(&[true, true, true]);
        assert!(b.remove(true));
        assert_eq!(b.occurrences_of(true), 2);
    }

    #[test]
    fn test_remove_all() {
        let mut b = BoolTreeBag::of(&[true, true, false]);
        assert!(b.remove_all(true));
        assert!(!b.contains(true));
        assert_eq!(b.size(), 1);
    }

    #[test]
    fn test_min_max() {
        let b = BoolTreeBag::of(&[true, false]);
        assert!(b.min().is_some());
        assert!(b.max().is_some());
    }

    #[test]
    fn test_clear() {
        let mut b = BoolTreeBag::of(&[true]);
        b.clear();
        assert!(b.is_empty());
    }

    #[test]
    fn test_display() {
        let b = BoolTreeBag::of(&[true]);
        assert!(!b.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_is_noop_happy_path() {
        let mut b = BoolTreeBag::new();
        b.try_reserve(100).unwrap();
        b.try_reserve(usize::MAX / 2).unwrap(); // no-op, never fails
    }
}

impl crate::traits::bool_collection::BoolCollection for BoolTreeBag {
    fn len(&self) -> usize {
        self.size()
    }
    fn contains(&self, value: bool) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter()
    }
}

impl crate::traits::bool_collection::BoolMutableCollection for BoolTreeBag {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::bool_collection::BoolBag for BoolTreeBag {
    fn occurrences_of(&self, value: bool) -> usize {
        self.occurrences_of(value)
    }
    fn size_distinct(&self) -> usize {
        self.size_distinct()
    }
}

impl crate::traits::bool_collection::BoolMutableBag for BoolTreeBag {
    fn add(&mut self, value: bool) {
        self.add(value)
    }
}
