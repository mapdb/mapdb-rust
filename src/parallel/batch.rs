// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Fixed-chunk batch iteration — the Rust analogue of Eclipse Collections'
//! `BatchIterable`.
//!
//! Unlike a [`Spliterator`](super::spliterator::Spliterator), which divides
//! recursively, and unlike rayon's work-stealing scheduler, a batch iterable
//! is partitioned **once** into `section_count` contiguous sections of roughly
//! equal length. Section *i* is then processed by
//! [`batch_for_each`](BatchIterable::batch_for_each). **No work is stolen or
//! rebalanced**: each section is handled start-to-finish by whoever owns it.
//!
//! The [`BatchIterable`] trait and the sectioning helpers are pure std and
//! always compiled. The parallel *executors* at the bottom of this file —
//! which spawn one scoped OS thread per section — are gated behind the
//! `parallel` feature, so that with the feature off the crate spawns no
//! threads and offers no parallel execution.

/// A collection that can describe and traverse itself in fixed contiguous
/// sections. Mirrors `org.eclipse.collections.impl.parallel.BatchIterable`.
pub trait BatchIterable<T> {
    /// The total number of elements.
    fn size(&self) -> usize;

    /// Applies `action` to every element of the `section_index`-th section out
    /// of `section_count` equal-sized contiguous sections, in order. A section
    /// index past the populated sections (possible when `section_count` exceeds
    /// `size`) yields nothing.
    fn batch_for_each(&self, action: impl FnMut(&T), section_index: usize, section_count: usize);

    /// The number of batches of at most `batch_size` elements needed to cover
    /// the collection, i.e. `ceil(size / batch_size)`, but never less than 1.
    /// Matches Eclipse Collections' `getBatchCount`.
    fn get_batch_count(&self, batch_size: usize) -> usize {
        let n = self.size();
        if batch_size == 0 || n == 0 {
            1
        } else {
            n.div_ceil(batch_size)
        }
    }
}

impl<T> BatchIterable<T> for [T] {
    fn size(&self) -> usize {
        self.len()
    }

    fn batch_for_each(
        &self,
        mut action: impl FnMut(&T),
        section_index: usize,
        section_count: usize,
    ) {
        let (lo, hi) = section_bounds(self.len(), section_index, section_count);
        for v in &self[lo..hi] {
            action(v);
        }
    }
}

impl<T> BatchIterable<T> for Vec<T> {
    fn size(&self) -> usize {
        self.len()
    }

    fn batch_for_each(&self, action: impl FnMut(&T), section_index: usize, section_count: usize) {
        self.as_slice()
            .batch_for_each(action, section_index, section_count)
    }
}

/// Returns the `[lo, hi)` element bounds of section `index` when `n` elements
/// are divided into `count` contiguous sections as evenly as possible.
///
/// The first `n % count` sections receive one extra element (the same scheme
/// the Go `parallel` port uses), so bounds tile `0..n` without gaps or overlap.
/// Returns an empty range when `index >= count` or `index` lands past the data.
pub fn section_bounds(n: usize, index: usize, count: usize) -> (usize, usize) {
    let count = count.max(1);
    if index >= count {
        return (n, n);
    }
    let base = n / count;
    let remainder = n % count;
    let lo = index * base + index.min(remainder);
    let hi = lo + base + usize::from(index < remainder);
    (lo, hi)
}

/// All non-empty section bounds for `n` elements split into `count` sections.
#[cfg(feature = "parallel")]
fn nonempty_sections(n: usize, count: usize) -> Vec<(usize, usize)> {
    let count = count.max(1);
    (0..count)
        .map(|i| section_bounds(n, i, count))
        .filter(|&(lo, hi)| lo < hi)
        .collect()
}

#[cfg(feature = "parallel")]
mod exec {
    use super::{nonempty_sections, BatchIterable};
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Minimum element count before batch parallelism engages. Below this, the
    /// thread-spawn overhead isn't worth it and operations run sequentially.
    /// Mirrors the Go port's `DefaultMinForkSize`.
    pub const DEFAULT_MIN_FORK_SIZE: usize = 10_000;

    /// The default number of sections (worker threads): `(NCPU + 1) * 2`,
    /// capped at 200 — the formula Eclipse Collections' `ParallelIterate` uses.
    pub fn default_task_count() -> usize {
        let ncpu = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        ((ncpu + 1) * 2).min(200)
    }

