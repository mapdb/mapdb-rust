// AUTO-GENERATED. DO NOT EDIT.

use std::collections::BTreeMap;
use std::fmt;

/// Sorted map from `i32` keys to `bool` values.
#[derive(Debug, Clone)]
pub struct I32BoolTreeMap {
    inner: BTreeMap<i32, bool>,
}

impl I32BoolTreeMap {
    pub fn new() -> Self {
        I32BoolTreeMap {
            inner: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: i32, value: bool) -> Option<bool> {
        self.inner.insert(key, value)
    }

    pub fn get(&self, key: i32) -> Option<bool> {
        self.inner.get(&key).copied()
    }

    pub fn remove(&mut self, key: i32) -> Option<bool> {
        self.inner.remove(&key)
    }

    pub fn contains_key(&self, key: i32) -> bool {
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

    pub fn min(&self) -> Option<(i32, bool)> {
        self.inner.iter().next().map(|(&k, &v)| (k, v))
    }

    pub fn max(&self) -> Option<(i32, bool)> {
        self.inner.iter().next_back().map(|(&k, &v)| (k, v))
    }

    pub fn iter(&self) -> impl Iterator<Item = (i32, bool)> + '_ {
        self.inner.iter().map(|(&k, &v)| (k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = i32> + '_ {
        self.inner.keys().copied()
    }

    pub fn values(&self) -> impl Iterator<Item = bool> + '_ {
        self.inner.values().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(i32, bool)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    pub fn select(&self, predicate: impl Fn(i32, bool) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                result.insert(k, v);
            }
        }
        result
    }

    pub fn any_satisfy(&self, predicate: impl Fn(i32, bool) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(i32, bool) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
}

impl Default for I32BoolTreeMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for I32BoolTreeMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}

impl fmt::Display for I32BoolTreeMap {
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
        let mut m = I32BoolTreeMap::new();
        m.insert(1, true);
        m.insert(2, false);
        m.insert(3, true);
        assert_eq!(m.len(), 3);
        assert_eq!(m.get(1), Some(true));
        assert_eq!(m.get(99), None);
    }

    #[test]
    fn test_sorted_keys() {
        let mut m = I32BoolTreeMap::new();
        m.insert(3, true);
        m.insert(1, true);
        m.insert(2, false);
        let keys: Vec<_> = m.keys().collect();
        assert_eq!(keys, vec![1, 2, 3]);
    }

    #[test]
    fn test_min_max() {
        let mut m = I32BoolTreeMap::new();
        m.insert(3, true);
        m.insert(1, true);
        assert_eq!(m.min().map(|(k, _)| k), Some(1));
        assert_eq!(m.max().map(|(k, _)| k), Some(3));
    }

    #[test]
    fn test_remove() {
        let mut m = I32BoolTreeMap::new();
        m.insert(1, true);
        m.insert(2, false);
        assert_eq!(m.remove(1), Some(true));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn test_select() {
        let mut m = I32BoolTreeMap::new();
        m.insert(1, true);
        m.insert(2, false);
        let sel = m.select(|_k, v| v == true);
        assert!(sel.len() >= 1);
    }

    #[test]
    fn test_display() {
        let mut m = I32BoolTreeMap::new();
        m.insert(1, true);
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = I32BoolTreeMap::new();
        m.try_reserve(100).unwrap();
    }
}

impl crate::traits::i32_bool_map::I32BoolMap for I32BoolTreeMap {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains_key(&self, key: i32) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: i32) -> Option<bool> {
        self.get(key)
    }
    fn iter(&self) -> impl Iterator<Item = (i32, bool)> + '_ {
        self.iter()
    }
}

impl crate::traits::i32_bool_map::I32BoolMutableMap for I32BoolTreeMap {
    fn insert(&mut self, key: i32, value: bool) -> Option<bool> {
        self.insert(key, value)
    }
    fn remove(&mut self, key: i32) -> Option<bool> {
        self.remove(key)
    }
    fn clear(&mut self) {
        self.clear()
    }
}
