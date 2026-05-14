// AUTO-GENERATED. DO NOT EDIT.

use crate::arraylist::i32_array_list::I32ArrayList;
use std::fmt;
use std::sync::Arc;

/// Immutable, cheaply cloneable list of `i32` values.
/// Backed by `Arc<[i32]>` — clone is O(1).
#[derive(Debug, Clone)]
pub struct ImmutableI32ArrayList {
    items: Arc<[i32]>,
}

impl ImmutableI32ArrayList {
    /// Creates from a mutable list (consumes the data).
    pub fn from_mutable(list: &I32ArrayList) -> Self {
        ImmutableI32ArrayList {
            items: Arc::from(list.to_vec().into_boxed_slice()),
        }
    }

    /// Creates from a slice.
    pub fn of(values: &[i32]) -> Self {
        ImmutableI32ArrayList {
            items: Arc::from(values),
        }
    }

    pub fn get(&self, index: usize) -> Option<i32> {
        self.items.get(index).copied()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the index of the first occurrence of `value`, or None if not found.
    pub fn index_of(&self, value: i32) -> Option<usize> {
        self.items.iter().position(|&v| v == value)
    }

    pub fn contains(&self, value: i32) -> bool {
        self.items.contains(&value)
    }

    pub fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        self.items.iter().copied()
    }

    pub fn select(&self, predicate: impl Fn(i32) -> bool) -> Self {
        let items: Vec<i32> = self
            .items
            .iter()
            .copied()
            .filter(|&v| predicate(v))
            .collect();
        ImmutableI32ArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn reject(&self, predicate: impl Fn(i32) -> bool) -> Self {
        let items: Vec<i32> = self
            .items
            .iter()
            .copied()
            .filter(|&v| !predicate(v))
            .collect();
        ImmutableI32ArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn detect(&self, predicate: impl Fn(i32) -> bool) -> Option<i32> {
        self.items.iter().copied().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(i32) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<i32> {
        self.items.to_vec()
    }

    /// Converts back to a mutable list.
    pub fn to_mutable(&self) -> I32ArrayList {
        I32ArrayList::of(&self.items)
    }
}

impl PartialEq for ImmutableI32ArrayList {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        self.items == other.items
    }
}

impl fmt::Display for ImmutableI32ArrayList {
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
        let im = ImmutableI32ArrayList::of(&[1, 2, 3]);
        assert_eq!(im.len(), 3);
        assert_eq!(im.get(1), Some(2));
    }

    #[test]
    fn test_contains() {
        let im = ImmutableI32ArrayList::of(&[1, 2]);
        assert!(im.contains(1));
    }

    #[test]
    fn test_clone_is_cheap() {
        let im = ImmutableI32ArrayList::of(&[1, 2]);
        let im2 = im.clone();
        assert_eq!(im, im2);
    }

    #[test]
    fn test_to_mutable_independent() {
        let im = ImmutableI32ArrayList::of(&[1]);
        let mut m = im.to_mutable();
        m.push(2);
        assert_eq!(im.len(), 1);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_display() {
        let im = ImmutableI32ArrayList::of(&[1]);
        assert!(!im.to_string().is_empty());
    }
}

impl crate::traits::i32_collection::I32Collection for ImmutableI32ArrayList {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: i32) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        self.iter()
    }
}

impl crate::traits::i32_collection::I32List for ImmutableI32ArrayList {
    fn get(&self, index: usize) -> Option<i32> {
        self.get(index)
    }
    fn index_of(&self, value: i32) -> Option<usize> {
        self.index_of(value)
    }
}