    fn parallelize(n: usize, min_fork_size: usize, task_count: usize) -> bool {
        n >= min_fork_size && task_count > 1
    }

    /// Applies `f` to every element, splitting `data` into fixed sections run
    /// on scoped threads (no work stealing). Order across sections is
    /// unspecified; within a section it is ascending.
    pub fn for_each<T, F>(data: &[T], f: F)
    where
        T: Sync,
        F: Fn(&T) + Sync,
    {
        for_each_with(data, f, DEFAULT_MIN_FORK_SIZE, default_task_count());
    }

    /// [`for_each`] with explicit `min_fork_size` and `task_count`.
    pub fn for_each_with<T, F>(data: &[T], f: F, min_fork_size: usize, task_count: usize)
    where
        T: Sync,
        F: Fn(&T) + Sync,
    {
        if data.is_empty() {
            return;
        }
        if !parallelize(data.len(), min_fork_size, task_count) {
            data.batch_for_each(|v| f(v), 0, 1);
            return;
        }
        let f = &f;
        std::thread::scope(|scope| {
            for (lo, hi) in nonempty_sections(data.len(), task_count) {
                let chunk = &data[lo..hi];
                scope.spawn(move || {
                    for v in chunk {
                        f(v);
                    }
                });
            }
        });
    }

    /// Maps every element through `transform` in parallel sections, preserving
    /// input order in the result.
    pub fn collect<T, R, F>(data: &[T], transform: F) -> Vec<R>
    where
        T: Sync,
        R: Send,
        F: Fn(&T) -> R + Sync,
    {
        collect_with(data, transform, DEFAULT_MIN_FORK_SIZE, default_task_count())
    }

