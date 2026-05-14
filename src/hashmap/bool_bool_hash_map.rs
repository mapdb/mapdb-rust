// AUTO-GENERATED. DO NOT EDIT.

use crate::hash_table::OpenHashMap;
use std::fmt;

/// Hash map from `bool` keys to `bool` values.
/// Open-addressing with linear probing and Robin Hood backward-shift deletion.
#[derive(Debug, Clone)]
pub struct BoolBoolHashMap {
    inner: OpenHashMap<bool, bool>,
}

impl BoolBoolHashMap {
    pub fn new() -> Self {
        BoolBoolHashMap {
            inner: OpenHashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        BoolBoolHashMap {
            inner: OpenHashMap::with_capacity(capacity),
        }
    }

    /// Inserts a key-value pair. Returns the old value if the key was already present.
    pub fn insert(&mut self, key: bool, value: bool) -> Option<bool> {
        self.inner.insert(key, value)
    }

    /// Returns the value for the key, or None.
    pub fn get(&self, key: bool) -> Option<bool> {
        self.inner.get(key)
    }

    /// Returns the value for the key, or the default.
    pub fn get_or_default(&self, key: bool, default: bool) -> bool {
        self.inner.get(key).unwrap_or(default)
    }

    /// Removes the key. Returns the old value if present.
    pub fn remove(&mut self, key: bool) -> Option<bool> {
        self.inner.remove(key)
    }

    pub fn contains_key(&self, key: bool) -> bool {
        self.inner.contains_key(key)
    }

    pub fn contains_value(&self, value: bool) -> bool {
        self.inner.iter().any(|(_, v)| v == value)
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
    pub fn iter(&self) -> impl Iterator<Item = (bool, bool)> + '_ {
        self.inner.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.iter().map(|(_, v)| v)
    }

    pub fn for_each(&self, mut f: impl FnMut(bool, bool)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    pub fn select(&self, predicate: impl Fn(bool, bool) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                result.insert(k, v);
            }
        }
        result
    }

    pub fn reject(&self, predicate: impl Fn(bool, bool) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if !predicate(k, v) {
                result.insert(k, v);
            }
        }
        result
    }

    pub fn detect(&self, predicate: impl Fn(bool, bool) -> bool) -> Option<(bool, bool)> {
        self.iter().find(|&(k, v)| predicate(k, v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(bool, bool) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(bool, bool) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }

    pub fn none_satisfy(&self, predicate: impl Fn(bool, bool) -> bool) -> bool {
        !self.iter().any(|(k, v)| predicate(k, v))
    }

    pub fn count(&self, predicate: impl Fn(bool, bool) -> bool) -> usize {
        self.iter().filter(|&(k, v)| predicate(k, v)).count()
    }

    pub fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, bool, bool) -> R) -> R {
        let mut acc = initial;
        for (k, v) in self.iter() {
            acc = f(acc, k, v);
        }
        acc
    }

    pub fn keys_to_vec(&self) -> Vec<bool> {
        self.keys().collect()
    }
    pub fn values_to_vec(&self) -> Vec<bool> {
        self.values().collect()
    }

    pub fn with_key_value(mut self, key: bool, value: bool) -> Self {
        self.insert(key, value);
        self
    }

    pub fn without_key(mut self, key: bool) -> Self {
        self.remove(key);
        self
    }
}

impl Default for BoolBoolHashMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for BoolBoolHashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}

impl fmt::Display for BoolBoolHashMap {
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
        let mut m = BoolBoolHashMap::new();
        m.insert(true, true);
        m.insert(false, false);
        assert_eq!(m.get(true), Some(true));

        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_insert_overwrite() {
        let mut m = BoolBoolHashMap::new();
        m.insert(true, true);
        let old = m.insert(true, false);
        assert_eq!(old, Some(true));
        assert_eq!(m.get(true), Some(false));
    }

    #[test]
    fn test_remove() {
        let mut m = BoolBoolHashMap::new();
        m.insert(true, true);
        m.insert(false, false);
        assert_eq!(m.remove(true), Some(true));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key(true));
    }

    #[test]
    fn test_contains_key() {
        let mut m = BoolBoolHashMap::new();
        m.insert(true, true);
        assert!(m.contains_key(true));
        assert!(!m.contains_key(false));
    }

    #[test]
    fn test_get_or_default() {
        let mut m = BoolBoolHashMap::new();
        m.insert(true, true);
        assert_eq!(m.get_or_default(true, true), true);
        assert_eq!(m.get_or_default(false, true), true);
    }

    #[test]
    fn test_clear() {
        let mut m = BoolBoolHashMap::new();
        m.insert(true, true);
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_select_reject() {
        let mut m = BoolBoolHashMap::new();
        m.insert(true, true);
        m.insert(false, false);
        let sel = m.select(|_k, v| v == true);
        assert!(sel.len() >= 1);
    }

    #[test]
    fn test_any_all_none() {
        let mut m = BoolBoolHashMap::new();
        m.insert(true, true);
        assert!(m.any_satisfy(|_k, v| v == true));
    }

    #[test]
    fn test_equals() {
        let mut m1 = BoolBoolHashMap::new();
        m1.insert(true, true);
        m1.insert(false, false);
        let mut m2 = BoolBoolHashMap::new();
        m2.insert(false, false);
        m2.insert(true, true);
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_display() {
        let mut m = BoolBoolHashMap::new();
        m.insert(true, true);
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_resize() {
        let mut m = BoolBoolHashMap::new();
        for i in 0..100usize {
            m.insert(i % 2 == 0, i % 3 == 0);
        }
        assert!(m.len() > 0);
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = BoolBoolHashMap::new();
        m.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut m = BoolBoolHashMap::new();
        assert!(m.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::bool_bool_map::BoolBoolMap for BoolBoolHashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: bool) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: bool) -> Option<bool> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (bool, bool)> + '_ {
        self.iter()
    }
}

impl crate::traits::bool_bool_map::BoolBoolMutableMap for BoolBoolHashMap {
    fn insert(&mut self, key: bool, value: bool) -> Option<bool> {
        self.insert(key, value)
    }
    fn remove(&mut self, key: bool) -> Option<bool> {
        self.remove(key)
    }
    fn clear(&mut self) {
        self.clear()
    }
}
