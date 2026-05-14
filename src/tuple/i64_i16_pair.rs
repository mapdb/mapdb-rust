// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i64`, `i16`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I64I16Pair {
    one: i64,
    two: i16,
}

impl I64I16Pair {
    pub fn new(one: i64, two: i16) -> Self {
        I64I16Pair { one, two }
    }
    pub fn one(&self) -> i64 {
        self.one
    }
    pub fn two(&self) -> i16 {
        self.two
    }

    pub fn swap(&self) -> I16I64Pair {
        I16I64Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I64I16Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::i16_i64_pair::I16I64Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = I64I16Pair::new(1, 2);
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), 2);
    }

    #[test]
    fn test_equals() {
        let p1 = I64I16Pair::new(1, 2);
        let p2 = I64I16Pair::new(1, 2);
        let p3 = I64I16Pair::new(2, 1);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I64I16Pair::new(1, 2);
        assert!(!p.to_string().is_empty());
    }
}
