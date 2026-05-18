// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use std::fmt;

/// Sealed trait that describes the signed-integer primitives over which
/// [`Interval`] is defined: `i8`, `i16`, `i32`, `i64`. The trait is
/// kept in-crate so the library carries no external numeric-trait
/// dependency (`num_traits` would pull a transitive dependency for two
/// methods).
///
/// All required methods are infallible widenings to / narrowings from
/// `i64`. The widening direction is exact for every implementor; the
/// narrowing direction is only ever invoked with values that the
/// algorithm has already proven to fit, per `algorithms.md` §"Interval
/// over signed integers".
pub trait SignedPrimInt:
    Copy + Eq + Ord + fmt::Display + fmt::Debug + private::Sealed + 'static
{
    const MIN_I64: i64;
    fn to_i64(self) -> i64;
    fn from_i64_truncate(v: i64) -> Self;
}

mod private {
    pub trait Sealed {}
    impl Sealed for i8 {}
    impl Sealed for i16 {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
}

macro_rules! impl_signed_prim_int {
    ($($t:ty),*) => {
        $(
            impl SignedPrimInt for $t {
                const MIN_I64: i64 = <$t>::MIN as i64;
                #[inline]
                fn to_i64(self) -> i64 { self as i64 }
                #[inline]
                fn from_i64_truncate(v: i64) -> Self { v as $t }
            }
        )*
    };
}

impl_signed_prim_int!(i8, i16, i32, i64);

/// Virtual `[from, to]` range with the given step. No elements are
/// materialised in memory; iteration produces them on demand.
///
/// The arithmetic exactly matches the cross-language canon from
/// `algorithms.md` §"Interval over signed integers":
///
/// - `size()` computes `distance() / abs_step() + 1` in `u64`, capping
///   at `usize::MAX` if the count would otherwise wrap.
/// - `contains` casts through `i64` first so the unsigned subtraction
///   preserves sign before wrapping.
/// - `get(i)` widens to `i64` for `from + step * i` to dodge overflow
///   at the boundary types (`i8`/`i16`/`i32`).
/// - `all()` iterates by index and calls `get`, never `current += step`
///   (which can wrap at the last step).
/// - `reversed()` **panics** for `T::MIN` step: negating the minimum
///   signed value is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval<T: SignedPrimInt> {
    from: T,
    to: T,
    step: T,
}

impl<T: SignedPrimInt> Interval<T> {
    /// `[from, to]` with the given non-zero step. Panics if `step` is
    /// zero, or if the step direction disagrees with the from/to
    /// direction.
    pub fn from_to_by(from: T, to: T, step: T) -> Self {
        let zero = T::from_i64_truncate(0);
        if step == zero {
            panic!("Interval: step must not be zero");
        }
        if from < to && step < zero {
            panic!("Interval: step must be positive when from < to");
        }
        if from > to && step > zero {
            panic!("Interval: step must be negative when from > to");
        }
        Interval { from, to, step }
    }

    /// `[from, to]` with step 1 (ascending) or -1 (descending).
    pub fn from_to(from: T, to: T) -> Self {
        let step = if from > to {
            T::from_i64_truncate(-1)
        } else {
            T::from_i64_truncate(1)
        };
        Interval { from, to, step }
    }

    /// `[1, to]`.
    pub fn one_to(to: T) -> Self {
        Self::from_to(T::from_i64_truncate(1), to)
    }

    /// `[0, to]`.
    pub fn zero_to(to: T) -> Self {
        Self::from_to(T::from_i64_truncate(0), to)
    }

    pub fn from(&self) -> T {
        self.from
    }
    pub fn to(&self) -> T {
        self.to
    }
    pub fn step(&self) -> T {
        self.step
    }

    /// `|step|` as `u64`. `i64::unsigned_abs` handles `step == i64::MIN`
    /// correctly (returns `2^63`), so this is safe for every signed
    /// primitive once widened.
    fn abs_step(&self) -> u64 {
        self.step.to_i64().unsigned_abs()
    }

    /// Number of `step` units between `from` and `to` as `u64`. Casts
    /// through `i64` first so the subtraction preserves sign even when
    /// `from` and `to` span the full signed range.
    fn distance(&self) -> u64 {
        let zero = T::from_i64_truncate(0);
        if self.step > zero {
            (self.to.to_i64() as u64).wrapping_sub(self.from.to_i64() as u64)
        } else {
            (self.from.to_i64() as u64).wrapping_sub(self.to.to_i64() as u64)
        }
    }

