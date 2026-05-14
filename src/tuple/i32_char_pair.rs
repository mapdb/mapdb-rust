// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i32`, `char`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I32CharPair {
    one: i32,
    two: char,
}

impl I32CharPair {
    pub fn new(one: i32, two: char) -> Self {
        I32CharPair { one, two }
    }
    pub fn one(&self) -> i32 {
        self.one
    }
    pub fn two(&self) -> char {
        self.two
    }

    pub fn swap(&self) -> CharI32Pair {
        CharI32Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I32CharPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::char_i32_pair::CharI32Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = I32CharPair::new(1, 'b');
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), 'b');
    }

    #[test]
    fn test_equals() {
        let p1 = I32CharPair::new(1, 'b');
        let p2 = I32CharPair::new(1, 'b');
        let p3 = I32CharPair::new(2, 'a');
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I32CharPair::new(1, 'b');
        assert!(!p.to_string().is_empty());
    }
}
