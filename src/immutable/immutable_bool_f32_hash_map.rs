// AUTO-GENERATED. DO NOT EDIT.
use crate::hashmap::bool_f32_hash_map::BoolF32HashMap;
use std::collections::HashMap;
use std::fmt;

/// Immutable hash map from `bool` keys to `f32` values. Clone is cheap (shared data).
#[derive(Debug, Clone)]
pub struct ImmutableBoolF32HashMap {
    inner: HashMap<bool, f32>,
}

impl ImmutableBoolF32HashMap {
    pub fn from_mutable(m: &BoolF32HashMap) -> Self {
        let mut inner = HashMap::new();
        m.for_each(|k, v| {
            inner.insert(k, v);
        });
        ImmutableBoolF32HashMap { inner }
    }
    pub fn get(&self, key: bool) -> Option<f32> {
        self.inner.get(&(key)).copied()
    }
    pub fn get_or_default(&self, key: bool, default: f32) -> f32 {
        self.get(key).unwrap_or(default)
    }
    pub fn contains_key(&self, key: bool) -> bool {
        self.inner.contains_key(&(key))
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (bool, f32)> + '_ {
        self.inner.iter().map(|(k, &v)| (*k, v))
    }
    pub fn keys(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.keys().copied()
    }
    pub fn values(&self) -> impl Iterator<Item = f32> + '_ {
        self.inner.values().copied()
    }
    pub fn for_each(&self, mut f: impl FnMut(bool, f32)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    pub fn select(&self, predicate: impl Fn(bool, f32) -> bool) -> Self {
        let mut inner = HashMap::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                inner.insert(k, v);
            }
        }
        ImmutableBoolF32HashMap { inner }
    }
    pub fn any_satisfy(&self, predicate: impl Fn(bool, f32) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    pub fn all_satisfy(&self, predicate: impl Fn(bool, f32) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    pub fn to_mutable(&self) -> BoolF32HashMap {
        let mut m = BoolF32HashMap::new();
        for (k, v) in self.iter() {
            m.insert(k, v);
        }
        m
    }
}
impl PartialEq for ImmutableBoolF32HashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v.to_bits() == ov.to_bits()))
    }
}
impl fmt::Display for ImmutableBoolF32HashMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (k, v) in self.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}={}", k, v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get() {
        let mut m = BoolF32HashMap::new();
        m.insert(true, 1.0f32);
        m.insert(false, 2.0f32);
        let im = ImmutableBoolF32HashMap::from_mutable(&m);
        assert_eq!(im.get(true), Some(1.0f32));
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_contains_key() {
        let mut m = BoolF32HashMap::new();
        m.insert(true, 1.0f32);
        let im = ImmutableBoolF32HashMap::from_mutable(&m);
        assert!(im.contains_key(true));
    }
    #[test]
    fn test_to_mutable_independent() {
        let mut m = BoolF32HashMap::new();
        m.insert(true, 1.0f32);
        let im = ImmutableBoolF32HashMap::from_mutable(&m);
        let mut m2 = im.to_mutable();
        m2.insert(false, 2.0f32);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        let mut m = BoolF32HashMap::new();
        m.insert(true, 1.0f32);
        assert!(!ImmutableBoolF32HashMap::from_mutable(&m)
            .to_string()
            .is_empty());
    }
}

impl crate::traits::bool_f32_map::BoolF32Map for ImmutableBoolF32HashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: bool) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: bool) -> Option<f32> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (bool, f32)> + '_ {
        self.iter()
    }
}
