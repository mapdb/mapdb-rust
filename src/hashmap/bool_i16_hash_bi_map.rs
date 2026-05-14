// AUTO-GENERATED. DO NOT EDIT.

use crate::hash_table::OpenHashMap;
use crate::hashmap::i16_bool_hash_bi_map::I16BoolHashBiMap;
use std::fmt;

/// Bidirectional hash map from `bool` keys to `i16` values.
/// Both key->value and value->key lookups are O(1).
/// Internally uses two `OpenHashMap` instances: forward (key->value) and reverse (value->key).
#[derive(Debug, Clone)]
pub struct BoolI16HashBiMap {
    forward: OpenHashMap<bool, i16>,
    reverse: OpenHashMap<i16, bool>,
}

impl BoolI16HashBiMap {
    pub fn new() -> Self {
        BoolI16HashBiMap {
            forward: OpenHashMap::new(),
            reverse: OpenHashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        BoolI16HashBiMap {
            forward: OpenHashMap::with_capacity(capacity),
            reverse: OpenHashMap::with_capacity(capacity),
        }
    }

    /// Inserts a key-value pair into the bi-map.
    /// If the key already existed, the old value is removed from the reverse map
    /// and returned. The new value->key mapping is then inserted.
    pub fn insert(&mut self, key: bool, value: i16) -> Option<i16> {
        // If this key already maps to an old value, remove old_value->key from reverse
        let old = self.forward.insert(key, value);
        if let Some(old_val) = old {
            self.reverse.remove(old_val);
        }
        // If this value already maps to an old key, remove old_key->value from forward
        if let Some(old_key) = self.reverse.insert(value, key) {
            if old_key != key {
                self.forward.remove(old_key);
            }
        }
        old
    }

    /// Forward lookup: returns the value for the given key.
    pub fn get(&self, key: bool) -> Option<i16> {
        self.forward.get(key)
    }

    /// Reverse lookup: returns the key for the given value.
    pub fn get_key(&self, value: i16) -> Option<bool> {
        self.reverse.get(value)
    }

    /// Removes by key. Returns the old value if present, and also removes
    /// the corresponding reverse mapping.
    pub fn remove(&mut self, key: bool) -> Option<i16> {
        if let Some(val) = self.forward.remove(key) {
            self.reverse.remove(val);
            Some(val)
        } else {
            None
        }
    }

    /// Removes by value. Returns the old key if present, and also removes
    /// the corresponding forward mapping.
    pub fn remove_value(&mut self, value: i16) -> Option<bool> {
        if let Some(key) = self.reverse.remove(value) {
            self.forward.remove(key);
            Some(key)
        } else {
            None
        }
    }

    pub fn contains_key(&self, key: bool) -> bool {
        self.forward.contains_key(key)
    }

    pub fn contains_value(&self, value: i16) -> bool {
        self.reverse.contains_key(value)
    }

    pub fn len(&self) -> usize {
        self.forward.len()
    }
    pub fn is_empty(&self) -> bool {
        self.forward.is_empty()
    }

    pub fn clear(&mut self) {
        self.forward.clear();
        self.reverse.clear();
    }

    /// Ensures both the forward and reverse maps can accept `additional`
    /// more entries without a rehash. Returns `TryReserveError` on
    /// allocator failure. See `docs/rust/error-handling.md`.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.forward.try_reserve(additional)?;
        self.reverse.try_reserve(additional)?;
        Ok(())
    }

    /// Iterates over (key, value) pairs via the forward map.
    pub fn iter(&self) -> impl Iterator<Item = (bool, i16)> + '_ {
        self.forward.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = bool> + '_ {
        self.forward.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = i16> + '_ {
        self.forward.iter().map(|(_, v)| v)
    }

    pub fn for_each(&self, mut f: impl FnMut(bool, i16)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    /// Returns a new BiMap with forward and reverse swapped.
    /// The returned map has value->key as forward and key->value as reverse.
    pub fn inverse(&self) -> I16BoolHashBiMap {
        I16BoolHashBiMap::from_maps(self.reverse.clone(), self.forward.clone())
    }

    /// Internal constructor from pre-built maps (used by the inverse type's `inverse()` call).
    pub(crate) fn from_maps(
        forward: OpenHashMap<bool, i16>,
        reverse: OpenHashMap<i16, bool>,
    ) -> Self {
        BoolI16HashBiMap { forward, reverse }
    }
}

impl Default for BoolI16HashBiMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for BoolI16HashBiMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}

impl fmt::Display for BoolI16HashBiMap {
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
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        m.insert(false, 2);
        assert_eq!(m.get(true), Some(1));
        assert_eq!(m.get(false), Some(2));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_get_key() {
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        m.insert(false, 2);
        assert_eq!(m.get_key(1), Some(true));
        assert_eq!(m.get_key(2), Some(false));
        assert_eq!(m.get_key(99), None);
    }

    #[test]
    fn test_insert_overwrite() {
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        let old = m.insert(true, 2);
        assert_eq!(old, Some(1));
        assert_eq!(m.get(true), Some(2));
        // Reverse should reflect the new mapping
        assert_eq!(m.get_key(2), Some(true));
        // Old reverse mapping should be gone
        assert_eq!(m.get_key(1), None);
    }

    #[test]
    fn test_remove() {
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        m.insert(false, 2);
        assert_eq!(m.remove(true), Some(1));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key(true));
        assert!(!m.contains_value(1));
    }

    #[test]
    fn test_remove_value() {
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        m.insert(false, 2);
        assert_eq!(m.remove_value(1), Some(true));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key(true));
        assert!(!m.contains_value(1));
    }

    #[test]
    fn test_contains() {
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        assert!(m.contains_key(true));
        assert!(!m.contains_key(false));
        assert!(m.contains_value(1));
        assert!(!m.contains_value(99));
    }

    #[test]
    fn test_inverse() {
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        m.insert(false, 2);
        let inv = m.inverse();
        // inverse forward lookup: old value -> old key
        assert_eq!(inv.get(1), Some(true));
        assert_eq!(inv.get(2), Some(false));
        // inverse reverse lookup: old key -> old value
        assert_eq!(inv.get_key(true), Some(1));
        assert_eq!(inv.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn test_display() {
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        let s = m.to_string();
        assert!(!s.is_empty());
        assert!(s.starts_with('{'));
        assert!(s.ends_with('}'));
    }

    #[test]
    fn test_for_each() {
        let mut m = BoolI16HashBiMap::new();
        m.insert(true, 1);
        m.insert(false, 2);
        let mut count = 0usize;
        m.for_each(|_k, _v| {
            count += 1;
        });
        assert_eq!(count, 2);
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = BoolI16HashBiMap::new();
        m.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut m = BoolI16HashBiMap::new();
        assert!(m.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::bool_i16_map::BoolI16Map for BoolI16HashBiMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: bool) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: bool) -> Option<i16> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (bool, i16)> + '_ {
        self.iter()
    }
}

impl crate::traits::bool_i16_map::BoolI16MutableMap for BoolI16HashBiMap {
    fn insert(&mut self, key: bool, value: i16) -> Option<i16> {
        self.insert(key, value)
    }
    fn remove(&mut self, key: bool) -> Option<i16> {
        self.remove(key)
    }
    fn clear(&mut self) {
        self.clear()
    }
}
