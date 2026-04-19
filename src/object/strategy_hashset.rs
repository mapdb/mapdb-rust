// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Open-addressing hash set with pluggable [`HashingStrategy`].

use super::strategy::HashingStrategy;
use std::fmt;

const DEFAULT_CAPACITY: usize = 16;

struct Entry<T> {
    value: Option<T>,
}

impl<T> Entry<T> {
    fn empty() -> Self {
        Entry { value: None }
    }

    fn is_occupied(&self) -> bool {
        self.value.is_some()
    }
}

/// An open-addressing hash set that uses a pluggable [`HashingStrategy`]
/// for identity. This allows case-insensitive sets, sets keyed by
/// extracted fields, etc.
pub struct HashSetWithStrategy<T> {
    entries: Vec<Entry<T>>,
    size: usize,
    strategy: HashingStrategy<T>,
}

impl<T: fmt::Debug> fmt::Debug for HashSetWithStrategy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T> HashSetWithStrategy<T> {
    /// Creates an empty set using the given hashing strategy.
    pub fn new(strategy: HashingStrategy<T>) -> Self {
        Self::with_capacity(strategy, DEFAULT_CAPACITY)
    }

    /// Creates an empty set with pre-allocated capacity.
    pub fn with_capacity(strategy: HashingStrategy<T>, capacity: usize) -> Self {
        let cap = next_pow2(capacity);
        let mut entries = Vec::with_capacity(cap);
        for _ in 0..cap {
            entries.push(Entry::empty());
        }
        HashSetWithStrategy {
            entries,
            size: 0,
            strategy,
        }
    }

    /// Adds a value to the set. Returns `true` if the value was newly added,
    /// `false` if it was already present (per the strategy's equality).
    pub fn add(&mut self, value: T) -> bool {
        if self.needs_resize() {
            self.resize();
        }
        let mask = self.entries.len() - 1;
        let mut idx = self.strategy.hash_code(&value) as usize & mask;
        loop {
            if !self.entries[idx].is_occupied() {
                self.entries[idx].value = Some(value);
                self.size += 1;
                return true;
            }
            if self
                .strategy
                .equals(self.entries[idx].value.as_ref().unwrap(), &value)
            {
                return false;
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Removes a value from the set. Returns `true` if the value was found
    /// and removed.
    pub fn remove(&mut self, value: &T) -> bool {
        if self.size == 0 {
            return false;
        }
        let mask = self.entries.len() - 1;
        let mut idx = self.strategy.hash_code(value) as usize & mask;
        loop {
            if !self.entries[idx].is_occupied() {
                return false;
            }
            if self
                .strategy
                .equals(self.entries[idx].value.as_ref().unwrap(), value)
            {
                self.entries[idx].value = None;
                self.size -= 1;
                self.rehash_from(idx, mask);
                return true;
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Returns `true` if the set contains the given value (per the strategy's
    /// equality).
    pub fn contains(&self, value: &T) -> bool {
        if self.size == 0 {
            return false;
        }
        let mask = self.entries.len() - 1;
        let mut idx = self.strategy.hash_code(value) as usize & mask;
        loop {
            if !self.entries[idx].is_occupied() {
                return false;
            }
            if self
                .strategy
                .equals(self.entries[idx].value.as_ref().unwrap(), value)
            {
                return true;
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns `true` if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Removes all elements from the set.
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.value = None;
        }
        self.size = 0;
    }

    /// Returns an iterator over references to the values in the set.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().filter_map(|e| e.value.as_ref())
    }

    /// Calls `f` for each element in the set.
    pub fn for_each(&self, mut f: impl FnMut(&T)) {
        for entry in &self.entries {
            if let Some(ref v) = entry.value {
                f(v);
            }
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

    // ── internal ────────────────────────────────────────────────────

    fn needs_resize(&self) -> bool {
        (self.size + 1) * 4 > self.entries.len() * 3
    }

    fn resize(&mut self) {
        let new_cap = self.entries.len() * 2;
        let old = std::mem::replace(&mut self.entries, {
            let mut v = Vec::with_capacity(new_cap);
            for _ in 0..new_cap {
                v.push(Entry::empty());
            }
            v
        });
        self.size = 0;
        for entry in old {
            if let Some(value) = entry.value {
                self.add(value);
            }
        }
    }

    fn rehash_from(&mut self, deleted: usize, mask: usize) {
        let cap = self.entries.len();
        let mut gap = deleted;
        let mut idx = (deleted + 1) & mask;
        while self.entries[idx].is_occupied() {
            let ideal =
                self.strategy
                    .hash_code(self.entries[idx].value.as_ref().unwrap()) as usize
                    & mask;
            let dist_current = (idx.wrapping_sub(ideal).wrapping_add(cap)) & mask;
            let dist_gap = (gap.wrapping_sub(ideal).wrapping_add(cap)) & mask;
            if dist_current > dist_gap {
                self.entries.swap(gap, idx);
                gap = idx;
            }
            idx = (idx + 1) & mask;
            if idx == gap {
                break;
            }
        }
    }
}

/// Set operations require `Clone` so we can copy values into new sets.
impl<T: Clone> HashSetWithStrategy<T> {
    /// Collects all elements into a `Vec`.
    pub fn to_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }
}

fn next_pow2(n: usize) -> usize {
    if n == 0 {
        return DEFAULT_CAPACITY;
    }
    let mut v = n - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v |= v >> 32;
    v + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::strategy::*;

    #[test]
    fn test_case_insensitive_set() {
        let mut s = HashSetWithStrategy::new(case_insensitive_hashing_strategy());
        assert!(s.add("Hello".to_string()));
        assert!(!s.add("hello".to_string())); // duplicate
        assert!(!s.add("HELLO".to_string())); // duplicate
        assert_eq!(s.len(), 1);
        assert!(s.contains(&"hElLo".to_string()));
        assert!(s.remove(&"HELLO".to_string()));
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_string_set() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        s.add("a".to_string());
        s.add("b".to_string());
        s.add("c".to_string());

        let sel = s.select(|v| v.as_str() != "b");
        assert_eq!(sel.len(), 2);

        let rej = s.reject(|v| v.as_str() == "a");
        assert_eq!(rej.len(), 2);
    }

    #[derive(Debug, Clone)]
    struct Person {
        name: String,
        age: i32,
    }

    #[test]
    fn test_by_field_set() {
        let strategy = by_field(|p: &Person| p.name.clone());
        let mut s = HashSetWithStrategy::new(strategy);
        s.add(Person {
            name: "Alice".into(),
            age: 30,
        });
        s.add(Person {
            name: "Alice".into(),
            age: 25,
        }); // same name -> duplicate
        s.add(Person {
            name: "Bob".into(),
            age: 30,
        });

        assert_eq!(s.len(), 2);
        assert!(s.contains(&Person {
            name: "Alice".into(),
            age: 99
        }));
    }

    #[test]
    fn test_resize_stress() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        for i in 0..1000 {
            s.add(format!("item_{}", i));
        }
        assert_eq!(s.len(), 1000);
        for i in 0..1000 {
            assert!(s.contains(&format!("item_{}", i)));
        }
    }

    #[test]
    fn test_clear() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        s.add("a".to_string());
        s.add("b".to_string());
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_iter() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        s.add("x".to_string());
        s.add("y".to_string());
        let mut items: Vec<&String> = s.iter().collect();
        items.sort();
        assert_eq!(items, vec![&"x".to_string(), &"y".to_string()]);
    }
}
