// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`bool`, `i32`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoolI32Pair {
    one: bool,
    two: i32,
}

impl BoolI32Pair {
    pub fn new(one: bool, two: i32) -> Self {
        BoolI32Pair { one, two }
    }
    pub fn one(&self) -> bool {
        self.one
    }
    pub fn two(&self) -> i32 {
        self.two
    }

    pub fn swap(&self) -> I32BoolPair {
        I32BoolPair::new(self.two, self.one)
    }
}

impl fmt::Display for BoolI32Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::i32_bool_pair::I32BoolPair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = BoolI32Pair::new(true, 2);
        assert_eq!(p.one(), true);
        assert_eq!(p.two(), 2);
    }

    #[test]
    fn test_equals() {
        let p1 = BoolI32Pair::new(true, 2);
        let p2 = BoolI32Pair::new(true, 2);
        let p3 = BoolI32Pair::new(false, 1);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = BoolI32Pair::new(true, 2);
        assert!(!p.to_string().is_empty());
    }
}
