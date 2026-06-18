// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

// Multimap that maps each key to a *set* of values: duplicate values
// for the same key are silently dropped. Backing is `OpenHashMap<K,
// Vec<V>>` plus linear-scan dedupe on `insert()` — same shape as the
// other three ports per `collections.md` §"Multimaps". The
// vec-not-set choice is deliberate: non-Hashable value types
// (`f32`/`f64`) work uniformly under this layout while a
// `OpenHashSet<V>` backing would force callers to wrap floats in
// `HashableFx`. Dedupe cost is `O(k)` per insert, fine for typical
// group-by workloads.

use crate::bulk::BulkError;
use crate::hash_table::OpenHashMap;
use crate::object::strategy::Comparator;
use std::cmp::Ordering;
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

    /// Bulk-loads a fresh set-multimap from `(key, value)` pairs in **any**
    /// order (hash/group accumulation — no sortedness claimed). Duplicate
    /// `(key, value)` pairs are deduped per key (set semantics) via the
    /// existing linear-scan `insert`. Value order per key is first-seen order.
    pub fn bulk_load<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut m = SetMultimap::new();
        for (k, v) in iter {
            m.insert(k, v);
        }
        m
    }

    /// Bulk-loads from input **grouped by key** (key runs strictly ascending
    /// under `cmp`). Within a key run, values are deduped (set semantics) by a
    /// linear scan, preserving first-seen order. Out-of-order keys are a
    /// [`BulkError::OutOfOrder`].
    pub fn from_sorted_keys<I: IntoIterator<Item = (K, V)>>(
        cmp: Comparator<K>,
        iter: I,
    ) -> Result<Self, BulkError> {
        let mut m = SetMultimap::new();
        let mut last_key: Option<K> = None;
        let mut bucket: Vec<V> = Vec::new();
        let close = |m: &mut SetMultimap<K, V>, key: K, b: Vec<V>| {
            m.size += b.len();
            m.data.insert(key, b);
        };
        for (index, (k, v)) in iter.into_iter().enumerate() {
            match last_key {
                None => {
                    last_key = Some(k);
                    bucket.push(v);
                }
                Some(ref prev) => match cmp.compare(prev, &k) {
                    Ordering::Equal => {
                        if !bucket.iter().any(|x| x == &v) {
                            bucket.push(v);
                        }
                    }
                    Ordering::Less => {
                        let prev_key = last_key.take().unwrap();
                        close(&mut m, prev_key, std::mem::take(&mut bucket));
                        last_key = Some(k);
                        bucket.push(v);
                    }
                    Ordering::Greater => return Err(BulkError::OutOfOrder { index }),
                },
            }
        }
        if let Some(prev_key) = last_key {
            close(&mut m, prev_key, bucket);
        }
        Ok(m)
    }

    /// Bulk-loads from input sorted by **key then value** (`key_cmp` strictly
    /// ascending across key runs; `val_cmp` non-decreasing within a run). Equal
    /// adjacent values within a run are deduped in O(1) (no linear scan needed
    /// because equal values are adjacent). A value that decreases within a run
    /// is a [`BulkError::OutOfOrder`].
    pub fn from_sorted_key_values<I: IntoIterator<Item = (K, V)>>(
        key_cmp: Comparator<K>,
        val_cmp: Comparator<V>,
        iter: I,
    ) -> Result<Self, BulkError> {
        let mut m = SetMultimap::new();
        let mut last_key: Option<K> = None;
        let mut bucket: Vec<V> = Vec::new();
        for (index, (k, v)) in iter.into_iter().enumerate() {
            match last_key {
                None => {
                    last_key = Some(k);
                    bucket.push(v);
                }
                Some(ref prev) => match key_cmp.compare(prev, &k) {
                    Ordering::Equal => {
                        // within a run: value must be >= last value.
                        let last_v = bucket.last().unwrap();
                        match val_cmp.compare(last_v, &v) {
                            Ordering::Less => bucket.push(v),
                            Ordering::Equal => { /* adjacent dup -> drop */ }
                            Ordering::Greater => return Err(BulkError::OutOfOrder { index }),
                        }
                    }
                    Ordering::Less => {
                        let prev_key = last_key.take().unwrap();
                        m.size += bucket.len();
                        m.data.insert(prev_key, std::mem::take(&mut bucket));
                        last_key = Some(k);
                        bucket.push(v);
                    }
                    Ordering::Greater => return Err(BulkError::OutOfOrder { index }),
                },
            }
        }
        if let Some(prev_key) = last_key {
            m.size += bucket.len();
            m.data.insert(prev_key, bucket);
        }
        Ok(m)
    }

    /// Inserts `value` into the set for `key`. Idempotent — a duplicate
    /// `value` for the same `key` is silently dropped. Dedupe is a
    /// linear scan of the existing bucket.
    pub fn insert(&mut self, key: K, value: V) {
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

    /// Returns the values for `key` as an immutable view. Empty slice if
    /// the key is absent. The order of values is the insertion order of
    /// unique values; this matches `Multimap`.
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

    /// Total number of unique values across all keys.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Number of distinct keys.
    pub fn distinct_len(&self) -> usize {
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

// Bridge to the parallel module — see `Multimap`'s impl for the key-sectioning
// rationale. Drive with `parallel::batch::for_each_in_batches`.
impl<K: Eq + Hash, V: Eq> crate::parallel::batch::BatchIterable<V> for SetMultimap<K, V> {
    fn len(&self) -> usize {
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

impl<K: Eq + Hash, V: Eq> Default for SetMultimap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrowing iteration over flattened `(&K, &V)` pairs (one per unique value).
impl<'a, K: Eq + Hash, V: Eq> IntoIterator for &'a SetMultimap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Box<dyn Iterator<Item = (&'a K, &'a V)> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
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
    fn insert_dedupes() {
        let mut m: SetMultimap<i32, i32> = SetMultimap::new();
        m.insert(1, 10);
        m.insert(1, 20);
        m.insert(1, 10); // duplicate, silently dropped
        m.insert(2, 30);
        assert_eq!(m.len(), 3);
        assert_eq!(m.distinct_len(), 2);
        assert_eq!(m.get(&1), &[10, 20]);
        assert_eq!(m.get(&2), &[30]);
        assert_eq!(m.get(&99), &[] as &[i32]);
    }

    #[test]
    fn remove_all_updates_size() {
        let mut m: SetMultimap<i32, &str> = SetMultimap::new();
        m.insert(1, "a");
        m.insert(1, "b");
        m.insert(2, "c");
        let removed = m.remove_all(&1);
        assert_eq!(removed, vec!["a", "b"]);
        assert_eq!(m.len(), 1);
        assert_eq!(m.distinct_len(), 1);
        assert_eq!(m.remove_all(&99), Vec::<&str>::new());
    }

    #[test]
    fn contains_key_value() {
        let mut m: SetMultimap<&str, i32> = SetMultimap::new();
        m.insert("a", 1);
        m.insert("a", 2);
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
        m.insert(1, 10);
        m.insert(1, 10);
        assert!(!m.is_empty());
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.distinct_len(), 0);
    }

    #[test]
    fn iter_and_for_each() {
        let mut m: SetMultimap<i32, &str> = SetMultimap::new();
        m.insert(1, "a");
        m.insert(1, "a"); // dedupe
        m.insert(2, "b");
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
        m.insert(1, HashableF64::from(1.5));
        m.insert(1, HashableF64::from(1.5)); // dedupe
        m.insert(1, HashableF64::from(-0.0));
        m.insert(1, HashableF64::from(0.0)); // distinct from -0.0 under HashableF64
        assert_eq!(m.len(), 3);

        // NaN value: HashableF64 uses bit-pattern Eq, so distinct
        // bit-pattern NaNs would be distinct values.
        m.insert(1, HashableF64::from(f64::NAN));
        m.insert(1, HashableF64::from(f64::NAN)); // same bits -> dedupe
        assert_eq!(m.len(), 4);
    }

    #[test]
    fn display_non_empty() {
        let mut m: SetMultimap<i32, i32> = SetMultimap::new();
        m.insert(1, 10);
        let s = m.to_string();
        assert!(s.contains("1=[10]"));
        let empty: SetMultimap<i32, i32> = SetMultimap::new();
        assert_eq!(empty.to_string(), "{}");
    }

    #[test]
    fn len_and_into_iter() {
        let mut m: SetMultimap<i32, i32> = SetMultimap::new();
        m.insert(1, 10);
        m.insert(1, 10); // dedupe
        m.insert(2, 20);
        assert_eq!(m.len(), 2);
        assert_eq!((&m).into_iter().count(), 2);
    }

    use crate::bulk::BulkError;
    use crate::object::strategy::natural_comparator;

    #[test]
    fn bulk_load_dedupes_equal_incremental() {
        let data = vec![(1, 10), (1, 20), (1, 10), (2, 30)];
        let bulk = SetMultimap::bulk_load(data.clone());
        let mut inc = SetMultimap::new();
        for (k, v) in &data {
            inc.insert(*k, *v);
        }
        assert_eq!(bulk.len(), inc.len());
        assert_eq!(bulk.get(&1), &[10, 20]);
        assert_eq!(bulk.len(), 3);
    }

    #[test]
    fn from_sorted_keys_dedupes_within_run() {
        let data = vec![(1, 10), (1, 20), (1, 10), (2, 30)];
        let m = SetMultimap::from_sorted_keys(natural_comparator::<i32>(), data).unwrap();
        assert_eq!(m.get(&1), &[10, 20]);
        assert_eq!(m.get(&2), &[30]);
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn from_sorted_key_values_adjacent_dedupe() {
        // sorted by key then value; equal adjacent values deduped O(1).
        let data = vec![(1, 10), (1, 10), (1, 20), (2, 5), (2, 30)];
        let m = SetMultimap::from_sorted_key_values(
            natural_comparator::<i32>(),
            natural_comparator::<i32>(),
            data,
        )
        .unwrap();
        assert_eq!(m.get(&1), &[10, 20]);
        assert_eq!(m.get(&2), &[5, 30]);
        assert_eq!(m.len(), 4);
    }

    #[test]
    fn from_sorted_key_values_value_decrease_errors() {
        let data = vec![(1, 20), (1, 10)]; // value decreases within key run
        let err = SetMultimap::from_sorted_key_values(
            natural_comparator::<i32>(),
            natural_comparator::<i32>(),
            data,
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 1 }));
    }

    #[test]
    fn from_sorted_keys_key_out_of_order_errors() {
        let data = vec![(2, 10), (1, 20)];
        let err = SetMultimap::from_sorted_keys(natural_comparator::<i32>(), data).unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 1 }));
    }
}
