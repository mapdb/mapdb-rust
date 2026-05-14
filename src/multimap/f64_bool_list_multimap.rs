// AUTO-GENERATED. DO NOT EDIT.

use std::collections::HashMap;
use std::fmt;

/// List multimap from `f64` keys to `bool` values.
/// Each key maps to a `Vec<bool>` of values, preserving insertion order per key.
///
/// Uses `std::collections::HashMap` internally because values are `Vec<bool>` (not `Copy`),
/// which is incompatible with our custom `OpenHashMap`.
/// Float keys are stored as their bit representation for correct hashing/equality.
#[derive(Debug, Clone)]
pub struct F64BoolListMultimap {
    inner: HashMap<u64, Vec<bool>>,
    total_size: usize,
}

impl F64BoolListMultimap {
    pub fn new() -> Self {
        F64BoolListMultimap {
            inner: HashMap::new(),
            total_size: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        F64BoolListMultimap {
            inner: HashMap::with_capacity(capacity),
            total_size: 0,
        }
    }

    /// Adds a value to the list for the given key.
    pub fn put(&mut self, key: f64, value: bool) {
        let internal_key = f64::to_bits(key);
        self.inner.entry(internal_key).or_default().push(value);
        self.total_size += 1;
    }

    /// Returns a slice of values for the given key, or an empty slice if absent.
    pub fn get(&self, key: f64) -> &[bool] {
        let internal_key = f64::to_bits(key);
        self.inner
            .get(&internal_key)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns all values for the given key as a new Vec.
    pub fn get_all(&self, key: f64) -> Vec<bool> {
        self.get(key).to_vec()
    }

    /// Removes all values for the given key. Returns the removed values.
    pub fn remove_all(&mut self, key: f64) -> Vec<bool> {
        let internal_key = f64::to_bits(key);
        let removed = self.inner.remove(&internal_key).unwrap_or_default();
        self.total_size -= removed.len();
        removed
    }

    /// Returns true if the multimap contains the given key.
    pub fn contains_key(&self, key: f64) -> bool {
        let internal_key = f64::to_bits(key);
        self.inner.contains_key(&internal_key)
    }

    /// Returns true if the multimap contains the given key-value pair.
    pub fn contains_key_value(&self, key: f64, value: bool) -> bool {
        self.get(key).contains(&value)
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
    /// failure. Note that per-key value lists (`Vec<bool>`) grow
    /// independently; this method reserves outer slots only. See
    /// `docs/rust/error-handling.md`.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.inner.try_reserve(additional)
    }

    /// Returns an iterator over the distinct keys.
    pub fn keys(&self) -> impl Iterator<Item = f64> + '_ {
        self.inner.keys().map(|k| f64::from_bits(*k))
    }

    /// Returns an iterator over all values (across all keys).
    pub fn values(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.values().flat_map(|v| v.iter().copied())
    }

    /// Calls the function for each key-value pair.
    pub fn for_each(&self, mut f: impl FnMut(f64, bool)) {
        for (k, vals) in &self.inner {
            let key = f64::from_bits(*k);
            for &val in vals {
                f(key, val);
            }
        }
    }

    /// Calls the function for each key with its list of values.
    pub fn for_each_key_value(&self, mut f: impl FnMut(f64, &[bool])) {
        for (k, vals) in &self.inner {
            f(f64::from_bits(*k), vals);
        }
    }

    /// Returns a new multimap containing only pairs that satisfy the predicate.
    pub fn select(&self, predicate: impl Fn(f64, bool) -> bool) -> Self {
        let mut result = Self::new();
        for (k, vals) in &self.inner {
            let key = f64::from_bits(*k);
            for &val in vals {
                if predicate(key, val) {
                    result.put(key, val);
                }
            }
        }
        result
    }

    /// Returns a new multimap containing only pairs that do not satisfy the predicate.
    pub fn reject(&self, predicate: impl Fn(f64, bool) -> bool) -> Self {
        let mut result = Self::new();
        for (k, vals) in &self.inner {
            let key = f64::from_bits(*k);
            for &val in vals {
                if !predicate(key, val) {
                    result.put(key, val);
                }
            }
        }
        result
    }

    /// Returns all key-value pairs as a Vec of tuples.
    pub fn to_vec(&self) -> Vec<(f64, bool)> {
        let mut result = Vec::with_capacity(self.total_size);
        for (k, vals) in &self.inner {
            let key = f64::from_bits(*k);
            for &val in vals {
                result.push((key, val));
            }
        }
        result
    }

    /// Returns the keys as a Vec.
    pub fn keys_to_vec(&self) -> Vec<f64> {
        self.keys().collect()
    }

    /// Returns all values as a Vec.
    pub fn values_to_vec(&self) -> Vec<bool> {
        self.values().collect()
    }

    /// Fluent API: adds a key-value pair and returns self.
    pub fn with_key_value(mut self, key: f64, value: bool) -> Self {
        self.put(key, value);
        self
    }
}

impl Default for F64BoolListMultimap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for F64BoolListMultimap {
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
                        if (*a) != (*b) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

impl fmt::Display for F64BoolListMultimap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first_key = true;
        for (k, vals) in &self.inner {
            if !first_key {
                write!(f, ", ")?;
            }
            let key = f64::from_bits(*k);
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
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(1.0f64, false);
        m.put(2.0f64, true);
        assert_eq!(m.get(1.0f64).len(), 2);
        assert_eq!(m.get(2.0f64).len(), 1);
        assert_eq!(m.get(99.0f64).len(), 0);
        assert_eq!(m.size(), 3);
        assert_eq!(m.keys_count(), 2);
    }

