// AUTO-GENERATED. DO NOT EDIT.

use std::collections::HashMap;
use std::fmt;

/// List multimap from `i16` keys to `f32` values.
/// Each key maps to a `Vec<f32>` of values, preserving insertion order per key.
///
/// Uses `std::collections::HashMap` internally because values are `Vec<f32>` (not `Copy`),
/// which is incompatible with our custom `OpenHashMap`.

#[derive(Debug, Clone)]
pub struct I16F32ListMultimap {
    inner: HashMap<i16, Vec<f32>>,
    total_size: usize,
}

impl I16F32ListMultimap {
    pub fn new() -> Self {
        I16F32ListMultimap {
            inner: HashMap::new(),
            total_size: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        I16F32ListMultimap {
            inner: HashMap::with_capacity(capacity),
            total_size: 0,
        }
    }

    /// Adds a value to the list for the given key.
    pub fn put(&mut self, key: i16, value: f32) {
        let internal_key = key;
        self.inner.entry(internal_key).or_default().push(value);
        self.total_size += 1;
    }

    /// Returns a slice of values for the given key, or an empty slice if absent.
    pub fn get(&self, key: i16) -> &[f32] {
        let internal_key = key;
        self.inner
            .get(&internal_key)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns all values for the given key as a new Vec.
    pub fn get_all(&self, key: i16) -> Vec<f32> {
        self.get(key).to_vec()
    }

    /// Removes all values for the given key. Returns the removed values.
    pub fn remove_all(&mut self, key: i16) -> Vec<f32> {
        let internal_key = key;
        let removed = self.inner.remove(&internal_key).unwrap_or_default();
        self.total_size -= removed.len();
        removed
    }

    /// Returns true if the multimap contains the given key.
    pub fn contains_key(&self, key: i16) -> bool {
        let internal_key = key;
        self.inner.contains_key(&internal_key)
    }

    /// Returns true if the multimap contains the given key-value pair.
    pub fn contains_key_value(&self, key: i16, value: f32) -> bool {
        self.get(key)
            .iter()
            .any(|v| (*v).to_bits() == value.to_bits())
    }

    /// Returns the number of distinct keys.
    pub fn keys_count(&self) -> usize {
        self.inner.len()
    }

    /// Returns the total number of values across all keys.
    pub fn size(&self) -> usize {
        self.total_size
    }

    pub fn is_empty(&self) -> bool {
        self.total_size == 0
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.total_size = 0;
    }

    /// Ensures the backing `HashMap` can accept `additional` more distinct
    /// keys without a rehash. Returns `TryReserveError` on allocator
    /// failure. Note that per-key value lists (`Vec<f32>`) grow
    /// independently; this method reserves outer slots only. See
    /// `docs/rust/error-handling.md`.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.inner.try_reserve(additional)
    }

    /// Returns an iterator over the distinct keys.
    pub fn keys(&self) -> impl Iterator<Item = i16> + '_ {
        self.inner.keys().copied()
    }

    /// Returns an iterator over all values (across all keys).
    pub fn values(&self) -> impl Iterator<Item = f32> + '_ {
        self.inner.values().flat_map(|v| v.iter().copied())
    }

    /// Calls the function for each key-value pair.
    pub fn for_each(&self, mut f: impl FnMut(i16, f32)) {
        for (k, vals) in &self.inner {
            let key = *k;
            for &val in vals {
                f(key, val);
            }
        }
    }

    /// Calls the function for each key with its list of values.
    pub fn for_each_key_value(&self, mut f: impl FnMut(i16, &[f32])) {
        for (k, vals) in &self.inner {
            f(*k, vals);
        }
    }

    /// Returns a new multimap containing only pairs that satisfy the predicate.
    pub fn select(&self, predicate: impl Fn(i16, f32) -> bool) -> Self {
        let mut result = Self::new();
        for (k, vals) in &self.inner {
            let key = *k;
            for &val in vals {
                if predicate(key, val) {
                    result.put(key, val);
                }
            }
        }
        result
    }

    /// Returns a new multimap containing only pairs that do not satisfy the predicate.
    pub fn reject(&self, predicate: impl Fn(i16, f32) -> bool) -> Self {
        let mut result = Self::new();
        for (k, vals) in &self.inner {
            let key = *k;
            for &val in vals {
                if !predicate(key, val) {
                    result.put(key, val);
                }
            }
        }
        result
    }

    /// Returns all key-value pairs as a Vec of tuples.
    pub fn to_vec(&self) -> Vec<(i16, f32)> {
        let mut result = Vec::with_capacity(self.total_size);
        for (k, vals) in &self.inner {
            let key = *k;
            for &val in vals {
                result.push((key, val));
            }
        }
        result
    }

    /// Returns the keys as a Vec.
    pub fn keys_to_vec(&self) -> Vec<i16> {
        self.keys().collect()
    }

    /// Returns all values as a Vec.
    pub fn values_to_vec(&self) -> Vec<f32> {
        self.values().collect()
    }

    /// Fluent API: adds a key-value pair and returns self.
    pub fn with_key_value(mut self, key: i16, value: f32) -> Self {
        self.put(key, value);
        self
    }
}

