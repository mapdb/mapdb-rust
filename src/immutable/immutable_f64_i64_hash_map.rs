// AUTO-GENERATED. DO NOT EDIT.
use crate::hashmap::f64_i64_hash_map::F64I64HashMap;
use std::collections::HashMap;
use std::fmt;

/// Immutable hash map from `f64` keys to `i64` values. Clone is cheap (shared data).
#[derive(Debug, Clone)]
pub struct ImmutableF64I64HashMap {
    inner: HashMap<u64, i64>,
}

impl ImmutableF64I64HashMap {
    pub fn from_mutable(m: &F64I64HashMap) -> Self {
        let mut inner = HashMap::new();
        m.for_each(|k, v| {
            inner.insert(k.to_bits(), v);
        });
        ImmutableF64I64HashMap { inner }
    }
    pub fn get(&self, key: f64) -> Option<i64> {
        self.inner.get(&(key.to_bits())).copied()
    }
    pub fn get_or_default(&self, key: f64, default: i64) -> i64 {
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
    pub fn iter(&self) -> impl Iterator<Item = (f64, i64)> + '_ {
        self.inner.iter().map(|(k, &v)| (f64::from_bits(*k), v))
    }
    pub fn keys(&self) -> impl Iterator<Item = f64> + '_ {
        self.inner.keys().map(|k| f64::from_bits(*k))
    }
    pub fn values(&self) -> impl Iterator<Item = i64> + '_ {
        self.inner.values().copied()
    }
    pub fn for_each(&self, mut f: impl FnMut(f64, i64)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    pub fn select(&self, predicate: impl Fn(f64, i64) -> bool) -> Self {
        let mut inner = HashMap::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                inner.insert(k.to_bits(), v);
            }
        }
        ImmutableF64I64HashMap { inner }
    }
    pub fn any_satisfy(&self, predicate: impl Fn(f64, i64) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    pub fn all_satisfy(&self, predicate: impl Fn(f64, i64) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    pub fn to_mutable(&self) -> F64I64HashMap {
        let mut m = F64I64HashMap::new();
        for (k, v) in self.iter() {
            m.insert(k, v);
        }
        m
    }
}
impl PartialEq for ImmutableF64I64HashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}
impl fmt::Display for ImmutableF64I64HashMap {
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
        let mut m = F64I64HashMap::new();
        m.insert(1.0f64, 1);
        m.insert(2.0f64, 2);
        let im = ImmutableF64I64HashMap::from_mutable(&m);
        assert_eq!(im.get(1.0f64), Some(1));
        assert_eq!(im.len(), 2);
        assert_eq!(im.get(99.0f64), None);
    }
    #[test]
    fn test_contains_key() {
        let mut m = F64I64HashMap::new();
        m.insert(1.0f64, 1);
        let im = ImmutableF64I64HashMap::from_mutable(&m);
        assert!(im.contains_key(1.0f64));
    }
    #[test]
    fn test_to_mutable_independent() {
        let mut m = F64I64HashMap::new();
        m.insert(1.0f64, 1);
        let im = ImmutableF64I64HashMap::from_mutable(&m);
        let mut m2 = im.to_mutable();
        m2.insert(2.0f64, 2);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        let mut m = F64I64HashMap::new();
        m.insert(1.0f64, 1);
        assert!(!ImmutableF64I64HashMap::from_mutable(&m)
            .to_string()
            .is_empty());
    }
}

impl crate::traits::f64_i64_map::F64I64Map for ImmutableF64I64HashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: f64) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: f64) -> Option<i64> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (f64, i64)> + '_ {
        self.iter()
    }
}
