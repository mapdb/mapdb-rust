// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i8`, `bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I8BoolPair {
    one: i8,
    two: bool,
}

impl I8BoolPair {
    pub fn new(one: i8, two: bool) -> Self {
        I8BoolPair { one, two }
    }
    pub fn one(&self) -> i8 {
        self.one
    }
    pub fn two(&self) -> bool {
        self.two
    }

    pub fn swap(&self) -> BoolI8Pair {
        BoolI8Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I8BoolPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::bool_i8_pair::BoolI8Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = I8BoolPair::new(1, false);
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), false);
    }

    #[test]
    fn test_equals() {
        let p1 = I8BoolPair::new(1, false);
        let p2 = I8BoolPair::new(1, false);
        let p3 = I8BoolPair::new(2, true);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I8BoolPair::new(1, false);
        assert!(!p.to_string().is_empty());
    }
}
