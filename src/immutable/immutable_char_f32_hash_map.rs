// AUTO-GENERATED. DO NOT EDIT.
use crate::hashmap::char_f32_hash_map::CharF32HashMap;
use std::collections::HashMap;
use std::fmt;

/// Immutable hash map from `char` keys to `f32` values. Clone is cheap (shared data).
#[derive(Debug, Clone)]
pub struct ImmutableCharF32HashMap {
    inner: HashMap<char, f32>,
}

impl ImmutableCharF32HashMap {
    pub fn from_mutable(m: &CharF32HashMap) -> Self {
        let mut inner = HashMap::new();
        m.for_each(|k, v| {
            inner.insert(k, v);
        });
        ImmutableCharF32HashMap { inner }
    }
    pub fn get(&self, key: char) -> Option<f32> {
        self.inner.get(&(key)).copied()
    }
    pub fn get_or_default(&self, key: char, default: f32) -> f32 {
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
    pub fn iter(&self) -> impl Iterator<Item = (char, f32)> + '_ {
        self.inner.iter().map(|(k, &v)| (*k, v))
    }
    pub fn keys(&self) -> impl Iterator<Item = char> + '_ {
        self.inner.keys().copied()
    }
    pub fn values(&self) -> impl Iterator<Item = f32> + '_ {
        self.inner.values().copied()
    }
    pub fn for_each(&self, mut f: impl FnMut(char, f32)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    pub fn select(&self, predicate: impl Fn(char, f32) -> bool) -> Self {
        let mut inner = HashMap::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                inner.insert(k, v);
            }
        }
        ImmutableCharF32HashMap { inner }
    }
    pub fn any_satisfy(&self, predicate: impl Fn(char, f32) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    pub fn all_satisfy(&self, predicate: impl Fn(char, f32) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    pub fn to_mutable(&self) -> CharF32HashMap {
        let mut m = CharF32HashMap::new();
        for (k, v) in self.iter() {
            m.insert(k, v);
        }
        m
    }
}
impl PartialEq for ImmutableCharF32HashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v.to_bits() == ov.to_bits()))
    }
}
impl fmt::Display for ImmutableCharF32HashMap {
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
        let mut m = CharF32HashMap::new();
        m.insert('a', 1.0f32);
        m.insert('b', 2.0f32);
        let im = ImmutableCharF32HashMap::from_mutable(&m);
        assert_eq!(im.get('a'), Some(1.0f32));
        assert_eq!(im.len(), 2);
        assert_eq!(im.get('z'), None);
    }
    #[test]
    fn test_contains_key() {
        let mut m = CharF32HashMap::new();
        m.insert('a', 1.0f32);
        let im = ImmutableCharF32HashMap::from_mutable(&m);
        assert!(im.contains_key('a'));
    }
    #[test]
    fn test_to_mutable_independent() {
        let mut m = CharF32HashMap::new();
        m.insert('a', 1.0f32);
        let im = ImmutableCharF32HashMap::from_mutable(&m);
        let mut m2 = im.to_mutable();
        m2.insert('b', 2.0f32);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        let mut m = CharF32HashMap::new();
        m.insert('a', 1.0f32);
        assert!(!ImmutableCharF32HashMap::from_mutable(&m)
            .to_string()
            .is_empty());
    }
}

impl crate::traits::char_f32_map::CharF32Map for ImmutableCharF32HashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: char) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: char) -> Option<f32> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (char, f32)> + '_ {
        self.iter()
    }
}
