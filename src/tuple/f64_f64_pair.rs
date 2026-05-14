// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`f64`, `f64`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F64F64Pair {
    one: f64,
    two: f64,
}

impl F64F64Pair {
    pub fn new(one: f64, two: f64) -> Self {
        F64F64Pair { one, two }
    }
    pub fn one(&self) -> f64 {
        self.one
    }
    pub fn two(&self) -> f64 {
        self.two
    }

    pub fn swap(&self) -> F64F64Pair {
        F64F64Pair::new(self.two, self.one)
    }
}

impl fmt::Display for F64F64Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
// Self-pair: swap returns same type

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = F64F64Pair::new(1.0f64, 2.0f64);
        assert_eq!(p.one(), 1.0f64);
        assert_eq!(p.two(), 2.0f64);
    }

    #[test]
    fn test_equals() {
        let p1 = F64F64Pair::new(1.0f64, 2.0f64);
        let p2 = F64F64Pair::new(1.0f64, 2.0f64);
        let p3 = F64F64Pair::new(2.0f64, 1.0f64);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = F64F64Pair::new(1.0f64, 2.0f64);
        assert!(!p.to_string().is_empty());
    }
}
