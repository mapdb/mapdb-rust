// AUTO-GENERATED. DO NOT EDIT.

use crate::hash_table::OpenHashMap;
use std::fmt;

/// Hash map from `f64` keys to `bool` values.
/// Open-addressing with linear probing and Robin Hood backward-shift deletion.
#[derive(Debug, Clone)]
pub struct F64BoolHashMap {
    inner: OpenHashMap<f64, bool>,
}

impl F64BoolHashMap {
    pub fn new() -> Self {
        F64BoolHashMap {
            inner: OpenHashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        F64BoolHashMap {
            inner: OpenHashMap::with_capacity(capacity),
        }
    }

    /// Inserts a key-value pair. Returns the old value if the key was already present.
    pub fn insert(&mut self, key: f64, value: bool) -> Option<bool> {
        self.inner.insert(key, value)
    }

    /// Returns the value for the key, or None.
    pub fn get(&self, key: f64) -> Option<bool> {
        self.inner.get(key)
    }

    /// Returns the value for the key, or the default.
    pub fn get_or_default(&self, key: f64, default: bool) -> bool {
        self.inner.get(key).unwrap_or(default)
    }

    /// Removes the key. Returns the old value if present.
    pub fn remove(&mut self, key: f64) -> Option<bool> {
        self.inner.remove(key)
    }

    pub fn contains_key(&self, key: f64) -> bool {
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
    pub fn iter(&self) -> impl Iterator<Item = (f64, bool)> + '_ {
        self.inner.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = f64> + '_ {
        self.inner.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.iter().map(|(_, v)| v)
    }

    pub fn for_each(&self, mut f: impl FnMut(f64, bool)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    pub fn select(&self, predicate: impl Fn(f64, bool) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                result.insert(k, v);
            }
        }
        result
    }

    pub fn reject(&self, predicate: impl Fn(f64, bool) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if !predicate(k, v) {
                result.insert(k, v);
            }
        }
        result
    }

    pub fn detect(&self, predicate: impl Fn(f64, bool) -> bool) -> Option<(f64, bool)> {
        self.iter().find(|&(k, v)| predicate(k, v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(f64, bool) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(f64, bool) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }

    pub fn none_satisfy(&self, predicate: impl Fn(f64, bool) -> bool) -> bool {
        !self.iter().any(|(k, v)| predicate(k, v))
    }

    pub fn count(&self, predicate: impl Fn(f64, bool) -> bool) -> usize {
        self.iter().filter(|&(k, v)| predicate(k, v)).count()
    }

    pub fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, f64, bool) -> R) -> R {
        let mut acc = initial;
        for (k, v) in self.iter() {
            acc = f(acc, k, v);
        }
        acc
    }

    pub fn keys_to_vec(&self) -> Vec<f64> {
        self.keys().collect()
    }
    pub fn values_to_vec(&self) -> Vec<bool> {
        self.values().collect()
    }

    pub fn with_key_value(mut self, key: f64, value: bool) -> Self {
        self.insert(key, value);
        self
    }

    pub fn without_key(mut self, key: f64) -> Self {
        self.remove(key);
        self
    }
}

impl Default for F64BoolHashMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for F64BoolHashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}

impl fmt::Display for F64BoolHashMap {
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
        let mut m = F64BoolHashMap::new();
        m.insert(1.0f64, true);
        m.insert(2.0f64, false);
        m.insert(3.0f64, true);
        assert_eq!(m.get(1.0f64), Some(true));
        assert_eq!(m.get(99.0f64), None);
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn test_insert_overwrite() {
        let mut m = F64BoolHashMap::new();
        m.insert(1.0f64, true);
        let old = m.insert(1.0f64, false);
        assert_eq!(old, Some(true));
        assert_eq!(m.get(1.0f64), Some(false));
    }

