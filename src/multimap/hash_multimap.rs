// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

// Hand-written — generic multimap (one key to many values).

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

/// A multimap that maps each key to a list of values.
/// Generic over key and value types.
#[derive(Debug, Clone)]
pub struct Multimap<K: Eq + Hash, V> {
    data: HashMap<K, Vec<V>>,
    size: usize,
}

impl<K: Eq + Hash, V> Multimap<K, V> {
    pub fn new() -> Self {
        Multimap {
            data: HashMap::new(),
            size: 0,
        }
    }

    /// Adds a single value for the key.
    pub fn put(&mut self, key: K, value: V) {
        self.data.entry(key).or_default().push(value);
        self.size += 1;
    }

    /// Returns the values for the key, or an empty slice.
    pub fn get(&self, key: &K) -> &[V] {
        self.data.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.data.contains_key(key)
    }

    /// Removes all values for the key. Returns the removed values.
    pub fn remove_all(&mut self, key: &K) -> Vec<V> {
        if let Some(values) = self.data.remove(key) {
            self.size -= values.len();
            values
        } else {
            Vec::new()
        }
    }

    /// Total number of key-value pairs (counting duplicates).
    pub fn size(&self) -> usize {
        self.size
    }

    /// Number of distinct keys.
    pub fn size_distinct(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.size = 0;
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.data.keys()
    }

    /// Iterates over all (key, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        self.data
            .iter()
            .flat_map(|(k, vs)| vs.iter().map(move |v| (k, v)))
    }

    pub fn for_each_key(&self, mut f: impl FnMut(&K, &[V])) {
        for (k, vs) in &self.data {
            f(k, vs);
        }
    }

    pub fn for_each(&self, mut f: impl FnMut(&K, &V)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
}

impl<K: Eq + Hash, V> Default for Multimap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + fmt::Display, V: fmt::Display> fmt::Display for Multimap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (k, vs) in &self.data {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}=[", k)?;
            for (i, v) in vs.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", v)?;
            }
            write!(f, "]")?;
            first = false;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get() {
        let mut m = Multimap::new();
        m.put("a", 1);
        m.put("a", 2);
        m.put("b", 3);
        assert_eq!(m.get(&"a"), &[1, 2]);
        assert_eq!(m.get(&"b"), &[3]);
        assert_eq!(m.get(&"c"), &[] as &[i32]);
        assert_eq!(m.size(), 3);
        assert_eq!(m.size_distinct(), 2);
    }

    #[test]
    fn test_remove_all() {
        let mut m = Multimap::new();
        m.put(1, "a");
        m.put(1, "b");
        m.put(2, "c");
        let removed = m.remove_all(&1);
        assert_eq!(removed, vec!["a", "b"]);
        assert_eq!(m.size(), 1);
    }

    #[test]
    fn test_contains_key() {
        let mut m = Multimap::<i32, i32>::new();
        m.put(1, 10);
        assert!(m.contains_key(&1));
        assert!(!m.contains_key(&2));
    }

    #[test]
    fn test_clear() {
        let mut m = Multimap::new();
        m.put(1, "a");
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_iter() {
        let mut m = Multimap::new();
        m.put(1, "a");
        m.put(1, "b");
        m.put(2, "c");
        assert_eq!(m.iter().count(), 3);
    }

    #[test]
    fn test_display() {
        let mut m = Multimap::new();
        m.put(1, "a");
        assert!(!m.to_string().is_empty());
    }
}
