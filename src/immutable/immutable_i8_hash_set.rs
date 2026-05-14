// AUTO-GENERATED. DO NOT EDIT.
use crate::hashset::i8_hash_set::I8HashSet;
use std::fmt;
use std::sync::Arc;

/// Immutable, cheaply cloneable set of `i8` values.
#[derive(Debug, Clone)]
pub struct ImmutableI8HashSet {
    items: Arc<[i8]>,
}

impl ImmutableI8HashSet {
    pub fn from_mutable(set: &I8HashSet) -> Self {
        ImmutableI8HashSet {
            items: Arc::from(set.to_vec().into_boxed_slice()),
        }
    }
    pub fn of(values: &[i8]) -> Self {
        let mut s = I8HashSet::new();
        for &v in values {
            s.add(v);
        }
        Self::from_mutable(&s)
    }
    pub fn contains(&self, value: i8) -> bool {
        self.items.contains(&value)
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = i8> + '_ {
        self.items.iter().copied()
    }
    pub fn select(&self, predicate: impl Fn(i8) -> bool) -> Self {
        let items: Vec<i8> = self
            .items
            .iter()
            .copied()
            .filter(|&v| predicate(v))
            .collect();
        ImmutableI8HashSet {
            items: Arc::from(items.into_boxed_slice()),
        }
    }
    pub fn to_vec(&self) -> Vec<i8> {
        self.items.to_vec()
    }
    pub fn to_mutable(&self) -> I8HashSet {
        I8HashSet::of(&self.items)
    }
}

impl PartialEq for ImmutableI8HashSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.items.iter().all(|&v| other.contains(v))
    }
}
impl fmt::Display for ImmutableI8HashSet {
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
        let im = ImmutableI8HashSet::of(&[1, 2]);
        assert!(im.contains(1));
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_clone_cheap() {
        let im = ImmutableI8HashSet::of(&[1]);
        let im2 = im.clone();
        assert_eq!(im, im2);
    }
    #[test]
    fn test_to_mutable_independent() {
        let im = ImmutableI8HashSet::of(&[1]);
        let mut m = im.to_mutable();
        m.add(2);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        assert!(!ImmutableI8HashSet::of(&[1]).to_string().is_empty());
    }
}

impl crate::traits::i8_collection::I8Collection for ImmutableI8HashSet {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: i8) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = i8> + '_ {
        self.iter()
    }
}

impl crate::traits::i8_collection::I8Set for ImmutableI8HashSet {}
