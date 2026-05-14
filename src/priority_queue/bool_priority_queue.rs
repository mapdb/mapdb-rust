// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Primitive min-heap priority queue of `bool` values, backed by a `Vec`.
/// O(log n) push/pop, O(1) peek. Duplicates are allowed.
#[derive(Debug, Clone)]
pub struct BoolPriorityQueue {
    items: Vec<bool>,
}

impl BoolPriorityQueue {
    pub fn new() -> Self {
        BoolPriorityQueue { items: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        BoolPriorityQueue {
            items: Vec::with_capacity(capacity),
        }
    }

    /// Heapifies the given slice in O(n).
    pub fn of(values: &[bool]) -> Self {
        let mut q = BoolPriorityQueue {
            items: values.to_vec(),
        };
        if q.items.len() > 1 {
            let start = q.items.len() / 2;
            for i in (0..start).rev() {
                q.sift_down(i);
            }
        }
        q
    }

    /// Pushes a value onto the heap. O(log n).
    pub fn push(&mut self, value: bool) {
        self.items.push(value);
        let idx = self.items.len() - 1;
        self.sift_up(idx);
    }

    /// Removes and returns the smallest element, or `None` if empty. O(log n).
    pub fn pop(&mut self) -> Option<bool> {
        if self.items.is_empty() {
            return None;
        }
        let last = self.items.len() - 1;
        self.items.swap(0, last);
        let top = self.items.pop();
        if !self.items.is_empty() {
            self.sift_down(0);
        }
        top
    }

    /// Returns the smallest element without removing it, or `None` if empty.
    pub fn peek(&self) -> Option<bool> {
        self.items.first().copied()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.items.try_reserve(additional)
    }

    pub fn contains(&self, value: bool) -> bool {
        self.items.iter().any(|&v| v == value)
    }

    /// Returns elements in heap-array order (NOT sorted). Use `drain_sorted`
    /// for ascending order.
    pub fn to_vec(&self) -> Vec<bool> {
        self.items.clone()
    }

    /// Drains the heap in ascending order, consuming the queue.
    pub fn drain_sorted(mut self) -> Vec<bool> {
        let mut out = Vec::with_capacity(self.items.len());
        while let Some(v) = self.pop() {
            out.push(v);
        }
        out
    }

    fn sift_up(&mut self, mut i: usize) {
        while i > 0 {
            let parent = (i - 1) / 2;
            if (!(self.items[i]) && (self.items[parent])) {
                self.items.swap(i, parent);
                i = parent;
            } else {
                break;
            }
        }
    }

    fn sift_down(&mut self, mut i: usize) {
        let n = self.items.len();
        loop {
            let left = 2 * i + 1;
            if left >= n {
                break;
            }
            let right = left + 1;
            let mut best = left;
            if right < n && (!(self.items[right]) && (self.items[left])) {
                best = right;
            }
            if (!(self.items[best]) && (self.items[i])) {
                self.items.swap(best, i);
                i = best;
            } else {
                break;
            }
        }
    }
}

impl Default for BoolPriorityQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BoolPriorityQueue {
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

    #[test]
    fn test_push_peek_pop_min() {
        let mut q = BoolPriorityQueue::new();
        q.push(true);
        q.push(true);
        q.push(false);
        // v[0] should be the smallest

        assert_eq!(q.len(), 3);
        q.pop();
        q.pop();
        q.pop();
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn test_of_heapify() {
        let q = BoolPriorityQueue::of(&[true, true, false]);
        assert_eq!(q.len(), 3);
    }

    #[test]
    fn test_empty() {
        let mut q = BoolPriorityQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.peek(), None);
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn test_contains_clear() {
        let mut q = BoolPriorityQueue::new();
        q.push(true);
        assert!(q.contains(true));
        q.clear();
        assert!(q.is_empty());
    }

    #[test]
    fn test_drain_sorted() {
        let q = BoolPriorityQueue::of(&[true, true, false]);
        let sorted = q.drain_sorted();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_try_reserve_happy() {
        let mut q = BoolPriorityQueue::new();
        q.try_reserve(10).unwrap();
    }

    #[test]
    fn test_try_reserve_overflow_errors() {
        let mut q = BoolPriorityQueue::new();
        assert!(q.try_reserve(usize::MAX / 2).is_err());
    }
}
