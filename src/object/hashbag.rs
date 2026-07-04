// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use super::traits::*;
use crate::bulk::BulkError;
use std::collections::HashMap;
use std::hash::Hash;

/// Generic multiset (bag) backed by `HashMap<T, usize>`.
#[derive(Debug, Clone)]
pub struct HashBag<T: Eq + Hash> {
    counts: HashMap<T, usize>,
    size: usize,
}

impl<T: Eq + Hash> HashBag<T> {
    pub fn new() -> Self {
        HashBag {
            counts: HashMap::new(),
            size: 0,
        }
    }

    /// Bulk-loads a fresh bag from a flat sequence of elements, pre-sizing the
    /// backing `std::HashMap` to the source's size hint. Equal elements
    /// increment the occurrence count (the bag's natural duplicate handling —
    /// no [`DuplicatePolicy`](crate::DuplicatePolicy) is consulted). Count
    /// addition is overflow-checked ([`BulkError::CountOverflow`]).
    ///
    /// Note: the backing is `std::HashMap`, so this uses `with_capacity` (a
    /// hint) and does **not** claim the zero-rehash contract.
    pub fn bulk_load<I: IntoIterator<Item = T>>(iter: I) -> Result<Self, BulkError> {
        let iter = iter.into_iter();
        let hint = iter.size_hint().0;
        let mut counts: HashMap<T, usize> = HashMap::with_capacity(hint);
        let mut size = 0usize;
        for (index, v) in iter.enumerate() {
            let c = counts.entry(v).or_insert(0);
            *c = c.checked_add(1).ok_or(BulkError::CountOverflow { index })?;
            size += 1;
        }
        Ok(HashBag { counts, size })
    }

    /// Bulk-loads a fresh bag from `(value, count)` pairs — strictly better
    /// than `n×insert` when the multiplicities are already known. Repeated
    /// values sum their counts (overflow-checked); a `count` of 0 is a no-op.
    pub fn bulk_load_counts<I: IntoIterator<Item = (T, usize)>>(
        iter: I,
    ) -> Result<Self, BulkError> {
        let iter = iter.into_iter();
        let hint = iter.size_hint().0;
        let mut counts: HashMap<T, usize> = HashMap::with_capacity(hint);
        let mut size = 0usize;
        for (index, (v, n)) in iter.enumerate() {
            if n == 0 {
                continue;
            }
            let c = counts.entry(v).or_insert(0);
            *c = c.checked_add(n).ok_or(BulkError::CountOverflow { index })?;
            size = size
                .checked_add(n)
                .ok_or(BulkError::CountOverflow { index })?;
        }
        Ok(HashBag { counts, size })
    }
}

impl<T: Eq + Hash> Collection<T> for HashBag<T> {
    fn len(&self) -> usize {
        self.size
    }
    fn contains(&self, value: &T) -> bool {
        self.counts.get(value).copied().unwrap_or(0) > 0
    }
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(
            self.counts
                .iter()
                .flat_map(|(v, &c)| std::iter::repeat_n(v, c)),
        )
    }
}

impl<T: Eq + Hash> MutableCollection<T> for HashBag<T> {
    fn clear(&mut self) {
        self.counts.clear();
        self.size = 0;
    }
}

impl<T: Eq + Hash> Bag<T> for HashBag<T> {
    fn occurrences_of(&self, value: &T) -> usize {
        self.counts.get(value).copied().unwrap_or(0)
    }
    fn distinct_len(&self) -> usize {
        self.counts.len()
    }
}

impl<T: Eq + Hash> MutableBag<T> for HashBag<T> {
    /// # Panics
    /// Panics if the per-value occurrence count or the total size would
    /// overflow `usize` (mirrors the overflow-checked `bulk_load_counts` path;
    /// Guava's `HashMultiset` throws in the same situation).
    fn insert(&mut self, value: T) {
        // Check `size` first (it is >= any per-value count, so it overflows
        // first): computing it before mutating `counts` keeps the bag
        // consistent even if the panic is caught — a bumped count with a stale
        // size can never be observed. Given `size` fit, `count + 1` cannot
        // overflow (count <= old size < new size <= usize::MAX).
        let new_size = self
            .size
            .checked_add(1)
            .expect("HashBag size overflowed usize");
        let c = self.counts.entry(value).or_insert(0);
        *c = c
            .checked_add(1)
            .expect("HashBag occurrence count overflowed usize");
        self.size = new_size;
    }
}

