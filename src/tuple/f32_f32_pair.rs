// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`f32`, `f32`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F32F32Pair {
    one: f32,
    two: f32,
}

impl F32F32Pair {
    pub fn new(one: f32, two: f32) -> Self {
        F32F32Pair { one, two }
    }
    pub fn one(&self) -> f32 {
        self.one
    }
    pub fn two(&self) -> f32 {
        self.two
    }

    pub fn swap(&self) -> F32F32Pair {
        F32F32Pair::new(self.two, self.one)
    }
}

impl fmt::Display for F32F32Pair {
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
        let p = F32F32Pair::new(1.0f32, 2.0f32);
        assert_eq!(p.one(), 1.0f32);
        assert_eq!(p.two(), 2.0f32);
    }

    #[test]
    fn test_equals() {
        let p1 = F32F32Pair::new(1.0f32, 2.0f32);
        let p2 = F32F32Pair::new(1.0f32, 2.0f32);
        let p3 = F32F32Pair::new(2.0f32, 1.0f32);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = F32F32Pair::new(1.0f32, 2.0f32);
        assert!(!p.to_string().is_empty());
    }
}
