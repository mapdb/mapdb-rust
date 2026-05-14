// AUTO-GENERATED. DO NOT EDIT.

use crate::hash_table::OpenHashMap;
use std::fmt;

/// Bidirectional hash map from `char` keys to `char` values.
/// Both key->value and value->key lookups are O(1).
/// Internally uses two `OpenHashMap` instances: forward (key->value) and reverse (value->key).
#[derive(Debug, Clone)]
pub struct CharCharHashBiMap {
    forward: OpenHashMap<char, char>,
    reverse: OpenHashMap<char, char>,
}

impl CharCharHashBiMap {
    pub fn new() -> Self {
        CharCharHashBiMap {
            forward: OpenHashMap::new(),
            reverse: OpenHashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        CharCharHashBiMap {
            forward: OpenHashMap::with_capacity(capacity),
            reverse: OpenHashMap::with_capacity(capacity),
        }
    }

    /// Inserts a key-value pair into the bi-map.
    /// If the key already existed, the old value is removed from the reverse map
    /// and returned. The new value->key mapping is then inserted.
    pub fn insert(&mut self, key: char, value: char) -> Option<char> {
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
    pub fn get(&self, key: char) -> Option<char> {
        self.forward.get(key)
    }

    /// Reverse lookup: returns the key for the given value.
    pub fn get_key(&self, value: char) -> Option<char> {
        self.reverse.get(value)
    }

    /// Removes by key. Returns the old value if present, and also removes
    /// the corresponding reverse mapping.
    pub fn remove(&mut self, key: char) -> Option<char> {
        if let Some(val) = self.forward.remove(key) {
            self.reverse.remove(val);
            Some(val)
        } else {
            None
        }
    }

    /// Removes by value. Returns the old key if present, and also removes
    /// the corresponding forward mapping.
    pub fn remove_value(&mut self, value: char) -> Option<char> {
        if let Some(key) = self.reverse.remove(value) {
            self.forward.remove(key);
            Some(key)
        } else {
            None
        }
    }

    pub fn contains_key(&self, key: char) -> bool {
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
    pub fn iter(&self) -> impl Iterator<Item = (char, char)> + '_ {
        self.forward.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = char> + '_ {
        self.forward.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = char> + '_ {
        self.forward.iter().map(|(_, v)| v)
    }

    pub fn for_each(&self, mut f: impl FnMut(char, char)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    /// Returns a new BiMap with forward and reverse swapped.
    /// The returned map has value->key as forward and key->value as reverse.
    pub fn inverse(&self) -> CharCharHashBiMap {
        CharCharHashBiMap::from_maps(self.reverse.clone(), self.forward.clone())
    }

    /// Internal constructor from pre-built maps (used by the inverse type's `inverse()` call).
    pub(crate) fn from_maps(
        forward: OpenHashMap<char, char>,
        reverse: OpenHashMap<char, char>,
    ) -> Self {
        CharCharHashBiMap { forward, reverse }
    }
}

impl Default for CharCharHashBiMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for CharCharHashBiMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}

impl fmt::Display for CharCharHashBiMap {
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
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        m.insert('b', 'b');
        assert_eq!(m.get('a'), Some('a'));
        assert_eq!(m.get('b'), Some('b'));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_get_key() {
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        m.insert('b', 'b');
        assert_eq!(m.get_key('a'), Some('a'));
        assert_eq!(m.get_key('b'), Some('b'));
        assert_eq!(m.get_key('z'), None);
    }

    #[test]
    fn test_insert_overwrite() {
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        let old = m.insert('a', 'b');
        assert_eq!(old, Some('a'));
        assert_eq!(m.get('a'), Some('b'));
        // Reverse should reflect the new mapping
        assert_eq!(m.get_key('b'), Some('a'));
        // Old reverse mapping should be gone
        assert_eq!(m.get_key('a'), None);
    }

    #[test]
    fn test_remove() {
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        m.insert('b', 'b');
        assert_eq!(m.remove('a'), Some('a'));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key('a'));
        assert!(!m.contains_value('a'));
    }

    #[test]
    fn test_remove_value() {
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        m.insert('b', 'b');
        assert_eq!(m.remove_value('a'), Some('a'));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key('a'));
        assert!(!m.contains_value('a'));
    }

    #[test]
    fn test_contains() {
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        assert!(m.contains_key('a'));
        assert!(!m.contains_key('z'));
        assert!(m.contains_value('a'));
        assert!(!m.contains_value('z'));
    }

    #[test]
    fn test_inverse() {
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        m.insert('b', 'b');
        let inv = m.inverse();
        // inverse forward lookup: old value -> old key
        assert_eq!(inv.get('a'), Some('a'));
        assert_eq!(inv.get('b'), Some('b'));
        // inverse reverse lookup: old key -> old value
        assert_eq!(inv.get_key('a'), Some('a'));
        assert_eq!(inv.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn test_display() {
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        let s = m.to_string();
        assert!(!s.is_empty());
        assert!(s.starts_with('{'));
        assert!(s.ends_with('}'));
    }

    #[test]
    fn test_for_each() {
        let mut m = CharCharHashBiMap::new();
        m.insert('a', 'a');
        m.insert('b', 'b');
        let mut count = 0usize;
        m.for_each(|_k, _v| {
            count += 1;
        });
        assert_eq!(count, 2);
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = CharCharHashBiMap::new();
        m.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut m = CharCharHashBiMap::new();
        assert!(m.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::char_char_map::CharCharMap for CharCharHashBiMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: char) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: char) -> Option<char> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (char, char)> + '_ {
        self.iter()
    }
}

impl crate::traits::char_char_map::CharCharMutableMap for CharCharHashBiMap {
    fn insert(&mut self, key: char, value: char) -> Option<char> {
        self.insert(key, value)
    }
    fn remove(&mut self, key: char) -> Option<char> {
        self.remove(key)
    }
    fn clear(&mut self) {
        self.clear()
    }
}
