// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// Immutable pair of (`char`, `f32`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharF32Pair {
    one: char,
    two: f32,
}

impl CharF32Pair {
    pub fn new(one: char, two: f32) -> Self {
        CharF32Pair { one, two }
    }
    pub fn one(&self) -> char {
        self.one
    }
    pub fn two(&self) -> f32 {
        self.two
    }

    pub fn swap(&self) -> F32CharPair {
        F32CharPair::new(self.two, self.one)
    }
}

impl fmt::Display for CharF32Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.one, self.two)
    }
}

// Re-export the swapped pair type if it's the same file (self-pair)
use super::f32_char_pair::F32CharPair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_two() {
        let p = CharF32Pair::new('a', 2.0f32);
        assert_eq!(p.one(), 'a');
        assert_eq!(p.two(), 2.0f32);
    }

    #[test]
    fn test_equals() {
        let p1 = CharF32Pair::new('a', 2.0f32);
        let p2 = CharF32Pair::new('a', 2.0f32);
        let p3 = CharF32Pair::new('b', 1.0f32);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_display() {
        let p = CharF32Pair::new('a', 2.0f32);
        assert!(!p.to_string().is_empty());
    }
}
