// AUTO-GENERATED. DO NOT EDIT.

use crate::hash_table::OpenHashMap;
use crate::hashmap::char_f32_hash_bi_map::CharF32HashBiMap;
use std::fmt;

/// Bidirectional hash map from `f32` keys to `char` values.
/// Both key->value and value->key lookups are O(1).
/// Internally uses two `OpenHashMap` instances: forward (key->value) and reverse (value->key).
#[derive(Debug, Clone)]
pub struct F32CharHashBiMap {
    forward: OpenHashMap<f32, char>,
    reverse: OpenHashMap<char, f32>,
}

impl F32CharHashBiMap {
    pub fn new() -> Self {
        F32CharHashBiMap {
            forward: OpenHashMap::new(),
            reverse: OpenHashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        F32CharHashBiMap {
            forward: OpenHashMap::with_capacity(capacity),
            reverse: OpenHashMap::with_capacity(capacity),
        }
    }

    /// Inserts a key-value pair into the bi-map.
    /// If the key already existed, the old value is removed from the reverse map
    /// and returned. The new value->key mapping is then inserted.
    pub fn insert(&mut self, key: f32, value: char) -> Option<char> {
        // If this key already maps to an old value, remove old_value->key from reverse
        let old = self.forward.insert(key, value);
        if let Some(old_val) = old {
            self.reverse.remove(old_val);
        }
        // If this value already maps to an old key, remove old_key->value from forward
        if let Some(old_key) = self.reverse.insert(value, key) {
            if old_key.to_bits() != key.to_bits() {
                self.forward.remove(old_key);
            }
        }
        old
    }

    /// Forward lookup: returns the value for the given key.
    pub fn get(&self, key: f32) -> Option<char> {
        self.forward.get(key)
    }

    /// Reverse lookup: returns the key for the given value.
    pub fn get_key(&self, value: char) -> Option<f32> {
        self.reverse.get(value)
    }

    /// Removes by key. Returns the old value if present, and also removes
    /// the corresponding reverse mapping.
    pub fn remove(&mut self, key: f32) -> Option<char> {
        if let Some(val) = self.forward.remove(key) {
            self.reverse.remove(val);
            Some(val)
        } else {
            None
        }
    }

    /// Removes by value. Returns the old key if present, and also removes
    /// the corresponding forward mapping.
    pub fn remove_value(&mut self, value: char) -> Option<f32> {
        if let Some(key) = self.reverse.remove(value) {
            self.forward.remove(key);
            Some(key)
        } else {
            None
        }
    }

    pub fn contains_key(&self, key: f32) -> bool {
        self.forward.contains_key(key)
    }

    pub fn contains_value(&self, value: char) -> bool {
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
    pub fn iter(&self) -> impl Iterator<Item = (f32, char)> + '_ {
        self.forward.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = f32> + '_ {
        self.forward.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = char> + '_ {
        self.forward.iter().map(|(_, v)| v)
    }

    pub fn for_each(&self, mut f: impl FnMut(f32, char)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    /// Returns a new BiMap with forward and reverse swapped.
    /// The returned map has value->key as forward and key->value as reverse.
    pub fn inverse(&self) -> CharF32HashBiMap {
        CharF32HashBiMap::from_maps(self.reverse.clone(), self.forward.clone())
    }

    /// Internal constructor from pre-built maps (used by the inverse type's `inverse()` call).
    pub(crate) fn from_maps(
        forward: OpenHashMap<f32, char>,
        reverse: OpenHashMap<char, f32>,
    ) -> Self {
        F32CharHashBiMap { forward, reverse }
    }
}

impl Default for F32CharHashBiMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for F32CharHashBiMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}

impl fmt::Display for F32CharHashBiMap {
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
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        assert_eq!(m.get(1.0f32), Some('a'));
        assert_eq!(m.get(2.0f32), Some('b'));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_get_key() {
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        assert_eq!(m.get_key('a'), Some(1.0f32));
        assert_eq!(m.get_key('b'), Some(2.0f32));
        assert_eq!(m.get_key('z'), None);
    }

    #[test]
    fn test_insert_overwrite() {
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        let old = m.insert(1.0f32, 'b');
        assert_eq!(old, Some('a'));
        assert_eq!(m.get(1.0f32), Some('b'));
        // Reverse should reflect the new mapping
        assert_eq!(m.get_key('b'), Some(1.0f32));
        // Old reverse mapping should be gone
        assert_eq!(m.get_key('a'), None);
    }

    #[test]
    fn test_remove() {
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        assert_eq!(m.remove(1.0f32), Some('a'));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key(1.0f32));
        assert!(!m.contains_value('a'));
    }

    #[test]
    fn test_remove_value() {
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        assert_eq!(m.remove_value('a'), Some(1.0f32));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key(1.0f32));
        assert!(!m.contains_value('a'));
    }

    #[test]
    fn test_contains() {
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        assert!(m.contains_key(1.0f32));
        assert!(!m.contains_key(99.0f32));
        assert!(m.contains_value('a'));
        assert!(!m.contains_value('z'));
    }

    #[test]
    fn test_inverse() {
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        let inv = m.inverse();
        // inverse forward lookup: old value -> old key
        assert_eq!(inv.get('a'), Some(1.0f32));
        assert_eq!(inv.get('b'), Some(2.0f32));
        // inverse reverse lookup: old key -> old value
        assert_eq!(inv.get_key(1.0f32), Some('a'));
        assert_eq!(inv.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn test_display() {
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        let s = m.to_string();
        assert!(!s.is_empty());
        assert!(s.starts_with('{'));
        assert!(s.ends_with('}'));
    }

    #[test]
    fn test_for_each() {
        let mut m = F32CharHashBiMap::new();
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        let mut count = 0usize;
        m.for_each(|_k, _v| {
            count += 1;
        });
        assert_eq!(count, 2);
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = F32CharHashBiMap::new();
        m.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut m = F32CharHashBiMap::new();
        assert!(m.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::f32_char_map::F32CharMap for F32CharHashBiMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: f32) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: f32) -> Option<char> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (f32, char)> + '_ {
        self.iter()
    }
}

impl crate::traits::f32_char_map::F32CharMutableMap for F32CharHashBiMap {
    fn insert(&mut self, key: f32, value: char) -> Option<char> {
        self.insert(key, value)
    }
    fn remove(&mut self, key: f32) -> Option<char> {
        self.remove(key)
    }
    fn clear(&mut self) {
        self.clear()
    }
}
