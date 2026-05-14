// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// LIFO stack of `f64` values, backed by a Vec.
#[derive(Debug, Clone)]
pub struct F64ArrayStack {
    items: Vec<f64>,
}

impl F64ArrayStack {
    pub fn new() -> Self {
        F64ArrayStack { items: Vec::new() }
    }

    pub fn of(values: &[f64]) -> Self {
        F64ArrayStack {
            items: values.to_vec(),
        }
    }

    /// Pushes a value onto the top of the stack.
    pub fn push(&mut self, value: f64) {
        self.items.push(value);
    }

    /// Removes and returns the top element, or None if empty.
    pub fn pop(&mut self) -> Option<f64> {
        self.items.pop()
    }

    /// Returns the top element without removing it, or None if empty.
    pub fn peek(&self) -> Option<f64> {
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

    pub fn contains(&self, value: f64) -> bool {
        self.items.iter().any(|&v| v.to_bits() == value.to_bits())
    }

    /// Iterates from top to bottom.
    pub fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.items.iter().rev().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(f64)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn select(&self, predicate: impl Fn(f64) -> bool) -> Self {
        F64ArrayStack {
            items: self
                .items
                .iter()
                .copied()
                .filter(|&v| predicate(v))
                .collect(),
        }
    }

    pub fn detect(&self, predicate: impl Fn(f64) -> bool) -> Option<f64> {
        self.iter().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(f64) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(f64) -> bool) -> bool {
        self.items.iter().all(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<f64> {
        self.iter().collect()
    }
}

impl Default for F64ArrayStack {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for F64ArrayStack {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        self.items
            .iter()
            .zip(other.items.iter())
            .all(|(a, b)| a.to_bits() == b.to_bits())
    }
}

impl fmt::Display for F64ArrayStack {
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
        let mut s = F64ArrayStack::new();
        s.push(1.0f64);
        s.push(2.0f64);
        s.push(3.0f64);
        assert_eq!(s.len(), 3);
        assert_eq!(s.peek(), Some(3.0f64));
        assert_eq!(s.pop(), Some(3.0f64));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_pop_empty() {
        let mut s = F64ArrayStack::new();
        assert_eq!(s.pop(), None);
        assert_eq!(s.peek(), None);
    }

    #[test]
    fn test_lifo_order() {
        let mut s = F64ArrayStack::new();
        s.push(1.0f64);
        s.push(2.0f64);
        s.push(3.0f64);
        assert_eq!(s.pop(), Some(3.0f64));
        assert_eq!(s.pop(), Some(2.0f64));
        assert_eq!(s.pop(), Some(1.0f64));
    }

    #[test]
    fn test_contains() {
        let mut s = F64ArrayStack::new();
        s.push(1.0f64);
        assert!(s.contains(1.0f64));
        assert!(!s.contains(3.0f64));
    }

    #[test]
    fn test_clear() {
        let mut s = F64ArrayStack::of(&[1.0f64]);
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_iter_top_to_bottom() {
        let mut s = F64ArrayStack::new();
        s.push(1.0f64);
        s.push(2.0f64);
        s.push(3.0f64);
        let vals: Vec<_> = s.iter().collect();
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0], 3.0f64); // top first
    }

    #[test]
    fn test_display() {
        let mut s = F64ArrayStack::new();
        s.push(1.0f64);
        assert!(!s.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_avoids_subsequent_realloc() {
        let mut s = F64ArrayStack::new();
        s.try_reserve(10).unwrap();
        let reserved = s.items.capacity();
        assert!(reserved >= 10);
        s.push(1.0f64);
        s.push(2.0f64);
        s.push(3.0f64);
        assert_eq!(reserved, s.items.capacity());
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut s = F64ArrayStack::new();
        assert!(s.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::f64_collection::F64Collection for F64ArrayStack {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: f64) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.iter()
    }
}

impl crate::traits::f64_collection::F64MutableCollection for F64ArrayStack {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::f64_collection::F64Stack for F64ArrayStack {
    fn peek(&self) -> Option<f64> {
        self.peek()
    }
}

impl crate::traits::f64_collection::F64MutableStack for F64ArrayStack {
    fn push(&mut self, value: f64) {
        self.push(value)
    }
    fn pop(&mut self) -> Option<f64> {
        self.pop()
    }
}
