// AUTO-GENERATED. DO NOT EDIT.
use crate::hashset::i16_hash_set::I16HashSet;
use std::fmt;
use std::sync::Arc;

/// Immutable, cheaply cloneable set of `i16` values.
#[derive(Debug, Clone)]
pub struct ImmutableI16HashSet {
    items: Arc<[i16]>,
}

impl ImmutableI16HashSet {
    pub fn from_mutable(set: &I16HashSet) -> Self {
        ImmutableI16HashSet {
            items: Arc::from(set.to_vec().into_boxed_slice()),
        }
    }
    pub fn of(values: &[i16]) -> Self {
        let mut s = I16HashSet::new();
        for &v in values {
            s.add(v);
        }
        Self::from_mutable(&s)
    }
    pub fn contains(&self, value: i16) -> bool {
        self.items.contains(&value)
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = i16> + '_ {
        self.items.iter().copied()
    }
    pub fn select(&self, predicate: impl Fn(i16) -> bool) -> Self {
        let items: Vec<i16> = self
            .items
            .iter()
            .copied()
            .filter(|&v| predicate(v))
            .collect();
        ImmutableI16HashSet {
            items: Arc::from(items.into_boxed_slice()),
        }
    }
    pub fn to_vec(&self) -> Vec<i16> {
        self.items.to_vec()
    }
    pub fn to_mutable(&self) -> I16HashSet {
        I16HashSet::of(&self.items)
    }
}

impl PartialEq for ImmutableI16HashSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.items.iter().all(|&v| other.contains(v))
    }
}
impl fmt::Display for ImmutableI16HashSet {
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
        let im = ImmutableI16HashSet::of(&[1, 2]);
        assert!(im.contains(1));
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_clone_cheap() {
        let im = ImmutableI16HashSet::of(&[1]);
        let im2 = im.clone();
        assert_eq!(im, im2);
    }
    #[test]
    fn test_to_mutable_independent() {
        let im = ImmutableI16HashSet::of(&[1]);
        let mut m = im.to_mutable();
        m.add(2);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        assert!(!ImmutableI16HashSet::of(&[1]).to_string().is_empty());
    }
}

impl crate::traits::i16_collection::I16Collection for ImmutableI16HashSet {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: i16) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = i16> + '_ {
        self.iter()
    }
}

impl crate::traits::i16_collection::I16Set for ImmutableI16HashSet {}
