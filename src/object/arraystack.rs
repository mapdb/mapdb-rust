// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

/// Generic LIFO stack backed by a `Vec<T>`.
#[derive(Debug, Clone)]
pub struct ArrayStack<T> {
    items: Vec<T>,
}

// ---- core stack API (formerly the trait tower) -----------------------------
//
// Only `contains` and the equality impls genuinely need `T: PartialEq`; the rest
// of the surface is `T`-generic, so an `ArrayStack` of non-`PartialEq` values
// (closures, non-comparable structs) is fully usable.

impl<T> ArrayStack<T> {
    pub fn new() -> Self {
        ArrayStack { items: Vec::new() }
    }
    /// An empty stack with room for `cap` elements before reallocating.
    pub fn with_capacity(cap: usize) -> Self {
        ArrayStack {
            items: Vec::with_capacity(cap),
        }
    }
    /// The number of elements.
    pub fn len(&self) -> usize {
        self.items.len()
    }
    /// Whether the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// Iterate top-to-bottom (most recently pushed first).
    pub fn iter(&self) -> std::iter::Rev<std::slice::Iter<'_, T>> {
        self.items.iter().rev()
    }
    /// Mutably iterate top-to-bottom.
    pub fn iter_mut(&mut self) -> std::iter::Rev<std::slice::IterMut<'_, T>> {
        self.items.iter_mut().rev()
    }
    /// The top element without removing it.
    pub fn peek(&self) -> Option<&T> {
        self.items.last()
    }
    /// Mutable reference to the top element without removing it.
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        self.items.last_mut()
    }
    /// Push `value` onto the top.
    pub fn push(&mut self, value: T) {
        self.items.push(value);
    }
    /// Pop the top element.
    pub fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }
    /// Remove all elements.
    pub fn clear(&mut self) {
        self.items.clear();
    }
    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        self.items.reserve(additional);
    }
    /// The element `depth` positions below the top (`depth = 0` is the top),
    /// or `None` if the stack is shallower than that.
    pub fn peek_at(&self, depth: usize) -> Option<&T> {
        if depth >= self.items.len() {
            return None;
        }
        Some(&self.items[self.items.len() - 1 - depth])
    }
}

impl<T: PartialEq> ArrayStack<T> {
    /// Whether `value` is present.
    pub fn contains(&self, value: &T) -> bool {
        self.items.contains(value)
    }
}

impl<T> Default for ArrayStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ---- idiomatic std-style additions ----------------------------------------
//
// Iteration order is top-to-bottom (matching `iter` and `peek`):
// the most recently pushed element comes first.

impl<'a, T> IntoIterator for &'a ArrayStack<T> {
    type Item = &'a T;
    type IntoIter = std::iter::Rev<std::slice::Iter<'a, T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter().rev()
    }
}

impl<T> IntoIterator for ArrayStack<T> {
    type Item = T;
    type IntoIter = std::iter::Rev<std::vec::IntoIter<T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter().rev()
    }
}

impl<T> FromIterator<T> for ArrayStack<T> {
    /// Pushes items in iteration order; the last item becomes the top.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        ArrayStack {
            items: iter.into_iter().collect(),
        }
    }
}

impl<T> Extend<T> for ArrayStack<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.items.extend(iter);
    }
}

/// Order-sensitive equality (same elements, same bottom-to-top order).
impl<T: PartialEq> PartialEq for ArrayStack<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl<T: Eq> Eq for ArrayStack<T> {}

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
        let s = ArrayStack::from_iter([1, 2, 3]);
        let v: Vec<_> = s.iter().copied().collect();
        assert_eq!(v, vec![3, 2, 1]);
    }

    #[test]
    fn test_contains() {
        let s = ArrayStack::from_iter(["a", "b", "c"]);
        assert!(s.contains(&"b"));
        assert!(!s.contains(&"z"));
    }

    #[test]
    fn test_into_iter_top_to_bottom() {
        let s = ArrayStack::from_iter([1, 2, 3]);
        let borrowed: Vec<i32> = (&s).into_iter().copied().collect();
        assert_eq!(borrowed, vec![3, 2, 1]);
        let owned: Vec<i32> = s.into_iter().collect();
        assert_eq!(owned, vec![3, 2, 1]);
    }

    #[test]
    fn test_from_iterator_and_extend() {
        let mut s: ArrayStack<i32> = (1..=3).collect();
        assert_eq!(s.peek(), Some(&3));
        s.extend([4, 5]);
        assert_eq!(s.peek(), Some(&5));
    }

    #[test]
    fn test_partial_eq() {
        let a = ArrayStack::from_iter([1, 2, 3]);
        let b = ArrayStack::from_iter([1, 2, 3]);
        let c = ArrayStack::from_iter([3, 2, 1]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn peek_mut_and_iter_mut() {
        let mut s: ArrayStack<i32> = ArrayStack::with_capacity(4);
        s.extend([1, 2, 3]); // top = 3
        *s.peek_mut().unwrap() = 30;
        assert_eq!(s.peek(), Some(&30));
        // iter_mut is top-to-bottom; double each.
        for v in s.iter_mut() {
            *v *= 10;
        }
        let v: Vec<i32> = s.iter().copied().collect();
        assert_eq!(v, vec![300, 20, 10]);
    }

    #[test]
    fn works_for_non_partialeq_element() {
        // A stack of closures (not `PartialEq`) must compile and operate — the
        // core API no longer over-requires `T: PartialEq`.
        let mut s: ArrayStack<Box<dyn Fn() -> i32>> = ArrayStack::new();
        s.push(Box::new(|| 1));
        s.push(Box::new(|| 2));
        assert_eq!(s.len(), 2);
        assert_eq!((s.peek().unwrap())(), 2);
        assert_eq!((s.pop().unwrap())(), 2);
    }
}
