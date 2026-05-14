// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i64`, `bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I64BoolPair {
    one: i64,
    two: bool,
}

impl I64BoolPair {
    pub fn new(one: i64, two: bool) -> Self {
        I64BoolPair { one, two }
    }
    pub fn one(&self) -> i64 {
        self.one
    }
    pub fn two(&self) -> bool {
        self.two
    }

    pub fn swap(&self) -> BoolI64Pair {
        BoolI64Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I64BoolPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::bool_i64_pair::BoolI64Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = I64BoolPair::new(1, false);
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), false);
    }

    #[test]
    fn test_equals() {
        let p1 = I64BoolPair::new(1, false);
        let p2 = I64BoolPair::new(1, false);
        let p3 = I64BoolPair::new(2, true);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I64BoolPair::new(1, false);
        assert!(!p.to_string().is_empty());
    }
}
