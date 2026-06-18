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

    // ── slice-backed bulk overrides ──────────────────────────────────────────
    //
    // Same rationale as `ArrayList` (avoid the `Box<dyn Iterator>` path, which is
    // bimodal — ~0 if devirtualized, up to ~50× if not — and blocks
    // autovectorization). `ArrayStack` is `Vec`-backed and iterates **top-to-
    // bottom**, so every override iterates `self.items` in **reverse** to
    // preserve the documented encounter order. Behaviour is identical to the
    // defaults; only the boxed iterator is removed.

    fn for_each(&self, f: impl FnMut(&T)) {
        self.items.iter().rev().for_each(f);
    }
    fn any_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.items.iter().any(predicate) // order-independent
    }
    fn all_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.items.iter().all(predicate)
    }
    fn none_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool {
        !self.items.iter().any(predicate)
    }
    fn count_where(&self, predicate: impl Fn(&T) -> bool) -> usize {
        self.items.iter().filter(|v| predicate(v)).count()
    }
    fn detect(&self, predicate: impl Fn(&T) -> bool) -> Option<&T> {
        self.items.iter().rev().find(|v| predicate(v)) // top-to-bottom
    }
    fn select(&self, predicate: impl Fn(&T) -> bool) -> Vec<T>
    where
        T: Clone,
    {
        self.items.iter().rev().filter(|v| predicate(v)).cloned().collect()
    }
    fn reject(&self, predicate: impl Fn(&T) -> bool) -> Vec<T>
    where
        T: Clone,
    {
        self.items.iter().rev().filter(|v| !predicate(v)).cloned().collect()
    }
    fn inject_into<R>(&self, initial: R, f: impl FnMut(R, &T) -> R) -> R {
        self.items.iter().rev().fold(initial, f) // top-to-bottom
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

// ---- idiomatic std-style additions ----------------------------------------
//
// Iteration order is top-to-bottom (matching `Collection::iter` and `peek`):
// the most recently pushed element comes first.

impl<'a, T: PartialEq> IntoIterator for &'a ArrayStack<T> {
    type Item = &'a T;
    type IntoIter = std::iter::Rev<std::slice::Iter<'a, T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter().rev()
    }
}

impl<T: PartialEq> IntoIterator for ArrayStack<T> {
    type Item = T;
    type IntoIter = std::iter::Rev<std::vec::IntoIter<T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter().rev()
    }
}

impl<T: PartialEq> FromIterator<T> for ArrayStack<T> {
    /// Pushes items in iteration order; the last item becomes the top.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        ArrayStack {
            items: iter.into_iter().collect(),
        }
    }
}

impl<T: PartialEq> Extend<T> for ArrayStack<T> {
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

impl<T: PartialEq + Eq> Eq for ArrayStack<T> {}

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
    fn slice_overrides_preserve_top_to_bottom_order() {
        // The bulk overrides must match the boxed `iter()` (top-to-bottom).
        let s = ArrayStack::from_iter([1, 2, 3, 4, 5]); // top is 5
        // order-sensitive: select/reject/detect/for_each/inject_into
        assert_eq!(s.select(|v| *v % 2 == 1), vec![5, 3, 1]);
        assert_eq!(s.reject(|v| *v % 2 == 1), vec![4, 2]);
        assert_eq!(s.detect(|v| *v < 4), Some(&3)); // first match top-to-bottom
        let mut seen = Vec::new();
        s.for_each(|v| seen.push(*v));
        assert_eq!(seen, vec![5, 4, 3, 2, 1]);
        // fold sees top-to-bottom: ((((0*10+5)*10+4)...)
        let folded = s.inject_into(0, |acc, v| acc * 10 + *v);
        assert_eq!(folded, 54321);
        // order-independent
        assert_eq!(s.count_where(|v| *v > 2), 3);
        assert!(s.any_satisfy(|v| *v == 5));
        assert!(s.all_satisfy(|v| *v > 0));
        assert!(s.none_satisfy(|v| *v > 5));
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
}
