// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use std::fmt;

/// Min-heap priority queue. `push` / `pop` are O(log n), `peek` is O(1).
/// `of` does an O(n) sift-down heapify rather than repeated `push`.
///
/// Min-heap semantics: callers wanting a max-heap should either negate
/// the value or `drain_sorted` and reverse. Float values must use the
/// [`HashableF32`](crate::HashableF32) / [`HashableF64`](crate::HashableF64)
/// wrappers so the ordering is total (NaN- and bit-pattern-aware) per
/// `algorithms.md` §"Float ordering for tree collections".
#[derive(Debug, Clone)]
pub struct PriorityQueue<T: Ord> {
    items: Vec<T>,
}

impl<T: Ord> PriorityQueue<T> {
    pub fn new() -> Self {
        PriorityQueue { items: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        PriorityQueue {
            items: Vec::with_capacity(capacity),
        }
    }

    /// Builds a queue from `values` and heapifies in O(n) (Floyd's
    /// bottom-up sift-down), not repeated `push`.
    pub fn of<I: IntoIterator<Item = T>>(values: I) -> Self {
        let mut q = PriorityQueue {
            items: values.into_iter().collect(),
        };
        if q.items.len() > 1 {
            for i in (0..q.items.len() / 2).rev() {
                q.sift_down(i);
            }
        }
        q
    }

    pub fn push(&mut self, value: T) {
        self.items.push(value);
        self.sift_up(self.items.len() - 1);
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.items.is_empty() {
            return None;
        }
        // `swap_remove(0)` returns the old root (the min) and moves the
        // previous last element into position 0; restore the heap with
        // a single sift-down.
        let top = self.items.swap_remove(0);
        if !self.items.is_empty() {
            self.sift_down(0);
        }
        Some(top)
    }

    pub fn peek(&self) -> Option<&T> {
        self.items.first()
    }

    pub fn size(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn contains(&self, value: &T) -> bool {
        self.items.iter().any(|v| v == value)
    }

    /// Drains every element in ascending order. Consumes the queue.
    pub fn drain_sorted(mut self) -> Vec<T> {
        let mut out = Vec::with_capacity(self.items.len());
        while let Some(v) = self.pop() {
            out.push(v);
        }
        out
    }

    fn sift_up(&mut self, start: usize) {
        let mut i = start;
        while i > 0 {
            let parent = (i - 1) / 2;
            if self.items[i] < self.items[parent] {
                self.items.swap(i, parent);
                i = parent;
            } else {
                break;
            }
        }
    }

    fn sift_down(&mut self, start: usize) {
        let mut i = start;
        let n = self.items.len();
        loop {
            let left = 2 * i + 1;
            if left >= n {
                break;
            }
            let right = left + 1;
            let mut best = left;
            if right < n && self.items[right] < self.items[left] {
                best = right;
            }
            if self.items[best] < self.items[i] {
                self.items.swap(best, i);
                i = best;
            } else {
                break;
            }
        }
    }
}

impl<T: Ord + Clone> PriorityQueue<T> {
    /// Returns a copy of the internal heap array. **Not sorted** — the
    /// order is the heap's array layout. Use [`drain_sorted`](Self::drain_sorted)
    /// for ascending order.
    pub fn to_vec(&self) -> Vec<T> {
        self.items.clone()
    }
}

impl<T: Ord> Default for PriorityQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord + fmt::Display> fmt::Display for PriorityQueue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.items.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", v)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HashableF64;

    #[test]
    fn empty_basics() {
        let mut q: PriorityQueue<i32> = PriorityQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.size(), 0);
        assert_eq!(q.peek(), None);
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn push_pop_min_first() {
        let mut q = PriorityQueue::new();
        q.push(5);
        q.push(1);
        q.push(3);
        q.push(2);
        q.push(4);
        assert_eq!(q.peek(), Some(&1));
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), Some(3));
        assert_eq!(q.pop(), Some(4));
        assert_eq!(q.pop(), Some(5));
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn of_heapifies() {
        let q = PriorityQueue::of([5, 1, 3, 2, 4]);
        assert_eq!(q.peek(), Some(&1));
        assert_eq!(q.size(), 5);
        let sorted = q.drain_sorted();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn of_empty_and_single() {
        let q: PriorityQueue<i32> = PriorityQueue::of(std::iter::empty());
        assert!(q.is_empty());
        let q2 = PriorityQueue::of([42]);
        assert_eq!(q2.peek(), Some(&42));
    }

    #[test]
    fn contains_clear() {
        let mut q = PriorityQueue::of([1, 2, 3]);
        assert!(q.contains(&2));
        assert!(!q.contains(&99));
        q.clear();
        assert!(q.is_empty());
        assert!(!q.contains(&1));
    }

    #[test]
    fn to_vec_is_heap_order_not_sorted() {
        let q = PriorityQueue::of([5, 4, 3, 2, 1]);
        let v = q.to_vec();
        // Min must be at index 0; sortedness is not guaranteed otherwise.
        assert_eq!(v[0], 1);
    }

    #[test]
    fn drain_sorted_handles_duplicates() {
        let q = PriorityQueue::of([3, 1, 2, 1, 3, 2]);
        assert_eq!(q.drain_sorted(), vec![1, 1, 2, 2, 3, 3]);
    }

    #[test]
    fn float_via_hashable_wrapper() {
        // Total order via HashableF64: NaN sorts after every finite value
        // (total_cmp puts +NaN at the top, -NaN below -Inf).
        let q = PriorityQueue::of([
            HashableF64::from(3.5),
            HashableF64::from(-1.0),
            HashableF64::from(2.0),
            HashableF64::from(f64::INFINITY),
            HashableF64::from(f64::NEG_INFINITY),
        ]);
        let sorted = q.drain_sorted();
        let raw: Vec<f64> = sorted.iter().map(|h| h.0).collect();
        assert_eq!(
            raw,
            vec![f64::NEG_INFINITY, -1.0, 2.0, 3.5, f64::INFINITY]
        );
    }
}
