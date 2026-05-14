// AUTO-GENERATED. DO NOT EDIT.

use std::collections::BTreeMap;
use std::fmt;

/// Sorted map from `i8` keys to `f32` values.
#[derive(Debug, Clone)]
pub struct I8F32TreeMap {
    inner: BTreeMap<i8, f32>,
}

impl I8F32TreeMap {
    pub fn new() -> Self {
        I8F32TreeMap {
            inner: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: i8, value: f32) -> Option<f32> {
        self.inner.insert(key, value)
    }

    pub fn get(&self, key: i8) -> Option<f32> {
        self.inner.get(&key).copied()
    }

    pub fn remove(&mut self, key: i8) -> Option<f32> {
        self.inner.remove(&key)
    }

    pub fn contains_key(&self, key: i8) -> bool {
        self.get(key).is_some()
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Ensures that `additional` more entries can be inserted without a
    /// reallocation on the backing store. Returns `TryReserveError` on
    /// allocator failure. See `docs/rust/error-handling.md`.
    ///
    /// Note: for tree maps keyed on `Ord` primitives, the backing store
    /// is `BTreeMap`, which grows node-by-node and does *not* expose a
    /// fallible reservation API. This method is therefore a no-op on
    /// that path; callers who need OOM recovery should prefer the
    /// hash-backed `HashMap` primitives instead. For float-keyed tree
    /// maps (Vec-backed sorted entries), the call delegates to
    /// `Vec::try_reserve` as expected.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        let _ = additional;
        Ok(())
    }

    pub fn min(&self) -> Option<(i8, f32)> {
        self.inner.iter().next().map(|(&k, &v)| (k, v))
    }

    pub fn max(&self) -> Option<(i8, f32)> {
        self.inner.iter().next_back().map(|(&k, &v)| (k, v))
    }

    pub fn iter(&self) -> impl Iterator<Item = (i8, f32)> + '_ {
        self.inner.iter().map(|(&k, &v)| (k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = i8> + '_ {
        self.inner.keys().copied()
    }

    pub fn values(&self) -> impl Iterator<Item = f32> + '_ {
        self.inner.values().copied()
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

    pub fn any_satisfy(&self, predicate: impl Fn(i8, f32) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(i8, f32) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
}

impl Default for I8F32TreeMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for I8F32TreeMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v.to_bits() == ov.to_bits()))
    }
}

impl fmt::Display for I8F32TreeMap {
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
        let mut m = I8F32TreeMap::new();
        m.insert(1, 1.0f32);
        m.insert(2, 2.0f32);
        m.insert(3, 3.0f32);
        assert_eq!(m.len(), 3);
        assert_eq!(m.get(1), Some(1.0f32));
        assert_eq!(m.get(99), None);
    }

    #[test]
    fn test_sorted_keys() {
        let mut m = I8F32TreeMap::new();
        m.insert(3, 3.0f32);
        m.insert(1, 1.0f32);
        m.insert(2, 2.0f32);
        let keys: Vec<_> = m.keys().collect();
        assert_eq!(keys, vec![1, 2, 3]);
    }

    #[test]
    fn test_min_max() {
        let mut m = I8F32TreeMap::new();
        m.insert(3, 3.0f32);
        m.insert(1, 1.0f32);
        assert_eq!(m.min().map(|(k, _)| k), Some(1));
        assert_eq!(m.max().map(|(k, _)| k), Some(3));
    }

    #[test]
    fn test_remove() {
        let mut m = I8F32TreeMap::new();
        m.insert(1, 1.0f32);
        m.insert(2, 2.0f32);
        assert_eq!(m.remove(1), Some(1.0f32));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn test_select() {
        let mut m = I8F32TreeMap::new();
        m.insert(1, 1.0f32);
        m.insert(2, 2.0f32);
        m.insert(3, 3.0f32);
        assert_eq!(m.select(|_k, v| v > 1.0f32).len(), 2);
    }

    #[test]
    fn test_display() {
        let mut m = I8F32TreeMap::new();
        m.insert(1, 1.0f32);
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = I8F32TreeMap::new();
        m.try_reserve(100).unwrap();
    }
}

impl crate::traits::i8_f32_map::I8F32Map for I8F32TreeMap {
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

impl crate::traits::i8_f32_map::I8F32MutableMap for I8F32TreeMap {
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
