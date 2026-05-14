// AUTO-GENERATED. DO NOT EDIT.
use crate::hashset::f64_hash_set::F64HashSet;
use std::fmt;
use std::sync::Arc;

/// Immutable, cheaply cloneable set of `f64` values.
#[derive(Debug, Clone)]
pub struct ImmutableF64HashSet {
    items: Arc<[f64]>,
}

impl ImmutableF64HashSet {
    pub fn from_mutable(set: &F64HashSet) -> Self {
        ImmutableF64HashSet {
            items: Arc::from(set.to_vec().into_boxed_slice()),
        }
    }
    pub fn of(values: &[f64]) -> Self {
        let mut s = F64HashSet::new();
        for &v in values {
            s.add(v);
        }
        Self::from_mutable(&s)
    }
    pub fn contains(&self, value: f64) -> bool {
        self.items.iter().any(|&v| v.to_bits() == value.to_bits())
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
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
        ImmutableF64HashSet {
            items: Arc::from(items.into_boxed_slice()),
        }
    }
    pub fn to_vec(&self) -> Vec<f64> {
        self.items.to_vec()
    }
    pub fn to_mutable(&self) -> F64HashSet {
        F64HashSet::of(&self.items)
    }
}

impl PartialEq for ImmutableF64HashSet {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.items.iter().all(|&v| other.contains(v))
    }
}
impl fmt::Display for ImmutableF64HashSet {
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
        let im = ImmutableF64HashSet::of(&[1.0f64, 2.0f64]);
        assert!(im.contains(1.0f64));
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_clone_cheap() {
        let im = ImmutableF64HashSet::of(&[1.0f64]);
        let im2 = im.clone();
        assert_eq!(im, im2);
    }
    #[test]
    fn test_to_mutable_independent() {
        let im = ImmutableF64HashSet::of(&[1.0f64]);
        let mut m = im.to_mutable();
        m.add(2.0f64);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        assert!(!ImmutableF64HashSet::of(&[1.0f64]).to_string().is_empty());
    }
}

impl crate::traits::f64_collection::F64Collection for ImmutableF64HashSet {
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

impl crate::traits::f64_collection::F64Set for ImmutableF64HashSet {}
