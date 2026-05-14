// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`char`, `f64`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharF64Pair {
    one: char,
    two: f64,
}

impl CharF64Pair {
    pub fn new(one: char, two: f64) -> Self {
        CharF64Pair { one, two }
    }
    pub fn one(&self) -> char {
        self.one
    }
    pub fn two(&self) -> f64 {
        self.two
    }

    pub fn swap(&self) -> F64CharPair {
        F64CharPair::new(self.two, self.one)
    }
}

impl fmt::Display for CharF64Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::f64_char_pair::F64CharPair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = CharF64Pair::new('a', 2.0f64);
        assert_eq!(p.one(), 'a');
        assert_eq!(p.two(), 2.0f64);
    }

    #[test]
    fn test_equals() {
        let p1 = CharF64Pair::new('a', 2.0f64);
        let p2 = CharF64Pair::new('a', 2.0f64);
        let p3 = CharF64Pair::new('b', 1.0f64);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = CharF64Pair::new('a', 2.0f64);
        assert!(!p.to_string().is_empty());
    }
}
