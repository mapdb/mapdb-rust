// AUTO-GENERATED. DO NOT EDIT.
use crate::hashmap::char_bool_hash_map::CharBoolHashMap;
use std::collections::HashMap;
use std::fmt;

/// Immutable hash map from `char` keys to `bool` values. Clone is cheap (shared data).
#[derive(Debug, Clone)]
pub struct ImmutableCharBoolHashMap {
    inner: HashMap<char, bool>,
}

impl ImmutableCharBoolHashMap {
    pub fn from_mutable(m: &CharBoolHashMap) -> Self {
        let mut inner = HashMap::new();
        m.for_each(|k, v| {
            inner.insert(k, v);
        });
        ImmutableCharBoolHashMap { inner }
    }
    pub fn get(&self, key: char) -> Option<bool> {
        self.inner.get(&(key)).copied()
    }
    pub fn get_or_default(&self, key: char, default: bool) -> bool {
        self.get(key).unwrap_or(default)
    }
    pub fn contains_key(&self, key: char) -> bool {
        self.inner.contains_key(&(key))
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (char, bool)> + '_ {
        self.inner.iter().map(|(k, &v)| (*k, v))
    }
    pub fn keys(&self) -> impl Iterator<Item = char> + '_ {
        self.inner.keys().copied()
    }
    pub fn values(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.values().copied()
    }
    pub fn for_each(&self, mut f: impl FnMut(char, bool)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    pub fn select(&self, predicate: impl Fn(char, bool) -> bool) -> Self {
        let mut inner = HashMap::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                inner.insert(k, v);
            }
        }
        ImmutableCharBoolHashMap { inner }
    }
    pub fn any_satisfy(&self, predicate: impl Fn(char, bool) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    pub fn all_satisfy(&self, predicate: impl Fn(char, bool) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    pub fn to_mutable(&self) -> CharBoolHashMap {
        let mut m = CharBoolHashMap::new();
        for (k, v) in self.iter() {
            m.insert(k, v);
        }
        m
    }
}
impl PartialEq for ImmutableCharBoolHashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}
impl fmt::Display for ImmutableCharBoolHashMap {
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
        let mut m = CharBoolHashMap::new();
        m.insert('a', true);
        m.insert('b', false);
        let im = ImmutableCharBoolHashMap::from_mutable(&m);
        assert_eq!(im.get('a'), Some(true));
        assert_eq!(im.len(), 2);
        assert_eq!(im.get('z'), None);
    }
    #[test]
    fn test_contains_key() {
        let mut m = CharBoolHashMap::new();
        m.insert('a', true);
        let im = ImmutableCharBoolHashMap::from_mutable(&m);
        assert!(im.contains_key('a'));
    }
    #[test]
    fn test_to_mutable_independent() {
        let mut m = CharBoolHashMap::new();
        m.insert('a', true);
        let im = ImmutableCharBoolHashMap::from_mutable(&m);
        let mut m2 = im.to_mutable();
        m2.insert('b', false);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        let mut m = CharBoolHashMap::new();
        m.insert('a', true);
        assert!(!ImmutableCharBoolHashMap::from_mutable(&m)
            .to_string()
            .is_empty());
    }
}

impl crate::traits::char_bool_map::CharBoolMap for ImmutableCharBoolHashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: char) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: char) -> Option<bool> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (char, bool)> + '_ {
        self.iter()
    }
}