    /// [`collect`] with explicit `min_fork_size` and `task_count`.
    pub fn collect_with<T, R, F>(
        data: &[T],
        transform: F,
        min_fork_size: usize,
        task_count: usize,
    ) -> Vec<R>
    where
        T: Sync,
        R: Send,
        F: Fn(&T) -> R + Sync,
    {
        if data.is_empty() {
            return Vec::new();
        }
        if !parallelize(data.len(), min_fork_size, task_count) {
            return data.iter().map(&transform).collect();
        }
        let transform = &transform;
        let parts: Vec<Vec<R>> = std::thread::scope(|scope| {
            let handles: Vec<_> = nonempty_sections(data.len(), task_count)
                .into_iter()
                .map(|(lo, hi)| {
                    let chunk = &data[lo..hi];
                    scope.spawn(move || chunk.iter().map(transform).collect::<Vec<R>>())
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        parts.into_iter().flatten().collect()
    }

    /// Returns the elements satisfying `predicate`, preserving input order.
    pub fn select<T, F>(data: &[T], predicate: F) -> Vec<T>
    where
        T: Sync + Clone + Send,
        F: Fn(&T) -> bool + Sync,
    {
        select_with(data, predicate, DEFAULT_MIN_FORK_SIZE, default_task_count())
    }

    /// [`select`] with explicit `min_fork_size` and `task_count`.
    pub fn select_with<T, F>(
        data: &[T],
        predicate: F,
        min_fork_size: usize,
        task_count: usize,
    ) -> Vec<T>
    where
        T: Sync + Clone + Send,
        F: Fn(&T) -> bool + Sync,
    {
        if data.is_empty() {
            return Vec::new();
        }
        if !parallelize(data.len(), min_fork_size, task_count) {
            return data.iter().filter(|v| predicate(v)).cloned().collect();
        }
        let predicate = &predicate;
        let parts: Vec<Vec<T>> = std::thread::scope(|scope| {
            let handles: Vec<_> = nonempty_sections(data.len(), task_count)
                .into_iter()
                .map(|(lo, hi)| {
                    let chunk = &data[lo..hi];
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .filter(|v| predicate(v))
                            .cloned()
                            .collect::<Vec<T>>()
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        parts.into_iter().flatten().collect()
    }

    /// Returns the elements *not* satisfying `predicate`, preserving order.
    pub fn reject<T, F>(data: &[T], predicate: F) -> Vec<T>
    where
        T: Sync + Clone + Send,
        F: Fn(&T) -> bool + Sync,
    {
        select(data, move |v| !predicate(v))
    }

    /// Counts the elements satisfying `predicate`.
    pub fn count<T, F>(data: &[T], predicate: F) -> usize
    where
        T: Sync,
        F: Fn(&T) -> bool + Sync,
    {
        count_with(data, predicate, DEFAULT_MIN_FORK_SIZE, default_task_count())
    }

    /// [`count`] with explicit `min_fork_size` and `task_count`.
    pub fn count_with<T, F>(
        data: &[T],
        predicate: F,
        min_fork_size: usize,
        task_count: usize,
    ) -> usize
    where
        T: Sync,
        F: Fn(&T) -> bool + Sync,
    {
        if data.is_empty() {
            return 0;
        }
        if !parallelize(data.len(), min_fork_size, task_count) {
            return data.iter().filter(|v| predicate(v)).count();
        }
        let predicate = &predicate;
        let counts: Vec<usize> = std::thread::scope(|scope| {
            let handles: Vec<_> = nonempty_sections(data.len(), task_count)
                .into_iter()
                .map(|(lo, hi)| {
                    let chunk = &data[lo..hi];
                    scope.spawn(move || chunk.iter().filter(|v| predicate(v)).count())
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        counts.into_iter().sum()
    }

    /// Returns `true` if any element satisfies `predicate`. All non-empty
    /// sections start, but each polls a shared flag and stops early once a
    /// match is found anywhere; the call returns after the spawned sections
    /// have joined.
    pub fn any_satisfy<T, F>(data: &[T], predicate: F) -> bool
    where
        T: Sync,
        F: Fn(&T) -> bool + Sync,
    {
        any_satisfy_with(data, predicate, DEFAULT_MIN_FORK_SIZE, default_task_count())
    }

    /// [`any_satisfy`] with explicit `min_fork_size` and `task_count`.
    pub fn any_satisfy_with<T, F>(
        data: &[T],
        predicate: F,
        min_fork_size: usize,
        task_count: usize,
    ) -> bool
    where
        T: Sync,
        F: Fn(&T) -> bool + Sync,
    {
        if data.is_empty() {
            return false;
        }
        if !parallelize(data.len(), min_fork_size, task_count) {
            return data.iter().any(&predicate);
        }
        let predicate = &predicate;
        let found = AtomicBool::new(false);
        let found_ref = &found;
        std::thread::scope(|scope| {
            for (lo, hi) in nonempty_sections(data.len(), task_count) {
                let chunk = &data[lo..hi];
                scope.spawn(move || {
                    for v in chunk {
                        if found_ref.load(Ordering::Relaxed) {
                            return;
                        }
                        if predicate(v) {
                            found_ref.store(true, Ordering::Relaxed);
                            return;
                        }
                    }
                });
            }
        });
        found.load(Ordering::Relaxed)
    }

    /// Returns `true` if every element satisfies `predicate`.
    pub fn all_satisfy<T, F>(data: &[T], predicate: F) -> bool
    where
        T: Sync,
        F: Fn(&T) -> bool + Sync,
    {
        all_satisfy_with(data, predicate, DEFAULT_MIN_FORK_SIZE, default_task_count())
    }

    /// [`all_satisfy`] with explicit `min_fork_size` and `task_count`.
    pub fn all_satisfy_with<T, F>(
        data: &[T],
        predicate: F,
        min_fork_size: usize,
        task_count: usize,
    ) -> bool
    where
        T: Sync,
        F: Fn(&T) -> bool + Sync,
    {
        !any_satisfy_with(data, move |v| !predicate(v), min_fork_size, task_count)
    }

    /// Sums `extract(element)` over all elements, combining per-section partial
    /// sums. `R` must be commutative/associative under `+` for the result to be
    /// independent of sectioning (true for the numeric primitives).
    pub fn sum_by<T, R, F>(data: &[T], extract: F) -> R
    where
        T: Sync,
        R: Send + std::iter::Sum + std::ops::Add<Output = R>,
        F: Fn(&T) -> R + Sync,
    {
        sum_by_with(data, extract, DEFAULT_MIN_FORK_SIZE, default_task_count())
    }

    /// [`sum_by`] with explicit `min_fork_size` and `task_count`.
    pub fn sum_by_with<T, R, F>(
        data: &[T],
        extract: F,
        min_fork_size: usize,
        task_count: usize,
    ) -> R
    where
        T: Sync,
        R: Send + std::iter::Sum + std::ops::Add<Output = R>,
        F: Fn(&T) -> R + Sync,
    {
        if data.is_empty() {
            return std::iter::empty::<R>().sum();
        }
        if !parallelize(data.len(), min_fork_size, task_count) {
            return data.iter().map(&extract).sum();
        }
        let extract = &extract;
        let partials: Vec<R> = std::thread::scope(|scope| {
            let handles: Vec<_> = nonempty_sections(data.len(), task_count)
                .into_iter()
                .map(|(lo, hi)| {
                    let chunk = &data[lo..hi];
                    scope.spawn(move || chunk.iter().map(extract).sum::<R>())
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        partials.into_iter().sum()
    }

    // ── generic driver over any BatchIterable ─────────────────────────────
    //
    // The Eclipse Collections `ParallelIterate`-over-`BatchIterable` model:
    // hand each of `task_count` fixed sections to its own scoped thread, with
    // no work stealing and no copy. Works for any collection that implements
    // `BatchIterable` (slices, `ArrayDeque`, the multimaps, …), so non-
    // contiguous collections get parallel iteration without exposing a slice.

    /// Drives `source` in parallel, applying `f` to every element across
    /// `default_task_count()` fixed sections. Falls back to sequential below
    /// `DEFAULT_MIN_FORK_SIZE` elements.
    pub fn for_each_in_batches<T, B, F>(source: &B, f: F)
    where
        B: BatchIterable<T> + Sync + ?Sized,
        F: Fn(&T) + Sync,
    {
        for_each_in_batches_with(source, f, DEFAULT_MIN_FORK_SIZE, default_task_count());
    }

    /// [`for_each_in_batches`] with explicit `min_fork_size` and `task_count`.
    pub fn for_each_in_batches_with<T, B, F>(
        source: &B,
        f: F,
        min_fork_size: usize,
        task_count: usize,
    ) where
        B: BatchIterable<T> + Sync + ?Sized,
        F: Fn(&T) + Sync,
    {
        let n = source.size();
        if n == 0 {
            return;
        }
        if !parallelize(n, min_fork_size, task_count) {
            source.batch_for_each(&f, 0, 1);
            return;
        }
        let f = &f;
        std::thread::scope(|scope| {
            for i in 0..task_count {
                scope.spawn(move || source.batch_for_each(f, i, task_count));
            }
        });
    }

    /// Counts elements satisfying `predicate` while driving any
    /// [`BatchIterable`] across fixed parallel sections.
    pub fn count_in_batches<T, B, F>(source: &B, predicate: F) -> usize
    where
        B: BatchIterable<T> + Sync + ?Sized,
        F: Fn(&T) -> bool + Sync,
    {
        count_in_batches_with(
            source,
            predicate,
            DEFAULT_MIN_FORK_SIZE,
            default_task_count(),
        )
    }

    /// [`count_in_batches`] with explicit `min_fork_size` and `task_count`.
    pub fn count_in_batches_with<T, B, F>(
        source: &B,
        predicate: F,
        min_fork_size: usize,
        task_count: usize,
    ) -> usize
    where
        B: BatchIterable<T> + Sync + ?Sized,
        F: Fn(&T) -> bool + Sync,
    {
        let n = source.size();
        if n == 0 {
            return 0;
        }
        if !parallelize(n, min_fork_size, task_count) {
            let mut c = 0;
            source.batch_for_each(
                |v| {
                    if predicate(v) {
                        c += 1
                    }
                },
                0,
                1,
            );
            return c;
        }
        let predicate = &predicate;
        let counts: Vec<usize> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..task_count)
                .map(|i| {
                    scope.spawn(move || {
                        let mut c = 0usize;
                        source.batch_for_each(
                            |v| {
                                if predicate(v) {
                                    c += 1
                                }
                            },
                            i,
                            task_count,
                        );
                        c
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        counts.into_iter().sum()
    }
}

#[cfg(feature = "parallel")]
pub use exec::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_batch_count_is_ceiling_min_one() {
        let data: Vec<i32> = (0..10).collect();
        assert_eq!(data.get_batch_count(3), 4); // ceil(10/3)
        assert_eq!(data.get_batch_count(10), 1);
        assert_eq!(data.get_batch_count(100), 1);
        assert_eq!(data.get_batch_count(0), 1); // degenerate batch size
        let empty: Vec<i32> = Vec::new();
        assert_eq!(empty.get_batch_count(3), 1);
    }

    #[test]
    fn section_bounds_tile_without_gaps() {
        let n = 10;
        let count = 3;
        let mut covered = Vec::new();
        for i in 0..count {
            let (lo, hi) = section_bounds(n, i, count);
            covered.extend(lo..hi);
        }
        assert_eq!(covered, (0..n).collect::<Vec<_>>());
        // 10 into 3 → 4,3,3.
        assert_eq!(section_bounds(10, 0, 3), (0, 4));
        assert_eq!(section_bounds(10, 1, 3), (4, 7));
        assert_eq!(section_bounds(10, 2, 3), (7, 10));
    }

    #[test]
    fn section_count_exceeding_size_yields_empty_tail() {
        // 2 elements, 5 sections → sections 2..5 are empty.
        assert_eq!(section_bounds(2, 0, 5), (0, 1));
        assert_eq!(section_bounds(2, 1, 5), (1, 2));
        assert_eq!(section_bounds(2, 2, 5), (2, 2));
        assert_eq!(section_bounds(2, 4, 5), (2, 2));
    }

    #[test]
    fn batch_for_each_processes_one_section() {
        let data: Vec<i32> = (0..10).collect();
        let mut got = Vec::new();
        data.batch_for_each(|v| got.push(*v), 1, 3); // middle section 4..7
        assert_eq!(got, vec![4, 5, 6]);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_for_each_visits_every_element_once() {
        use std::sync::atomic::{AtomicU64, Ordering};
        let n = 50_000;
        let data: Vec<u64> = (0..n).collect();
        let xor = AtomicU64::new(0);
        let cnt = AtomicU64::new(0);
        // Force parallelism with a low fork threshold.
        for_each_with(
            &data,
            |v| {
                xor.fetch_xor(*v, Ordering::Relaxed);
                cnt.fetch_add(1, Ordering::Relaxed);
            },
            1,
            8,
        );
        let expected_xor = (0..n).fold(0u64, |a, b| a ^ b);
        assert_eq!(cnt.load(Ordering::Relaxed), n);
        assert_eq!(xor.load(Ordering::Relaxed), expected_xor);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_collect_select_count_match_sequential() {
        let data: Vec<i64> = (0..20_000).collect();
        let doubled = collect_with(&data, |v| v * 2, 1, 8);
        assert_eq!(doubled, data.iter().map(|v| v * 2).collect::<Vec<_>>());

        let evens = select_with(&data, |v| v % 2 == 0, 1, 8);
        assert_eq!(
            evens,
            data.iter()
                .copied()
                .filter(|v| v % 2 == 0)
                .collect::<Vec<_>>()
        );

        assert_eq!(count_with(&data, |v| *v > 10_000, 1, 8), 9_999);
        assert_eq!(reject(&data, |v| v % 2 == 0).len(), 10_000);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_any_all_sum() {
        let data: Vec<i64> = (0..30_000).collect();
        assert!(any_satisfy(&data, |v| *v == 29_999));
        assert!(!any_satisfy(&data, |v| *v < 0));
        assert!(all_satisfy(&data, |v| *v >= 0));
        assert!(!all_satisfy(&data, |v| *v < 29_999));
        assert_eq!(sum_by(&data, |v| *v), data.iter().sum::<i64>());
    }

    #[test]
    fn contiguous_collections_bridge_via_as_slice() {
        use crate::object::ArrayList;
        use crate::ImmutableList;

        let list = ArrayList::of(0..100);
        let slice = list.as_slice();
        // BatchIterable applies to the borrowed slice with no per-type impl.
        assert_eq!(slice.get_batch_count(30), 4);
        let mut middle = Vec::new();
        slice.batch_for_each(|v| middle.push(*v), 1, 4);
        assert_eq!(middle, (25..50).collect::<Vec<_>>());

        let frozen = ImmutableList::from_slice(&(0..100).collect::<Vec<_>>());
        assert_eq!(frozen.as_slice().get_batch_count(100), 1);
    }

    #[test]
    fn section_bounds_count_zero_treated_as_one() {
        // count == 0 is clamped to a single section covering all elements.
        assert_eq!(section_bounds(10, 0, 0), (0, 10));
        assert_eq!(section_bounds(0, 0, 0), (0, 0));
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn with_variants_force_parallelism_below_default_threshold() {
        // Inputs far below DEFAULT_MIN_FORK_SIZE: the `_with` variants must
        // still split across the requested sections and agree with sequential.
        let data: Vec<i64> = (0..1_000).collect();
        assert!(any_satisfy_with(&data, |v| *v == 999, 1, 16));
        assert!(!any_satisfy_with(&data, |v| *v < 0, 1, 16));
        assert!(all_satisfy_with(&data, |v| *v >= 0, 1, 16));
        assert!(!all_satisfy_with(&data, |v| *v < 999, 1, 16));
        assert_eq!(sum_by_with(&data, |v| *v, 1, 16), data.iter().sum::<i64>());
        // A high threshold keeps them sequential but answers must match.
        assert_eq!(
            sum_by_with(&data, |v| *v, usize::MAX, 16),
            data.iter().sum::<i64>()
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn generic_driver_over_slice() {
        use std::sync::atomic::{AtomicU64, Ordering};
        // `[T]` is itself a BatchIterable, so the generic driver works on it.
        let data: Vec<u64> = (0..50_000).collect();
        let sum = AtomicU64::new(0);
        for_each_in_batches_with(
            data.as_slice(),
            |v| {
                sum.fetch_add(*v, Ordering::Relaxed);
            },
            1,
            8,
        );
        assert_eq!(sum.load(Ordering::Relaxed), data.iter().sum::<u64>());
        assert_eq!(
            count_in_batches_with(data.as_slice(), |v| *v % 2 == 0, 1, 8),
            25_000
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn generic_driver_over_array_deque() {
        use crate::ArrayDeque;
        use std::sync::atomic::{AtomicU64, Ordering};
        let dq = ArrayDeque::of(0..20_000u64);
        let sum = AtomicU64::new(0);
        // Zero-copy parallel iteration over a non-contiguous VecDeque-backed
        // collection, via its BatchIterable impl.
        for_each_in_batches_with(
            &dq,
            |v| {
                sum.fetch_add(*v, Ordering::Relaxed);
            },
            1,
            8,
        );
        assert_eq!(sum.load(Ordering::Relaxed), (0..20_000u64).sum());
        assert_eq!(count_in_batches_with(&dq, |v| *v >= 10_000, 1, 8), 10_000);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn generic_driver_over_multimaps() {
        use crate::{Multimap, SetMultimap};
        use std::sync::atomic::{AtomicI64, Ordering};

        let mut mm: Multimap<i32, i64> = Multimap::new();
        let mut expected = 0i64;
        for k in 0..500 {
            for j in 0..10 {
                let v = (k * 10 + j) as i64;
                mm.put(k, v);
                expected += v;
            }
        }
        let sum = AtomicI64::new(0);
        for_each_in_batches_with(
            &mm,
            |v| {
                sum.fetch_add(*v, Ordering::Relaxed);
            },
            1,
            8,
        );
        // Every value is visited exactly once across key-sections.
        assert_eq!(sum.load(Ordering::Relaxed), expected);
        assert_eq!(count_in_batches_with(&mm, |_| true, 1, 8), mm.size());

        // SetMultimap dedupes; each distinct value visited once.
        let mut sm: SetMultimap<i32, i64> = SetMultimap::new();
        for k in 0..200 {
            sm.put(k, 1);
            sm.put(k, 1); // dropped
            sm.put(k, 2);
        }
        assert_eq!(count_in_batches_with(&sm, |_| true, 1, 8), sm.size());
        assert_eq!(sm.size(), 400); // 200 keys × 2 distinct values
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn reference_bridge_parallelizes_hash_collections() {
        use crate::OpenHashSet;

        // Non-contiguous hash collections bridge by collecting borrowed refs
        // once (no element clone), then running the slice executor on `&[&T]`.
        let mut set: OpenHashSet<i64> = OpenHashSet::new();
        for v in 0..30_000 {
            set.add(v);
        }
        let refs: Vec<&i64> = set.iter().collect();
        assert_eq!(count_with(&refs, |r| **r % 2 == 0, 1, 8), 15_000);
        assert_eq!(sum_by_with(&refs, |r| **r, 1, 8), (0..30_000i64).sum());
    }
}
