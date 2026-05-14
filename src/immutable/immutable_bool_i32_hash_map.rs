// AUTO-GENERATED. DO NOT EDIT.
use crate::hashmap::bool_i32_hash_map::BoolI32HashMap;
use std::collections::HashMap;
use std::fmt;

/// Immutable hash map from `bool` keys to `i32` values. Clone is cheap (shared data).
#[derive(Debug, Clone)]
pub struct ImmutableBoolI32HashMap {
    inner: HashMap<bool, i32>,
}

impl ImmutableBoolI32HashMap {
    pub fn from_mutable(m: &BoolI32HashMap) -> Self {
        let mut inner = HashMap::new();
        m.for_each(|k, v| {
            inner.insert(k, v);
        });
        ImmutableBoolI32HashMap { inner }
    }
    pub fn get(&self, key: bool) -> Option<i32> {
        self.inner.get(&(key)).copied()
    }
    pub fn get_or_default(&self, key: bool, default: i32) -> i32 {
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
    pub fn iter(&self) -> impl Iterator<Item = (bool, i32)> + '_ {
        self.inner.iter().map(|(k, &v)| (*k, v))
    }
    pub fn keys(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.keys().copied()
    }
    pub fn values(&self) -> impl Iterator<Item = i32> + '_ {
        self.inner.values().copied()
    }
    pub fn for_each(&self, mut f: impl FnMut(bool, i32)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    pub fn select(&self, predicate: impl Fn(bool, i32) -> bool) -> Self {
        let mut inner = HashMap::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                inner.insert(k, v);
            }
        }
        ImmutableBoolI32HashMap { inner }
    }
    pub fn any_satisfy(&self, predicate: impl Fn(bool, i32) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    pub fn all_satisfy(&self, predicate: impl Fn(bool, i32) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    pub fn to_mutable(&self) -> BoolI32HashMap {
        let mut m = BoolI32HashMap::new();
        for (k, v) in self.iter() {
            m.insert(k, v);
        }
        m
    }
}
impl PartialEq for ImmutableBoolI32HashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}
impl fmt::Display for ImmutableBoolI32HashMap {
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
        let mut m = BoolI32HashMap::new();
        m.insert(true, 1);
        m.insert(false, 2);
        let im = ImmutableBoolI32HashMap::from_mutable(&m);
        assert_eq!(im.get(true), Some(1));
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_contains_key() {
        let mut m = BoolI32HashMap::new();
        m.insert(true, 1);
        let im = ImmutableBoolI32HashMap::from_mutable(&m);
        assert!(im.contains_key(true));
    }
    #[test]
    fn test_to_mutable_independent() {
        let mut m = BoolI32HashMap::new();
        m.insert(true, 1);
        let im = ImmutableBoolI32HashMap::from_mutable(&m);
        let mut m2 = im.to_mutable();
        m2.insert(false, 2);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        let mut m = BoolI32HashMap::new();
        m.insert(true, 1);
        assert!(!ImmutableBoolI32HashMap::from_mutable(&m)
            .to_string()
            .is_empty());
    }
}

impl crate::traits::bool_i32_map::BoolI32Map for ImmutableBoolI32HashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: bool) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: bool) -> Option<i32> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (bool, i32)> + '_ {
        self.iter()
    }
}
