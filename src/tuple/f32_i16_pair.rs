// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`f32`, `i16`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F32I16Pair {
    one: f32,
    two: i16,
}

impl F32I16Pair {
    pub fn new(one: f32, two: i16) -> Self {
        F32I16Pair { one, two }
    }
    pub fn one(&self) -> f32 {
        self.one
    }
    pub fn two(&self) -> i16 {
        self.two
    }

    pub fn swap(&self) -> I16F32Pair {
        I16F32Pair::new(self.two, self.one)
    }
}

impl fmt::Display for F32I16Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::i16_f32_pair::I16F32Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = F32I16Pair::new(1.0f32, 2);
        assert_eq!(p.one(), 1.0f32);
        assert_eq!(p.two(), 2);
    }

    #[test]
    fn test_equals() {
        let p1 = F32I16Pair::new(1.0f32, 2);
        let p2 = F32I16Pair::new(1.0f32, 2);
        let p3 = F32I16Pair::new(2.0f32, 1);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = F32I16Pair::new(1.0f32, 2);
        assert!(!p.to_string().is_empty());
    }
}
