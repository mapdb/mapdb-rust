// AUTO-GENERATED. DO NOT EDIT.

use crate::arraylist::bool_array_list::BoolArrayList;
use std::fmt;
use std::sync::Arc;

/// Immutable, cheaply cloneable list of `bool` values.
/// Backed by `Arc<[bool]>` — clone is O(1).
#[derive(Debug, Clone)]
pub struct ImmutableBoolArrayList {
    items: Arc<[bool]>,
}

impl ImmutableBoolArrayList {
    /// Creates from a mutable list (consumes the data).
    pub fn from_mutable(list: &BoolArrayList) -> Self {
        ImmutableBoolArrayList {
            items: Arc::from(list.to_vec().into_boxed_slice()),
        }
    }

    /// Creates from a slice.
    pub fn of(values: &[bool]) -> Self {
        ImmutableBoolArrayList {
            items: Arc::from(values),
        }
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        self.items.get(index).copied()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the index of the first occurrence of `value`, or None if not found.
    pub fn index_of(&self, value: bool) -> Option<usize> {
        self.items.iter().position(|&v| v == value)
    }

    pub fn contains(&self, value: bool) -> bool {
        self.items.contains(&value)
    }

    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.items.iter().copied()
    }

    pub fn select(&self, predicate: impl Fn(bool) -> bool) -> Self {
        let items: Vec<bool> = self
            .items
            .iter()
            .copied()
            .filter(|&v| predicate(v))
            .collect();
        ImmutableBoolArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn reject(&self, predicate: impl Fn(bool) -> bool) -> Self {
        let items: Vec<bool> = self
            .items
            .iter()
            .copied()
            .filter(|&v| !predicate(v))
            .collect();
        ImmutableBoolArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn detect(&self, predicate: impl Fn(bool) -> bool) -> Option<bool> {
        self.items.iter().copied().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(bool) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<bool> {
        self.items.to_vec()
    }

    /// Converts back to a mutable list.
    pub fn to_mutable(&self) -> BoolArrayList {
        BoolArrayList::of(&self.items)
    }
}

impl PartialEq for ImmutableBoolArrayList {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        self.items == other.items
    }
}

impl fmt::Display for ImmutableBoolArrayList {
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
        let im = ImmutableBoolArrayList::of(&[true, false, true]);
        assert_eq!(im.len(), 3);
        assert_eq!(im.get(1), Some(false));
    }

    #[test]
    fn test_contains() {
        let im = ImmutableBoolArrayList::of(&[true, false]);
        assert!(im.contains(true));
    }

    #[test]
    fn test_clone_is_cheap() {
        let im = ImmutableBoolArrayList::of(&[true, false]);
        let im2 = im.clone();
        assert_eq!(im, im2);
    }

    #[test]
    fn test_to_mutable_independent() {
        let im = ImmutableBoolArrayList::of(&[true]);
        let mut m = im.to_mutable();
        m.push(false);
        assert_eq!(im.len(), 1);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_display() {
        let im = ImmutableBoolArrayList::of(&[true]);
        assert!(!im.to_string().is_empty());
    }
}

impl crate::traits::bool_collection::BoolCollection for ImmutableBoolArrayList {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: bool) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter()
    }
}

impl crate::traits::bool_collection::BoolList for ImmutableBoolArrayList {
    fn get(&self, index: usize) -> Option<bool> {
        self.get(index)
    }
    fn index_of(&self, value: bool) -> Option<usize> {
        self.index_of(value)
    }
}
