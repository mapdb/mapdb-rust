// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use super::traits::*;
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
    pub fn of(values: impl IntoIterator<Item = T>) -> Self {
        let mut bag = Self::new();
        for v in values {
            bag.add(v);
        }
        bag
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
    fn size_distinct(&self) -> usize {
        self.counts.len()
    }
}

impl<T: Eq + Hash> MutableBag<T> for HashBag<T> {
    fn add(&mut self, value: T) {
        *self.counts.entry(value).or_insert(0) += 1;
        self.size += 1;
    }
}

impl<T: Eq + Hash> HashBag<T> {
    pub fn add_occurrences(&mut self, value: T, n: usize) {
        if n == 0 {
            return;
        }
        *self.counts.entry(value).or_insert(0) += n;
        self.size += n;
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
        pairs.sort_by(|a, b| b.1.cmp(&a.1));
        pairs.truncate(n);
        pairs
    }

    pub fn bottom_occurrences(&self, n: usize) -> Vec<(&T, usize)> {
        let mut pairs: Vec<_> = self.counts.iter().map(|(v, &c)| (v, c)).collect();
        pairs.sort_by(|a, b| a.1.cmp(&b.1));
        pairs.truncate(n);
        pairs
    }
}

impl<T: Eq + Hash> Default for HashBag<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut bag = HashBag::new();
        bag.add("a");
        bag.add("a");
        bag.add("b");
        assert_eq!(bag.len(), 3);
        assert_eq!(bag.size_distinct(), 2);
        assert_eq!(bag.occurrences_of(&"a"), 2);
        assert_eq!(bag.occurrences_of(&"b"), 1);
        assert_eq!(bag.occurrences_of(&"c"), 0);
    }

    #[test]
    fn test_top_bottom() {
        let bag = HashBag::of(vec!["a", "a", "a", "b", "b", "c"]);
        let top = bag.top_occurrences(2);
        assert_eq!(top[0].1, 3);
        assert_eq!(top[1].1, 2);
        let bot = bag.bottom_occurrences(1);
        assert_eq!(bot[0].1, 1);
    }

    #[test]
    fn test_remove() {
        let mut bag = HashBag::of(vec![1, 1, 2]);
        assert!(bag.remove_one(&1));
        assert_eq!(bag.occurrences_of(&1), 1);
        assert!(bag.remove_one(&1));
        assert_eq!(bag.occurrences_of(&1), 0);
        assert!(!bag.remove_one(&1));
    }
}
