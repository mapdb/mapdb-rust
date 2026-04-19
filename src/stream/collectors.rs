// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

// Collector functions — terminal operations that consume iterators.
// These complement Rust's built-in Iterator methods (sum, count, collect, etc.)
// with Eclipse Collections-style operations (group_by, partition, etc.)

use std::collections::HashMap;
use std::hash::Hash;

/// Groups elements by a key function, returning a HashMap of key -> Vec<V>.
pub fn group_by<V, K: Eq + Hash>(
    iter: impl Iterator<Item = V>,
    key_fn: impl Fn(&V) -> K,
) -> HashMap<K, Vec<V>> {
    let mut result: HashMap<K, Vec<V>> = HashMap::new();
    for v in iter {
        let k = key_fn(&v);
        result.entry(k).or_default().push(v);
    }
    result
}

/// Partitions elements into two Vecs based on a predicate.
/// Returns (matching, non_matching).
pub fn partition<V>(
    iter: impl Iterator<Item = V>,
    predicate: impl Fn(&V) -> bool,
) -> (Vec<V>, Vec<V>) {
    let mut matching = Vec::new();
    let mut non_matching = Vec::new();
    for v in iter {
        if predicate(&v) {
            matching.push(v);
        } else {
            non_matching.push(v);
        }
    }
    (matching, non_matching)
}

/// Joins string elements with a separator.
pub fn joining(iter: impl Iterator<Item = String>, separator: &str) -> String {
    let parts: Vec<String> = iter.collect();
    parts.join(separator)
}

/// Joins string elements with separator, prefix, and suffix.
pub fn joining_with(
    iter: impl Iterator<Item = String>,
    separator: &str,
    prefix: &str,
    suffix: &str,
) -> String {
    let inner = joining(iter, separator);
    format!("{}{}{}", prefix, inner, suffix)
}

/// Collects into a HashMap using key and value extractor functions.
pub fn to_map_by<V, K: Eq + Hash, MV>(
    iter: impl Iterator<Item = V>,
    key_fn: impl Fn(&V) -> K,
    value_fn: impl Fn(&V) -> MV,
) -> HashMap<K, MV> {
    let mut result = HashMap::new();
    for v in iter {
        result.insert(key_fn(&v), value_fn(&v));
    }
    result
}

/// Sums values extracted from elements.
pub fn sum_by<V, N>(iter: impl Iterator<Item = V>, extract: impl Fn(&V) -> N) -> N
where
    N: std::ops::Add<Output = N> + Default,
{
    iter.fold(N::default(), |acc, v| acc + extract(&v))
}

/// Returns the minimum element by a comparison function.
pub fn min_by<V>(iter: impl Iterator<Item = V>, less: impl Fn(&V, &V) -> bool) -> Option<V> {
    iter.reduce(|a, b| if less(&b, &a) { b } else { a })
}

/// Returns the maximum element by a comparison function.
pub fn max_by<V>(iter: impl Iterator<Item = V>, less: impl Fn(&V, &V) -> bool) -> Option<V> {
    iter.reduce(|a, b| if less(&a, &b) { b } else { a })
}

/// Collects elements into fixed-size chunks. The final chunk may be shorter
/// than `size`. Panics if `size == 0`.
pub fn chunked<V>(iter: impl Iterator<Item = V>, size: usize) -> Vec<Vec<V>> {
    assert!(size > 0, "chunked: size must be > 0");
    let mut result: Vec<Vec<V>> = Vec::new();
    let mut current: Vec<V> = Vec::with_capacity(size);
    for v in iter {
        current.push(v);
        if current.len() == size {
            result.push(std::mem::replace(&mut current, Vec::with_capacity(size)));
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

/// Partitions elements by a key function, returning a HashMap of
/// key -> Vec<V>. Alias for `group_by` matching the Eclipse Collections
/// naming. Retained for API symmetry with `partition`.
pub fn group_by_each<V, K: Eq + Hash>(
    iter: impl Iterator<Item = V>,
    key_fn: impl Fn(&V) -> Vec<K>,
) -> HashMap<K, Vec<V>>
where
    V: Clone,
{
    let mut result: HashMap<K, Vec<V>> = HashMap::new();
    for v in iter {
        let keys = key_fn(&v);
        for k in keys {
            result.entry(k).or_default().push(v.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by() {
        let groups = group_by(vec![1, 2, 3, 4, 5, 6].into_iter(), |v| v % 2);
        assert_eq!(groups[&0].len(), 3); // 2, 4, 6
        assert_eq!(groups[&1].len(), 3); // 1, 3, 5
    }

    #[test]
    fn test_partition() {
        let (evens, odds) = partition(vec![1, 2, 3, 4, 5].into_iter(), |v| v % 2 == 0);
        assert_eq!(evens, vec![2, 4]);
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[test]
    fn test_joining() {
        let result = joining(vec!["a", "b", "c"].into_iter().map(String::from), ", ");
        assert_eq!(result, "a, b, c");
    }

    #[test]
    fn test_joining_with() {
        let result = joining_with(vec!["a", "b"].into_iter().map(String::from), ", ", "[", "]");
        assert_eq!(result, "[a, b]");
    }

    #[test]
    fn test_to_map_by() {
        let m = to_map_by(
            vec!["hi", "hello"].into_iter(),
            |s| s.len(),
            |s| s.to_uppercase(),
        );
        assert_eq!(m[&2], "HI");
        assert_eq!(m[&5], "HELLO");
    }

    #[test]
    fn test_sum_by() {
        let total: i32 = sum_by(vec!["hi", "hey", "hello"].into_iter(), |s| s.len() as i32);
        assert_eq!(total, 10); // 2 + 3 + 5
    }

    #[test]
    fn test_min_max_by() {
        let data = vec![3, 1, 4, 1, 5];
        assert_eq!(min_by(data.iter(), |a, b| a < b), Some(&1));
        assert_eq!(max_by(data.iter(), |a, b| a < b), Some(&5));
    }

    #[test]
    fn test_chunked_even() {
        let out = chunked(vec![1, 2, 3, 4, 5, 6].into_iter(), 2);
        assert_eq!(out, vec![vec![1, 2], vec![3, 4], vec![5, 6]]);
    }

    #[test]
    fn test_chunked_uneven_tail() {
        let out = chunked(vec![1, 2, 3, 4, 5].into_iter(), 2);
        assert_eq!(out, vec![vec![1, 2], vec![3, 4], vec![5]]);
    }

    #[test]
    fn test_chunked_empty() {
        let out: Vec<Vec<i32>> = chunked(std::iter::empty(), 3);
        assert!(out.is_empty());
    }

    #[test]
    #[should_panic]
    fn test_chunked_zero_size_panics() {
        let _ = chunked(vec![1].into_iter(), 0);
    }

    #[test]
    fn test_group_by_each() {
        // Each element belongs to multiple groups via its divisors.
        let groups = group_by_each(vec![1, 2, 3, 4].into_iter(), |v| {
            (1..=*v).filter(|d| v % d == 0).collect()
        });
        // 1 divides 1,2,3,4
        assert_eq!(groups[&1].len(), 4);
        // 2 divides 2,4
        assert_eq!(groups[&2].len(), 2);
        // 4 divides only 4
        assert_eq!(groups[&4].len(), 1);
    }
}
