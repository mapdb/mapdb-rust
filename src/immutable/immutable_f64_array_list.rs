// AUTO-GENERATED. DO NOT EDIT.

use crate::arraylist::f64_array_list::F64ArrayList;
use std::fmt;
use std::sync::Arc;

/// Immutable, cheaply cloneable list of `f64` values.
/// Backed by `Arc<[f64]>` — clone is O(1).
#[derive(Debug, Clone)]
pub struct ImmutableF64ArrayList {
    items: Arc<[f64]>,
}

impl ImmutableF64ArrayList {
    /// Creates from a mutable list (consumes the data).
    pub fn from_mutable(list: &F64ArrayList) -> Self {
        ImmutableF64ArrayList {
            items: Arc::from(list.to_vec().into_boxed_slice()),
        }
    }

    /// Creates from a slice.
    pub fn of(values: &[f64]) -> Self {
        ImmutableF64ArrayList {
            items: Arc::from(values),
        }
    }

    pub fn get(&self, index: usize) -> Option<f64> {
        self.items.get(index).copied()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the index of the first occurrence of `value`, or None if not found.
    pub fn index_of(&self, value: f64) -> Option<usize> {
        self.items
            .iter()
            .position(|&v| v.to_bits() == value.to_bits())
    }

    pub fn contains(&self, value: f64) -> bool {
        self.items.iter().any(|&v| v.to_bits() == value.to_bits())
    }

    pub fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.items.iter().copied()
    }

    pub fn select(&self, predicate: impl Fn(f64) -> bool) -> Self {
        let items: Vec<f64> = self
            .items
            .iter()
            .copied()
            .filter(|&v| predicate(v))
            .collect();
        ImmutableF64ArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn reject(&self, predicate: impl Fn(f64) -> bool) -> Self {
        let items: Vec<f64> = self
            .items
            .iter()
            .copied()
            .filter(|&v| !predicate(v))
            .collect();
        ImmutableF64ArrayList {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn detect(&self, predicate: impl Fn(f64) -> bool) -> Option<f64> {
        self.items.iter().copied().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(f64) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<f64> {
        self.items.to_vec()
    }

    /// Converts back to a mutable list.
    pub fn to_mutable(&self) -> F64ArrayList {
        F64ArrayList::of(&self.items)
    }
}

impl PartialEq for ImmutableF64ArrayList {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        self.items
            .iter()
            .zip(other.items.iter())
            .all(|(a, b)| a.to_bits() == b.to_bits())
    }
}

impl fmt::Display for ImmutableF64ArrayList {
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
        let im = ImmutableF64ArrayList::of(&[1.0f64, 2.0f64, 3.0f64]);
        assert_eq!(im.len(), 3);
        assert_eq!(im.get(1), Some(2.0f64));
    }

    #[test]
    fn test_contains() {
        let im = ImmutableF64ArrayList::of(&[1.0f64, 2.0f64]);
        assert!(im.contains(1.0f64));
    }

    #[test]
    fn test_clone_is_cheap() {
        let im = ImmutableF64ArrayList::of(&[1.0f64, 2.0f64]);
        let im2 = im.clone();
        assert_eq!(im, im2);
    }

    #[test]
    fn test_to_mutable_independent() {
        let im = ImmutableF64ArrayList::of(&[1.0f64]);
        let mut m = im.to_mutable();
        m.push(2.0f64);
        assert_eq!(im.len(), 1);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_display() {
        let im = ImmutableF64ArrayList::of(&[1.0f64]);
        assert!(!im.to_string().is_empty());
    }
}

impl crate::traits::f64_collection::F64Collection for ImmutableF64ArrayList {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: f64) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.iter()
    }
}

impl crate::traits::f64_collection::F64List for ImmutableF64ArrayList {
    fn get(&self, index: usize) -> Option<f64> {
        self.get(index)
    }
    fn index_of(&self, value: f64) -> Option<usize> {
        self.index_of(value)
    }
}
