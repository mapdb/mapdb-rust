// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// LIFO stack of `i8` values, backed by a Vec.
#[derive(Debug, Clone)]
pub struct I8ArrayStack {
    items: Vec<i8>,
}

impl I8ArrayStack {
    pub fn new() -> Self {
        I8ArrayStack { items: Vec::new() }
    }

    pub fn of(values: &[i8]) -> Self {
        I8ArrayStack {
            items: values.to_vec(),
        }
    }

    /// Pushes a value onto the top of the stack.
    pub fn push(&mut self, value: i8) {
        self.items.push(value);
    }

    /// Removes and returns the top element, or None if empty.
    pub fn pop(&mut self) -> Option<i8> {
        self.items.pop()
    }

    /// Returns the top element without removing it, or None if empty.
    pub fn peek(&self) -> Option<i8> {
        self.items.last().copied()
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

    /// Ensures that `additional` more items can be pushed without a
    /// reallocation. Returns `TryReserveError` on allocator failure. See
    /// `docs/rust/error-handling.md`.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.items.try_reserve(additional)
    }

    /// See [`Vec::try_reserve_exact`].
    pub fn try_reserve_exact(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.items.try_reserve_exact(additional)
    }

    pub fn contains(&self, value: i8) -> bool {
        self.items.contains(&value)
    }

    /// Iterates from top to bottom.
    pub fn iter(&self) -> impl Iterator<Item = i8> + '_ {
        self.items.iter().rev().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(i8)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn select(&self, predicate: impl Fn(i8) -> bool) -> Self {
        I8ArrayStack {
            items: self
                .items
                .iter()
                .copied()
                .filter(|&v| predicate(v))
                .collect(),
        }
    }

    pub fn detect(&self, predicate: impl Fn(i8) -> bool) -> Option<i8> {
        self.iter().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(i8) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(i8) -> bool) -> bool {
        self.items.iter().all(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<i8> {
        self.iter().collect()
    }
}

impl Default for I8ArrayStack {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for I8ArrayStack {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        self.items
            .iter()
            .zip(other.items.iter())
            .all(|(a, b)| a == b)
    }
}

impl fmt::Display for I8ArrayStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.items.iter().rev().enumerate() {
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
    fn test_push_peek_pop() {
        let mut s = I8ArrayStack::new();
        s.push(1);
        s.push(2);
        s.push(3);
        assert_eq!(s.len(), 3);
        assert_eq!(s.peek(), Some(3));
        assert_eq!(s.pop(), Some(3));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_pop_empty() {
        let mut s = I8ArrayStack::new();
        assert_eq!(s.pop(), None);
        assert_eq!(s.peek(), None);
    }

    #[test]
    fn test_lifo_order() {
        let mut s = I8ArrayStack::new();
        s.push(1);
        s.push(2);
        s.push(3);
        assert_eq!(s.pop(), Some(3));
        assert_eq!(s.pop(), Some(2));
        assert_eq!(s.pop(), Some(1));
    }

    #[test]
    fn test_contains() {
        let mut s = I8ArrayStack::new();
        s.push(1);
        assert!(s.contains(1));
        assert!(!s.contains(3));
    }

    #[test]
    fn test_clear() {
        let mut s = I8ArrayStack::of(&[1]);
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_iter_top_to_bottom() {
        let mut s = I8ArrayStack::new();
        s.push(1);
        s.push(2);
        s.push(3);
        let vals: Vec<_> = s.iter().collect();
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0], 3); // top first
    }

    #[test]
    fn test_display() {
        let mut s = I8ArrayStack::new();
        s.push(1);
        assert!(!s.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_avoids_subsequent_realloc() {
        let mut s = I8ArrayStack::new();
        s.try_reserve(10).unwrap();
        let reserved = s.items.capacity();
        assert!(reserved >= 10);
        s.push(1);
        s.push(2);
        s.push(3);
        assert_eq!(reserved, s.items.capacity());
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut s = I8ArrayStack::new();
        assert!(s.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::i8_collection::I8Collection for I8ArrayStack {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: i8) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = i8> + '_ {
        self.iter()
    }
}

impl crate::traits::i8_collection::I8MutableCollection for I8ArrayStack {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::i8_collection::I8Stack for I8ArrayStack {
    fn peek(&self) -> Option<i8> {
        self.peek()
    }
}

impl crate::traits::i8_collection::I8MutableStack for I8ArrayStack {
    fn push(&mut self, value: i8) {
        self.push(value)
    }
    fn pop(&mut self) -> Option<i8> {
        self.pop()
    }
}
