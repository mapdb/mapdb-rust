// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`f64`, `bool`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F64BoolPair {
    one: f64,
    two: bool,
}

impl F64BoolPair {
    pub fn new(one: f64, two: bool) -> Self {
        F64BoolPair { one, two }
    }
    pub fn one(&self) -> f64 {
        self.one
    }
    pub fn two(&self) -> bool {
        self.two
    }

    pub fn swap(&self) -> BoolF64Pair {
        BoolF64Pair::new(self.two, self.one)
    }
}

impl fmt::Display for F64BoolPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::bool_f64_pair::BoolF64Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = F64BoolPair::new(1.0f64, false);
        assert_eq!(p.one(), 1.0f64);
        assert_eq!(p.two(), false);
    }

    #[test]
    fn test_equals() {
        let p1 = F64BoolPair::new(1.0f64, false);
        let p2 = F64BoolPair::new(1.0f64, false);
        let p3 = F64BoolPair::new(2.0f64, true);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = F64BoolPair::new(1.0f64, false);
        assert!(!p.to_string().is_empty());
    }
}
