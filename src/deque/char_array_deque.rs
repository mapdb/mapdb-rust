// AUTO-GENERATED. DO NOT EDIT.

use std::collections::VecDeque;
use std::fmt;

/// Double-ended queue of `char` values, backed by `VecDeque`.
/// O(1) amortized push/pop at both ends.
#[derive(Debug, Clone)]
pub struct CharArrayDeque {
    inner: VecDeque<char>,
}

impl CharArrayDeque {
    pub fn new() -> Self {
        CharArrayDeque {
            inner: VecDeque::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        CharArrayDeque {
            inner: VecDeque::with_capacity(capacity),
        }
    }

    pub fn of(values: &[char]) -> Self {
        CharArrayDeque {
            inner: values.iter().copied().collect(),
        }
    }

    /// Adds a value to the front of the deque.
    pub fn add_first(&mut self, value: char) {
        self.inner.push_front(value);
    }

    /// Adds a value to the back of the deque.
    pub fn add_last(&mut self, value: char) {
        self.inner.push_back(value);
    }

    /// Removes and returns the front element, or None if empty.
    pub fn remove_first(&mut self) -> Option<char> {
        self.inner.pop_front()
    }

    /// Removes and returns the back element, or None if empty.
    pub fn remove_last(&mut self) -> Option<char> {
        self.inner.pop_back()
    }

    /// Returns the front element without removing it, or None if empty.
    pub fn peek_first(&self) -> Option<char> {
        self.inner.front().copied()
    }

    /// Returns the back element without removing it, or None if empty.
    pub fn peek_last(&self) -> Option<char> {
        self.inner.back().copied()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Ensures the deque has capacity for `additional` more elements.
    /// Returns `TryReserveError` on allocator failure.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.inner.try_reserve(additional)
    }

    pub fn contains(&self, value: char) -> bool {
        self.inner.iter().any(|&v| v == value)
    }

    /// Iterates from front to back.
    pub fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.inner.iter().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(char)) {
        for v in self.iter() {
            f(v);
        }
    }

    pub fn any_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
        self.inner.iter().any(|&v| predicate(v))
    }

    pub fn all_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
        self.inner.iter().all(|&v| predicate(v))
    }

    pub fn to_vec(&self) -> Vec<char> {
        self.iter().collect()
    }
}

impl Default for CharArrayDeque {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for CharArrayDeque {
    fn eq(&self, other: &Self) -> bool {
        if self.inner.len() != other.inner.len() {
            return false;
        }
        self.inner
            .iter()
            .zip(other.inner.iter())
            .all(|(a, b)| a == b)
    }
}

impl fmt::Display for CharArrayDeque {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.inner.iter().enumerate() {
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
    fn test_add_last_remove_first_fifo() {
        let mut d = CharArrayDeque::new();
        d.add_last('a');
        d.add_last('b');
        d.add_last('c');
        assert_eq!(d.len(), 3);
        assert_eq!(d.remove_first(), Some('a'));
        assert_eq!(d.remove_first(), Some('b'));
        assert_eq!(d.remove_first(), Some('c'));
        assert!(d.is_empty());
    }

    #[test]
    fn test_add_first_remove_last() {
        let mut d = CharArrayDeque::new();
        d.add_first('a');
        d.add_first('b');
        d.add_first('c');
        // front: v[2], v[1], v[0]
        assert_eq!(d.peek_first(), Some('c'));
        assert_eq!(d.peek_last(), Some('a'));
        assert_eq!(d.remove_last(), Some('a'));
        assert_eq!(d.remove_last(), Some('b'));
        assert_eq!(d.remove_last(), Some('c'));
    }

    #[test]
    fn test_remove_empty() {
        let mut d = CharArrayDeque::new();
        assert_eq!(d.remove_first(), None);
        assert_eq!(d.remove_last(), None);
        assert_eq!(d.peek_first(), None);
        assert_eq!(d.peek_last(), None);
    }

    #[test]
    fn test_mixed_ops() {
        let mut d = CharArrayDeque::new();
        d.add_last('b');
        d.add_first('a');
        d.add_last('c');
        // order: v[0], v[1], v[2]
        assert_eq!(d.remove_first(), Some('a'));
        assert_eq!(d.remove_last(), Some('c'));
        assert_eq!(d.remove_first(), Some('b'));
    }

    #[test]
    fn test_contains() {
        let mut d = CharArrayDeque::new();
        d.add_last('a');
        assert!(d.contains('a'));
        assert!(!d.contains('c'));
    }

    #[test]
    fn test_clear() {
        let mut d = CharArrayDeque::of(&['a', 'b']);
        d.clear();
        assert!(d.is_empty());
    }

    #[test]
    fn test_iter_front_to_back() {
        let d = CharArrayDeque::of(&['a', 'b', 'c']);
        let vals: Vec<_> = d.iter().collect();
        assert_eq!(vals, vec!['a', 'b', 'c']);
    }

    #[test]
    fn test_display() {
        let mut d = CharArrayDeque::new();
        d.add_last('a');
        assert!(!d.to_string().is_empty());
    }

    #[test]
    fn test_try_reserve_happy_path() {
        let mut d = CharArrayDeque::new();
        d.try_reserve(10).unwrap();
    }

    #[test]
    fn test_try_reserve_propagates_capacity_overflow() {
        let mut d = CharArrayDeque::new();
        assert!(d.try_reserve(usize::MAX / 2).is_err());
    }
}
