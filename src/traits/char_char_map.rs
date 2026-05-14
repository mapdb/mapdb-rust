// AUTO-GENERATED. DO NOT EDIT.

/// Read-only trait for any map from `char` keys to `char` values.
///
/// Implementors provide four required methods — [`len`], [`contains_key`],
/// [`get`], and [`iter`] — and get a rich set of defaulted query methods
/// for free, following the same pattern as the Collection trait.
pub trait CharCharMap {
    // ── Required methods ────────────────────────────────────────────

    /// Returns the number of key-value entries.
    fn len(&self) -> usize;

    /// Returns true if the map contains an entry for the key.
    fn contains_key(&self, key: char) -> bool;

    /// Returns the value for the key, or None if absent.
    fn get(&self, key: char) -> Option<char>;

    /// Returns an iterator over (key, value) pairs.
    fn iter(&self) -> impl Iterator<Item = (char, char)> + '_;

    // ── Defaulted methods ───────────────────────────────────────────

    /// Returns true if the map is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the map contains an entry with the given value.
    fn contains_value(&self, value: char) -> bool {
        self.iter().any(|(_, v)| v == value)
    }

    /// Calls the function for each key-value pair.
    fn for_each_key_value(&self, mut f: impl FnMut(char, char)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    /// Returns true if any entry satisfies the predicate.
    fn any_satisfy(&self, predicate: impl Fn(char, char) -> bool) -> bool {
        self.iter().any(|(k, v)| predicate(k, v))
    }

    /// Returns true if all entries satisfy the predicate.
    fn all_satisfy(&self, predicate: impl Fn(char, char) -> bool) -> bool {
        self.iter().all(|(k, v)| predicate(k, v))
    }

    /// Returns true if no entry satisfies the predicate.
    fn none_satisfy(&self, predicate: impl Fn(char, char) -> bool) -> bool {
        !self.iter().any(|(k, v)| predicate(k, v))
    }

    /// Returns the count of entries satisfying the predicate.
    fn count_where(&self, predicate: impl Fn(char, char) -> bool) -> usize {
        self.iter().filter(|&(k, v)| predicate(k, v)).count()
    }

    /// Returns the first entry satisfying the predicate, or None.
    fn detect(&self, predicate: impl Fn(char, char) -> bool) -> Option<(char, char)> {
        self.iter().find(|&(k, v)| predicate(k, v))
    }

    /// Folds all entries using the given function and initial value.
    fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, char, char) -> R) -> R {
        let mut acc = initial;
        for (k, v) in self.iter() {
            acc = f(acc, k, v);
        }
        acc
    }

    /// Returns all keys as a Vec.
    fn keys_to_vec(&self) -> Vec<char> {
        self.iter().map(|(k, _)| k).collect()
    }

    /// Returns all values as a Vec.
    fn values_to_vec(&self) -> Vec<char> {
        self.iter().map(|(_, v)| v).collect()
    }
}

/// Mutable map trait extending CharCharMap.
pub trait CharCharMutableMap: CharCharMap {
    /// Inserts a key/value pair. Returns the previous value if present.
    fn insert(&mut self, key: char, value: char) -> Option<char>;

    /// Removes the entry for the key. Returns the removed value if present.
    fn remove(&mut self, key: char) -> Option<char>;

    /// Removes all entries.
    fn clear(&mut self);
}

#[cfg(test)]
mod verify {
    use super::*;
    fn _assert_map<T: CharCharMap>() {}
    fn _assert_mut<T: CharCharMutableMap>() {}

    /// Compile-time verification: every concrete `char` → `char` map type
    /// implements the read-only and (where applicable) mutable map trait.
    #[test]
    fn types_implement_traits() {
        _assert_map::<crate::hashmap::char_char_hash_map::CharCharHashMap>();
        _assert_mut::<crate::hashmap::char_char_hash_map::CharCharHashMap>();
        _assert_map::<crate::treemap::char_char_tree_map::CharCharTreeMap>();
        _assert_mut::<crate::treemap::char_char_tree_map::CharCharTreeMap>();
        _assert_map::<crate::hashmap::char_char_hash_bi_map::CharCharHashBiMap>();
        _assert_mut::<crate::hashmap::char_char_hash_bi_map::CharCharHashBiMap>();
        _assert_map::<crate::immutable::immutable_char_char_hash_map::ImmutableCharCharHashMap>();
    }
}
