// AUTO-GENERATED. DO NOT EDIT.

/// Read-only trait for any map from `f64` keys to `f64` values.
///
/// Implementors provide four required methods — [`len`], [`contains_key`],
/// [`get`], and [`iter`] — and get a rich set of defaulted query methods
/// for free, following the same pattern as the Collection trait.
pub trait F64F64Map {
    // ── Required methods ────────────────────────────────────────────

    /// Returns the number of key-value entries.
    fn len(&self) -> usize;

    /// Returns true if the map contains an entry for the key.
    fn contains_key(&self, key: f64) -> bool;

    /// Returns the value for the key, or None if absent.
    fn get(&self, key: f64) -> Option<f64>;

    /// Returns an iterator over (key, value) pairs.
    fn iter(&self) -> impl Iterator<Item = (f64, f64)> + '_;

    // ── Defaulted methods ───────────────────────────────────────────

    /// Returns true if the map is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the map contains an entry with the given value.
    fn contains_value(&self, value: f64) -> bool {
        self.iter().any(|(_, v)| v.to_bits() == value.to_bits())
    }

    /// Calls the function for each key-value pair.
    fn for_each_key_value(&self, mut f: impl FnMut(f64, f64)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    /// Returns true if any entry satisfies the predicate.
    fn any_satisfy(&self, predicate: impl Fn(f64, f64) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }

    /// Returns true if all entries satisfy the predicate.
    fn all_satisfy(&self, predicate: impl Fn(f64, f64) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }

    /// Returns true if no entry satisfies the predicate.
    fn none_satisfy(&self, predicate: impl Fn(f64, f64) -> bool) -> bool {
        !self.iter().any(|(k, v)| predicate(k, v))
    }

    /// Returns the count of entries satisfying the predicate.
    fn count_where(&self, predicate: impl Fn(f64, f64) -> bool) -> usize {
        self.iter().filter(|&(k, v)| predicate(k, v)).count()
    }

    /// Returns the first entry satisfying the predicate, or None.
    fn detect(&self, predicate: impl Fn(f64, f64) -> bool) -> Option<(f64, f64)> {
        self.iter().find(|&(k, v)| predicate(k, v))
    }

    /// Folds all entries using the given function and initial value.
    fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, f64, f64) -> R) -> R {
        let mut acc = initial;
        for (k, v) in self.iter() {
            acc = f(acc, k, v);
        }
        acc
    }

    /// Returns all keys as a Vec.
    fn keys_to_vec(&self) -> Vec<f64> {
        self.iter().map(|(k, _)| k).collect()
    }

    /// Returns all values as a Vec.
    fn values_to_vec(&self) -> Vec<f64> {
        self.iter().map(|(_, v)| v).collect()
    }
}

/// Mutable map trait extending F64F64Map.
pub trait F64F64MutableMap: F64F64Map {
    /// Inserts a key/value pair. Returns the previous value if present.
    fn insert(&mut self, key: f64, value: f64) -> Option<f64>;

    /// Removes the entry for the key. Returns the removed value if present.
    fn remove(&mut self, key: f64) -> Option<f64>;

    /// Removes all entries.
    fn clear(&mut self);
}

#[cfg(test)]
mod verify {
    use super::*;
    fn _assert_map<T: F64F64Map>() {}
    fn _assert_mut<T: F64F64MutableMap>() {}

    /// Compile-time verification: every concrete `f64` → `f64` map type
    /// implements the read-only and (where applicable) mutable map trait.
    #[test]
    fn types_implement_traits() {
        _assert_map::<crate::hashmap::f64_f64_hash_map::F64F64HashMap>();
        _assert_mut::<crate::hashmap::f64_f64_hash_map::F64F64HashMap>();
        _assert_map::<crate::treemap::f64_f64_tree_map::F64F64TreeMap>();
        _assert_mut::<crate::treemap::f64_f64_tree_map::F64F64TreeMap>();
        _assert_map::<crate::hashmap::f64_f64_hash_bi_map::F64F64HashBiMap>();
        _assert_mut::<crate::hashmap::f64_f64_hash_bi_map::F64F64HashBiMap>();
        _assert_map::<crate::immutable::immutable_f64_f64_hash_map::ImmutableF64F64HashMap>();
    }
}
