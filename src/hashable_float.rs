// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

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

    // NaN sign/payload ordering is verified natively here (NOT in the shared
    // cross-language suite): in TypeScript all NaN bit patterns are a single
    // ECMAScript language-level NaN, so an f32 NaN's SIGN and PAYLOAD are not
    // cross-language-observable. These are the production-comparator behaviors
    // phase 3 fixed.
    #[test]
    fn neg_nan_sorts_below_neg_infinity() {
        // -NaN (0xffc00000) must sort BELOW -Infinity under total_cmp.
        let neg_nan = HashableF32(f32::from_bits(0xffc0_0000));
        let neg_inf = HashableF32(f32::NEG_INFINITY);
        assert!(neg_nan < neg_inf);
        // And below the most negative finite value.
        let very_neg = HashableF32(f32::MIN);
        assert!(neg_nan < very_neg);
    }

    #[test]
    fn pos_nan_payloads_order_ascending() {
        // Distinct positive NaN payloads order ascending by bit pattern, and
        // both sort ABOVE +Infinity (the top of the total order).
        let p0 = HashableF32(f32::from_bits(0x7fc0_0000));
        let p1 = HashableF32(f32::from_bits(0x7fc0_0001));
        let pos_inf = HashableF32(f32::INFINITY);
        assert!(p0 < p1);
        assert!(pos_inf < p0);
        assert!(pos_inf < p1);
    }

    #[test]
    fn f32_total_order_full_chain() {
        // -NaN < -Inf < -finite < -0.0 < +0.0 < +finite < +Inf < +NaN
        let chain = [
            HashableF32(f32::from_bits(0xffc0_0000)), // -NaN
            HashableF32(f32::NEG_INFINITY),
            HashableF32(-3.0),
            HashableF32(-0.0),
            HashableF32(0.0),
            HashableF32(2.0),
            HashableF32(f32::INFINITY),
            HashableF32(f32::from_bits(0x7fc0_0000)), // +NaN
            HashableF32(f32::from_bits(0x7fc0_0001)), // +NaN, larger payload
        ];
        for w in chain.windows(2) {
            assert!(w[0] < w[1], "expected {:?} < {:?}", w[0], w[1]);
        }
    }
}
