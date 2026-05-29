// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Splittable iterators — the Rust analogue of `java.util.Spliterator`.
//!
//! A [`Spliterator`] traverses elements like an iterator but can also *split*
//! itself ([`try_split`](Spliterator::try_split)), handing back a new
//! spliterator that covers a portion of the remaining elements. This is the
//! decomposition primitive Java parallel streams build on; here it underpins
//! both the fixed-chunk batch executor and the rayon-backed work-stealing
//! `ParallelIterable`.
//!
//! This module is **pure std** and always compiled — it carries no dependency
//! and performs no parallel execution by itself; it only describes how work
//! can be divided.

/// Spliterator characteristic flags, matching the bit values of
/// `java.util.Spliterator` so that ported code reads identically.
pub mod characteristics {
    /// Elements are distinct (no duplicates) — e.g. a set source.
    pub const DISTINCT: u32 = 0x0000_0001;
    /// Elements follow a defined sort order (see also `ORDERED`).
    pub const SORTED: u32 = 0x0000_0004;
    /// Encounter order is meaningful and preserved across splits.
    pub const ORDERED: u32 = 0x0000_0010;
    /// `estimate_size` is an exact element count before traversal.
    pub const SIZED: u32 = 0x0000_0040;
    /// No element is null (always true for non-`Option` Rust sources).
    pub const NONNULL: u32 = 0x0000_0100;
    /// The source will not be structurally modified during traversal.
    pub const IMMUTABLE: u32 = 0x0000_0400;
    /// The source may be safely modified concurrently by other threads.
    pub const CONCURRENT: u32 = 0x0000_1000;
    /// All spliterators produced by `try_split` are themselves `SIZED`.
    pub const SUBSIZED: u32 = 0x0000_4000;
}

/// A traversable, splittable source of elements.
///
/// Mirrors `java.util.Spliterator`: [`try_advance`](Self::try_advance) consumes
/// one element, [`try_split`](Self::try_split) partitions the remainder, and
/// [`characteristics`](Self::characteristics) advertises ordering/size guarantees.
pub trait Spliterator: Sized {
    /// The element type yielded by traversal.
    type Item;

    /// If an element remains, passes it to `action` and returns `true`;
    /// otherwise returns `false` and does nothing.
    fn try_advance(&mut self, action: impl FnMut(Self::Item)) -> bool;

    /// Attempts to split off a prefix of the remaining elements into a new
    /// spliterator, leaving the suffix in `self` (the Java convention).
    ///
    /// Returns `None` when the source is too small to divide usefully, in which
    /// case `self` is unchanged and should be traversed sequentially.
    fn try_split(&mut self) -> Option<Self>;

    /// An estimate of the number of elements that would be traversed by
    /// [`for_each_remaining`](Self::for_each_remaining). Exact when the
    /// `SIZED` characteristic is reported (see [`exact_size`](Self::exact_size)).
    fn estimate_size(&self) -> u64;

    /// The bitwise-OR of this spliterator's [`characteristics`] flags.
    fn characteristics(&self) -> u32;

    /// Traverses every remaining element, applying `action` to each.
    fn for_each_remaining(&mut self, mut action: impl FnMut(Self::Item)) {
        while self.try_advance(&mut action) {}
    }

    /// Returns `true` if all of the given characteristic bits are set.
    fn has_characteristics(&self, flags: u32) -> bool {
        self.characteristics() & flags == flags
    }

    /// The exact remaining count if `SIZED`, otherwise `None`.
    fn exact_size(&self) -> Option<u64> {
        if self.has_characteristics(characteristics::SIZED) {
            Some(self.estimate_size())
        } else {
            None
        }
    }
}

/// A [`Spliterator`] over a borrowed slice, yielding `&T`.
///
/// Splitting is O(1): the backing slice is halved with `split_at`. Reports
/// `ORDERED | SIZED | SUBSIZED` since slices have an exact, stable length.
pub struct SliceSpliterator<'a, T> {
    slice: &'a [T],
}

