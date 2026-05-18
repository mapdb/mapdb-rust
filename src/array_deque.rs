// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use std::collections::VecDeque;
use std::fmt;

/// Double-ended queue backed by `std::collections::VecDeque`. Both ends
/// are O(1) amortised, matching the contract documented for the Rust
/// port in `spec/collections.md`. The Go port uses a power-of-two ring
/// buffer and the TS/Zig ports a dynamic array with O(n) front; here we
/// just lean on `VecDeque`.
#[derive(Debug, Clone)]
pub struct ArrayDeque<T> {
    data: VecDeque<T>,
}

impl<T> ArrayDeque<T> {
    pub fn new() -> Self {
        ArrayDeque {
            data: VecDeque::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        ArrayDeque {
            data: VecDeque::with_capacity(capacity),
        }
    }

    /// Builds a deque from `values` in front-to-back order.
    pub fn of<I: IntoIterator<Item = T>>(values: I) -> Self {
        ArrayDeque {
            data: values.into_iter().collect(),
        }
    }

    pub fn push_front(&mut self, value: T) {
        self.data.push_front(value);
    }

    pub fn push_back(&mut self, value: T) {
        self.data.push_back(value);
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.data.pop_front()
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.data.pop_back()
    }

    pub fn peek_front(&self) -> Option<&T> {
        self.data.front()
    }

    pub fn peek_back(&self) -> Option<&T> {
        self.data.back()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, T> {
        self.data.iter()
    }
}

impl<T: PartialEq> ArrayDeque<T> {
    pub fn contains(&self, value: &T) -> bool {
        self.data.iter().any(|v| v == value)
    }
}

impl<T: Clone> ArrayDeque<T> {
    /// Returns a `Vec<T>` of elements in front-to-back order.
    pub fn to_vec(&self) -> Vec<T> {
        self.data.iter().cloned().collect()
    }
}

impl<T> Default for ArrayDeque<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PartialEq> PartialEq for ArrayDeque<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T: Eq> Eq for ArrayDeque<T> {}

impl<'a, T> IntoIterator for &'a ArrayDeque<T> {
    type Item = &'a T;
    type IntoIter = std::collections::vec_deque::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<T> IntoIterator for ArrayDeque<T> {
    type Item = T;
    type IntoIter = std::collections::vec_deque::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<T: fmt::Display> fmt::Display for ArrayDeque<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.data.iter().enumerate() {
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
    fn empty_basics() {
        let mut d: ArrayDeque<i32> = ArrayDeque::new();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
        assert_eq!(d.peek_front(), None);
        assert_eq!(d.peek_back(), None);
        assert_eq!(d.pop_front(), None);
        assert_eq!(d.pop_back(), None);
    }

    #[test]
    fn push_back_pop_front_fifo() {
        let mut d = ArrayDeque::new();
        d.push_back(1);
        d.push_back(2);
        d.push_back(3);
        assert_eq!(d.len(), 3);
        assert_eq!(d.pop_front(), Some(1));
        assert_eq!(d.pop_front(), Some(2));
        assert_eq!(d.pop_front(), Some(3));
        assert!(d.is_empty());
    }

    #[test]
    fn push_front_pop_back() {
        let mut d = ArrayDeque::new();
        d.push_front(1);
        d.push_front(2);
        d.push_front(3);
        assert_eq!(d.peek_front(), Some(&3));
        assert_eq!(d.peek_back(), Some(&1));
        assert_eq!(d.pop_back(), Some(1));
        assert_eq!(d.pop_back(), Some(2));
        assert_eq!(d.pop_back(), Some(3));
    }

    #[test]
    fn mixed_ops() {
        let mut d = ArrayDeque::new();
        d.push_back(2);
        d.push_front(1);
        d.push_back(3);
        assert_eq!(d.to_vec(), vec![1, 2, 3]);
        assert_eq!(d.pop_front(), Some(1));
        assert_eq!(d.pop_back(), Some(3));
        assert_eq!(d.pop_front(), Some(2));
    }

    #[test]
    fn contains_and_clear() {
        let mut d = ArrayDeque::of([1, 2, 3]);
        assert!(d.contains(&2));
        assert!(!d.contains(&99));
        d.clear();
        assert!(d.is_empty());
        assert!(!d.contains(&1));
    }

    #[test]
    fn of_and_iter() {
        let d = ArrayDeque::of([10, 20, 30]);
        let collected: Vec<_> = d.iter().copied().collect();
        assert_eq!(collected, vec![10, 20, 30]);
        let owned: Vec<_> = (&d).into_iter().copied().collect();
        assert_eq!(owned, vec![10, 20, 30]);
    }

    #[test]
    fn equality_and_display() {
        let a = ArrayDeque::of([1, 2, 3]);
        let b = ArrayDeque::of([1, 2, 3]);
        let c = ArrayDeque::of([1, 2]);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(format!("{}", a), "[1, 2, 3]");
        let e: ArrayDeque<i32> = ArrayDeque::new();
        assert_eq!(format!("{}", e), "[]");
    }
}
