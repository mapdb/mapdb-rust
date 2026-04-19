// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

// Stream generators — lazy iterators for common patterns.
// Rust's Iterator trait is already powerful, so these are convenience constructors.

/// Creates an iterator that yields values from start (inclusive) to end (exclusive).
pub fn range<T>(start: T, end: T) -> impl Iterator<Item = T>
where
    T: Copy + PartialOrd + std::ops::Add<Output = T> + From<u8>,
{
    let one = T::from(1u8);
    std::iter::successors(Some(start), move |&prev| {
        let next = prev + one;
        if next < end {
            Some(next)
        } else {
            None
        }
    })
}

/// Creates an iterator that yields values from start to end (both inclusive).
pub fn range_closed<T>(start: T, end: T) -> impl Iterator<Item = T>
where
    T: Copy + PartialOrd + std::ops::Add<Output = T> + From<u8>,
{
    let one = T::from(1u8);
    let mut done = false;
    std::iter::successors(Some(start), move |&prev| {
        if done {
            return None;
        }
        if prev == end {
            done = true;
            return None;
        }
        let next = prev + one;
        if next > end {
            done = true;
            Some(end)
        } else {
            Some(next)
        }
    })
}

/// Creates an iterator that repeats a value n times.
pub fn repeat<T: Clone>(value: T, n: usize) -> impl Iterator<Item = T> {
    std::iter::repeat_n(value, n)
}

/// Creates an infinite iterator from a seed and a function.
pub fn iterate<T: Clone>(seed: T, f: impl Fn(&T) -> T) -> impl Iterator<Item = T> {
    std::iter::successors(Some(seed), move |prev| Some(f(prev)))
}

/// Creates an iterator from a supplier function that is called for each element.
pub fn generate<T>(mut supplier: impl FnMut() -> T) -> impl Iterator<Item = T> {
    std::iter::from_fn(move || Some(supplier()))
}

/// Creates an iterator from the given values.
pub fn of<T>(values: Vec<T>) -> impl Iterator<Item = T> {
    values.into_iter()
}

/// Creates an empty iterator.
pub fn empty<T>() -> impl Iterator<Item = T> {
    std::iter::empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range() {
        let v: Vec<i32> = range(1, 5).collect();
        assert_eq!(v, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_range_closed() {
        let v: Vec<i32> = range_closed(1, 5).collect();
        assert_eq!(v, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_repeat() {
        let v: Vec<i32> = repeat(42, 3).collect();
        assert_eq!(v, vec![42, 42, 42]);
    }

    #[test]
    fn test_iterate() {
        let v: Vec<i32> = iterate(1, |x| x * 2).take(5).collect();
        assert_eq!(v, vec![1, 2, 4, 8, 16]);
    }

    #[test]
    fn test_generate() {
        let mut counter = 0;
        let v: Vec<i32> = generate(move || {
            counter += 1;
            counter
        })
        .take(3)
        .collect();
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn test_of() {
        let v: Vec<i32> = of(vec![1, 2, 3]).collect();
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn test_empty() {
        let v: Vec<i32> = empty().collect();
        assert!(v.is_empty());
    }
}