    pub fn size(&self) -> usize {
        let zero = T::from_i64_truncate(0);
        if (self.step > zero && self.from > self.to) || (self.step < zero && self.from < self.to) {
            return 0;
        }
        // `distance / abs_step` is at most `u64::MAX` (full i64 range
        // with step 1), so the `+1` must be checked. On overflow the
        // count is at least `2^64`, which is well above any `usize`,
        // so cap straight at `usize::MAX`.
        let quot = self.distance() / self.abs_step();
        let count = match quot.checked_add(1) {
            Some(c) => c,
            None => return usize::MAX,
        };
        if count > usize::MAX as u64 {
            usize::MAX
        } else {
            count as usize
        }
    }

    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    pub fn contains(&self, value: T) -> bool {
        let zero = T::from_i64_truncate(0);
        if self.step > zero {
            if value < self.from || value > self.to {
                return false;
            }
            let diff = (value.to_i64() as u64).wrapping_sub(self.from.to_i64() as u64);
            diff % self.abs_step() == 0
        } else {
            if value > self.from || value < self.to {
                return false;
            }
            let diff = (self.from.to_i64() as u64).wrapping_sub(value.to_i64() as u64);
            diff % self.abs_step() == 0
        }
    }

    /// `from + step * index`, computed in `i64` to dodge overflow at
    /// the boundary types. Returns `None` for out-of-range indices.
    pub fn get(&self, index: usize) -> Option<T> {
        if index >= self.size() {
            return None;
        }
        let v = self.from.to_i64() + self.step.to_i64() * (index as i64);
        Some(T::from_i64_truncate(v))
    }

    /// Iterates every element in interval order, by index (never by
    /// `current += step`).
    pub fn all(&self) -> IntervalIter<'_, T> {
        IntervalIter {
            interval: self,
            index: 0,
            size: self.size(),
        }
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.all().collect()
    }

    pub fn for_each<F: FnMut(T)>(&self, mut f: F) {
        for v in self.all() {
            f(v);
        }
    }

    pub fn any_satisfy<F: FnMut(T) -> bool>(&self, p: F) -> bool {
        self.all().any(p)
    }

    pub fn all_satisfy<F: FnMut(T) -> bool>(&self, p: F) -> bool {
        self.all().all(p)
    }

    pub fn none_satisfy<F: FnMut(T) -> bool>(&self, p: F) -> bool {
        !self.any_satisfy(p)
    }

    /// `[to, from]` with `-step`. **Panics** if `step == T::MIN` —
    /// negating the minimum signed value is unrepresentable on every
    /// architecture we target. See `algorithms.md` §"Reversed() panics
    /// at minimum step".
    pub fn reversed(&self) -> Self {
        if self.step.to_i64() == T::MIN_I64 {
            panic!("Interval: cannot reverse interval with minimum step");
        }
        let neg_step = T::from_i64_truncate(-self.step.to_i64());
        Interval {
            from: self.to,
            to: self.from,
            step: neg_step,
        }
    }
}

pub struct IntervalIter<'a, T: SignedPrimInt> {
    interval: &'a Interval<T>,
    index: usize,
    size: usize,
}

impl<'a, T: SignedPrimInt> Iterator for IntervalIter<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.index >= self.size {
            return None;
        }
        let v = self.interval.get(self.index);
        self.index += 1;
        v
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.size - self.index;
        (remaining, Some(remaining))
    }
}

