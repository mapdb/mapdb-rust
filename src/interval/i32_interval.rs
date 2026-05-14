// AUTO-GENERATED. DO NOT EDIT.

use std::fmt;

/// A virtual collection representing a range of `i32` values `[from, to]` with
/// a given step. No elements are materialised in memory — iteration computes
/// values on the fly.
///
/// Equivalent to Java's `IntInterval`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct I32Interval {
    from: i32,
    to: i32,
    step: i32,
}

impl I32Interval {
    /// Creates an interval from `from` to `to` (inclusive) with the given step.
    ///
    /// # Panics
    /// Panics if `step` is zero, or if the step direction doesn't match the
    /// from/to direction (e.g. `from < to` but `step < 0`).
    pub fn from_to_by(from: i32, to: i32, step: i32) -> Self {
        assert!(step != 0, "I32Interval: step must not be zero");
        if from < to {
            assert!(
                step > 0,
                "I32Interval: step must be positive when from < to"
            );
        } else if from > to {
            assert!(
                step < 0,
                "I32Interval: step must be negative when from > to"
            );
        }
        I32Interval { from, to, step }
    }

    /// Creates an interval from `from` to `to` (inclusive) with step 1 or -1.
    pub fn from_to(from: i32, to: i32) -> Self {
        let step: i32 = if from <= to { 1 } else { -1 };
        I32Interval { from, to, step }
    }

    /// Creates an interval from 1 to `to` (inclusive).
    pub fn one_to(to: i32) -> Self {
        Self::from_to(1, to)
    }

    /// Creates an interval from 0 to `to - 1` (inclusive).
    pub fn zero_to(to: i32) -> Self {
        Self::from_to(0, to)
    }

    /// Returns the start of the interval.
    pub fn from(&self) -> i32 {
        self.from
    }

    /// Returns the end of the interval (inclusive).
    pub fn to(&self) -> i32 {
        self.to
    }

    /// Returns the step.
    pub fn step(&self) -> i32 {
        self.step
    }

    /// Returns the number of elements in the interval.
    pub fn len(&self) -> usize {
        if (self.step > 0 && self.from > self.to) || (self.step < 0 && self.from < self.to) {
            return 0;
        }
        (((self.to as i64) - (self.from as i64)) / (self.step as i64) + 1) as usize
    }

    /// Returns true if the interval contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the interval contains the given value.
    pub fn contains(&self, value: i32) -> bool {
        if self.step > 0 {
            value >= self.from && value <= self.to && (value - self.from) % self.step == 0
        } else {
            value <= self.from && value >= self.to && (self.from - value) % (-self.step) == 0
        }
    }

    /// Returns the element at the given index, or None if out of bounds.
    pub fn get(&self, index: usize) -> Option<i32> {
        if index >= self.len() {
            None
        } else {
            Some(self.from + self.step * index as i32)
        }
    }

    /// Returns an iterator over the elements.
    pub fn iter(&self) -> I32IntervalIter {
        I32IntervalIter {
            current: self.from,
            to: self.to,
            step: self.step,
            done: false,
        }
    }

    pub fn to_vec(&self) -> Vec<i32> {
        self.iter().collect()
    }

    /// Returns a new interval with elements in reverse order.
    pub fn reversed(&self) -> Self {
        I32Interval {
            from: self.to,
            to: self.from,
            step: -self.step,
        }
    }
}

/// Iterator for `I32Interval`.
pub struct I32IntervalIter {
    current: i32,
    to: i32,
    step: i32,
    done: bool,
}

impl Iterator for I32IntervalIter {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        if self.done {
            return None;
        }
        if self.step > 0 {
            if self.current > self.to {
                return None;
            }
        } else {
            if self.current < self.to {
                return None;
            }
        }
        let val = self.current;
        // Check for overflow before advancing
        match self.current.checked_add(self.step) {
            Some(next) => self.current = next,
            None => self.done = true,
        }
        Some(val)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.done {
            return (0, Some(0));
        }
        let remaining = if self.step > 0 {
            if self.current > self.to {
                0
            } else {
                ((self.to as i64 - self.current as i64) / self.step as i64 + 1) as usize
            }
        } else {
            if self.current < self.to {
                0
            } else {
                ((self.current as i64 - self.to as i64) / (-self.step as i64) + 1) as usize
            }
        };
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for I32IntervalIter {}

impl fmt::Display for I32Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "I32Interval({}, {}, step {})",
            self.from, self.to, self.step
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_to_ascending() {
        let iv = I32Interval::from_to(1, 5);
        assert_eq!(iv.len(), 5);
        assert_eq!(iv.to_vec(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_from_to_descending() {
        let iv = I32Interval::from_to(5, 1);
        assert_eq!(iv.len(), 5);
        assert_eq!(iv.to_vec(), vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_from_to_by() {
        let iv = I32Interval::from_to_by(0, 10, 2);
        assert_eq!(iv.len(), 6);
        assert_eq!(iv.to_vec(), vec![0, 2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_from_to_by_negative_step() {
        let iv = I32Interval::from_to_by(10, 0, -3);
        assert_eq!(iv.to_vec(), vec![10, 7, 4, 1]);
    }

    #[test]
    fn test_single_element() {
        let iv = I32Interval::from_to(3, 3);
        assert_eq!(iv.len(), 1);
        assert_eq!(iv.to_vec(), vec![3]);
    }

    #[test]
    fn test_contains() {
        let iv = I32Interval::from_to_by(0, 10, 2);
        assert!(iv.contains(0));
        assert!(iv.contains(4));
        assert!(iv.contains(10));
        assert!(!iv.contains(3));
        assert!(!iv.contains(11));
        assert!(!iv.contains(-1));
    }

    #[test]
    fn test_get() {
        let iv = I32Interval::from_to(1, 5);
        assert_eq!(iv.get(0), Some(1));
        assert_eq!(iv.get(4), Some(5));
        assert_eq!(iv.get(5), None);
    }

    #[test]
    fn test_reversed() {
        let iv = I32Interval::from_to(1, 5);
        let rev = iv.reversed();
        assert_eq!(rev.to_vec(), vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_one_to() {
        let iv = I32Interval::one_to(3);
        assert_eq!(iv.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn test_zero_to() {
        let iv = I32Interval::zero_to(3);
        assert_eq!(iv.to_vec(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_is_empty() {
        let iv = I32Interval::from_to(1, 5);
        assert!(!iv.is_empty());
    }

    #[test]
    fn test_display() {
        let iv = I32Interval::from_to(1, 5);
        assert!(iv.to_string().contains("1"));
    }

    #[test]
    #[should_panic]
    fn test_zero_step_panics() {
        I32Interval::from_to_by(1, 5, 0);
    }

    #[test]
    fn test_exact_size_iterator() {
        let iv = I32Interval::from_to_by(0, 10, 2);
        let iter = iv.iter();
        assert_eq!(iter.len(), 6);
    }
}

impl crate::traits::i32_collection::I32Collection for I32Interval {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: i32) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        self.iter()
    }
}
