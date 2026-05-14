// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i16`, `f64`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct I16F64Pair {
    one: i16,
    two: f64,
}

impl I16F64Pair {
    pub fn new(one: i16, two: f64) -> Self {
        I16F64Pair { one, two }
    }
    pub fn one(&self) -> i16 {
        self.one
    }
    pub fn two(&self) -> f64 {
        self.two
    }

    pub fn swap(&self) -> F64I16Pair {
        F64I16Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I16F64Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::f64_i16_pair::F64I16Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = I16F64Pair::new(1, 2.0f64);
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), 2.0f64);
    }

    #[test]
    fn test_equals() {
        let p1 = I16F64Pair::new(1, 2.0f64);
        let p2 = I16F64Pair::new(1, 2.0f64);
        let p3 = I16F64Pair::new(2, 1.0f64);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I16F64Pair::new(1, 2.0f64);
        assert!(!p.to_string().is_empty());
    }
}
