// AUTO-GENERATED. DO NOT EDIT.

use crate::arraylist::i16_array_list::I16ArrayList;
use std::fmt;
use std::sync::Arc;

/// Immutable, cheaply cloneable list of `i16` values.
/// Backed by `Arc<[i16]>` — clone is O(1).
#[derive(Debug, Clone)]
pub struct ImmutableI16ArrayList {
    items: Arc<[i16]>,
}

impl ImmutableI16ArrayList {
    /// Creates from a mutable list (consumes the data).
    pub fn from_mutable(list: &I16ArrayList) -> Self {
        ImmutableI16ArrayList {
            items: Arc::from(list.to_vec().into_boxed_slice()),
        }
    }

    /// Creates from a slice.
    pub fn of(values: &[i16]) -> Self {
        ImmutableI16ArrayList {
            items: Arc::from(values),
        }
    }

    pub fn get(&self, index: usize) -> Option<i16> {
        self.items.get(index).copied()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the index of the first occurrence of `value`, or None if not found.
    pub fn index_of(&self, value: i16) -> Option<usize> {
        self.items.iter().position(|&v| v == value)
    }

    pub fn contains(&self, value: i16) -> bool {
        self.items.contains(&value)
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
        ImmutableI16ArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn reject(&self, predicate: impl Fn(i16) -> bool) -> Self {
        let items: Vec<i16> = self
            .items
            .iter()
            .copied()
            .filter(|&v| !predicate(v))
            .collect();
        ImmutableI16ArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn detect(&self, predicate: impl Fn(i16) -> bool) -> Option<i16> {
        self.items.iter().copied().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(i16) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<i16> {
        self.items.to_vec()
    }

    /// Converts back to a mutable list.
    pub fn to_mutable(&self) -> I16ArrayList {
        I16ArrayList::of(&self.items)
    }
}

impl PartialEq for ImmutableI16ArrayList {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        self.items == other.items
    }
}

impl fmt::Display for ImmutableI16ArrayList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.items.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", v)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_of_and_get() {
        let im = ImmutableI16ArrayList::of(&[1, 2, 3]);
        assert_eq!(im.len(), 3);
        assert_eq!(im.get(1), Some(2));
    }

    #[test]
    fn test_contains() {
        let im = ImmutableI16ArrayList::of(&[1, 2]);
        assert!(im.contains(1));
    }

    #[test]
    fn test_clone_is_cheap() {
        let im = ImmutableI16ArrayList::of(&[1, 2]);
        let im2 = im.clone();
        assert_eq!(im, im2);
    }

    #[test]
    fn test_to_mutable_independent() {
        let im = ImmutableI16ArrayList::of(&[1]);
        let mut m = im.to_mutable();
        m.push(2);
        assert_eq!(im.len(), 1);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_display() {
        let im = ImmutableI16ArrayList::of(&[1]);
        assert!(!im.to_string().is_empty());
    }
}

impl crate::traits::i16_collection::I16Collection for ImmutableI16ArrayList {
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

impl crate::traits::i16_collection::I16List for ImmutableI16ArrayList {
    fn get(&self, index: usize) -> Option<i16> {
        self.get(index)
    }
    fn index_of(&self, value: i16) -> Option<usize> {
        self.index_of(value)
    }
}
