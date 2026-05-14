// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i32`, `i64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I32I64Pair {
    one: i32,
    two: i64,
}

impl I32I64Pair {
    pub fn new(one: i32, two: i64) -> Self {
        I32I64Pair { one, two }
    }
    pub fn one(&self) -> i32 {
        self.one
    }
    pub fn two(&self) -> i64 {
        self.two
    }

    pub fn swap(&self) -> I64I32Pair {
        I64I32Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I32I64Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::i64_i32_pair::I64I32Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = I32I64Pair::new(1, 2);
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), 2);
    }

    #[test]
    fn test_equals() {
        let p1 = I32I64Pair::new(1, 2);
        let p2 = I32I64Pair::new(1, 2);
        let p3 = I32I64Pair::new(2, 1);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I32I64Pair::new(1, 2);
        assert!(!p.to_string().is_empty());
    }
}
