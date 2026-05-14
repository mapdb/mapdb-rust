// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`bool`, `i16`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoolI16Pair {
    one: bool,
    two: i16,
}

impl BoolI16Pair {
    pub fn new(one: bool, two: i16) -> Self {
        BoolI16Pair { one, two }
    }
    pub fn one(&self) -> bool {
        self.one
    }
    pub fn two(&self) -> i16 {
        self.two
    }

    pub fn swap(&self) -> I16BoolPair {
        I16BoolPair::new(self.two, self.one)
    }
}

impl fmt::Display for BoolI16Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::i16_bool_pair::I16BoolPair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = BoolI16Pair::new(true, 2);
        assert_eq!(p.one(), true);
        assert_eq!(p.two(), 2);
    }

    #[test]
    fn test_equals() {
        let p1 = BoolI16Pair::new(true, 2);
        let p2 = BoolI16Pair::new(true, 2);
        let p3 = BoolI16Pair::new(false, 1);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = BoolI16Pair::new(true, 2);
        assert!(!p.to_string().is_empty());
    }
}
