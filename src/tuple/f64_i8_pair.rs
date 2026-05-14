// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`f64`, `i8`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F64I8Pair {
    one: f64,
    two: i8,
}

impl F64I8Pair {
    pub fn new(one: f64, two: i8) -> Self {
        F64I8Pair { one, two }
    }
    pub fn one(&self) -> f64 {
        self.one
    }
    pub fn two(&self) -> i8 {
        self.two
    }

    pub fn swap(&self) -> I8F64Pair {
        I8F64Pair::new(self.two, self.one)
    }
}

impl fmt::Display for F64I8Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::i8_f64_pair::I8F64Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = F64I8Pair::new(1.0f64, 2);
        assert_eq!(p.one(), 1.0f64);
        assert_eq!(p.two(), 2);
    }

    #[test]
    fn test_equals() {
        let p1 = F64I8Pair::new(1.0f64, 2);
        let p2 = F64I8Pair::new(1.0f64, 2);
        let p3 = F64I8Pair::new(2.0f64, 1);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = F64I8Pair::new(1.0f64, 2);
        assert!(!p.to_string().is_empty());
    }
}
