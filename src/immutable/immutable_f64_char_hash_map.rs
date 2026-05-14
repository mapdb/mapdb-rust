// AUTO-GENERATED. DO NOT EDIT.
use crate::hashmap::f64_char_hash_map::F64CharHashMap;
use std::collections::HashMap;
use std::fmt;

/// Immutable hash map from `f64` keys to `char` values. Clone is cheap (shared data).
#[derive(Debug, Clone)]
pub struct ImmutableF64CharHashMap {
    inner: HashMap<u64, char>,
}

impl ImmutableF64CharHashMap {
    pub fn from_mutable(m: &F64CharHashMap) -> Self {
        let mut inner = HashMap::new();
        m.for_each(|k, v| {
            inner.insert(k.to_bits(), v);
        });
        ImmutableF64CharHashMap { inner }
    }
    pub fn get(&self, key: f64) -> Option<char> {
        self.inner.get(&(key.to_bits())).copied()
    }
    pub fn get_or_default(&self, key: f64, default: char) -> char {
        self.get(key).unwrap_or(default)
    }
    pub fn contains_key(&self, key: f64) -> bool {
        self.inner.contains_key(&(key.to_bits()))
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (f64, char)> + '_ {
        self.inner.iter().map(|(k, &v)| (f64::from_bits(*k), v))
    }
    pub fn keys(&self) -> impl Iterator<Item = f64> + '_ {
        self.inner.keys().map(|k| f64::from_bits(*k))
    }
    pub fn values(&self) -> impl Iterator<Item = char> + '_ {
        self.inner.values().copied()
    }
    pub fn for_each(&self, mut f: impl FnMut(f64, char)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    pub fn select(&self, predicate: impl Fn(f64, char) -> bool) -> Self {
        let mut inner = HashMap::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                inner.insert(k.to_bits(), v);
            }
        }
        ImmutableF64CharHashMap { inner }
    }
    pub fn any_satisfy(&self, predicate: impl Fn(f64, char) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    pub fn all_satisfy(&self, predicate: impl Fn(f64, char) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    pub fn to_mutable(&self) -> F64CharHashMap {
        let mut m = F64CharHashMap::new();
        for (k, v) in self.iter() {
            m.insert(k, v);
        }
        m
    }
}
impl PartialEq for ImmutableF64CharHashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}
impl fmt::Display for ImmutableF64CharHashMap {
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
        let mut m = F64CharHashMap::new();
        m.insert(1.0f64, 'a');
        m.insert(2.0f64, 'b');
        let im = ImmutableF64CharHashMap::from_mutable(&m);
        assert_eq!(im.get(1.0f64), Some('a'));
        assert_eq!(im.len(), 2);
        assert_eq!(im.get(99.0f64), None);
    }
    #[test]
    fn test_contains_key() {
        let mut m = F64CharHashMap::new();
        m.insert(1.0f64, 'a');
        let im = ImmutableF64CharHashMap::from_mutable(&m);
        assert!(im.contains_key(1.0f64));
    }
    #[test]
    fn test_to_mutable_independent() {
        let mut m = F64CharHashMap::new();
        m.insert(1.0f64, 'a');
        let im = ImmutableF64CharHashMap::from_mutable(&m);
        let mut m2 = im.to_mutable();
        m2.insert(2.0f64, 'b');
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        let mut m = F64CharHashMap::new();
        m.insert(1.0f64, 'a');
        assert!(!ImmutableF64CharHashMap::from_mutable(&m)
            .to_string()
            .is_empty());
    }
}

impl crate::traits::f64_char_map::F64CharMap for ImmutableF64CharHashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: f64) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: f64) -> Option<char> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (f64, char)> + '_ {
        self.iter()
    }
}
