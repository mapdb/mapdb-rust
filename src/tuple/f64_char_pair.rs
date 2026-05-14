// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`f64`, `char`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F64CharPair {
    one: f64,
    two: char,
}

impl F64CharPair {
    pub fn new(one: f64, two: char) -> Self {
        F64CharPair { one, two }
    }
    pub fn one(&self) -> f64 {
        self.one
    }
    pub fn two(&self) -> char {
        self.two
    }

    pub fn swap(&self) -> CharF64Pair {
        CharF64Pair::new(self.two, self.one)
    }
}

impl fmt::Display for F64CharPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::char_f64_pair::CharF64Pair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = F64CharPair::new(1.0f64, 'b');
        assert_eq!(p.one(), 1.0f64);
        assert_eq!(p.two(), 'b');
    }

    #[test]
    fn test_equals() {
        let p1 = F64CharPair::new(1.0f64, 'b');
        let p2 = F64CharPair::new(1.0f64, 'b');
        let p3 = F64CharPair::new(2.0f64, 'a');
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = F64CharPair::new(1.0f64, 'b');
        assert!(!p.to_string().is_empty());
    }
}
