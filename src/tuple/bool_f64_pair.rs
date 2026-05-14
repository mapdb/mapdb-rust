// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`bool`, `f64`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoolF64Pair {
    one: bool,
    two: f64,
}

impl BoolF64Pair {
    pub fn new(one: bool, two: f64) -> Self {
        BoolF64Pair { one, two }
    }
    pub fn one(&self) -> bool {
        self.one
    }
    pub fn two(&self) -> f64 {
        self.two
    }

    pub fn swap(&self) -> F64BoolPair {
        F64BoolPair::new(self.two, self.one)
    }
}

impl fmt::Display for BoolF64Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::f64_bool_pair::F64BoolPair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = BoolF64Pair::new(true, 2.0f64);
        assert_eq!(p.one(), true);
        assert_eq!(p.two(), 2.0f64);
    }

    #[test]
    fn test_equals() {
        let p1 = BoolF64Pair::new(true, 2.0f64);
        let p2 = BoolF64Pair::new(true, 2.0f64);
        let p3 = BoolF64Pair::new(false, 1.0f64);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = BoolF64Pair::new(true, 2.0f64);
        assert!(!p.to_string().is_empty());
    }
}
