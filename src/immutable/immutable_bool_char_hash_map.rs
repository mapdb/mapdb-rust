// AUTO-GENERATED. DO NOT EDIT.
use crate::hashmap::bool_char_hash_map::BoolCharHashMap;
use std::collections::HashMap;
use std::fmt;

/// Immutable hash map from `bool` keys to `char` values. Clone is cheap (shared data).
#[derive(Debug, Clone)]
pub struct ImmutableBoolCharHashMap {
    inner: HashMap<bool, char>,
}

impl ImmutableBoolCharHashMap {
    pub fn from_mutable(m: &BoolCharHashMap) -> Self {
        let mut inner = HashMap::new();
        m.for_each(|k, v| {
            inner.insert(k, v);
        });
        ImmutableBoolCharHashMap { inner }
    }
    pub fn get(&self, key: bool) -> Option<char> {
        self.inner.get(&(key)).copied()
    }
    pub fn get_or_default(&self, key: bool, default: char) -> char {
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
    pub fn iter(&self) -> impl Iterator<Item = (bool, char)> + '_ {
        self.inner.iter().map(|(k, &v)| (*k, v))
    }
    pub fn keys(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.keys().copied()
    }
    pub fn values(&self) -> impl Iterator<Item = char> + '_ {
        self.inner.values().copied()
    }
    pub fn for_each(&self, mut f: impl FnMut(bool, char)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    pub fn select(&self, predicate: impl Fn(bool, char) -> bool) -> Self {
        let mut inner = HashMap::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                inner.insert(k, v);
            }
        }
        ImmutableBoolCharHashMap { inner }
    }
    pub fn any_satisfy(&self, predicate: impl Fn(bool, char) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    pub fn all_satisfy(&self, predicate: impl Fn(bool, char) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    pub fn to_mutable(&self) -> BoolCharHashMap {
        let mut m = BoolCharHashMap::new();
        for (k, v) in self.iter() {
            m.insert(k, v);
        }
        m
    }
}
impl PartialEq for ImmutableBoolCharHashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}
impl fmt::Display for ImmutableBoolCharHashMap {
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
        let mut m = BoolCharHashMap::new();
        m.insert(true, 'a');
        m.insert(false, 'b');
        let im = ImmutableBoolCharHashMap::from_mutable(&m);
        assert_eq!(im.get(true), Some('a'));
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_contains_key() {
        let mut m = BoolCharHashMap::new();
        m.insert(true, 'a');
        let im = ImmutableBoolCharHashMap::from_mutable(&m);
        assert!(im.contains_key(true));
    }
    #[test]
    fn test_to_mutable_independent() {
        let mut m = BoolCharHashMap::new();
        m.insert(true, 'a');
        let im = ImmutableBoolCharHashMap::from_mutable(&m);
        let mut m2 = im.to_mutable();
        m2.insert(false, 'b');
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        let mut m = BoolCharHashMap::new();
        m.insert(true, 'a');
        assert!(!ImmutableBoolCharHashMap::from_mutable(&m)
            .to_string()
            .is_empty());
    }
}

impl crate::traits::bool_char_map::BoolCharMap for ImmutableBoolCharHashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: bool) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: bool) -> Option<char> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (bool, char)> + '_ {
        self.iter()
    }
}
