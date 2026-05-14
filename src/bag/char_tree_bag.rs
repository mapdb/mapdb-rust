// AUTO-GENERATED. DO NOT EDIT.

use std::collections::BTreeMap;
use std::fmt;

/// Sorted bag (multiset) of `char` values with occurrence counting.
/// Elements are kept in sorted order via BTreeMap.
#[derive(Debug, Clone)]
pub struct CharTreeBag {
    counts: BTreeMap<char, usize>,
    size: usize,
}

impl CharTreeBag {
    pub fn new() -> Self {
        CharTreeBag {
            counts: BTreeMap::new(),
            size: 0,
        }
    }

    pub fn of(values: &[char]) -> Self {
        let mut b = Self::new();
        for &v in values {
            b.add(v);
        }
        b
    }

    pub fn add(&mut self, value: char) {
        *self.counts.entry(value).or_insert(0) += 1;
        self.size += 1;
    }

    /// Removes one occurrence. Returns true if the value was present.
    pub fn remove(&mut self, value: char) -> bool {
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
    pub fn remove_all(&mut self, value: char) -> bool {
        let k = value;
        if let Some(count) = self.counts.remove(&k) {
            self.size -= count;
            true
        } else {
            false
        }
    }

    pub fn occurrences_of(&self, value: char) -> usize {
        self.counts.get(&(value)).copied().unwrap_or(0)
    }

    pub fn contains(&self, value: char) -> bool {
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
    pub fn min(&self) -> Option<char> {
        self.counts.keys().next().copied()
    }

    /// Returns the maximum element, or None.
    pub fn max(&self) -> Option<char> {
        self.counts.keys().next_back().copied()
    }

    /// Iterates over all elements in sorted order, repeating each value by its count.
    pub fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.counts
            .iter()
            .flat_map(|(&k, &c)| std::iter::repeat_n(k, c))
    }

    /// Iterates over distinct values in sorted order.
    pub fn iter_distinct(&self) -> impl Iterator<Item = char> + '_ {
        self.counts.keys().copied()
    }

    pub fn for_each_with_occurrences(&self, mut f: impl FnMut(char, usize)) {
        for (&k, &c) in &self.counts {
            f(k, c);
        }
    }

    pub fn select(&self, predicate: impl Fn(char) -> bool) -> Self {
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

    pub fn to_vec(&self) -> Vec<char> {
        let mut result = Vec::with_capacity(self.size);
        self.for_each_with_occurrences(|v, c| {
            for _ in 0..c {
                result.push(v);
            }
        });
        result
    }

    /// Returns elements as a sorted slice (each element repeated per count).
    pub fn to_sorted_vec(&self) -> Vec<char> {
        self.to_vec() // already sorted since BTreeMap iterates in order
    }
}

impl Default for CharTreeBag {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CharTreeBag {
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
        let mut b = CharTreeBag::new();
        b.add('a');
        b.add('a');
        b.add('b');
        assert_eq!(b.occurrences_of('a'), 2);
        assert_eq!(b.occurrences_of('b'), 1);
        assert_eq!(b.size(), 3);
        assert_eq!(b.size_distinct(), 2);
    }

    #[test]
    fn test_remove() {
        let mut b = CharTreeBag::of(&['a', 'a', 'a']);
        assert!(b.remove('a'));
        assert_eq!(b.occurrences_of('a'), 2);
    }

    #[test]
    fn test_remove_all() {
        let mut b = CharTreeBag::of(&['a', 'a', 'b']);
        assert!(b.remove_all('a'));
        assert!(!b.contains('a'));
        assert_eq!(b.size(), 1);
    }

    #[test]
    fn test_sorted_iteration() {
        let b = CharTreeBag::of(&['c', 'a', 'b']);
        let items = b.to_sorted_vec();
        assert_eq!(items[0], 'a');
        assert_eq!(items[items.len() - 1], 'c');
    }

    #[test]
    fn test_min_max() {
        let b = CharTreeBag::of(&['c', 'a', 'b']);
        assert_eq!(b.min(), Some('a'));
        assert_eq!(b.max(), Some('c'));
    }

    #[test]
    fn test_clear() {
        let mut b = CharTreeBag::of(&['a']);
        b.clear();
        assert!(b.is_empty());
    }

    #[test]
    fn test_display() {
        let b = CharTreeBag::of(&['a']);
        assert!(!b.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_is_noop_happy_path() {
        let mut b = CharTreeBag::new();
        b.try_reserve(100).unwrap();
        b.try_reserve(usize::MAX / 2).unwrap(); // no-op, never fails
    }
}

impl crate::traits::char_collection::CharCollection for CharTreeBag {
    fn len(&self) -> usize {
        self.size()
    }
    fn contains(&self, value: char) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.iter()
    }
}

impl crate::traits::char_collection::CharMutableCollection for CharTreeBag {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::char_collection::CharBag for CharTreeBag {
    fn occurrences_of(&self, value: char) -> usize {
        self.occurrences_of(value)
    }
    fn size_distinct(&self) -> usize {
        self.size_distinct()
    }
}

impl crate::traits::char_collection::CharMutableBag for CharTreeBag {
    fn add(&mut self, value: char) {
        self.add(value)
    }
}
