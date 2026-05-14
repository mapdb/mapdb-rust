// AUTO-GENERATED. DO NOT EDIT.

use crate::hash_table::OpenHashMap;
use std::fmt;

/// Hash map from `i8` keys to `f32` values.
/// Open-addressing with linear probing and Robin Hood backward-shift deletion.
#[derive(Debug, Clone)]
pub struct I8F32HashMap {
    inner: OpenHashMap<i8, f32>,
}

impl I8F32HashMap {
    pub fn new() -> Self {
        I8F32HashMap {
            inner: OpenHashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        I8F32HashMap {
            inner: OpenHashMap::with_capacity(capacity),
        }
    }

    /// Inserts a key-value pair. Returns the old value if the key was already present.
    pub fn insert(&mut self, key: i8, value: f32) -> Option<f32> {
        self.inner.insert(key, value)
    }

    /// Returns the value for the key, or None.
    pub fn get(&self, key: i8) -> Option<f32> {
        self.inner.get(key)
    }

    /// Returns the value for the key, or the default.
    pub fn get_or_default(&self, key: i8, default: f32) -> f32 {
        self.inner.get(key).unwrap_or(default)
    }

    /// Removes the key. Returns the old value if present.
    pub fn remove(&mut self, key: i8) -> Option<f32> {
        self.inner.remove(key)
    }

    pub fn contains_key(&self, key: i8) -> bool {
        self.inner.contains_key(key)
    }

    pub fn contains_value(&self, value: f32) -> bool {
        self.inner
            .iter()
            .any(|(_, v)| v.to_bits() == value.to_bits())
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Ensures the map can accept `additional` more entries without a
    /// rehash. Returns `TryReserveError` on allocator failure. See
    /// `docs/rust/error-handling.md`.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.inner.try_reserve(additional)
    }

    /// Iterates over (key, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (i8, f32)> + '_ {
        self.inner.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = i8> + '_ {
        self.inner.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = f32> + '_ {
        self.inner.iter().map(|(_, v)| v)
    }

    pub fn for_each(&self, mut f: impl FnMut(i8, f32)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    pub fn select(&self, predicate: impl Fn(i8, f32) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                result.insert(k, v);
            }
        }
        result
    }

    pub fn reject(&self, predicate: impl Fn(i8, f32) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if !predicate(k, v) {
                result.insert(k, v);
            }
        }
        result
    }

    pub fn detect(&self, predicate: impl Fn(i8, f32) -> bool) -> Option<(i8, f32)> {
        self.iter().find(|&(k, v)| predicate(k, v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(i8, f32) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(i8, f32) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }

    pub fn none_satisfy(&self, predicate: impl Fn(i8, f32) -> bool) -> bool {
        !self.iter().any(|(k, v)| predicate(k, v))
    }

    pub fn count(&self, predicate: impl Fn(i8, f32) -> bool) -> usize {
        self.iter().filter(|&(k, v)| predicate(k, v)).count()
    }

    pub fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, i8, f32) -> R) -> R {
        let mut acc = initial;
        for (k, v) in self.iter() {
            acc = f(acc, k, v);
        }
        acc
    }

    pub fn keys_to_vec(&self) -> Vec<i8> {
        self.keys().collect()
    }
    pub fn values_to_vec(&self) -> Vec<f32> {
        self.values().collect()
    }

    pub fn with_key_value(mut self, key: i8, value: f32) -> Self {
        self.insert(key, value);
        self
    }

    pub fn without_key(mut self, key: i8) -> Self {
        self.remove(key);
        self
    }
}

impl Default for I8F32HashMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for I8F32HashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v.to_bits() == ov.to_bits()))
    }
}

impl fmt::Display for I8F32HashMap {
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
    fn test_insert_get() {
        let mut m = I8F32HashMap::new();
        m.insert(1, 1.0f32);
        m.insert(2, 2.0f32);
        m.insert(3, 3.0f32);
        assert_eq!(m.get(1), Some(1.0f32));
        assert_eq!(m.get(99), None);
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn test_insert_overwrite() {
        let mut m = I8F32HashMap::new();
        m.insert(1, 1.0f32);
        let old = m.insert(1, 2.0f32);
        assert_eq!(old, Some(1.0f32));
        assert_eq!(m.get(1), Some(2.0f32));
    }

    #[test]
    fn test_remove() {
        let mut m = I8F32HashMap::new();
        m.insert(1, 1.0f32);
        m.insert(2, 2.0f32);
        assert_eq!(m.remove(1), Some(1.0f32));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key(1));
    }

    #[test]
    fn test_contains_key() {
        let mut m = I8F32HashMap::new();
        m.insert(1, 1.0f32);
        assert!(m.contains_key(1));
        assert!(!m.contains_key(99));
    }

    #[test]
    fn test_get_or_default() {
        let mut m = I8F32HashMap::new();
        m.insert(1, 1.0f32);
        assert_eq!(m.get_or_default(1, 3.0f32), 1.0f32);
        assert_eq!(m.get_or_default(99, 3.0f32), 3.0f32);
    }

    #[test]
    fn test_clear() {
        let mut m = I8F32HashMap::new();
        m.insert(1, 1.0f32);
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_select_reject() {
        let mut m = I8F32HashMap::new();
        m.insert(1, 1.0f32);
        m.insert(2, 2.0f32);
        m.insert(3, 3.0f32);
        assert_eq!(m.select(|_k, v| v > 1.0f32).len(), 2);
        assert_eq!(m.reject(|_k, v| v > 1.0f32).len(), 1);
    }

    #[test]
    fn test_any_all_none() {
        let mut m = I8F32HashMap::new();
        m.insert(1, 1.0f32);
        m.insert(2, 2.0f32);
        assert!(m.any_satisfy(|_k, v| v == 2.0f32));
        assert!(m.all_satisfy(|_k, v| v > 0.0));
        assert!(m.none_satisfy(|_k, v| v == 99.0f32));
    }

    #[test]
    fn test_equals() {
        let mut m1 = I8F32HashMap::new();
        m1.insert(1, 1.0f32);
        m1.insert(2, 2.0f32);
        let mut m2 = I8F32HashMap::new();
        m2.insert(2, 2.0f32);
        m2.insert(1, 1.0f32);
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_display() {
        let mut m = I8F32HashMap::new();
        m.insert(1, 1.0f32);
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_resize() {
        let mut m = I8F32HashMap::new();
        for i in 0..100 {
            m.insert(i as i8, (i * 10) as f32);
        }
        assert!(m.len() > 0);
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = I8F32HashMap::new();
        m.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut m = I8F32HashMap::new();
        assert!(m.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::i8_f32_map::I8F32Map for I8F32HashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: i8) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: i8) -> Option<f32> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (i8, f32)> + '_ {
        self.iter()
    }
}

impl crate::traits::i8_f32_map::I8F32MutableMap for I8F32HashMap {
    fn insert(&mut self, key: i8, value: f32) -> Option<f32> {
        self.insert(key, value)
    }
    fn remove(&mut self, key: i8) -> Option<f32> {
        self.remove(key)
    }
    fn clear(&mut self) {
        self.clear()
    }
}