impl<'a, T> SliceSpliterator<'a, T> {
    /// Creates a spliterator covering the whole of `slice`.
    pub fn new(slice: &'a [T]) -> Self {
        SliceSpliterator { slice }
    }

    /// The not-yet-traversed remainder, as a slice.
    pub fn remainder(&self) -> &'a [T] {
        self.slice
    }
}

impl<'a, T> Spliterator for SliceSpliterator<'a, T> {
    type Item = &'a T;

    fn try_advance(&mut self, mut action: impl FnMut(&'a T)) -> bool {
        match self.slice.split_first() {
            Some((first, rest)) => {
                self.slice = rest;
                action(first);
                true
            }
            None => false,
        }
    }

    fn try_split(&mut self) -> Option<Self> {
        let len = self.slice.len();
        if len < 2 {
            return None;
        }
        let (prefix, suffix) = self.slice.split_at(len / 2);
        self.slice = suffix;
        Some(SliceSpliterator { slice: prefix })
    }

    fn estimate_size(&self) -> u64 {
        self.slice.len() as u64
    }

    fn characteristics(&self) -> u32 {
        characteristics::ORDERED | characteristics::SIZED | characteristics::SUBSIZED
    }
}

#[cfg(test)]
mod tests {
    use super::characteristics as ch;
    use super::*;

    #[test]
    fn try_advance_walks_every_element() {
        let data = [1, 2, 3];
        let mut sp = SliceSpliterator::new(&data);
        let mut seen = Vec::new();
        while sp.try_advance(|v| seen.push(*v)) {}
        assert_eq!(seen, vec![1, 2, 3]);
        // Exhausted: a further advance is a no-op returning false.
        assert!(!sp.try_advance(|_| panic!("should not be called")));
    }

    #[test]
    fn for_each_remaining_visits_all() {
        let data = [10, 20, 30, 40];
        let mut sp = SliceSpliterator::new(&data);
        let mut sum = 0;
        sp.for_each_remaining(|v| sum += *v);
        assert_eq!(sum, 100);
    }

    #[test]
    fn try_split_returns_prefix_keeps_suffix() {
        let data = [1, 2, 3, 4, 5];
        let mut sp = SliceSpliterator::new(&data);
        let prefix = sp.try_split().expect("splittable");
        // Java convention: prefix covers the front, self keeps the back.
        assert_eq!(prefix.remainder(), &[1, 2]);
        assert_eq!(sp.remainder(), &[3, 4, 5]);
    }

    #[test]
    fn split_recursively_covers_every_element_once() {
        let data: Vec<i32> = (0..1000).collect();
        let mut collected = Vec::new();
        let mut work = vec![SliceSpliterator::new(&data)];
        while let Some(mut sp) = work.pop() {
            match sp.try_split() {
                Some(prefix) => {
                    work.push(prefix);
                    work.push(sp);
                }
                None => sp.for_each_remaining(|v| collected.push(*v)),
            }
        }
        collected.sort_unstable();
        assert_eq!(collected, data);
    }

    #[test]
    fn singletons_and_empties_do_not_split() {
        let one = [42];
        assert!(SliceSpliterator::new(&one).try_split().is_none());
        let none: [i32; 0] = [];
        assert!(SliceSpliterator::new(&none).try_split().is_none());
    }

    #[test]
    fn characteristics_and_exact_size() {
        let data = [1, 2, 3];
        let sp = SliceSpliterator::new(&data);
        assert!(sp.has_characteristics(ch::SIZED));
        assert!(sp.has_characteristics(ch::ORDERED | ch::SUBSIZED));
        assert!(!sp.has_characteristics(ch::SORTED));
        assert_eq!(sp.exact_size(), Some(3));
        assert_eq!(sp.estimate_size(), 3);
    }
}
