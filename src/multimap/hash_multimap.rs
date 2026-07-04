// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.

// Generic multimap (one key to many values), built on the project's ported
// `OpenHashMap` rather than `std::HashMap`.

use crate::bulk::BulkError;
use crate::hash_table::OpenHashMap;
use crate::object::strategy::Comparator;
use std::cmp::Ordering;
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

    pub fn insert(&mut self, key: K, value: V) {
        // Single probe via the entry API (was contains+insert double-probe).
        self.data.entry(key).or_default().push(value);
        self.size += 1;
    }

    /// Bulk-loads a fresh multimap from `(key, value)` pairs in **any** order
    /// (hash/group accumulation — no sortedness is claimed or required). Values
    /// for a key preserve their input order; duplicate `(key, value)` pairs are
    /// all kept (list semantics). O(n) amortised, one bucket allocation per
    /// distinct key.
    pub fn bulk_load<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut m = Multimap::new();
        for (k, v) in iter {
            m.insert(k, v);
        }
        m
    }

    /// Bulk-loads from input already **grouped by key** (all values of a key
    /// appear in one contiguous run). Key runs are validated to be strictly
    /// ascending under `cmp`; comparator-equal keys must also be `Eq`-equal.
    /// A key that reappears after its run closed, a comparator/`Eq`
    /// inconsistency, or an out-of-order key is a [`BulkError::OutOfOrder`].
    /// Value order within each key run is preserved. One bucket allocation per
    /// key. O(n).
    pub fn from_sorted_keys<I: IntoIterator<Item = (K, V)>>(
        cmp: Comparator<K>,
        iter: I,
    ) -> Result<Self, BulkError> {
        let mut m = Multimap::new();
        let mut last_key: Option<K> = None;
        let mut bucket: Vec<V> = Vec::new();
        for (index, (k, v)) in iter.into_iter().enumerate() {
            match last_key {
                None => {
                    last_key = Some(k);
                    bucket.push(v);
                }
                Some(ref prev) => match cmp.compare(prev, &k) {
                    Ordering::Equal => {
                        if prev != &k {
                            return Err(BulkError::OutOfOrder { index });
                        }
                        bucket.push(v);
                    }
                    Ordering::Less => {
                        if prev == &k || m.data.contains_key(&k) {
                            return Err(BulkError::OutOfOrder { index });
                        }
                        // close previous run
                        let prev_key = last_key.take().unwrap();
                        m.size += bucket.len();
                        m.data.insert(prev_key, std::mem::take(&mut bucket));
                        last_key = Some(k);
                        bucket.push(v);
                    }
                    Ordering::Greater => {
                        return Err(BulkError::OutOfOrder { index });
                    }
                },
            }
        }
        if let Some(prev_key) = last_key {
            m.size += bucket.len();
            m.data.insert(prev_key, bucket);
        }
        Ok(m)
    }

    /// Bulk-loads from input sorted by **key then value** (`key_cmp` strictly
    /// ascending across key runs; `val_cmp` non-decreasing within a run).
    /// Comparator-equal keys must also be `Eq`-equal. List semantics preserve
    /// equal adjacent values exactly; the value-order check exists so callers
    /// can use the same source contract as set-valued multimaps when their
    /// upstream data is sorted by `(key, value)`.
    pub fn from_sorted_key_values<I: IntoIterator<Item = (K, V)>>(
        key_cmp: Comparator<K>,
        val_cmp: Comparator<V>,
        iter: I,
    ) -> Result<Self, BulkError> {
        let mut m = Multimap::new();
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
                        if prev != &k {
                            return Err(BulkError::OutOfOrder { index });
                        }
                        let last_v = bucket.last().unwrap();
                        if val_cmp.compare(last_v, &v) == Ordering::Greater {
                            return Err(BulkError::OutOfOrder { index });
                        }
                        bucket.push(v);
                    }
                    Ordering::Less => {
                        if prev == &k || m.data.contains_key(&k) {
                            return Err(BulkError::OutOfOrder { index });
                        }
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

    /// Total number of values across all keys.
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

    /// Retain only the `(key, value)` pairs for which `keep(&k, &v)` returns
    /// `true`. Values a key loses are dropped from its list (order preserved);
    /// a key left with no values is removed entirely. The total-value count
    /// (`len`) stays exact. O(total values).
    ///
    /// # Panics
    /// If `keep` panics, the panic propagates and `size` is recomputed from the
    /// surviving values on the way out (via a drop guard), so a caught panic
    /// still leaves `len()` consistent with the pairs actually present.
    pub fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(&K, &V) -> bool,
    {
        // `size` is side state kept beside the backing map; recompute it from
        // the survivors via a guard so a `keep` panic (which unwinds through the
        // kernel's rebuild-in-place `retain`, leaving `data` valid) cannot leave
        // it stale — the same panic-consistency pattern as `HashBag::retain`.
        struct FixSize<'a, K: Eq + Hash, V>(&'a mut Multimap<K, V>);
        impl<K: Eq + Hash, V> Drop for FixSize<'_, K, V> {
            fn drop(&mut self) {
                self.0.size = self.0.data.iter().map(|(_, vs)| vs.len()).sum();
            }
        }
        let guard = FixSize(self);
        guard.0.data.retain(|k, vs| {
            vs.retain(|v| keep(k, v));
            // Drop the key when it has no values left.
            !vs.is_empty()
        });
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