impl Default for I16F32ListMultimap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for I16F32ListMultimap {
    fn eq(&self, other: &Self) -> bool {
        if self.total_size != other.total_size {
            return false;
        }
        if self.inner.len() != other.inner.len() {
            return false;
        }
        for (k, vals) in &self.inner {
            match other.inner.get(k) {
                None => return false,
                Some(other_vals) => {
                    if vals.len() != other_vals.len() {
                        return false;
                    }
                    for (a, b) in vals.iter().zip(other_vals.iter()) {
                        if (*a).to_bits() != (*b).to_bits() {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

impl fmt::Display for I16F32ListMultimap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first_key = true;
        for (k, vals) in &self.inner {
            if !first_key {
                write!(f, ", ")?;
            }
            let key = *k;
            write!(f, "{}=[", key)?;
            let mut first_val = true;
            for val in vals {
                if !first_val {
                    write!(f, ", ")?;
                }
                write!(f, "{}", val)?;
                first_val = false;
            }
            write!(f, "]")?;
            first_key = false;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(1, 2.0f32);
        m.put(2, 3.0f32);
        assert_eq!(m.get(1).len(), 2);
        assert_eq!(m.get(2).len(), 1);
        assert_eq!(m.get(99).len(), 0);
        assert_eq!(m.size(), 3);
        assert_eq!(m.keys_count(), 2);
    }

    #[test]
    fn test_get_all() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(1, 2.0f32);
        let vals = m.get_all(1);
        assert_eq!(vals.len(), 2);
    }

    #[test]
    fn test_remove_all() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(1, 2.0f32);
        m.put(2, 3.0f32);
        let removed = m.remove_all(1);
        assert_eq!(removed.len(), 2);
        assert_eq!(m.size(), 1);
        assert_eq!(m.keys_count(), 1);
        assert!(!m.contains_key(1));
    }

    #[test]
    fn test_contains_key() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        assert!(m.contains_key(1));
        assert!(!m.contains_key(99));
    }

    #[test]
    fn test_contains_key_value() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(1, 2.0f32);
        assert!(m.contains_key_value(1, 1.0f32));
        assert!(m.contains_key_value(1, 2.0f32));
        assert!(!m.contains_key_value(1, 3.0f32));
    }

    #[test]
    fn test_clear() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(2, 2.0f32);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.size(), 0);
        assert_eq!(m.keys_count(), 0);
    }

    #[test]
    fn test_for_each() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(1, 2.0f32);
        m.put(2, 3.0f32);
        let mut count = 0usize;
        m.for_each(|_k, _v| {
            count += 1;
        });
        assert_eq!(count, 3);
    }

    #[test]
    fn test_for_each_key_value() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(1, 2.0f32);
        m.put(2, 3.0f32);
        let mut key_count = 0usize;
        m.for_each_key_value(|_k, vals| {
            key_count += 1;
            assert!(!vals.is_empty());
        });
        assert_eq!(key_count, 2);
    }

    #[test]
    fn test_select_reject() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(1, 2.0f32);
        m.put(2, 3.0f32);
        let sel = m.select(|_k, v| v > 1.0f32);
        assert_eq!(sel.size(), 2);
        let rej = m.reject(|_k, v| v > 1.0f32);
        assert_eq!(rej.size(), 1);
    }

    #[test]
    fn test_to_vec() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(2, 2.0f32);
        let pairs = m.to_vec();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn test_keys_values() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        m.put(1, 2.0f32);
        m.put(2, 3.0f32);
        assert_eq!(m.keys_to_vec().len(), 2);
        assert_eq!(m.values_to_vec().len(), 3);
    }

    #[test]
    fn test_equals() {
        let mut m1 = I16F32ListMultimap::new();
        m1.put(1, 1.0f32);
        m1.put(1, 2.0f32);
        let mut m2 = I16F32ListMultimap::new();
        m2.put(1, 1.0f32);
        m2.put(1, 2.0f32);
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_display() {
        let mut m = I16F32ListMultimap::new();
        m.put(1, 1.0f32);
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_with_key_value_fluent() {
        let m = I16F32ListMultimap::new()
            .with_key_value(1, 1.0f32)
            .with_key_value(1, 2.0f32)
            .with_key_value(2, 3.0f32);
        assert_eq!(m.size(), 3);
        assert_eq!(m.keys_count(), 2);
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = I16F32ListMultimap::new();
        m.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut m = I16F32ListMultimap::new();
        assert!(m.try_reserve(usize::MAX / 2).is_err());
    }
}
