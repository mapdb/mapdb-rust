// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Work-stealing parallel views — the Rust analogue of Eclipse Collections'
//! `ParallelIterable` and `RichIterable.asParallel(...)`.
//!
//! Where [`batch`](super::batch) splits the input into fixed sections once,
//! a [`ParallelSlice`] hands the work to [rayon](https://crates.io/crates/rayon),
//! whose scheduler **recursively splits and steals** work between threads —
//! the same idea as Java's `Spliterator` + `ForkJoinPool` behind a parallel
//! stream. This is the only part of the crate that depends on a third-party
//! crate, so the whole module is gated behind the `parallel` feature.
//!
//! Terminal operations mirror the `RichIterable` surface used elsewhere in the
//! crate (`select` / `reject` / `collect` / `count` / `any_satisfy` / …).

use rayon::prelude::*;

/// A parallel view over a borrowed slice. Construct with [`as_parallel`].
///
/// The view is lazy only in the rayon sense: each terminal method below builds
/// and drives a fresh parallel iterator over the underlying data.
#[derive(Clone, Copy)]
pub struct ParallelSlice<'a, T> {
    data: &'a [T],
}

/// Creates a work-stealing parallel view over `data`.
///
/// Analogous to `richIterable.asParallel(executor, batchSize)` in Eclipse
/// Collections; rayon owns the thread pool, so no executor argument is needed.
pub fn as_parallel<T: Sync>(data: &[T]) -> ParallelSlice<'_, T> {
    ParallelSlice { data }
}

impl<'a, T: Sync> ParallelSlice<'a, T> {
    /// The number of elements in the view.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the view is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Applies `f` to every element in parallel. Iteration order is unspecified.
    pub fn for_each<F>(&self, f: F)
    where
        F: Fn(&T) + Sync + Send,
    {
        self.data.par_iter().for_each(f);
    }

    /// Collects, in input order, every element satisfying `predicate`.
    pub fn select<F>(&self, predicate: F) -> Vec<T>
    where
        T: Clone + Send,
        F: Fn(&T) -> bool + Sync + Send,
    {
        self.data
            .par_iter()
            .filter(|v| predicate(v))
            .cloned()
            .collect()
    }

    /// Collects, in input order, every element *not* satisfying `predicate`.
    pub fn reject<F>(&self, predicate: F) -> Vec<T>
    where
        T: Clone + Send,
        F: Fn(&T) -> bool + Sync + Send,
    {
        self.select(move |v| !predicate(v))
    }

    /// Maps every element through `transform` in parallel, preserving order.
    pub fn collect<R, F>(&self, transform: F) -> Vec<R>
    where
        R: Send,
        F: Fn(&T) -> R + Sync + Send,
    {
        self.data.par_iter().map(transform).collect()
    }

    /// Counts elements satisfying `predicate`.
    pub fn count<F>(&self, predicate: F) -> usize
    where
        F: Fn(&T) -> bool + Sync + Send,
    {
        self.data.par_iter().filter(|v| predicate(v)).count()
    }

    /// Whether any element satisfies `predicate` (short-circuits).
    pub fn any_satisfy<F>(&self, predicate: F) -> bool
    where
        F: Fn(&T) -> bool + Sync + Send,
    {
        self.data.par_iter().any(predicate)
    }

    /// Whether all elements satisfy `predicate` (short-circuits).
    pub fn all_satisfy<F>(&self, predicate: F) -> bool
    where
        F: Fn(&T) -> bool + Sync + Send,
    {
        self.data.par_iter().all(predicate)
    }

    /// Whether no element satisfies `predicate` (short-circuits).
    pub fn none_satisfy<F>(&self, predicate: F) -> bool
    where
        F: Fn(&T) -> bool + Sync + Send,
    {
        !self.any_satisfy(predicate)
    }

