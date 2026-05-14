// AUTO-GENERATED. DO NOT EDIT.
use crate::hashmap::i8_i8_hash_map::I8I8HashMap;
use std::collections::HashMap;
use std::fmt;

/// Immutable hash map from `i8` keys to `i8` values. Clone is cheap (shared data).
#[derive(Debug, Clone)]
pub struct ImmutableI8I8HashMap {
    inner: HashMap<i8, i8>,
}

impl ImmutableI8I8HashMap {
    pub fn from_mutable(m: &I8I8HashMap) -> Self {
        let mut inner = HashMap::new();
        m.for_each(|k, v| {
            inner.insert(k, v);
        });
        ImmutableI8I8HashMap { inner }
    }
    pub fn get(&self, key: i8) -> Option<i8> {
        self.inner.get(&(key)).copied()
    }
    pub fn get_or_default(&self, key: i8, default: i8) -> i8 {
        self.get(key).unwrap_or(default)
    }
    pub fn contains_key(&self, key: i8) -> bool {
        self.inner.contains_key(&(key))
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (i8, i8)> + '_ {
        self.inner.iter().map(|(k, &v)| (*k, v))
    }
    pub fn keys(&self) -> impl Iterator<Item = i8> + '_ {
        self.inner.keys().copied()
    }
    pub fn values(&self) -> impl Iterator<Item = i8> + '_ {
        self.inner.values().copied()
    }
    pub fn for_each(&self, mut f: impl FnMut(i8, i8)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    pub fn select(&self, predicate: impl Fn(i8, i8) -> bool) -> Self {
        let mut inner = HashMap::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                inner.insert(k, v);
            }
        }
        ImmutableI8I8HashMap { inner }
    }
    pub fn any_satisfy(&self, predicate: impl Fn(i8, i8) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    pub fn all_satisfy(&self, predicate: impl Fn(i8, i8) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    pub fn to_mutable(&self) -> I8I8HashMap {
        let mut m = I8I8HashMap::new();
        for (k, v) in self.iter() {
            m.insert(k, v);
        }
        m
    }
}
impl PartialEq for ImmutableI8I8HashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}
impl fmt::Display for ImmutableI8I8HashMap {
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
        let mut m = I8I8HashMap::new();
        m.insert(1, 1);
        m.insert(2, 2);
        let im = ImmutableI8I8HashMap::from_mutable(&m);
        assert_eq!(im.get(1), Some(1));
        assert_eq!(im.len(), 2);
        assert_eq!(im.get(99), None);
    }
    #[test]
    fn test_contains_key() {
        let mut m = I8I8HashMap::new();
        m.insert(1, 1);
        let im = ImmutableI8I8HashMap::from_mutable(&m);
        assert!(im.contains_key(1));
    }
    #[test]
    fn test_to_mutable_independent() {
        let mut m = I8I8HashMap::new();
        m.insert(1, 1);
        let im = ImmutableI8I8HashMap::from_mutable(&m);
        let mut m2 = im.to_mutable();
        m2.insert(2, 2);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        let mut m = I8I8HashMap::new();
        m.insert(1, 1);
        assert!(!ImmutableI8I8HashMap::from_mutable(&m)
            .to_string()
            .is_empty());
    }
}

impl crate::traits::i8_i8_map::I8I8Map for ImmutableI8I8HashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: i8) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: i8) -> Option<i8> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (i8, i8)> + '_ {
        self.iter()
    }
}
