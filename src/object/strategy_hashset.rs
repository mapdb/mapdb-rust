// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Open-addressing hash set with pluggable [`HashingStrategy`].
//!
//! A thin wrapper over [`HashMapWithStrategy<T, ()>`](super::HashMapWithStrategy)
//! — one probing implementation for both (blueprint M6), the same move as
//! `LinkedHashSet` = `LinkedHashMap<T, ()>`. It inherits the map's kernel:
//! [`IndexTable`](crate::index_table) + [`SlotList`](crate::slot_list), with
//! backward-shift deletion that never re-invokes the user's strategy.

use super::strategy::HashingStrategy;
use super::HashMapWithStrategy;
use std::fmt;

/// An open-addressing hash set that uses a pluggable [`HashingStrategy`]
/// for identity. This allows case-insensitive sets, sets keyed by
/// extracted fields, etc.
pub struct HashSetWithStrategy<T> {
    map: HashMapWithStrategy<T, ()>,
}

impl<T: fmt::Debug> fmt::Debug for HashSetWithStrategy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T> HashSetWithStrategy<T> {
    /// Creates an empty set using the given hashing strategy.
    pub fn new(strategy: HashingStrategy<T>) -> Self {
        HashSetWithStrategy {
            map: HashMapWithStrategy::new(strategy),
        }
    }

    /// Creates an empty set with pre-allocated capacity.
    pub fn with_capacity(strategy: HashingStrategy<T>, capacity: usize) -> Self {
        HashSetWithStrategy {
            map: HashMapWithStrategy::with_capacity(strategy, capacity),
        }
    }

    /// Inserts a value into the set. Returns `true` if the value was newly
    /// inserted, `false` if it was already present (per the strategy's equality).
    pub fn insert(&mut self, value: T) -> bool {
        self.map.insert(value, ()).is_none()
    }

    /// Removes a value from the set. Returns `true` if the value was found
    /// and removed.
    pub fn remove(&mut self, value: &T) -> bool {
        self.map.remove(value).is_some()
    }

    /// Returns `true` if the set contains the given value (per the strategy's
    /// equality).
    pub fn contains(&self, value: &T) -> bool {
        self.map.contains_key(value)
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Removes all elements from the set.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Returns an iterator over references to the values in the set.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.map.keys()
    }

    /// Calls `f` for each element in the set.
    pub fn for_each(&self, mut f: impl FnMut(&T)) {
        for v in self.iter() {
            f(v);
        }
    }

    /// Returns elements matching the predicate as a `Vec`.
    pub fn select(&self, predicate: impl Fn(&T) -> bool) -> Vec<&T> {
        self.iter().filter(|v| predicate(v)).collect()
    }

    /// Returns elements not matching the predicate as a `Vec`.
    pub fn reject(&self, predicate: impl Fn(&T) -> bool) -> Vec<&T> {
        self.iter().filter(|v| !predicate(v)).collect()
    }
}

/// Set operations require `Clone` so we can copy values into new sets.
impl<T: Clone> HashSetWithStrategy<T> {
    /// Collects all elements into a `Vec`.
    pub fn to_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::strategy::*;

    #[test]
    fn test_case_insensitive_set() {
        let mut s = HashSetWithStrategy::new(case_insensitive_hashing_strategy());
        assert!(s.insert("Hello".to_string()));
        assert!(!s.insert("hello".to_string())); // duplicate
        assert!(!s.insert("HELLO".to_string())); // duplicate
        assert_eq!(s.len(), 1);
        assert!(s.contains(&"hElLo".to_string()));
        assert!(s.remove(&"HELLO".to_string()));
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_string_set() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        s.insert("a".to_string());
        s.insert("b".to_string());
        s.insert("c".to_string());

        let sel = s.select(|v| v.as_str() != "b");
        assert_eq!(sel.len(), 2);

        let rej = s.reject(|v| v.as_str() == "a");
        assert_eq!(rej.len(), 2);
    }

    #[derive(Debug, Clone)]
    struct Person {
        name: String,
        _age: i32,
    }

    #[test]
    fn test_by_field_set() {
        let strategy = by_field(|p: &Person| p.name.clone());
        let mut s = HashSetWithStrategy::new(strategy);
        s.insert(Person {
            name: "Alice".into(),
            _age: 30,
        });
        s.insert(Person {
            name: "Alice".into(),
            _age: 25,
        }); // same name -> duplicate
        s.insert(Person {
            name: "Bob".into(),
            _age: 30,
        });

        assert_eq!(s.len(), 2);
        assert!(s.contains(&Person {
            name: "Alice".into(),
            _age: 99
        }));
    }

    #[test]
    fn test_resize_stress() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        for i in 0..1000 {
            s.insert(format!("item_{}", i));
        }
        assert_eq!(s.len(), 1000);
        for i in 0..1000 {
            assert!(s.contains(&format!("item_{}", i)));
        }
    }

    // Remove-heavy churn: exercises backward-shift deletion + slot recycling
    // through the wrapped map. The surviving set must be exactly correct.
    #[test]
    fn test_remove_churn() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        for i in 0..300 {
            s.insert(format!("x{}", i));
        }
        for i in (0..300).step_by(3) {
            assert!(s.remove(&format!("x{}", i)));
        }
        assert_eq!(s.len(), 200);
        for i in 0..300 {
            assert_eq!(s.contains(&format!("x{}", i)), i % 3 != 0);
        }
    }

    #[test]
    fn test_clear() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        s.insert("a".to_string());
        s.insert("b".to_string());
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_iter() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        s.insert("x".to_string());
        s.insert("y".to_string());
        let mut items: Vec<&String> = s.iter().collect();
        items.sort();
        assert_eq!(items, vec![&"x".to_string(), &"y".to_string()]);
    }
}
