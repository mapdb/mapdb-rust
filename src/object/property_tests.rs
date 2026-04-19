// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Property-based tests using a simple LCG random number generator.
//! No external dependencies required.

#[cfg(test)]
mod tests {
    use crate::object::*;

    const TRIALS: usize = 1000;

    struct Rng(u64);
    impl Rng {
        fn new(seed: u64) -> Self {
            Rng(seed)
        }
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 33
        }
        fn next_usize(&mut self, bound: usize) -> usize {
            (self.next() as usize) % bound
        }
        fn next_i32(&mut self, bound: i32) -> i32 {
            (self.next() as i32).abs() % bound
        }
    }

    // Property: After pushing N elements, len() == N
    #[test]
    fn prop_arraylist_len_after_push() {
        let mut rng = Rng::new(42);
        for _ in 0..TRIALS {
            let n = rng.next_usize(200);
            let mut list = ArrayList::new();
            for i in 0..n as i32 {
                list.push(i);
            }
            assert_eq!(list.len(), n);
        }
    }

    // Property: Contains(v) == true iff v was added to set
    #[test]
    fn prop_hashset_contains_after_add() {
        let mut rng = Rng::new(43);
        for _ in 0..TRIALS {
            let n = rng.next_usize(200);
            let mut set = HashSet::new();
            let mut added = std::collections::HashSet::new();
            for _ in 0..n {
                let v = rng.next_i32(500);
                set.add(v);
                added.insert(v);
            }
            for &v in &added {
                assert!(set.contains(&v), "set should contain {}", v);
            }
            for probe in 500..600 {
                if !added.contains(&probe) {
                    assert!(!set.contains(&probe), "set should not contain {}", probe);
                }
            }
        }
    }

    // Property: Bag occurrences match add count
    #[test]
    fn prop_hashbag_occurrences() {
        let mut rng = Rng::new(44);
        for _ in 0..TRIALS {
            let n = rng.next_usize(200);
            let mut bag = HashBag::new();
            let mut counts = std::collections::HashMap::new();
            for _ in 0..n {
                let v = rng.next_i32(50);
                bag.add(v);
                *counts.entry(v).or_insert(0usize) += 1;
            }
            for (&v, &expected) in &counts {
                assert_eq!(bag.occurrences_of(&v), expected);
            }
            assert_eq!(bag.len(), n);
        }
    }

    // Property: HashMap get returns what was put
    #[test]
    fn prop_hashmap_get_after_put() {
        let mut rng = Rng::new(45);
        for _ in 0..TRIALS {
            let n = rng.next_usize(200);
            let mut map = HashMap::new();
            let mut expected = std::collections::HashMap::new();
            for _ in 0..n {
                let k = rng.next_i32(100);
                let v = rng.next_i32(10000);
                map.insert(k, v);
                expected.insert(k, v);
            }
            for (&k, &v) in &expected {
                assert_eq!(map.get(&k), Some(&v));
            }
            assert_eq!(map.len(), expected.len());
        }
    }

    // Property: Stack LIFO order
    #[test]
    fn prop_arraystack_lifo() {
        let mut rng = Rng::new(46);
        for _ in 0..TRIALS {
            let n = rng.next_usize(100);
            let mut stack = ArrayStack::new();
            let mut values = Vec::new();
            for _ in 0..n {
                let v = rng.next_i32(10000);
                values.push(v);
                stack.push(v);
            }
            for v in values.into_iter().rev() {
                assert_eq!(stack.pop(), Some(v));
            }
            assert!(stack.is_empty());
        }
    }

    // Property: iter() visits exactly len() elements
    #[test]
    fn prop_arraylist_iter_count() {
        let mut rng = Rng::new(47);
        for _ in 0..TRIALS {
            let n = rng.next_usize(200);
            let mut list = ArrayList::new();
            for _ in 0..n {
                list.push(rng.next_i32(10000));
            }
            let count = list.iter().count();
            assert_eq!(count, n);
        }
    }

    // Property: BiMap bijection — values are unique
    #[test]
    fn prop_hashbimap_bijection() {
        let mut rng = Rng::new(48);
        for _ in 0..TRIALS {
            let n = rng.next_usize(100);
            let mut bm = HashBiMap::new();
            for _ in 0..n {
                let k = rng.next_i32(200);
                let v = rng.next_i32(200);
                bm.put(k, v);
            }
            let mut seen_values = std::collections::HashSet::new();
            for (_, v) in bm.iter() {
                assert!(seen_values.insert(v), "duplicate value in bimap");
            }
        }
    }
}
