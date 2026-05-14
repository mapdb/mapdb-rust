// AUTO-GENERATED. DO NOT EDIT.
use crate::hashmap::i32_i64_hash_map::I32I64HashMap;
use std::collections::HashMap;
use std::fmt;

/// Immutable hash map from `i32` keys to `i64` values. Clone is cheap (shared data).
#[derive(Debug, Clone)]
pub struct ImmutableI32I64HashMap {
    inner: HashMap<i32, i64>,
}

impl ImmutableI32I64HashMap {
    pub fn from_mutable(m: &I32I64HashMap) -> Self {
        let mut inner = HashMap::new();
        m.for_each(|k, v| {
            inner.insert(k, v);
        });
        ImmutableI32I64HashMap { inner }
    }
    pub fn get(&self, key: i32) -> Option<i64> {
        self.inner.get(&(key)).copied()
    }
    pub fn get_or_default(&self, key: i32, default: i64) -> i64 {
        self.get(key).unwrap_or(default)
    }
    pub fn contains_key(&self, key: i32) -> bool {
        self.inner.contains_key(&(key))
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (i32, i64)> + '_ {
        self.inner.iter().map(|(k, &v)| (*k, v))
    }
    pub fn keys(&self) -> impl Iterator<Item = i32> + '_ {
        self.inner.keys().copied()
    }
    pub fn values(&self) -> impl Iterator<Item = i64> + '_ {
        self.inner.values().copied()
    }
    pub fn for_each(&self, mut f: impl FnMut(i32, i64)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    pub fn select(&self, predicate: impl Fn(i32, i64) -> bool) -> Self {
        let mut inner = HashMap::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                inner.insert(k, v);
            }
        }
        ImmutableI32I64HashMap { inner }
    }
    pub fn any_satisfy(&self, predicate: impl Fn(i32, i64) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    pub fn all_satisfy(&self, predicate: impl Fn(i32, i64) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    pub fn to_mutable(&self) -> I32I64HashMap {
        let mut m = I32I64HashMap::new();
        for (k, v) in self.iter() {
            m.insert(k, v);
        }
        m
    }
}
impl PartialEq for ImmutableI32I64HashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}
impl fmt::Display for ImmutableI32I64HashMap {
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
        let mut m = I32I64HashMap::new();
        m.insert(1, 1);
        m.insert(2, 2);
        let im = ImmutableI32I64HashMap::from_mutable(&m);
        assert_eq!(im.get(1), Some(1));
        assert_eq!(im.len(), 2);
        assert_eq!(im.get(99), None);
    }
    #[test]
    fn test_contains_key() {
        let mut m = I32I64HashMap::new();
        m.insert(1, 1);
        let im = ImmutableI32I64HashMap::from_mutable(&m);
        assert!(im.contains_key(1));
    }
    #[test]
    fn test_to_mutable_independent() {
        let mut m = I32I64HashMap::new();
        m.insert(1, 1);
        let im = ImmutableI32I64HashMap::from_mutable(&m);
        let mut m2 = im.to_mutable();
        m2.insert(2, 2);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        let mut m = I32I64HashMap::new();
        m.insert(1, 1);
        assert!(!ImmutableI32I64HashMap::from_mutable(&m)
            .to_string()
            .is_empty());
    }
}

impl crate::traits::i32_i64_map::I32I64Map for ImmutableI32I64HashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: i32) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: i32) -> Option<i64> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (i32, i64)> + '_ {
        self.iter()
    }
}
