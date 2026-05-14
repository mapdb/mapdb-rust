// AUTO-GENERATED. DO NOT EDIT.

use crate::arraylist::char_array_list::CharArrayList;
use std::fmt;
use std::sync::Arc;

/// Immutable, cheaply cloneable list of `char` values.
/// Backed by `Arc<[char]>` — clone is O(1).
#[derive(Debug, Clone)]
pub struct ImmutableCharArrayList {
    items: Arc<[char]>,
}

impl ImmutableCharArrayList {
    /// Creates from a mutable list (consumes the data).
    pub fn from_mutable(list: &CharArrayList) -> Self {
        ImmutableCharArrayList {
            items: Arc::from(list.to_vec().into_boxed_slice()),
        }
    }

    /// Creates from a slice.
    pub fn of(values: &[char]) -> Self {
        ImmutableCharArrayList {
            items: Arc::from(values),
        }
    }

    pub fn get(&self, index: usize) -> Option<char> {
        self.items.get(index).copied()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the index of the first occurrence of `value`, or None if not found.
    pub fn index_of(&self, value: char) -> Option<usize> {
        self.items.iter().position(|&v| v == value)
    }

    pub fn contains(&self, value: char) -> bool {
        self.items.contains(&value)
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
        ImmutableCharArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn reject(&self, predicate: impl Fn(char) -> bool) -> Self {
        let items: Vec<char> = self
            .items
            .iter()
            .copied()
            .filter(|&v| !predicate(v))
            .collect();
        ImmutableCharArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn detect(&self, predicate: impl Fn(char) -> bool) -> Option<char> {
        self.items.iter().copied().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<char> {
        self.items.to_vec()
    }

    /// Converts back to a mutable list.
    pub fn to_mutable(&self) -> CharArrayList {
        CharArrayList::of(&self.items)
    }
}

impl PartialEq for ImmutableCharArrayList {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        self.items == other.items
    }
}

impl fmt::Display for ImmutableCharArrayList {
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
        let im = ImmutableCharArrayList::of(&['a', 'b', 'c']);
        assert_eq!(im.len(), 3);
        assert_eq!(im.get(1), Some('b'));
    }

    #[test]
    fn test_contains() {
        let im = ImmutableCharArrayList::of(&['a', 'b']);
        assert!(im.contains('a'));
    }

    #[test]
    fn test_clone_is_cheap() {
        let im = ImmutableCharArrayList::of(&['a', 'b']);
        let im2 = im.clone();
        assert_eq!(im, im2);
    }

    #[test]
    fn test_to_mutable_independent() {
        let im = ImmutableCharArrayList::of(&['a']);
        let mut m = im.to_mutable();
        m.push('b');
        assert_eq!(im.len(), 1);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_display() {
        let im = ImmutableCharArrayList::of(&['a']);
        assert!(!im.to_string().is_empty());
    }
}

impl crate::traits::char_collection::CharCollection for ImmutableCharArrayList {
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

impl crate::traits::char_collection::CharList for ImmutableCharArrayList {
    fn get(&self, index: usize) -> Option<char> {
        self.get(index)
    }
    fn index_of(&self, value: char) -> Option<usize> {
        self.index_of(value)
    }
}
