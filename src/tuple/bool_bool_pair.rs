// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`bool`, `bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoolBoolPair {
    one: bool,
    two: bool,
}

impl BoolBoolPair {
    pub fn new(one: bool, two: bool) -> Self {
        BoolBoolPair { one, two }
    }
    pub fn one(&self) -> bool {
        self.one
    }
    pub fn two(&self) -> bool {
        self.two
    }

    pub fn swap(&self) -> BoolBoolPair {
        BoolBoolPair::new(self.two, self.one)
    }
}

impl fmt::Display for BoolBoolPair {
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
        let p = BoolBoolPair::new(true, false);
        assert_eq!(p.one(), true);
        assert_eq!(p.two(), false);
    }

    #[test]
    fn test_equals() {
        let p1 = BoolBoolPair::new(true, false);
        let p2 = BoolBoolPair::new(true, false);
        let p3 = BoolBoolPair::new(false, true);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = BoolBoolPair::new(true, false);
        assert!(!p.to_string().is_empty());
    }
}