    #[test]
    fn test_get_all() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(1.0f64, false);
        let vals = m.get_all(1.0f64);
        assert_eq!(vals.len(), 2);
    }

    #[test]
    fn test_remove_all() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(1.0f64, false);
        m.put(2.0f64, true);
        let removed = m.remove_all(1.0f64);
        assert_eq!(removed.len(), 2);
        assert_eq!(m.size(), 1);
        assert_eq!(m.keys_count(), 1);
        assert!(!m.contains_key(1.0f64));
    }

    #[test]
    fn test_contains_key() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        assert!(m.contains_key(1.0f64));
        assert!(!m.contains_key(99.0f64));
    }

    #[test]
    fn test_contains_key_value() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(1.0f64, false);
        assert!(m.contains_key_value(1.0f64, true));
        assert!(m.contains_key_value(1.0f64, false));
    }

    #[test]
    fn test_clear() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(2.0f64, false);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.size(), 0);
        assert_eq!(m.keys_count(), 0);
    }

    #[test]
    fn test_for_each() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(1.0f64, false);
        m.put(2.0f64, true);
        let mut count = 0usize;
        m.for_each(|_k, _v| {
            count += 1;
        });
        assert_eq!(count, 3);
    }

    #[test]
    fn test_for_each_key_value() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(1.0f64, false);
        m.put(2.0f64, true);
        let mut key_count = 0usize;
        m.for_each_key_value(|_k, vals| {
            key_count += 1;
            assert!(!vals.is_empty());
        });
        assert_eq!(key_count, 2);
    }

    #[test]
    fn test_select_reject() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(2.0f64, false);
        let sel = m.select(|_k, v| v == true);
        assert!(sel.size() >= 1);
    }

    #[test]
    fn test_to_vec() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(2.0f64, false);
        let pairs = m.to_vec();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn test_keys_values() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        m.put(1.0f64, false);
        m.put(2.0f64, true);
        assert_eq!(m.keys_to_vec().len(), 2);
        assert_eq!(m.values_to_vec().len(), 3);
    }

    #[test]
    fn test_equals() {
        let mut m1 = F64BoolListMultimap::new();
        m1.put(1.0f64, true);
        m1.put(1.0f64, false);
        let mut m2 = F64BoolListMultimap::new();
        m2.put(1.0f64, true);
        m2.put(1.0f64, false);
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_display() {
        let mut m = F64BoolListMultimap::new();
        m.put(1.0f64, true);
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_with_key_value_fluent() {
        let m = F64BoolListMultimap::new()
            .with_key_value(1.0f64, true)
            .with_key_value(1.0f64, false)
            .with_key_value(2.0f64, true);
        assert_eq!(m.size(), 3);
        assert_eq!(m.keys_count(), 2);
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = F64BoolListMultimap::new();
        m.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut m = F64BoolListMultimap::new();
        assert!(m.try_reserve(usize::MAX / 2).is_err());
    }
}
