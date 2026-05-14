// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i32`, `f32`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct I32F32Pair {
    one: i32,
    two: f32,
}

impl I32F32Pair {
    pub fn new(one: i32, two: f32) -> Self {
        I32F32Pair { one, two }
    }
    pub fn one(&self) -> i32 {
        self.one
    }
    pub fn two(&self) -> f32 {
        self.two
    }

    pub fn swap(&self) -> F32I32Pair {
        F32I32Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I32F32Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::f32_i32_pair::F32I32Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = I32F32Pair::new(1, 2.0f32);
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), 2.0f32);
    }

    #[test]
    fn test_equals() {
        let p1 = I32F32Pair::new(1, 2.0f32);
        let p2 = I32F32Pair::new(1, 2.0f32);
        let p3 = I32F32Pair::new(2, 1.0f32);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I32F32Pair::new(1, 2.0f32);
        assert!(!p.to_string().is_empty());
    }
}
