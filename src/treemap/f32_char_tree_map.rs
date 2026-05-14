// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Sorted map from `f32` keys to `char` values.
#[derive(Debug, Clone)]
pub struct F32CharTreeMap {
    entries: Vec<(f32, char)>,
}

impl F32CharTreeMap {
    pub fn new() -> Self {
        F32CharTreeMap {
            entries: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: f32, value: char) -> Option<char> {
        match self.entries.binary_search_by(|e| e.0.total_cmp(&key)) {
            Ok(idx) => {
                let old = self.entries[idx].1;
                self.entries[idx].1 = value;
                Some(old)
            }
            Err(idx) => {
                self.entries.insert(idx, (key, value));
                None
            }
        }
    }

    pub fn get(&self, key: f32) -> Option<char> {
        self.entries
            .binary_search_by(|e| e.0.total_cmp(&key))
            .ok()
            .map(|idx| self.entries[idx].1)
    }

    pub fn remove(&mut self, key: f32) -> Option<char> {
        match self.entries.binary_search_by(|e| e.0.total_cmp(&key)) {
            Ok(idx) => Some(self.entries.remove(idx).1),
            Err(_) => None,
        }
    }

    pub fn contains_key(&self, key: f32) -> bool {
        self.get(key).is_some()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn clear(&mut self) {
        self.entries.clear();
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
        self.entries.try_reserve(additional)
    }

    pub fn min(&self) -> Option<(f32, char)> {
        self.entries.first().copied()
    }

    pub fn max(&self) -> Option<(f32, char)> {
        self.entries.last().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (f32, char)> + '_ {
        self.entries.iter().copied()
    }

    pub fn keys(&self) -> impl Iterator<Item = f32> + '_ {
        self.entries.iter().map(|e| e.0)
    }

    pub fn values(&self) -> impl Iterator<Item = char> + '_ {
        self.entries.iter().map(|e| e.1)
    }

    pub fn for_each(&self, mut f: impl FnMut(f32, char)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    pub fn select(&self, predicate: impl Fn(f32, char) -> bool) -> Self {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                result.insert(k, v);
            }
        }
        result
    }

    pub fn any_satisfy(&self, predicate: impl Fn(f32, char) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(f32, char) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }
}

impl Default for F32CharTreeMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for F32CharTreeMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}

impl fmt::Display for F32CharTreeMap {
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
        let mut m = F32CharTreeMap::new();
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        m.insert(3.0f32, 'c');
        assert_eq!(m.len(), 3);
        assert_eq!(m.get(1.0f32), Some('a'));
        assert_eq!(m.get(99.0f32), None);
    }

    #[test]
    fn test_sorted_keys() {
        let mut m = F32CharTreeMap::new();
        m.insert(3.0f32, 'c');
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        let keys: Vec<_> = m.keys().collect();
        assert_eq!(keys, vec![1.0f32, 2.0f32, 3.0f32]);
    }

    #[test]
    fn test_min_max() {
        let mut m = F32CharTreeMap::new();
        m.insert(3.0f32, 'c');
        m.insert(1.0f32, 'a');
        assert_eq!(m.min().map(|(k, _)| k), Some(1.0f32));
        assert_eq!(m.max().map(|(k, _)| k), Some(3.0f32));
    }

    #[test]
    fn test_remove() {
        let mut m = F32CharTreeMap::new();
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        assert_eq!(m.remove(1.0f32), Some('a'));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn test_select() {
        let mut m = F32CharTreeMap::new();
        m.insert(1.0f32, 'a');
        m.insert(2.0f32, 'b');
        m.insert(3.0f32, 'c');
        assert_eq!(m.select(|_k, v| v > 'a').len(), 2);
    }

    #[test]
    fn test_display() {
        let mut m = F32CharTreeMap::new();
        m.insert(1.0f32, 'a');
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut m = F32CharTreeMap::new();
        m.try_reserve(100).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut m = F32CharTreeMap::new();
        assert!(m.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::f32_char_map::F32CharMap for F32CharTreeMap {
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

impl crate::traits::f32_char_map::F32CharMutableMap for F32CharTreeMap {
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
