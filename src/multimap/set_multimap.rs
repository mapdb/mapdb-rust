// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

// Multimap that maps each key to a *set* of values: duplicate values
// for the same key are silently dropped. Backing is `OpenHashMap<K,
// Vec<V>>` plus linear-scan dedupe on `put()` — same shape as the
// other three ports per `collections.md` §"Multimaps". The
// vec-not-set choice is deliberate: non-Hashable value types
// (`f32`/`f64`) work uniformly under this layout while a
// `OpenHashSet<V>` backing would force callers to wrap floats in
// `HashableFx`. Dedupe cost is `O(k)` per insert, fine for typical
// group-by workloads.

use crate::hash_table::OpenHashMap;
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct SetMultimap<K: Eq + Hash, V: Eq> {
    data: OpenHashMap<K, Vec<V>>,
    size: usize,
}

impl<K: Eq + Hash, V: Eq> SetMultimap<K, V> {
    pub fn new() -> Self {
        SetMultimap {
            data: OpenHashMap::new(),
            size: 0,
        }
    }

    /// Adds `value` to the set for `key`. Idempotent — a duplicate
    /// `value` for the same `key` is silently dropped. Dedupe is a
    /// linear scan of the existing bucket.
    pub fn put(&mut self, key: K, value: V) {
        if let Some(bucket) = self.data.get_mut(&key) {
            if bucket.iter().any(|v| v == &value) {
                return;
            }
            bucket.push(value);
        } else {
            self.data.insert(key, vec![value]);
        }
        self.size += 1;
    }

    /// Returns the values for `key` as a slice. Empty slice if the key
    /// is absent. The order of values is the insertion order of unique
    /// values; this matches `Multimap`.
    pub fn get(&self, key: &K) -> &[V] {
        self.data.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.data.contains_key(key)
    }

    pub fn contains_key_value(&self, key: &K, value: &V) -> bool {
        self.data
            .get(key)
            .map(|vs| vs.iter().any(|v| v == value))
            .unwrap_or(false)
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

impl<K: Eq + Hash, V: Eq> Default for SetMultimap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + fmt::Display, V: Eq + fmt::Display> fmt::Display for SetMultimap<K, V> {
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
    use crate::HashableF64;

    #[test]
    fn put_dedupes() {
        let mut m: SetMultimap<i32, i32> = SetMultimap::new();
        m.put(1, 10);
        m.put(1, 20);
        m.put(1, 10); // duplicate, silently dropped
        m.put(2, 30);
        assert_eq!(m.size(), 3);
        assert_eq!(m.size_distinct(), 2);
        assert_eq!(m.get(&1), &[10, 20]);
        assert_eq!(m.get(&2), &[30]);
        assert_eq!(m.get(&99), &[] as &[i32]);
    }

    #[test]
    fn remove_all_updates_size() {
        let mut m: SetMultimap<i32, &str> = SetMultimap::new();
        m.put(1, "a");
        m.put(1, "b");
        m.put(2, "c");
        let removed = m.remove_all(&1);
        assert_eq!(removed, vec!["a", "b"]);
        assert_eq!(m.size(), 1);
        assert_eq!(m.size_distinct(), 1);
        assert_eq!(m.remove_all(&99), Vec::<&str>::new());
    }

    #[test]
    fn contains_key_value() {
        let mut m: SetMultimap<&str, i32> = SetMultimap::new();
        m.put("a", 1);
        m.put("a", 2);
        assert!(m.contains_key(&"a"));
        assert!(!m.contains_key(&"b"));
        assert!(m.contains_key_value(&"a", &1));
        assert!(!m.contains_key_value(&"a", &99));
        assert!(!m.contains_key_value(&"b", &1));
    }

    #[test]
    fn clear_and_is_empty() {
        let mut m: SetMultimap<i32, i32> = SetMultimap::new();
        assert!(m.is_empty());
        m.put(1, 10);
        m.put(1, 10);
        assert!(!m.is_empty());
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.size_distinct(), 0);
    }

    #[test]
    fn iter_and_for_each() {
        let mut m: SetMultimap<i32, &str> = SetMultimap::new();
        m.put(1, "a");
        m.put(1, "a"); // dedupe
        m.put(2, "b");
        assert_eq!(m.iter().count(), 2);
        let mut acc = 0;
        m.for_each(|_k, _v| acc += 1);
        assert_eq!(acc, 2);
        let mut buckets = 0;
        m.for_each_key(|_k, _vs| buckets += 1);
        assert_eq!(buckets, 2);
    }

    #[test]
    fn float_value_via_hashable_wrapper() {
        // The rationale for vec-backing: lets us hold un-Hashable
        // values like raw f64s directly. We exercise both: a HashableF64
        // for keys (any usable key in OpenHashMap) and raw f64 values.
        // Dedupe on the value uses Eq, which f64 implements (with the
        // usual NaN != NaN caveat).
        let mut m: SetMultimap<i32, HashableF64> = SetMultimap::new();
        m.put(1, HashableF64::from(1.5));
        m.put(1, HashableF64::from(1.5)); // dedupe
        m.put(1, HashableF64::from(-0.0));
        m.put(1, HashableF64::from(0.0)); // distinct from -0.0 under HashableF64
        assert_eq!(m.size(), 3);

        // NaN value: HashableF64 uses bit-pattern Eq, so distinct
        // bit-pattern NaNs would be distinct values.
        m.put(1, HashableF64::from(f64::NAN));
        m.put(1, HashableF64::from(f64::NAN)); // same bits -> dedupe
        assert_eq!(m.size(), 4);
    }

    #[test]
    fn display_non_empty() {
        let mut m: SetMultimap<i32, i32> = SetMultimap::new();
        m.put(1, 10);
        let s = m.to_string();
        assert!(s.contains("1=[10]"));
        let empty: SetMultimap<i32, i32> = SetMultimap::new();
        assert_eq!(empty.to_string(), "{}");
    }
}
