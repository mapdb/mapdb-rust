// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.

// Generic multimap (one key to many values), built on the project's ported
// `OpenHashMap` rather than `std::HashMap`.

use crate::hash_table::OpenHashMap;
use std::fmt;
use std::hash::Hash;

/// A multimap that maps each key to a list of values.
#[derive(Debug, Clone)]
pub struct Multimap<K: Eq + Hash, V> {
    data: OpenHashMap<K, Vec<V>>,
    size: usize,
}

impl<K: Eq + Hash, V> Multimap<K, V> {
    pub fn new() -> Self {
        Multimap {
            data: OpenHashMap::new(),
            size: 0,
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        if let Some(bucket) = self.data.get_mut(&key) {
            bucket.push(value);
        } else {
            self.data.insert(key, vec![value]);
        }
        self.size += 1;
    }

    /// Returns the values for `key` as an immutable view.
    ///
    /// This is intentionally zero-copy: safe Rust cannot mutate the
    /// backing multimap through `&[V]`, and the borrow is tied to `self`.
    /// Call `.to_vec()` on the returned slice when an owned snapshot is
    /// needed.
    pub fn get(&self, key: &K) -> &[V] {
        self.data.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.data.contains_key(key)
    }

    pub fn remove_all(&mut self, key: &K) -> Vec<V> {
        if let Some(values) = self.data.remove(key) {
            self.size -= values.len();
            values
        } else {
            Vec::new()
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

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

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        self.data
            .iter()
            .flat_map(|(k, vs)| vs.iter().map(move |v| (k, v)))
    }

    /// Calls `f` once per key with an immutable, lifetime-bound view of
    /// that key's values.
    pub fn for_each_key(&self, mut f: impl FnMut(&K, &[V])) {
        for (k, vs) in self.data.iter() {
            f(k, vs);
        }
    }

    pub fn for_each(&self, mut f: impl FnMut(&K, &V)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
}

// Bridge to the parallel module: iterate the multimap's *values* in fixed
// sections. Sections are whole keys (a contiguous range of the key set), so a
// section holds every value of its keys — value counts per section may differ.
// Drive with `parallel::batch::for_each_in_batches` for parallel value
// iteration with no copy. `get_batch_count` is therefore key-based.
impl<K: Eq + Hash, V> crate::parallel::batch::BatchIterable<V> for Multimap<K, V> {
    fn size(&self) -> usize {
        self.size
    }

    fn batch_for_each(
        &self,
        mut action: impl FnMut(&V),
        section_index: usize,
        section_count: usize,
    ) {
        let (lo, hi) =
            crate::parallel::batch::section_bounds(self.data.len(), section_index, section_count);
        for (i, (_k, vs)) in self.data.iter().enumerate() {
            if i >= hi {
                break;
            }
            if i >= lo {
                for v in vs {
                    action(v);
                }
            }
        }
    }

    fn get_batch_count(&self, batch_size: usize) -> usize {
        let keys = self.data.len();
        if batch_size == 0 || keys == 0 {
            1
        } else {
            keys.div_ceil(batch_size)
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
        for (k, vs) in self.data.iter() {
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
