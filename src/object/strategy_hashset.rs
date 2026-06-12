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
        let cap = capacity
            .max(DEFAULT_CAPACITY)
            .checked_next_power_of_two()
            .unwrap_or(usize::MAX);
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

    /// Inserts a value into the set. Returns `true` if the value was newly
    /// inserted, `false` if it was already present (per the strategy's equality).
    pub fn insert(&mut self, value: T) -> bool {
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
        // Grow strictly *below* the 0.75 load factor: the table must
        // not hold `(size+1)` entries once that count reaches `cap*3/4`.
        // `>=` (not `>`) ensures we grow when `cap*3 == (size+1)*4`
        // exactly (e.g. the 12th insert into a capacity-16 table).
        (self.size + 1) * 4 >= self.entries.len() * 3
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
                self.insert(value);
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

    // Exercises the std `next_power_of_two()` capacity path (previously a
    // hand-rolled `next_pow2` with an ungated `v >> 32` that panicked on
    // 32-bit targets). A from-scratch grow drives the same code path.
    #[test]
    fn test_capacity_growth_via_next_power_of_two() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        assert_eq!(s.entries.len(), 16);
        for i in 0..200 {
            s.insert(format!("k{}", i));
        }
        assert!(s.entries.len() > 16);
        assert!(s.entries.len().is_power_of_two());
        assert_eq!(s.len(), 200);
    }

    // Spec: load factor must stay strictly below 0.75. With a capacity-16
    // table, the 12th distinct entry (size would reach 12 == 16*0.75) must
    // trigger a grow before it is stored.
    #[test]
    fn test_load_factor_strictly_below_three_quarters() {
        let mut s = HashSetWithStrategy::new(string_hashing_strategy());
        assert_eq!(s.entries.len(), 16);
        for i in 0..11 {
            s.insert(format!("k{}", i));
        }
        assert_eq!(s.entries.len(), 16);
        s.insert("k11".to_string());
        assert_eq!(s.entries.len(), 32);
        assert_eq!(s.len(), 12);
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