impl<K: Eq + Hash, V> Default for Multimap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrowing iteration over flattened `(&K, &V)` pairs (one per stored value).
impl<'a, K: Eq + Hash, V> IntoIterator for &'a Multimap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Box<dyn Iterator<Item = (&'a K, &'a V)> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
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
        m.insert("a", 1);
        m.insert("a", 2);
        m.insert("b", 3);
        assert_eq!(m.get(&"a"), &[1, 2]);
        assert_eq!(m.get(&"b"), &[3]);
        assert_eq!(m.get(&"c"), &[] as &[i32]);
        assert_eq!(m.len(), 3);
        assert_eq!(m.distinct_len(), 2);
    }

    #[test]
    fn test_remove_all() {
        let mut m = Multimap::new();
        m.insert(1, "a");
        m.insert(1, "b");
        m.insert(2, "c");
        let removed = m.remove_all(&1);
        assert_eq!(removed, vec!["a", "b"]);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn test_contains_key() {
        let mut m = Multimap::<i32, i32>::new();
        m.insert(1, 10);
        assert!(m.contains_key(&1));
        assert!(!m.contains_key(&2));
    }

    #[test]
    fn test_clear() {
        let mut m = Multimap::new();
        m.insert(1, "a");
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_iter() {
        let mut m = Multimap::new();
        m.insert(1, "a");
        m.insert(1, "b");
        m.insert(2, "c");
        assert_eq!(m.iter().count(), 3);
    }

    #[test]
    fn test_display() {
        let mut m = Multimap::new();
        m.insert(1, "a");
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_len_counts_values() {
        let mut m = Multimap::new();
        m.insert(1, "a");
        m.insert(1, "b");
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_into_iter_borrowing() {
        let mut m = Multimap::new();
        m.insert(1, "a");
        m.insert(1, "b");
        m.insert(2, "c");
        assert_eq!((&m).into_iter().count(), 3);
    }

    use crate::bulk::BulkError;
    use crate::object::strategy::{natural_comparator, Comparator};

    #[test]
    fn bulk_load_equals_incremental() {
        let data = vec![(1, "a"), (2, "b"), (1, "c"), (3, "d"), (2, "e")];
        let bulk = Multimap::bulk_load(data.clone());
        let mut inc = Multimap::new();
        for (k, v) in &data {
            inc.insert(*k, *v);
        }
        assert_eq!(bulk.len(), inc.len());
        assert_eq!(bulk.distinct_len(), inc.distinct_len());
        for k in [1, 2, 3] {
            assert_eq!(bulk.get(&k), inc.get(&k));
        }
    }

    #[test]
    fn from_sorted_keys_groups_and_preserves_value_order() {
        let data = vec![(1, "a"), (1, "b"), (2, "c"), (3, "d"), (3, "e")];
        let m = Multimap::from_sorted_keys(natural_comparator::<i32>(), data).unwrap();
        assert_eq!(m.get(&1), &["a", "b"]); // value order preserved within run
        assert_eq!(m.get(&2), &["c"]);
        assert_eq!(m.get(&3), &["d", "e"]);
        assert_eq!(m.len(), 5);
        assert_eq!(m.distinct_len(), 3);
    }

    #[test]
    fn from_sorted_key_values_preserves_adjacent_duplicates() {
        let data = vec![(1, 10), (1, 10), (1, 20), (2, 5), (2, 5)];
        let m = Multimap::from_sorted_key_values(
            natural_comparator::<i32>(),
            natural_comparator::<i32>(),
            data,
        )
        .unwrap();
        assert_eq!(m.get(&1), &[10, 10, 20]);
        assert_eq!(m.get(&2), &[5, 5]);
        assert_eq!(m.len(), 5);
        assert_eq!(m.distinct_len(), 2);
    }

    #[test]
    fn sorted_builders_reject_comparator_equal_eq_distinct_keys() {
        let abs_cmp = Comparator::new(Box::new(|a: &i32, b: &i32| a.abs().cmp(&b.abs())));
        let err =
            Multimap::from_sorted_keys(abs_cmp.clone(), vec![(1, "a"), (-1, "b")]).unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 1 }));
        let err = Multimap::from_sorted_key_values(
            abs_cmp,
            natural_comparator::<i32>(),
            vec![(1, 10), (-1, 20)],
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 1 }));
    }

    #[test]
    fn from_sorted_key_values_value_decrease_errors() {
        let data = vec![(1, 10), (1, 9), (2, 20)];
        let err = Multimap::from_sorted_key_values(
            natural_comparator::<i32>(),
            natural_comparator::<i32>(),
            data,
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 1 }));
    }

    #[test]
    fn from_sorted_key_values_key_out_of_order_errors() {
        let data = vec![(1, 10), (2, 20), (1, 30)];
        let err = Multimap::from_sorted_key_values(
            natural_comparator::<i32>(),
            natural_comparator::<i32>(),
            data,
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 2 }));
    }

    #[test]
    fn from_sorted_keys_out_of_order_errors() {
        let data = vec![(1, "a"), (3, "b"), (2, "c")];
        let err = Multimap::from_sorted_keys(natural_comparator::<i32>(), data).unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 2 }));
    }

    #[test]
    fn from_sorted_keys_empty() {
        let m: Multimap<i32, i32> =
            Multimap::from_sorted_keys(natural_comparator::<i32>(), Vec::new()).unwrap();
        assert!(m.is_empty());
    }

    #[test]
    fn retain_filters_pairs_and_drops_empty_keys() {
        let mut m = Multimap::new();
        for (k, v) in [(1, 10), (1, 11), (1, 12), (2, 20), (3, 30), (3, 31)] {
            m.insert(k, v);
        }
        assert_eq!(m.len(), 6);
        // Keep only even values.
        m.retain(|_, v| v % 2 == 0);
        assert_eq!(m.get(&1), &[10, 12]); // 11 dropped, order preserved
        assert_eq!(m.get(&2), &[20]);
        assert_eq!(m.get(&3), &[30]); // 31 dropped
        assert_eq!(m.len(), 4);
        assert_eq!(m.distinct_len(), 3);
        // A key whose values all fail is removed entirely.
        m.retain(|&k, _| k != 2);
        assert!(!m.contains_key(&2));
        assert_eq!(m.distinct_len(), 2);
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn retain_key_emptied_is_removed() {
        let mut m = Multimap::new();
        m.insert("x", 1);
        m.insert("x", 2);
        m.insert("y", 3);
        m.retain(|k, _| *k == "y"); // drops all of x's values
        assert!(!m.contains_key(&"x"));
        assert_eq!(m.distinct_len(), 1);
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(&"y"), &[3]);
    }

    #[test]
    fn retain_size_consistent_after_caught_panic() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let mut m = Multimap::new();
        for (k, v) in [(1, 1), (1, 2), (2, 3)] {
            m.insert(k, v);
        }
        let r = catch_unwind(AssertUnwindSafe(|| {
            m.retain(|_, _| panic!("boom"));
        }));
        assert!(r.is_err());
        // len() must equal the actual number of surviving (k, v) pairs.
        assert_eq!(m.len(), m.iter().count());
    }
}
