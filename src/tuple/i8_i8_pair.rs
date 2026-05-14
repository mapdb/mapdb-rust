// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`i8`, `i8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I8I8Pair {
    one: i8,
    two: i8,
}

impl I8I8Pair {
    pub fn new(one: i8, two: i8) -> Self {
        I8I8Pair { one, two }
    }
    pub fn one(&self) -> i8 {
        self.one
    }
    pub fn two(&self) -> i8 {
        self.two
    }

    pub fn swap(&self) -> I8I8Pair {
        I8I8Pair::new(self.two, self.one)
    }
}

impl fmt::Display for I8I8Pair {
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
        let p = I8I8Pair::new(1, 2);
        assert_eq!(p.one(), 1);
        assert_eq!(p.two(), 2);
    }

    #[test]
    fn test_equals() {
        let p1 = I8I8Pair::new(1, 2);
        let p2 = I8I8Pair::new(1, 2);
        let p3 = I8I8Pair::new(2, 1);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = I8I8Pair::new(1, 2);
        assert!(!p.to_string().is_empty());
    }
}
