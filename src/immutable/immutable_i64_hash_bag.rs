// AUTO-GENERATED. DO NOT EDIT.
use crate::bag::i64_hash_bag::I64HashBag;
use std::collections::HashMap;
use std::fmt;

/// Immutable bag (multiset) of `i64` values with occurrence counts.
#[derive(Debug, Clone)]
pub struct ImmutableI64HashBag {
    counts: HashMap<i64, usize>,
    size: usize,
}

impl ImmutableI64HashBag {
    pub fn from_mutable(bag: &I64HashBag) -> Self {
        let mut im = ImmutableI64HashBag {
            counts: HashMap::new(),
            size: 0,
        };
        bag.for_each_with_occurrences(|v, c| {
            im.counts.insert(v, c);
            im.size += c;
        });
        im
    }
    pub fn of(values: &[i64]) -> Self {
        let m = I64HashBag::of(values);
        Self::from_mutable(&m)
    }
    pub fn occurrences_of(&self, value: i64) -> usize {
        self.counts.get(&value).copied().unwrap_or(0)
    }
    pub fn contains(&self, value: i64) -> bool {
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
    /// Iterates over all elements, repeating each value by its count.
    pub fn iter(&self) -> impl Iterator<Item = i64> + '_ {
        self.counts
            .iter()
            .flat_map(|(&v, &c)| std::iter::repeat_n(v, c))
    }
    pub fn to_mutable(&self) -> I64HashBag {
        let mut m = I64HashBag::new();
        for (&v, &c) in &self.counts {
            for _ in 0..c {
                m.add(v);
            }
        }
        m
    }
    /// Expands the bag to a `Vec` containing each value repeated by its count.
    pub fn to_vec(&self) -> Vec<i64> {
        let mut out = Vec::with_capacity(self.size);
        for (&v, &c) in &self.counts {
            for _ in 0..c {
                out.push(v);
            }
        }
        out
    }
}

impl crate::traits::i64_collection::I64Collection for ImmutableI64HashBag {
    fn len(&self) -> usize {
        self.size()
    }
    fn contains(&self, value: i64) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = i64> + '_ {
        self.iter()
    }
}

impl crate::traits::i64_collection::I64Bag for ImmutableI64HashBag {
    fn occurrences_of(&self, value: i64) -> usize {
        self.occurrences_of(value)
    }
    fn size_distinct(&self) -> usize {
        self.size_distinct()
    }
}

impl fmt::Display for ImmutableI64HashBag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ImmutableBag(size={})", self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_occurrences() {
        let im = ImmutableI64HashBag::of(&[1, 1, 2]);
        assert_eq!(im.occurrences_of(1), 2);
        assert_eq!(im.size(), 3);
    }
    #[test]
    fn test_to_mutable() {
        let im = ImmutableI64HashBag::of(&[1]);
        let mut m = im.to_mutable();
        m.add(2);
        assert_eq!(im.size(), 1);
    }
    #[test]
    fn test_display() {
        assert!(!ImmutableI64HashBag::of(&[1]).to_string().is_empty());
    }
}
