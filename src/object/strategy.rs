// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Hashing strategies and comparators for strategy-backed collections.
//!
//! A [`HashingStrategy`] externalises identity for hash-based collections,
//! allowing case-insensitive keys, identity by extracted field, etc.
//!
//! A [`Comparator`] defines an ordering between two values, enabling
//! sorted collections with pluggable ordering.

use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

type HashFn<T> = dyn Fn(&T) -> u64;
type EqFn<T> = dyn Fn(&T, &T) -> bool;
type CmpFn<T> = dyn Fn(&T, &T) -> Ordering;

// ── HashingStrategy ─────────────────────────────────────────────────

/// Externalises identity (hash + equality) for hash-based collections.
///
/// Instead of relying on the element's own `Hash`/`Eq` implementations,
/// the collection delegates to the strategy. This enables case-insensitive
/// keys, identity by extracted field, etc.
pub struct HashingStrategy<T: ?Sized> {
    pub hash: Box<HashFn<T>>,
    pub eq: Box<EqFn<T>>,
}

impl<T: ?Sized> HashingStrategy<T> {
    /// Creates a new hashing strategy from hash and equality functions.
    pub fn new(hash: Box<HashFn<T>>, eq: Box<EqFn<T>>) -> Self {
        HashingStrategy { hash, eq }
    }

    /// Compute the hash of a value using this strategy.
    pub fn hash_code(&self, value: &T) -> u64 {
        (self.hash)(value)
    }

    /// Test equality of two values using this strategy.
    pub fn equals(&self, a: &T, b: &T) -> bool {
        (self.eq)(a, b)
    }
}

impl<T: ?Sized> std::fmt::Debug for HashingStrategy<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashingStrategy").finish_non_exhaustive()
    }
}

/// Returns the default hashing strategy for strings (case-sensitive).
pub fn string_hashing_strategy() -> HashingStrategy<String> {
    HashingStrategy {
        hash: Box::new(|s: &String| {
            let mut h = DefaultHasher::new();
            s.hash(&mut h);
            h.finish()
        }),
        eq: Box::new(|a: &String, b: &String| a == b),
    }
}

/// Returns a hashing strategy for strings that ignores case.
/// `"Hello"` and `"hello"` are considered equal.
pub fn case_insensitive_hashing_strategy() -> HashingStrategy<String> {
    HashingStrategy {
        hash: Box::new(|s: &String| {
            let mut h = DefaultHasher::new();
            s.to_lowercase().hash(&mut h);
            h.finish()
        }),
        eq: Box::new(|a: &String, b: &String| a.eq_ignore_ascii_case(b)),
    }
}

/// Returns a hashing strategy that hashes and compares by an extracted field.
///
/// # Example
/// ```ignore
/// let strategy = by_field(|p: &Person| p.name.clone());
/// ```
pub fn by_field<T: 'static, F: Hash + Eq + 'static, E>(extract: E) -> HashingStrategy<T>
where
    E: Fn(&T) -> F + 'static,
{
    let extract: Arc<dyn Fn(&T) -> F> = Arc::new(extract);
    let eq_extract = Arc::clone(&extract);
    HashingStrategy {
        hash: Box::new(move |v: &T| {
            let f = extract(v);
            let mut h = DefaultHasher::new();
            f.hash(&mut h);
            h.finish()
        }),
        eq: Box::new(move |a: &T, b: &T| eq_extract(a) == eq_extract(b)),
    }
}

// ── Comparator ──────────────────────────────────────────────────────

/// A comparator defines an ordering between two values.
///
/// Stored as a boxed closure so it can capture state (e.g. field extractors).
pub struct Comparator<T: ?Sized> {
    cmp: Box<CmpFn<T>>,
}

impl<T: ?Sized> Comparator<T> {
    /// Creates a new comparator from a comparison function.
    pub fn new(cmp: Box<CmpFn<T>>) -> Self {
        Comparator { cmp }
    }

    /// Compare two values using this comparator.
    pub fn compare(&self, a: &T, b: &T) -> Ordering {
        (self.cmp)(a, b)
    }
}

impl<T: ?Sized> std::fmt::Debug for Comparator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Comparator").finish_non_exhaustive()
    }
}

/// Returns a comparator using the natural ordering of `Ord` types.
pub fn natural_comparator<T: Ord + 'static>() -> Comparator<T> {
    Comparator {
        cmp: Box::new(|a: &T, b: &T| a.cmp(b)),
    }
}

/// Returns a comparator with reversed natural ordering.
pub fn reverse_comparator<T: Ord + 'static>() -> Comparator<T> {
    Comparator {
        cmp: Box::new(|a: &T, b: &T| b.cmp(a)),
    }
}

/// Returns a comparator that orders by an extracted field.
///
/// # Example
/// ```ignore
/// let cmp = comparator_by_field(|p: &Person| p.name.clone());
/// ```
pub fn comparator_by_field<T: 'static, F: Ord + 'static, E>(extract: E) -> Comparator<T>
where
    E: Fn(&T) -> F + 'static,
{
    Comparator {
        cmp: Box::new(move |a: &T, b: &T| extract(a).cmp(&extract(b))),
    }
}

/// Returns a comparator that reverses the given one. Works on any comparator
/// (unlike [`reverse_comparator`], which requires `Ord`).
///
/// # Example
/// ```ignore
/// let by_name = comparator_by_field(|p: &Person| p.name.clone());
/// let by_name_desc = reversed(by_name);
/// ```
pub fn reversed<T: 'static>(cmp: Comparator<T>) -> Comparator<T> {
    Comparator {
        cmp: Box::new(move |a: &T, b: &T| cmp.compare(b, a)),
    }
}

