// AUTO-GENERATED. DO NOT EDIT.
use crate::hashset::char_hash_set::CharHashSet;
use std::fmt;
use std::sync::Arc;

/// Immutable, cheaply cloneable set of `char` values.
#[derive(Debug, Clone)]
pub struct ImmutableCharHashSet {
    items: Arc<[char]>,
}

impl ImmutableCharHashSet {
    pub fn from_mutable(set: &CharHashSet) -> Self {
        ImmutableCharHashSet {
            items: Arc::from(set.to_vec().into_boxed_slice()),
        }
    }
    pub fn of(values: &[char]) -> Self {
        let mut s = CharHashSet::new();
        for &v in values {
            s.add(v);
        }
        Self::from_mutable(&s)
    }
    pub fn contains(&self, value: char) -> bool {
        self.items.contains(&value)
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.items.iter().copied()
    }
    pub fn select(&self, predicate: impl Fn(char) -> bool) -> Self {
        let items: Vec<char> = self
            .items
            .iter()
            .copied()
            .filter(|&v| predicate(v))
            .collect();
        ImmutableCharHashSet {
            items: Arc::from(items.into_boxed_slice()),
        }
    }
    pub fn to_vec(&self) -> Vec<char> {
        self.items.to_vec()
    }
    pub fn to_mutable(&self) -> CharHashSet {
        CharHashSet::of(&self.items)
    }
}

impl PartialEq for ImmutableCharHashSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.items.iter().all(|&v| other.contains(v))
    }
}
impl fmt::Display for ImmutableCharHashSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, v) in self.items.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", v)?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_of_contains() {
        let im = ImmutableCharHashSet::of(&['a', 'b']);
        assert!(im.contains('a'));
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_clone_cheap() {
        let im = ImmutableCharHashSet::of(&['a']);
        let im2 = im.clone();
        assert_eq!(im, im2);
    }
    #[test]
    fn test_to_mutable_independent() {
        let im = ImmutableCharHashSet::of(&['a']);
        let mut m = im.to_mutable();
        m.add('b');
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        assert!(!ImmutableCharHashSet::of(&['a']).to_string().is_empty());
    }
}

impl crate::traits::char_collection::CharCollection for ImmutableCharHashSet {
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

impl crate::traits::char_collection::CharSet for ImmutableCharHashSet {}
