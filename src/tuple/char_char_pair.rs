// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`char`, `char`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharCharPair {
    one: char,
    two: char,
}

impl CharCharPair {
    pub fn new(one: char, two: char) -> Self {
        CharCharPair { one, two }
    }
    pub fn one(&self) -> char {
        self.one
    }
    pub fn two(&self) -> char {
        self.two
    }

    pub fn swap(&self) -> CharCharPair {
        CharCharPair::new(self.two, self.one)
    }
}

impl fmt::Display for CharCharPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
// Self-pair: swap returns same type

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = CharCharPair::new('a', 'b');
        assert_eq!(p.one(), 'a');
        assert_eq!(p.two(), 'b');
    }

    #[test]
    fn test_equals() {
        let p1 = CharCharPair::new('a', 'b');
        let p2 = CharCharPair::new('a', 'b');
        let p3 = CharCharPair::new('b', 'a');
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = CharCharPair::new('a', 'b');
        assert!(!p.to_string().is_empty());
    }
}
