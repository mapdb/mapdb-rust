// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`char`, `i32`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharI32Pair {
    one: char,
    two: i32,
}

impl CharI32Pair {
    pub fn new(one: char, two: i32) -> Self {
        CharI32Pair { one, two }
    }
    pub fn one(&self) -> char {
        self.one
    }
    pub fn two(&self) -> i32 {
        self.two
    }

    pub fn swap(&self) -> I32CharPair {
        I32CharPair::new(self.two, self.one)
    }
}

impl fmt::Display for CharI32Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::i32_char_pair::I32CharPair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = CharI32Pair::new('a', 2);
        assert_eq!(p.one(), 'a');
        assert_eq!(p.two(), 2);
    }

    #[test]
    fn test_equals() {
        let p1 = CharI32Pair::new('a', 2);
        let p2 = CharI32Pair::new('a', 2);
        let p3 = CharI32Pair::new('b', 1);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = CharI32Pair::new('a', 2);
        assert!(!p.to_string().is_empty());
    }
}