impl<T: SignedPrimInt> fmt::Display for Interval<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.size() == 0 {
            return write!(f, "[]");
        }
        write!(f, "[")?;
        let mut first = true;
        for v in self.all() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", v)?;
            first = false;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_to_ascending() {
        let iv: Interval<i32> = Interval::from_to(1, 5);
        assert_eq!(iv.size(), 5);
        assert_eq!(iv.to_vec(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn from_to_descending() {
        let iv: Interval<i32> = Interval::from_to(5, 1);
        assert_eq!(iv.size(), 5);
        assert_eq!(iv.to_vec(), vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn from_to_by_positive_step() {
        let iv: Interval<i32> = Interval::from_to_by(0, 10, 2);
        assert_eq!(iv.size(), 6);
        assert_eq!(iv.to_vec(), vec![0, 2, 4, 6, 8, 10]);
    }

    #[test]
    fn from_to_by_negative_step() {
        let iv: Interval<i32> = Interval::from_to_by(10, 0, -2);
        assert_eq!(iv.size(), 6);
        assert_eq!(iv.to_vec(), vec![10, 8, 6, 4, 2, 0]);
    }

    #[test]
    fn one_to_and_zero_to() {
        let a: Interval<i32> = Interval::one_to(3);
        assert_eq!(a.to_vec(), vec![1, 2, 3]);
        let b: Interval<i32> = Interval::zero_to(3);
        assert_eq!(b.to_vec(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn contains_basic() {
        let iv: Interval<i32> = Interval::from_to_by(0, 10, 3);
        assert!(iv.contains(0));
        assert!(iv.contains(3));
        assert!(iv.contains(9));
        assert!(!iv.contains(10)); // 10 isn't reachable from 0 with step 3
        assert!(!iv.contains(-3));
        assert!(!iv.contains(2));

        let desc: Interval<i32> = Interval::from_to_by(10, 0, -3);
        assert!(desc.contains(10));
        assert!(desc.contains(7));
        assert!(desc.contains(1));
        assert!(!desc.contains(0));
        assert!(!desc.contains(11));
    }

    #[test]
    fn boundary_does_not_wrap_i8() {
        // algorithms.md test: the interval [126, 127] of i8 must not
        // overflow size/contains/get. 127 - 126 = 1 fits in i8, but the
        // uint64 arithmetic guards against the more delicate cases.
        let iv: Interval<i8> = Interval::from_to(126, 127);
        assert_eq!(iv.size(), 2);
        assert_eq!(iv.to_vec(), vec![126_i8, 127_i8]);
        assert!(iv.contains(126));
        assert!(iv.contains(127));
        assert!(!iv.contains(125));

        // Full-range i8 ascending: from_to(MIN, MAX) is 256 elements.
        let full: Interval<i8> = Interval::from_to(i8::MIN, i8::MAX);
        assert_eq!(full.size(), 256);
        assert_eq!(full.get(0), Some(i8::MIN));
        assert_eq!(full.get(255), Some(i8::MAX));
        assert!(full.contains(0));
        assert!(full.contains(i8::MIN));
        assert!(full.contains(i8::MAX));
    }

    #[test]
    fn full_range_i64_does_not_panic() {
        let iv: Interval<i64> = Interval::from_to(i64::MIN, i64::MAX);
        // Cap rule: size would be 2^64, can't fit in usize on any
        // architecture, so clamps to usize::MAX.
        assert_eq!(iv.size(), usize::MAX);
        assert_eq!(iv.get(0), Some(i64::MIN));
        // Index 1 widens to i64 and adds step=1, giving i64::MIN+1.
        assert_eq!(iv.get(1), Some(i64::MIN + 1));
    }

    #[test]
    fn get_out_of_bounds() {
        let iv: Interval<i32> = Interval::from_to(1, 3);
        assert_eq!(iv.get(0), Some(1));
        assert_eq!(iv.get(2), Some(3));
        assert_eq!(iv.get(3), None);
    }

    #[test]
    fn reversed_basic() {
        let iv: Interval<i32> = Interval::from_to_by(0, 10, 2);
        let r = iv.reversed();
        assert_eq!(r.from(), 10);
        assert_eq!(r.to(), 0);
        assert_eq!(r.step(), -2);
        assert_eq!(r.to_vec(), vec![10, 8, 6, 4, 2, 0]);
    }

    #[test]
    #[should_panic(expected = "minimum step")]
    fn reversed_minimum_step_panics_i8() {
        let iv: Interval<i8> = Interval::from_to_by(0, -1, i8::MIN);
        let _ = iv.reversed();
    }

    #[test]
    #[should_panic(expected = "minimum step")]
    fn reversed_minimum_step_panics_i64() {
        // For i64 the step has to be at least negative; from > to,
        // so step < 0 is required. step = i64::MIN satisfies that.
        let iv: Interval<i64> = Interval::from_to_by(0, -1, i64::MIN);
        let _ = iv.reversed();
    }

    #[test]
    #[should_panic(expected = "step must not be zero")]
    fn zero_step_panics() {
        let _: Interval<i32> = Interval::from_to_by(0, 10, 0);
    }

    #[test]
    fn empty_singleton_display() {
        let single: Interval<i32> = Interval::from_to(7, 7);
        assert_eq!(single.size(), 1);
        assert_eq!(single.to_vec(), vec![7]);
        assert_eq!(format!("{}", single), "[7]");

        let multi: Interval<i32> = Interval::from_to(1, 3);
        assert_eq!(format!("{}", multi), "[1, 2, 3]");
    }

    #[test]
    fn predicates() {
        let iv: Interval<i32> = Interval::from_to(1, 5);
        assert!(iv.any_satisfy(|v| v == 3));
        assert!(!iv.any_satisfy(|v| v == 99));
        assert!(iv.all_satisfy(|v| v >= 1));
        assert!(!iv.all_satisfy(|v| v > 1));
        assert!(iv.none_satisfy(|v| v < 0));
    }

    #[test]
    fn iter_size_hint() {
        let iv: Interval<i32> = Interval::from_to(1, 10);
        let it = iv.all();
        assert_eq!(it.size_hint(), (10, Some(10)));
    }
}