impl<T: Eq + Hash> HashBag<T> {
    /// Adds `n` occurrences of `value`.
    ///
    /// # Panics
    /// Panics if the per-value occurrence count or the total size would
    /// overflow `usize` (mirrors `bulk_load_counts`).
    pub fn add_occurrences(&mut self, value: T, n: usize) {
        if n == 0 {
            return;
        }
        // See `insert`: size overflows first, so check it before mutating.
        let new_size = self
            .size
            .checked_add(n)
            .expect("HashBag size overflowed usize");
        let c = self.counts.entry(value).or_insert(0);
        *c = c
            .checked_add(n)
            .expect("HashBag occurrence count overflowed usize");
        self.size = new_size;
    }

    pub fn remove_one(&mut self, value: &T) -> bool {
        if let Some(c) = self.counts.get_mut(value) {
            *c -= 1;
            self.size -= 1;
            if *c == 0 {
                self.counts.remove(value);
            }
            true
        } else {
            false
        }
    }

    pub fn for_each_with_occurrences(&self, mut f: impl FnMut(&T, usize)) {
        for (v, &c) in &self.counts {
            f(v, c);
        }
    }

    pub fn top_occurrences(&self, n: usize) -> Vec<(&T, usize)> {
        let mut pairs: Vec<_> = self.counts.iter().map(|(v, &c)| (v, c)).collect();
        pairs.sort_by_key(|x| std::cmp::Reverse(x.1));
        pairs.truncate(n);
        pairs
    }

    pub fn bottom_occurrences(&self, n: usize) -> Vec<(&T, usize)> {
        let mut pairs: Vec<_> = self.counts.iter().map(|(v, &c)| (v, c)).collect();
        pairs.sort_by_key(|x| x.1);
        pairs.truncate(n);
        pairs
    }
}

impl<T: Eq + Hash> Default for HashBag<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ---- idiomatic std-style additions ----------------------------------------

/// Borrowed iterator yielding each element once per occurrence (matching
/// [`Collection::iter`]).
pub struct HashBagIter<'a, T> {
    inner: std::collections::hash_map::Iter<'a, T, usize>,
    current: Option<(&'a T, usize)>,
}

impl<'a, T> Iterator for HashBagIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        loop {
            if let Some((v, remaining)) = self.current {
                if remaining > 0 {
                    self.current = Some((v, remaining - 1));
                    return Some(v);
                }
            }
            let (v, &c) = self.inner.next()?;
            self.current = Some((v, c));
        }
    }
}

impl<'a, T: Eq + Hash> IntoIterator for &'a HashBag<T> {
    type Item = &'a T;
    type IntoIter = HashBagIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        HashBagIter {
            inner: self.counts.iter(),
            current: None,
        }
    }
}

impl<T: Eq + Hash> FromIterator<T> for HashBag<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut bag = HashBag::new();
        bag.extend(iter);
        bag
    }
}

impl<T: Eq + Hash> Extend<T> for HashBag<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for v in iter {
            self.insert(v);
        }
    }
}

/// Multiset equality: equal element-to-occurrence-count maps.
impl<T: Eq + Hash> PartialEq for HashBag<T> {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.counts == other.counts
    }
}

