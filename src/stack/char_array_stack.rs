// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// LIFO stack of `char` values, backed by a Vec.
#[derive(Debug, Clone)]
pub struct CharArrayStack {
    items: Vec<char>,
}

impl CharArrayStack {
    pub fn new() -> Self {
        CharArrayStack { items: Vec::new() }
    }

    pub fn of(values: &[char]) -> Self {
        CharArrayStack {
            items: values.to_vec(),
        }
    }

    /// Pushes a value onto the top of the stack.
    pub fn push(&mut self, value: char) {
        self.items.push(value);
    }

    /// Removes and returns the top element, or None if empty.
    pub fn pop(&mut self) -> Option<char> {
        self.items.pop()
    }

    /// Returns the top element without removing it, or None if empty.
    pub fn peek(&self) -> Option<char> {
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

    pub fn contains(&self, value: char) -> bool {
        self.items.contains(&value)
    }

    /// Iterates from top to bottom.
    pub fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.items.iter().rev().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(char)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn select(&self, predicate: impl Fn(char) -> bool) -> Self {
        CharArrayStack {
            items: self
                .items
                .iter()
                .copied()
                .filter(|&v| predicate(v))
                .collect(),
        }
    }

    pub fn detect(&self, predicate: impl Fn(char) -> bool) -> Option<char> {
        self.iter().find(|&v| predicate(v))
    }

    pub fn any_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
        self.items.iter().any(|&v| predicate(v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
        self.items.iter().all(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<char> {
        self.iter().collect()
    }
}

impl Default for CharArrayStack {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for CharArrayStack {
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

impl fmt::Display for CharArrayStack {
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
        let mut s = CharArrayStack::new();
        s.push('a');
        s.push('b');
        s.push('c');
        assert_eq!(s.len(), 3);
        assert_eq!(s.peek(), Some('c'));
        assert_eq!(s.pop(), Some('c'));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_pop_empty() {
        let mut s = CharArrayStack::new();
        assert_eq!(s.pop(), None);
        assert_eq!(s.peek(), None);
    }

    #[test]
    fn test_lifo_order() {
        let mut s = CharArrayStack::new();
        s.push('a');
        s.push('b');
        s.push('c');
        assert_eq!(s.pop(), Some('c'));
        assert_eq!(s.pop(), Some('b'));
        assert_eq!(s.pop(), Some('a'));
    }

    #[test]
    fn test_contains() {
        let mut s = CharArrayStack::new();
        s.push('a');
        assert!(s.contains('a'));
        assert!(!s.contains('c'));
    }

    #[test]
    fn test_clear() {
        let mut s = CharArrayStack::of(&['a']);
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_iter_top_to_bottom() {
        let mut s = CharArrayStack::new();
        s.push('a');
        s.push('b');
        s.push('c');
        let vals: Vec<_> = s.iter().collect();
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0], 'c'); // top first
    }

    #[test]
    fn test_display() {
        let mut s = CharArrayStack::new();
        s.push('a');
        assert!(!s.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_avoids_subsequent_realloc() {
        let mut s = CharArrayStack::new();
        s.try_reserve(10).unwrap();
        let reserved = s.items.capacity();
        assert!(reserved >= 10);
        s.push('a');
        s.push('b');
        s.push('c');
        assert_eq!(reserved, s.items.capacity());
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut s = CharArrayStack::new();
        assert!(s.try_reserve(usize::MAX / 2).is_err());
    }
}

impl crate::traits::char_collection::CharCollection for CharArrayStack {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: char) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.iter()
    }
}

impl crate::traits::char_collection::CharMutableCollection for CharArrayStack {
    fn clear(&mut self) {
        self.clear()
    }
}

impl crate::traits::char_collection::CharStack for CharArrayStack {
    fn peek(&self) -> Option<char> {
        self.peek()
    }
}

impl crate::traits::char_collection::CharMutableStack for CharArrayStack {
    fn push(&mut self, value: char) {
        self.push(value)
    }
    fn pop(&mut self) -> Option<char> {
        self.pop()
    }
}
