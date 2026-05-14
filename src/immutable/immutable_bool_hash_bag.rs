// AUTO-GENERATED. DO NOT EDIT.
use crate::bag::bool_hash_bag::BoolHashBag;
use std::collections::HashMap;
use std::fmt;

/// Immutable bag (multiset) of `bool` values with occurrence counts.
#[derive(Debug, Clone)]
pub struct ImmutableBoolHashBag {
    counts: HashMap<bool, usize>,
    size: usize,
}

impl ImmutableBoolHashBag {
    pub fn from_mutable(bag: &BoolHashBag) -> Self {
        let mut im = ImmutableBoolHashBag {
            counts: HashMap::new(),
            size: 0,
        };
        bag.for_each_with_occurrences(|v, c| {
            im.counts.insert(v, c);
            im.size += c;
        });
        im
    }
    pub fn of(values: &[bool]) -> Self {
        let m = BoolHashBag::of(values);
        Self::from_mutable(&m)
    }
    pub fn occurrences_of(&self, value: bool) -> usize {
        self.counts.get(&value).copied().unwrap_or(0)
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
    /// Iterates over all elements, repeating each value by its count.
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.counts
            .iter()
            .flat_map(|(&v, &c)| std::iter::repeat_n(v, c))
    }
    pub fn to_mutable(&self) -> BoolHashBag {
        let mut m = BoolHashBag::new();
        for (&v, &c) in &self.counts {
            for _ in 0..c {
                m.add(v);
            }
        }
        m
    }
    /// Expands the bag to a `Vec` containing each value repeated by its count.
    pub fn to_vec(&self) -> Vec<bool> {
        let mut out = Vec::with_capacity(self.size);
        for (&v, &c) in &self.counts {
            for _ in 0..c {
                out.push(v);
            }
        }
        out
    }
}

impl crate::traits::bool_collection::BoolCollection for ImmutableBoolHashBag {
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

impl crate::traits::bool_collection::BoolBag for ImmutableBoolHashBag {
    fn occurrences_of(&self, value: bool) -> usize {
        self.occurrences_of(value)
    }
    fn size_distinct(&self) -> usize {
        self.size_distinct()
    }
}

impl fmt::Display for ImmutableBoolHashBag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ImmutableBag(size={})", self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_occurrences() {
        let im = ImmutableBoolHashBag::of(&[true, true, false]);
        assert_eq!(im.occurrences_of(true), 2);
        assert_eq!(im.size(), 3);
    }
    #[test]
    fn test_to_mutable() {
        let im = ImmutableBoolHashBag::of(&[true]);
        let mut m = im.to_mutable();
        m.add(false);
        assert_eq!(im.size(), 1);
    }
    #[test]
    fn test_display() {
        assert!(!ImmutableBoolHashBag::of(&[true]).to_string().is_empty());
    }
}
