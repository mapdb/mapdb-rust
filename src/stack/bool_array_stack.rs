// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// LIFO stack of `bool` values, backed by a Vec.
#[derive(Debug, Clone)]
pub struct BoolArrayStack {
    items: Vec<bool>,
}

impl BoolArrayStack {
    pub fn new() -> Self {
        BoolArrayStack { items: Vec::new() }
    }

    pub fn of(values: &[bool]) -> Self {
        BoolArrayStack {
            items: values.to_vec(),
        }
    }

    /// Pushes a value onto the top of the stack.
    pub fn push(&mut self, value: bool) {
        self.items.push(value);
    }

    /// Removes and returns the top element, or None if empty.
    pub fn pop(&mut self) -> Option<bool> {
        self.items.pop()
    }

    /// Returns the top element without removing it, or None if empty.
    pub fn peek(&self) -> Option<bool> {
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

    pub fn contains(&self, value: bool) -> bool {
        self.items.contains(&value)
    }

    /// Iterates from top to bottom.
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.items.iter().rev().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(bool)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn select(&self, predicate: impl Fn(bool) -> bool) -> Self {
        BoolArrayStack {
            items: self
                .items
                .iter()
                .copied()
                .filter(|&v| predicate(v))
                .collect(),
        }
    }

    pub fn detect(&self, predicate: impl Fn(bool) -> bool) -> Option<bool> {
        self.iter().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(bool) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(bool) -> bool) -> bool {
        self.items.iter().all(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<bool> {
        self.iter().collect()
    }
}

impl Default for BoolArrayStack {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for BoolArrayStack {
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

impl fmt::Display for BoolArrayStack {
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
        let mut s = BoolArrayStack::new();
        s.push(true);
        s.push(false);
        s.push(true);
        assert_eq!(s.len(), 3);
        assert_eq!(s.peek(), Some(true));
        assert_eq!(s.pop(), Some(true));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_pop_empty() {
        let mut s = BoolArrayStack::new();
        assert_eq!(s.pop(), None);
        assert_eq!(s.peek(), None);
    }

    #[test]
    fn test_lifo_order() {
        let mut s = BoolArrayStack::new();
        s.push(true);
        s.push(false);
        s.push(true);
        assert_eq!(s.pop(), Some(true));
        assert_eq!(s.pop(), Some(false));
        assert_eq!(s.pop(), Some(true));
    }

    #[test]
    fn test_contains() {
        let mut s = BoolArrayStack::new();
        s.push(true);
        assert!(s.contains(true));
    }

    #[test]
    fn test_clear() {
        let mut s = BoolArrayStack::of(&[true]);
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_iter_top_to_bottom() {
        let mut s = BoolArrayStack::new();
        s.push(true);
        s.push(false);
        s.push(true);
        let vals: Vec<_> = s.iter().collect();
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0], true); // top first
    }

    #[test]
    fn test_display() {
        let mut s = BoolArrayStack::new();
        s.push(true);
        assert!(!s.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_avoids_subsequent_realloc() {
        let mut s = BoolArrayStack::new();
        s.try_reserve(10).unwrap();
        let reserved = s.items.capacity();
        assert!(reserved >= 10);
        s.push(true);
        s.push(false);
        s.push(true);
        assert_eq!(reserved, s.items.capacity());
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut s = BoolArrayStack::new();
        assert!(s.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::bool_collection::BoolCollection for BoolArrayStack {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: bool) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter()
    }
}

impl crate::traits::bool_collection::BoolMutableCollection for BoolArrayStack {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::bool_collection::BoolStack for BoolArrayStack {
    fn peek(&self) -> Option<bool> {
        self.peek()
    }
}

impl crate::traits::bool_collection::BoolMutableStack for BoolArrayStack {
    fn push(&mut self, value: bool) {
        self.push(value)
    }
    fn pop(&mut self) -> Option<bool> {
        self.pop()
    }
}
