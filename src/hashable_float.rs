// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.

//! Newtype wrappers for `f32`/`f64` that implement `Hash + Eq + Ord` via
//! bit-pattern semantics (Java `Float.floatToIntBits` / Go `math.Float32bits`).
//!
//! - `Hash`/`Eq` use the raw IEEE-754 bit pattern, so NaN keys are findable
//!   (NaN-of-same-bits == NaN-of-same-bits) and `+0.0` is distinct from `-0.0`.
//! - `Ord` uses `total_cmp` (IEEE total ordering), which orders NaNs at the
//!   extremes and is total even in the presence of NaN.
//!
//! Both wrappers are `#[repr(transparent)]`, so they have identical memory
//! layout to the wrapped primitive — no runtime cost for the wrapping.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, Default)]
#[repr(transparent)]
pub struct HashableF32(pub f32);

impl PartialEq for HashableF32 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for HashableF32 {}

impl Hash for HashableF32 {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for HashableF32 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HashableF32 {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl std::fmt::Display for HashableF32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<f32> for HashableF32 {
    #[inline]
    fn from(v: f32) -> Self {
        HashableF32(v)
    }
}

impl From<HashableF32> for f32 {
    #[inline]
    fn from(v: HashableF32) -> Self {
        v.0
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(transparent)]
pub struct HashableF64(pub f64);

impl PartialEq for HashableF64 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for HashableF64 {}

impl Hash for HashableF64 {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for HashableF64 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HashableF64 {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl std::fmt::Display for HashableF64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<f64> for HashableF64 {
    #[inline]
    fn from(v: f64) -> Self {
        HashableF64(v)
    }
}

impl From<HashableF64> for f64 {
    #[inline]
    fn from(v: HashableF64) -> Self {
        v.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repr_transparent_size() {
        assert_eq!(
            std::mem::size_of::<HashableF32>(),
            std::mem::size_of::<f32>()
        );
        assert_eq!(
            std::mem::size_of::<HashableF64>(),
            std::mem::size_of::<f64>()
        );
    }

    #[test]
    fn nan_eq_via_bits() {
        let n1 = HashableF32(f32::NAN);
        let n2 = HashableF32(f32::NAN);
        // Same bit pattern → equal under to_bits()-based Eq.
        assert_eq!(n1, n2);
    }

    #[test]
    fn nan_payloads_distinct() {
        let n1 = HashableF32(f32::from_bits(0x7fc0_0001));
        let n2 = HashableF32(f32::from_bits(0x7fc0_0002));
        assert_ne!(n1, n2);
    }

    #[test]
    fn signed_zero_distinct() {
        let pos = HashableF64(0.0_f64);
        let neg = HashableF64(-0.0_f64);
        assert_ne!(pos, neg);
    }

    #[test]
    fn ord_handles_nan() {
        let a = HashableF64(1.0);
        let nan = HashableF64(f64::NAN);
        // total_cmp orders NaN positively (above +∞).
        assert!(a < nan);
    }
}
