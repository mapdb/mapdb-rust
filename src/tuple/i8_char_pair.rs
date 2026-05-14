// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i8`, `char`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I8CharPair {
    one: i8,
    two: char,
}

impl I8CharPair {
    pub fn new(one: i8, two: char) -> Self {
        I8CharPair { one, two }
    }
    pub fn one(&self) -> i8 {
        self.one
    }
    pub fn two(&self) -> char {
        self.two
    }

    pub fn swap(&self) -> CharI8Pair {
        CharI8Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I8CharPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::char_i8_pair::CharI8Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = I8CharPair::new(1, 'b');
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), 'b');
    }

    #[test]
    fn test_equals() {
        let p1 = I8CharPair::new(1, 'b');
        let p2 = I8CharPair::new(1, 'b');
        let p3 = I8CharPair::new(2, 'a');
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I8CharPair::new(1, 'b');
        assert!(!p.to_string().is_empty());
    }
}