/// Returns a comparator that orders by an extracted field using a custom
/// sub-comparator (instead of natural ordering). Useful for e.g. sorting
/// by a string field case-insensitively.
///
/// # Example
/// ```ignore
/// let ci = Comparator::new(Box::new(|a: &String, b: &String|
///     a.to_lowercase().cmp(&b.to_lowercase())));
/// let by_name_ci = comparator_by_field_with(|p: &Person| p.name.clone(), ci);
/// ```
pub fn comparator_by_field_with<T: 'static, F: 'static, E>(
    extract: E,
    sub: Comparator<F>,
) -> Comparator<T>
where
    E: Fn(&T) -> F + 'static,
{
    Comparator {
        cmp: Box::new(move |a: &T, b: &T| sub.compare(&extract(a), &extract(b))),
    }
}

/// Chains two comparators: uses the secondary when the primary returns `Equal`.
pub fn then_comparing<T: 'static>(
    primary: Comparator<T>,
    secondary: Comparator<T>,
) -> Comparator<T> {
    Comparator {
        cmp: Box::new(move |a: &T, b: &T| {
            let r = primary.compare(a, b);
            if r != Ordering::Equal {
                r
            } else {
                secondary.compare(a, b)
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_hashing_strategy() {
        let s = string_hashing_strategy();
        let a = "hello".to_string();
        let b = "hello".to_string();
        let c = "world".to_string();
        assert!(s.equals(&a, &b));
        assert!(!s.equals(&a, &c));
        assert_eq!(s.hash_code(&a), s.hash_code(&b));
    }

    #[test]
    fn test_case_insensitive_strategy() {
        let s = case_insensitive_hashing_strategy();
        let a = "Hello".to_string();
        let b = "hello".to_string();
        let c = "HELLO".to_string();
        assert!(s.equals(&a, &b));
        assert!(s.equals(&b, &c));
        assert_eq!(s.hash_code(&a), s.hash_code(&b));
    }

    #[derive(Debug, Clone)]
    struct Person {
        name: String,
        age: i32,
    }

    #[test]
    fn test_by_field() {
        let s = by_field(|p: &Person| p.name.clone());
        let a = Person {
            name: "Alice".into(),
            age: 30,
        };
        let b = Person {
            name: "Alice".into(),
            age: 25,
        };
        let c = Person {
            name: "Bob".into(),
            age: 30,
        };
        assert!(s.equals(&a, &b));
        assert!(!s.equals(&a, &c));
        assert_eq!(s.hash_code(&a), s.hash_code(&b));
    }

    #[test]
    fn test_natural_comparator() {
        let cmp = natural_comparator::<i32>();
        assert_eq!(cmp.compare(&1, &2), Ordering::Less);
        assert_eq!(cmp.compare(&2, &2), Ordering::Equal);
        assert_eq!(cmp.compare(&3, &2), Ordering::Greater);
    }

    #[test]
    fn test_reverse_comparator() {
        let cmp = reverse_comparator::<i32>();
        assert_eq!(cmp.compare(&1, &2), Ordering::Greater);
        assert_eq!(cmp.compare(&3, &2), Ordering::Less);
    }

    #[test]
    fn test_comparator_by_field() {
        let cmp = comparator_by_field(|p: &Person| p.name.clone());
        let a = Person {
            name: "Alice".into(),
            age: 30,
        };
        let b = Person {
            name: "Bob".into(),
            age: 25,
        };
        assert_eq!(cmp.compare(&a, &b), Ordering::Less);
    }

    #[test]
    fn test_reversed() {
        // Reverse an arbitrary comparator (not just natural ordering).
        let by_age = comparator_by_field(|p: &Person| p.age);
        let by_age_desc = reversed(by_age);

        let mut people = [
            Person {
                name: "A".into(),
                age: 20,
            },
            Person {
                name: "B".into(),
                age: 30,
            },
            Person {
                name: "C".into(),
                age: 10,
            },
        ];
        people.sort_by(|a, b| by_age_desc.compare(a, b));
        let ages: Vec<i32> = people.iter().map(|p| p.age).collect();
        assert_eq!(ages, vec![30, 20, 10]);
    }

    #[test]
    fn test_comparator_by_field_with() {
        // Sort persons by name case-insensitively.
        let ci_str: Comparator<String> = Comparator::new(Box::new(|a: &String, b: &String| {
            a.to_lowercase().cmp(&b.to_lowercase())
        }));
        let by_name_ci = comparator_by_field_with(|p: &Person| p.name.clone(), ci_str);

        let mut people = [
            Person {
                name: "bob".into(),
                age: 0,
            },
            Person {
                name: "Alice".into(),
                age: 0,
            },
            Person {
                name: "CAROL".into(),
                age: 0,
            },
        ];
        people.sort_by(|a, b| by_name_ci.compare(a, b));
        let names: Vec<String> = people.iter().map(|p| p.name.clone()).collect();
        assert_eq!(
            names,
            vec!["Alice".to_string(), "bob".to_string(), "CAROL".to_string()]
        );
    }

    #[test]
    fn test_then_comparing() {
        let by_age = comparator_by_field(|p: &Person| p.age);
        let by_name = comparator_by_field(|p: &Person| p.name.clone());
        let cmp = then_comparing(by_age, by_name);

        let alice = Person {
            name: "Alice".into(),
            age: 30,
        };
        let bob = Person {
            name: "Bob".into(),
            age: 25,
        };
        let charlie = Person {
            name: "Charlie".into(),
            age: 30,
        };

        // Bob(25) < Alice(30)
        assert_eq!(cmp.compare(&bob, &alice), Ordering::Less);
        // Alice(30) < Charlie(30) — same age, compared by name
        assert_eq!(cmp.compare(&alice, &charlie), Ordering::Less);
    }
}
