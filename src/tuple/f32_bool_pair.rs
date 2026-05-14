// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`f32`, `bool`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F32BoolPair {
    one: f32,
    two: bool,
}

impl F32BoolPair {
    pub fn new(one: f32, two: bool) -> Self {
        F32BoolPair { one, two }
    }
    pub fn one(&self) -> f32 {
        self.one
    }
    pub fn two(&self) -> bool {
        self.two
    }

    pub fn swap(&self) -> BoolF32Pair {
        BoolF32Pair::new(self.two, self.one)
    }
}

impl fmt::Display for F32BoolPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::bool_f32_pair::BoolF32Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = F32BoolPair::new(1.0f32, false);
        assert_eq!(p.one(), 1.0f32);
        assert_eq!(p.two(), false);
    }

    #[test]
    fn test_equals() {
        let p1 = F32BoolPair::new(1.0f32, false);
        let p2 = F32BoolPair::new(1.0f32, false);
        let p3 = F32BoolPair::new(2.0f32, true);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = F32BoolPair::new(1.0f32, false);
        assert!(!p.to_string().is_empty());
    }
}