    /// Returns some element satisfying `predicate`, if any. Which matching
    /// element is returned is unspecified (rayon's `find_any`).
    pub fn detect<F>(&self, predicate: F) -> Option<&'a T>
    where
        F: Fn(&T) -> bool + Sync + Send,
    {
        self.data.par_iter().find_any(|v| predicate(v))
    }

    /// Sums `extract(element)` over all elements in parallel. `R` must be
    /// associative/commutative under `+` for the result to be independent of
    /// how rayon splits the work (true for the numeric primitives).
    pub fn sum_by<R, F>(&self, extract: F) -> R
    where
        R: Send + std::iter::Sum,
        F: Fn(&T) -> R + Sync + Send,
    {
        self.data.par_iter().map(extract).sum()
    }

    /// The minimum element under `compare`, or `None` if empty.
    pub fn min_by<F>(&self, compare: F) -> Option<&'a T>
    where
        F: Fn(&T, &T) -> std::cmp::Ordering + Sync + Send,
    {
        self.data.par_iter().min_by(|a, b| compare(a, b))
    }

    /// The maximum element under `compare`, or `None` if empty.
    pub fn max_by<F>(&self, compare: F) -> Option<&'a T>
    where
        F: Fn(&T, &T) -> std::cmp::Ordering + Sync + Send,
    {
        self.data.par_iter().max_by(|a, b| compare(a, b))
    }

    /// A cloned owned copy of every element, in order.
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone + Send,
    {
        self.data.par_iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_reject_collect_preserve_order() {
        let data: Vec<i64> = (0..1000).collect();
        let p = as_parallel(&data);
        assert_eq!(
            p.select(|v| v % 2 == 0),
            data.iter()
                .copied()
                .filter(|v| v % 2 == 0)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            p.reject(|v| v % 2 == 0),
            data.iter()
                .copied()
                .filter(|v| v % 2 != 0)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            p.collect(|v| v * v),
            data.iter().map(|v| v * v).collect::<Vec<_>>()
        );
    }

    #[test]
    fn count_any_all_none_detect() {
        let data: Vec<i32> = (0..1000).collect();
        let p = as_parallel(&data);
        assert_eq!(p.count(|v| *v >= 500), 500);
        assert!(p.any_satisfy(|v| *v == 999));
        assert!(!p.any_satisfy(|v| *v == 1000));
        assert!(p.all_satisfy(|v| *v >= 0));
        assert!(p.none_satisfy(|v| *v < 0));
        assert_eq!(p.detect(|v| *v == 42), Some(&42));
        assert_eq!(p.detect(|v| *v == 1000), None);
    }

    #[test]
    fn sum_min_max() {
        let data: Vec<i64> = (1..=1000).collect();
        let p = as_parallel(&data);
        assert_eq!(p.sum_by(|v| *v), 500_500);
        assert_eq!(p.min_by(|a, b| a.cmp(b)), Some(&1));
        assert_eq!(p.max_by(|a, b| a.cmp(b)), Some(&1000));
    }

    #[test]
    fn as_parallel_over_collection_slice() {
        use crate::object::ArrayList;
        let list = ArrayList::of(0..1000i64);
        // The bridge: a collection flows into the work-stealing view through
        // its `as_slice()` borrow — no `par_*` method on the collection.
        let p = as_parallel(list.as_slice());
        assert_eq!(p.count(|v| *v % 2 == 0), 500);
        assert_eq!(p.sum_by(|v| *v), (0..1000i64).sum());
        assert_eq!(p.select(|v| *v < 10), (0..10i64).collect::<Vec<_>>());
    }

    #[test]
    fn empty_view() {
        let data: Vec<i32> = Vec::new();
        let p = as_parallel(&data);
        assert!(p.is_empty());
        assert_eq!(p.len(), 0);
        assert_eq!(p.count(|_| true), 0);
        assert!(!p.any_satisfy(|_| true));
        assert!(p.all_satisfy(|_| false)); // vacuously true
        assert_eq!(p.sum_by(|v: &i32| *v), 0);
        assert_eq!(p.min_by(|a, b| a.cmp(b)), None);
    }
}