impl<T: Eq + Hash> Eq for HashBag<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "size overflowed")]
    fn insert_overflow_panics() {
        // Regression: unchecked `+=` wrapped the count (and `size`) in release.
        // `size` is checked first (it is >= count), so it is what trips here.
        let mut bag = HashBag::new();
        bag.add_occurrences("a", usize::MAX);
        bag.insert("a"); // 1 more occurrence overflows usize
    }

    #[test]
    fn test_basic() {
        let mut bag = HashBag::new();
        bag.insert("a");
        bag.insert("a");
        bag.insert("b");
        assert_eq!(bag.len(), 3);
        assert_eq!(bag.distinct_len(), 2);
        assert_eq!(bag.occurrences_of(&"a"), 2);
        assert_eq!(bag.occurrences_of(&"b"), 1);
        assert_eq!(bag.occurrences_of(&"c"), 0);
    }

    #[test]
    fn test_top_bottom() {
        let bag = HashBag::from_iter(["a", "a", "a", "b", "b", "c"]);
        let top = bag.top_occurrences(2);
        assert_eq!(top[0].1, 3);
        assert_eq!(top[1].1, 2);
        let bot = bag.bottom_occurrences(1);
        assert_eq!(bot[0].1, 1);
    }

    // Regression: the clippy `unnecessary_sort_by` fix swapped `sort_by`
    // for `sort_by_key`/`Reverse`. Both are stable sorts, so the count
    // ordering must be unchanged: top_occurrences strictly descending by
    // count, bottom_occurrences strictly ascending by count.
    #[test]
    fn test_occurrence_sort_order_unchanged() {
        let bag = HashBag::from_iter(["a", "a", "a", "a", "b", "b", "b", "c", "c", "d"]);
        // counts: a=4, b=3, c=2, d=1
        let top = bag.top_occurrences(4);
        let top_counts: Vec<usize> = top.iter().map(|(_, c)| *c).collect();
        assert_eq!(top_counts, vec![4, 3, 2, 1]);

        let bot = bag.bottom_occurrences(4);
        let bot_counts: Vec<usize> = bot.iter().map(|(_, c)| *c).collect();
        assert_eq!(bot_counts, vec![1, 2, 3, 4]);

        // Truncation respects n.
        assert_eq!(bag.top_occurrences(2).len(), 2);
        assert_eq!(bag.bottom_occurrences(1).len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut bag = HashBag::from_iter([1, 1, 2]);
        assert!(bag.remove_one(&1));
        assert_eq!(bag.occurrences_of(&1), 1);
        assert!(bag.remove_one(&1));
        assert_eq!(bag.occurrences_of(&1), 0);
        assert!(!bag.remove_one(&1));
    }

    #[test]
    fn test_into_iter_yields_each_occurrence() {
        let bag = HashBag::from_iter(["a", "a", "b"]);
        let mut items: Vec<&str> = (&bag).into_iter().copied().collect();
        items.sort();
        assert_eq!(items, vec!["a", "a", "b"]);
        assert_eq!((&bag).into_iter().count(), 3);
    }

    #[test]
    fn test_from_iterator_and_extend() {
        let mut bag: HashBag<&str> = ["a", "a", "b"].into_iter().collect();
        assert_eq!(bag.occurrences_of(&"a"), 2);
        bag.extend(["b", "c"]);
        assert_eq!(bag.occurrences_of(&"b"), 2);
        assert_eq!(bag.len(), 5);
    }

    #[test]
    fn test_partial_eq_by_occurrences() {
        let a = HashBag::from_iter([1, 1, 2]);
        let b = HashBag::from_iter([2, 1, 1]);
        let c = HashBag::from_iter([1, 2, 2]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn bulk_load_counts_occurrences_equal_incremental() {
        let data = ["a", "a", "a", "b", "c", "c"];
        let bulk = HashBag::bulk_load(data).unwrap();
        let mut inc = HashBag::new();
        for v in data {
            inc.insert(v);
        }
        assert_eq!(bulk, inc);
        assert_eq!(bulk.occurrences_of(&"a"), 3);
        assert_eq!(bulk.len(), 6);
    }

    #[test]
    fn bulk_load_counts_pairs_sum() {
        let bag = HashBag::bulk_load_counts([("a", 3usize), ("b", 1), ("a", 2), ("z", 0)]).unwrap();
        assert_eq!(bag.occurrences_of(&"a"), 5);
        assert_eq!(bag.occurrences_of(&"b"), 1);
        assert_eq!(bag.occurrences_of(&"z"), 0); // zero count is a no-op
        assert_eq!(bag.len(), 6);
    }

    #[test]
    fn bulk_load_counts_overflow_errors() {
        use crate::bulk::BulkError;
        let err = HashBag::bulk_load_counts([("x", usize::MAX), ("x", 1)]).unwrap_err();
        assert!(matches!(err, BulkError::CountOverflow { index: 1 }));
    }

    #[test]
    fn bulk_load_empty() {
        let bag: HashBag<i32> = HashBag::bulk_load(Vec::new()).unwrap();
        assert!(bag.len() == 0);
    }
}