    #[test]
    fn test_remove() {
        let mut m = F64BoolHashMap::new();
        m.insert(1.0f64, true);
        m.insert(2.0f64, false);
        assert_eq!(m.remove(1.0f64), Some(true));
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key(1.0f64));
    }

    #[test]
    fn test_contains_key() {
        let mut m = F64BoolHashMap::new();
        m.insert(1.0f64, true);
        assert!(m.contains_key(1.0f64));
        assert!(!m.contains_key(99.0f64));
    }

    #[test]
    fn test_get_or_default() {
        let mut m = F64BoolHashMap::new();
        m.insert(1.0f64, true);
        assert_eq!(m.get_or_default(1.0f64, true), true);
        assert_eq!(m.get_or_default(99.0f64, true), true);
    }

    #[test]
    fn test_clear() {
        let mut m = F64BoolHashMap::new();
        m.insert(1.0f64, true);
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_select_reject() {
        let mut m = F64BoolHashMap::new();
        m.insert(1.0f64, true);
        m.insert(2.0f64, false);
        let sel = m.select(|_k, v| v == true);
        assert!(sel.len() >= 1);
    }

    #[test]
    fn test_any_all_none() {
        let mut m = F64BoolHashMap::new();
        m.insert(1.0f64, true);
        assert!(m.any_satisfy(|_k, v| v == true));
    }

    #[test]
    fn test_equals() {
        let mut m1 = F64BoolHashMap::new();
        m1.insert(1.0f64, true);
        m1.insert(2.0f64, false);
        let mut m2 = F64BoolHashMap::new();
        m2.insert(2.0f64, false);
        m2.insert(1.0f64, true);
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_display() {
        let mut m = F64BoolHashMap::new();
        m.insert(1.0f64, true);
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_resize() {
        let mut m = F64BoolHashMap::new();
        for i in 0..100 {
            m.insert(i as f64, i % 3 == 0);
        }
        assert!(m.len() > 0);
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = F64BoolHashMap::new();
        m.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut m = F64BoolHashMap::new();
        assert!(m.try_reserve(usize::MAX / 2).is_err());
    }

    // -----------------------------------------------------------------------
    // IEEE 754 edge cases — float key semantics
    // -----------------------------------------------------------------------
    // Locks in bit-level equality:  NaN keys are findable, ±0.0 are distinct,
    // ±Inf round-trip cleanly. See docs/float-nan-semantics-audit.md.

    #[test]
    fn test_nan_key_findable() {
        let mut m = F64BoolHashMap::new();
        m.insert(f64::NAN, true);
        assert!(m.contains_key(f64::NAN));
        assert_eq!(m.get(f64::NAN), Some(true));
    }

    #[test]
    fn test_nan_key_replaces_does_not_duplicate() {
        let mut m = F64BoolHashMap::new();
        m.insert(f64::NAN, true);
        m.insert(f64::NAN, false);
        m.insert(f64::NAN, true);
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(f64::NAN), Some(true));
    }

    #[test]
    fn test_nan_key_remove() {
        let mut m = F64BoolHashMap::new();
        m.insert(f64::NAN, true);
        let removed = m.remove(f64::NAN);
        assert_eq!(removed, Some(true));
        assert_eq!(m.len(), 0);
        assert!(!m.contains_key(f64::NAN));
    }

    #[test]
    fn test_negative_zero_distinct_from_positive_zero() {
        let mut m = F64BoolHashMap::new();
        m.insert(0.0f64, true);
        m.insert(-0.0f64, false);
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(0.0f64), Some(true));
        assert_eq!(m.get(-0.0f64), Some(false));
    }

    #[test]
    fn test_infinity_keys() {
        let mut m = F64BoolHashMap::new();
        m.insert(f64::INFINITY, true);
        m.insert(f64::NEG_INFINITY, false);
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(f64::INFINITY), Some(true));
        assert_eq!(m.get(f64::NEG_INFINITY), Some(false));
        assert!(m.contains_key(f64::INFINITY));
        assert!(m.contains_key(f64::NEG_INFINITY));
    }
}

impl crate::traits::f64_bool_map::F64BoolMap for F64BoolHashMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: f64) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: f64) -> Option<bool> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (f64, bool)> + '_ {
        self.iter()
    }
}

impl crate::traits::f64_bool_map::F64BoolMutableMap for F64BoolHashMap {
    fn insert(&mut self, key: f64, value: bool) -> Option<bool> {
        self.insert(key, value)
    }
    fn remove(&mut self, key: f64) -> Option<bool> {
        self.remove(key)
    }
    fn clear(&mut self) {
        self.clear()
    }
}
