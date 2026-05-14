// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`char`, `bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharBoolPair {
    one: char,
    two: bool,
}

impl CharBoolPair {
    pub fn new(one: char, two: bool) -> Self {
        CharBoolPair { one, two }
    }
    pub fn one(&self) -> char {
        self.one
    }
    pub fn two(&self) -> bool {
        self.two
    }

    pub fn swap(&self) -> BoolCharPair {
        BoolCharPair::new(self.two, self.one)
    }
}

impl fmt::Display for CharBoolPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::bool_char_pair::BoolCharPair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = CharBoolPair::new('a', false);
        assert_eq!(p.one(), 'a');
        assert_eq!(p.two(), false);
    }

    #[test]
    fn test_equals() {
        let p1 = CharBoolPair::new('a', false);
        let p2 = CharBoolPair::new('a', false);
        let p3 = CharBoolPair::new('b', true);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = CharBoolPair::new('a', false);
        assert!(!p.to_string().is_empty());
    }
}
