// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use super::traits::*;

/// Generic LIFO stack backed by a `Vec<T>`.
#[derive(Debug, Clone)]
pub struct ArrayStack<T> {
    items: Vec<T>,
}

impl<T> ArrayStack<T> {
    pub fn new() -> Self {
        ArrayStack { items: Vec::new() }
    }
    pub fn of(values: impl IntoIterator<Item = T>) -> Self {
        ArrayStack {
            items: values.into_iter().collect(),
        }
    }
}

impl<T: PartialEq> Collection<T> for ArrayStack<T> {
    fn len(&self) -> usize {
        self.items.len()
    }
    fn contains(&self, value: &T) -> bool {
        self.items.contains(value)
    }
    /// Iterates top-to-bottom.
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(self.items.iter().rev())
    }
}

impl<T: PartialEq> MutableCollection<T> for ArrayStack<T> {
    fn clear(&mut self) {
        self.items.clear();
    }
}

impl<T: PartialEq> Stack<T> for ArrayStack<T> {
    fn peek(&self) -> Option<&T> {
        self.items.last()
    }
}

impl<T: PartialEq> MutableStack<T> for ArrayStack<T> {
    fn push(&mut self, value: T) {
        self.items.push(value);
    }
    fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }
}

impl<T: PartialEq> ArrayStack<T> {
    pub fn peek_at(&self, depth: usize) -> Option<&T> {
        if depth >= self.items.len() {
            return None;
        }
        Some(&self.items[self.items.len() - 1 - depth])
    }
}

impl<T: PartialEq> Default for ArrayStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop_peek() {
        let mut s = ArrayStack::new();
        s.push(1);
        s.push(2);
        s.push(3);
        assert_eq!(s.peek(), Some(&3));
        assert_eq!(s.pop(), Some(3));
        assert_eq!(s.pop(), Some(2));
        assert_eq!(s.len(), 1);
        assert_eq!(s.peek_at(0), Some(&1));
    }

    #[test]
    fn test_empty_ops() {
        let mut s: ArrayStack<i32> = ArrayStack::new();
        assert!(s.is_empty());
        assert_eq!(s.peek(), None);
        assert_eq!(s.pop(), None);
    }

    #[test]
    fn test_iter_top_to_bottom() {
        let s = ArrayStack::of(vec![1, 2, 3]);
        let v: Vec<_> = s.iter().copied().collect();
        assert_eq!(v, vec![3, 2, 1]);
    }

    #[test]
    fn test_contains() {
        let s = ArrayStack::of(vec!["a", "b", "c"]);
        assert!(s.contains(&"b"));
        assert!(!s.contains(&"z"));
    }
}
