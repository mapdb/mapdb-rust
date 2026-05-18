// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Generic 2-tuple. Replaces the 65 monomorphised `XxxYyyPair` files.

use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct Pair<A, B> {
    pub one: A,
    pub two: B,
}

impl<A, B> Pair<A, B> {
    pub const fn new(one: A, two: B) -> Self {
        Pair { one, two }
    }

    pub fn one(&self) -> &A {
        &self.one
    }

    pub fn two(&self) -> &B {
        &self.two
    }

    pub fn into_tuple(self) -> (A, B) {
        (self.one, self.two)
    }
}

impl<A> Pair<A, A> {
    /// Only available when both components have the same type. Returns a new
    /// pair with the elements swapped.
    pub fn swap(self) -> Pair<A, A> {
        Pair {
            one: self.two,
            two: self.one,
        }
    }
}

impl<A: PartialEq, B: PartialEq> PartialEq for Pair<A, B> {
    fn eq(&self, other: &Self) -> bool {
        self.one == other.one && self.two == other.two
    }
}

impl<A: Eq, B: Eq> Eq for Pair<A, B> {}

impl<A: std::hash::Hash, B: std::hash::Hash> std::hash::Hash for Pair<A, B> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.one.hash(state);
        self.two.hash(state);
    }
}

impl<A: PartialOrd, B: PartialOrd> PartialOrd for Pair<A, B> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.one.partial_cmp(&other.one)? {
            Ordering::Equal => self.two.partial_cmp(&other.two),
            other => Some(other),
        }
    }
}

impl<A: Ord, B: Ord> Ord for Pair<A, B> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.one.cmp(&other.one) {
            Ordering::Equal => self.two.cmp(&other.two),
            ord => ord,
        }
    }
}

impl<A: fmt::Display, B: fmt::Display> fmt::Display for Pair<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

impl<A, B> From<(A, B)> for Pair<A, B> {
    fn from(t: (A, B)) -> Self {
        Pair::new(t.0, t.1)
    }
}

impl<A, B> From<Pair<A, B>> for (A, B) {
    fn from(p: Pair<A, B>) -> Self {
        (p.one, p.two)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_access() {
        let p = Pair::new(1, "hi");
        assert_eq!(p.one, 1);
        assert_eq!(p.two, "hi");
    }

    #[test]
    fn swap_self_pair() {
        let p = Pair::new(1, 2);
        let s = p.swap();
        assert_eq!(s.one, 2);
        assert_eq!(s.two, 1);
    }

    #[test]
    fn equality_and_order() {
        let a = Pair::new(1, 2);
        let b = Pair::new(1, 2);
        let c = Pair::new(1, 3);
        assert_eq!(a, b);
        assert!(a < c);
    }

    #[test]
    fn display() {
        let p = Pair::new(1, 2);
        assert_eq!(p.to_string(), "(1, 2)");
    }
}
