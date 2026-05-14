// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i32`, `bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I32BoolPair {
    one: i32,
    two: bool,
}

impl I32BoolPair {
    pub fn new(one: i32, two: bool) -> Self {
        I32BoolPair { one, two }
    }
    pub fn one(&self) -> i32 {
        self.one
    }
    pub fn two(&self) -> bool {
        self.two
    }

    pub fn swap(&self) -> BoolI32Pair {
        BoolI32Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I32BoolPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::bool_i32_pair::BoolI32Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = I32BoolPair::new(1, false);
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), false);
    }

    #[test]
    fn test_equals() {
        let p1 = I32BoolPair::new(1, false);
        let p2 = I32BoolPair::new(1, false);
        let p3 = I32BoolPair::new(2, true);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I32BoolPair::new(1, false);
        assert!(!p.to_string().is_empty());
    }
}
