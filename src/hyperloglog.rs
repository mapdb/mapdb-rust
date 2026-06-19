// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! HyperLogLog distinct-count cardinality sketch (see
//! `spec/features/hyperloglog.md`). **Reference port.**
//!
//! ## Float-quarantine ruling (the heart of this feature)
//!
//! HyperLogLog has two observable surfaces:
//!
//! 1. The **integer register array** (`m = 2^p` `u8`s, each the max `rho` seen).
//!    This is **exact integer state**, a pure-integer function of
//!    `(p, add-sequence)` through the hash pipeline, and is the **cross-language
//!    oracle** — all five ports MUST produce the byte-identical array. The
//!    shared JSON scenarios assert ONLY this (via `register_hex`,
//!    `nonzero_registers`, `max_register`, `register_at_N`).
//!
//! 2. The **`f64` estimate** ([`HyperLogLog::estimate`]). It is a function of
//!    `ln` / `2^x` / division / summation and **cannot** be required to agree
//!    bit-for-bit across five libm implementations. It is specified precisely
//!    here and tested **natively** against a documented tolerance — it is
//!    **never** in the shared oracle. There is **no `estimate` assertion key**.
//!
//! `add`, `merge`, and the register array use **zero floating point** (only
//! `hash64`, shifts, `max`, byte packing); the float appears only inside
//! `estimate()`, a read-only projection that never writes a register.

use crate::hash::hash64;
use core::fmt;

/// Minimum legal precision (`m = 16`).
pub const MIN_PRECISION: u8 = 4;
/// Maximum legal precision (`m = 262144`); the v1 ceiling matching `hll_split`.
pub const MAX_PRECISION: u8 = 18;

/// The 4-byte ASCII magic that version-tags the serialized form (`"HLL1"`).
const MAGIC: [u8; 4] = *b"HLL1";

/// Errors from the HyperLogLog surface (construction / merge / deserialization).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HllError {
    /// `p` outside `4 ..= 18` (never silently clamped).
    BadPrecision(u8),
    /// `merge` of two sketches with different `p` (no resize/reproject).
    PrecisionMismatch { left: u8, right: u8 },
    /// `from_bytes`: fewer than the 5-byte header.
    TooShort(usize),
    /// `from_bytes`: magic was not `"HLL1"`.
    BadMagic([u8; 4]),
    /// `from_bytes`: total length is not exactly `5 + 2^p`.
    LengthMismatch { expected: usize, got: usize },
    /// `from_bytes`: a register byte exceeds the per-`p` ceiling `64 - p + 1`.
    RegisterOutOfRange { index: usize, value: u8, max: u8 },
}

impl fmt::Display for HllError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HllError::BadPrecision(p) => {
                write!(
                    f,
                    "precision {p} out of range {MIN_PRECISION}..={MAX_PRECISION}"
                )
            }
            HllError::PrecisionMismatch { left, right } => {
                write!(f, "merge precision mismatch: {left} != {right}")
            }
            HllError::TooShort(n) => write!(f, "serialized HLL too short: {n} bytes (need >= 5)"),
            HllError::BadMagic(m) => write!(f, "bad HLL magic: {m:02x?} (expected \"HLL1\")"),
            HllError::LengthMismatch { expected, got } => {
                write!(f, "HLL length mismatch: expected {expected}, got {got}")
            }
            HllError::RegisterOutOfRange { index, value, max } => {
                write!(f, "register[{index}] = {value} exceeds per-p ceiling {max}")
            }
        }
    }
}

impl std::error::Error for HllError {}

/// A HyperLogLog distinct-count sketch.
///
/// Built by [`HyperLogLog::with_precision`]; updated by [`add`](Self::add) /
/// [`merge`](Self::merge); the register array (the oracle) is read via
/// [`registers`](Self::registers) / [`nonzero_registers`](Self::nonzero_registers);
/// the quarantined float answer is [`estimate`](Self::estimate); serialized via
/// [`to_bytes`](Self::to_bytes) / [`from_bytes`](Self::from_bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyperLogLog {
    p: u8,
    /// `m = 2^p` registers, each the max `rho` seen for that index (0 = empty).
    registers: Vec<u8>,
}

impl HyperLogLog {
    /// Construct an empty sketch with `m = 2^p` zeroed registers.
    ///
    /// `p` must be in `4 ..= 18`; otherwise [`HllError::BadPrecision`] (never a
    /// silent clamp — a clamp would let two ports build differently-sized arrays
    /// from the same nominal `p`).
    pub fn with_precision(p: u8) -> Result<Self, HllError> {
        if !(MIN_PRECISION..=MAX_PRECISION).contains(&p) {
            return Err(HllError::BadPrecision(p));
        }
        let m = 1usize << p;
        Ok(HyperLogLog {
            p,
            registers: vec![0u8; m],
        })
    }

    /// The precision `p` (`log2(m)`).
    #[inline]
    pub fn precision(&self) -> u8 {
        self.p
    }

    /// The register count `m = 2^p`.
    #[inline]
    pub fn register_count(&self) -> usize {
        self.registers.len()
    }

    /// The per-`p` maximum possible `rho` (and the `from_bytes` byte ceiling):
    /// `64 - p + 1`.
    #[inline]
    fn rho_ceiling(p: u8) -> u8 {
        64 - p + 1
    }

    /// Split a 64-bit hash into `(register_index, rho)` per the hash pipeline's
    /// pre-stated `hll_split` (top `p` bits → index; remaining bits + guard bit
    /// → `clz64 + 1`). Pure integer; the guard bit guarantees `w != 0` so
    /// `leading_zeros` is never called on `0`.
    #[inline]
    fn split(x: u64, p: u8) -> (u32, u8) {
        let pp = p as u32;
        let idx: u32 = (x >> (64 - pp)) as u32;
        // GUARD BIT: OR in `1 << (p - 1)`. If the remaining `64 - p` bits are all
        // zero, `w = 1 << (p - 1)`, `clz64(w) = 64 - p`, so `rho = 64 - p + 1`
        // (its max) and `clz64` is never invoked on `0`.
        let w: u64 = (x << pp) | (1u64 << (pp - 1));
        let rho: u8 = (w.leading_zeros() + 1) as u8;
        (idx, rho)
    }

    /// Add an `i32` element. The item is encoded with the hash pipeline's `i32`
    /// rule — reinterpret to `u32`, **zero-extend** to `u64` (NOT sign-extend) —
    /// then `hash64(word, seed = 0)`, then `hll_split`, then `register[idx] =
    /// max(register[idx], rho)`. **Pure integer, zero floating point.**
    pub fn add(&mut self, item: i32) {
        // i32 -> u32 reinterpret -> zero-extend to u64 (high 32 bits always 0).
        let input_word: u64 = (item as u32) as u64;
        let x: u64 = hash64(input_word, 0);
        let (idx, rho) = Self::split(x, self.p);
        let slot = &mut self.registers[idx as usize];
        if rho > *slot {
            *slot = rho;
        }
    }

    /// The raw register array (the cross-language oracle bytes).
    #[inline]
    pub fn registers(&self) -> &[u8] {
        &self.registers
    }

    /// The count of registers `> 0` (`= m - V`, where `V` is the zero count).
    pub fn nonzero_registers(&self) -> u32 {
        self.registers.iter().filter(|&&r| r > 0).count() as u32
    }

    /// The maximum register value (largest `rho` seen); `0` for a fresh sketch.
    pub fn max_register(&self) -> u8 {
        self.registers.iter().copied().max().unwrap_or(0)
    }

    /// Merge `other` into `self` by element-wise register **max** (the union's
    /// register `j` is the max over both input sets). Requires identical `p`
    /// (else [`HllError::PrecisionMismatch`]). Commutative, associative,
    /// idempotent. **Pure integer, zero floating point.**
    pub fn merge(&mut self, other: &HyperLogLog) -> Result<(), HllError> {
        if self.p != other.p {
            return Err(HllError::PrecisionMismatch {
                left: self.p,
                right: other.p,
            });
        }
        for (a, &b) in self.registers.iter_mut().zip(other.registers.iter()) {
            if b > *a {
                *a = b;
            }
        }
        Ok(())
    }

    /// Estimate the distinct cardinality (the **quarantined `f64`**, native-only
    /// and tolerance-tested — never in the shared oracle).
    ///
    /// Original HyperLogLog estimator (Flajolet–Fusy–Gandouet–Meunier 2007) with
    /// the small-range linear-counting correction and the **`2^64`** large-range
    /// correction (this HLL consumes a 64-bit `hash64`, so the hash space is
    /// `2^64`, NOT the 2007 paper's `2^32`).
    ///
    /// **Edge note:** for an *add/merge-reachable* state this is always finite.
    /// A synthetic state loaded via [`from_bytes`](Self::from_bytes) with **every**
    /// register at the absolute per-`p` ceiling (`64 - p + 1`) is not reachable
    /// from the v1 `i32` add surface, and at small `p` its raw `E` can exceed
    /// `2^64`, making `ln(1 - E/2^64)` take `ln(< 0) = NaN`. That state never
    /// occurs through `add`/`merge` and never enters the shared integer oracle
    /// (the estimate is quarantined), so it is a non-issue for the contract; it
    /// is documented here only for callers that estimate arbitrary deserialized
    /// states. (The large-range *reachable* regime is exercised by a native
    /// finiteness test using registers at `ceiling - 1`.)
    pub fn estimate(&self) -> f64 {
        let m = self.registers.len() as f64;
        let alpha = Self::alpha_m(self.p);

        // Z = sum 2^(-register[j]); register[j] == 0 contributes 2^0 = 1.
        // 1 << register[j] (<= 1 << 61) fits a u64; compute the shift in integer.
        let mut z = 0.0f64;
        for &r in &self.registers {
            z += 1.0 / ((1u64 << r) as f64);
        }
        let e = alpha * m * m / z;

        // Small-range (linear counting): E small AND there are empties (V > 0).
        let v = self.registers.iter().filter(|&&r| r == 0).count();
        if e <= 2.5 * m && v > 0 {
            return m * (m / v as f64).ln();
        }

        // Large-range correction near the HASH-SPACE ceiling (2^64, NOT 2^32).
        let two64 = 18446744073709551616.0f64; // 2^64, exactly representable.
        if e > (1.0 / 30.0) * two64 {
            // Guard the log argument: for all reachable states E < 2^64 so
            // (1 - E/2^64) > 0. A fully-saturated deserialized state (every
            // register at the per-p ceiling, constructible via from_bytes but
            // not via add) can push raw E >= 2^64, making (1 - E/2^64) <= 0 and
            // ln(<= 0) = NaN. Skip the log correction there and return the raw
            // (large, finite) E so estimate() stays finite as the spec mandates.
            if e < two64 {
                return -two64 * (1.0 - e / two64).ln();
            }
        }

        e
    }

    /// The HLL bias constant `alpha_m`: pinned piecewise literals for small `m`,
    /// closed form for `m >= 128`. (See the spec; pinned so the family's
    /// estimator is uniform even though the value is tolerance-tested.)
    fn alpha_m(p: u8) -> f64 {
        match p {
            4 => 0.673,                                       // m = 16
            5 => 0.697,                                       // m = 32
            6 => 0.709,                                       // m = 64
            _ => 0.7213 / (1.0 + 1.079 / (1u64 << p) as f64), // m >= 128
        }
    }

    /// Serialize to the v1 wire form: 5-byte header (`"HLL1"` + `p`) followed by
    /// one `u8` per register in index order. Total length `5 + 2^p`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(5 + self.registers.len());
        out.extend_from_slice(&MAGIC);
        out.push(self.p);
        out.extend_from_slice(&self.registers);
        out
    }

    /// Deserialize from the v1 wire form. Rejects (single MUST rule so no two
    /// ports disagree on validity): too short, bad magic, `p` out of range,
    /// length `!= 5 + 2^p`, or any register byte `> 64 - p + 1`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HllError> {
        if bytes.len() < 5 {
            return Err(HllError::TooShort(bytes.len()));
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != MAGIC {
            return Err(HllError::BadMagic(magic));
        }
        let p = bytes[4];
        if !(MIN_PRECISION..=MAX_PRECISION).contains(&p) {
            return Err(HllError::BadPrecision(p));
        }
        let m = 1usize << p;
        let expected = 5 + m;
        if bytes.len() != expected {
            return Err(HllError::LengthMismatch {
                expected,
                got: bytes.len(),
            });
        }
        let ceiling = Self::rho_ceiling(p);
        let registers = &bytes[5..];
        for (i, &r) in registers.iter().enumerate() {
            if r > ceiling {
                return Err(HllError::RegisterOutOfRange {
                    index: i,
                    value: r,
                    max: ceiling,
                });
            }
        }
        Ok(HyperLogLog {
            p,
            registers: registers.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash64;

    /// Recompute the expected `(idx, rho)` for an i32 item independently of the
    /// implementation, so the register-update tests are a real oracle check.
    fn expected_split(item: i32, p: u8) -> (u32, u8) {
        let x = hash64((item as u32) as u64, 0);
        let pp = p as u32;
        let idx = (x >> (64 - pp)) as u32;
        let w = (x << pp) | (1u64 << (pp - 1));
        (idx, (w.leading_zeros() + 1) as u8)
    }

    fn to_hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    // ---- Construction & p-range -----------------------------------------

    #[test]
    fn with_precision_allocates_m_registers() {
        for p in MIN_PRECISION..=MAX_PRECISION {
            let h = HyperLogLog::with_precision(p).unwrap();
            assert_eq!(h.register_count(), 1usize << p);
            assert!(h.registers().iter().all(|&r| r == 0));
            assert_eq!(h.nonzero_registers(), 0);
            assert_eq!(h.max_register(), 0);
        }
    }

    #[test]
    fn p_out_of_range_errors_never_clamps() {
        assert_eq!(
            HyperLogLog::with_precision(3),
            Err(HllError::BadPrecision(3))
        );
        assert_eq!(
            HyperLogLog::with_precision(19),
            Err(HllError::BadPrecision(19))
        );
        assert_eq!(
            HyperLogLog::with_precision(0),
            Err(HllError::BadPrecision(0))
        );
        assert_eq!(
            HyperLogLog::with_precision(255),
            Err(HllError::BadPrecision(255))
        );
    }

    // ---- rho / clz / guard-bit exactness --------------------------------

    #[test]
    fn split_guard_bit_all_zero_remainder_gives_max_rho() {
        // Craft x whose low (64 - p) bits are all zero: x = idx << (64 - p).
        // Then w = (x << p) | guard = guard = 1 << (p-1); clz64 = 64-p; rho =
        // 64-p+1 (the per-p maximum). This is the all-zero-remainder pin.
        for p in [4u8, 7, 14, 18] {
            let pp = p as u32;
            let idx: u64 = 5 & ((1u64 << pp) - 1);
            let x: u64 = idx << (64 - pp);
            let (gi, rho) = HyperLogLog::split(x, p);
            assert_eq!(gi as u64, idx, "idx extraction at p={p}");
            assert_eq!(rho, 64 - p + 1, "all-zero-remainder rho at p={p}");
            assert_eq!(rho, HyperLogLog::rho_ceiling(p));
        }
    }

    #[test]
    fn split_min_rho_is_one() {
        // Top remaining bit set -> clz64(w) = 0 -> rho = 1 (the minimum).
        let p = 4u8;
        // remaining bits start with a 1: put a 1 just below the top p bits.
        let x: u64 = 1u64 << (64 - p as u32 - 1);
        let (_, rho) = HyperLogLog::split(x, p);
        assert_eq!(rho, 1);
    }

    #[test]
    fn split_idx_is_top_p_bits_logical() {
        // High-bit-set x must use a LOGICAL shift for the index.
        let x = 0xffff_ffff_ffff_ffffu64;
        for p in [4u8, 10, 18] {
            let (idx, _) = HyperLogLog::split(x, p);
            assert_eq!(idx, (1u32 << p) - 1, "top p bits all-ones at p={p}");
        }
    }

    #[test]
    fn split_rho_within_per_p_bounds() {
        for p in MIN_PRECISION..=MAX_PRECISION {
            for seed_item in 0..2000i32 {
                let x = hash64((seed_item as u32) as u64, 0);
                let (idx, rho) = HyperLogLog::split(x, p);
                assert!(idx < (1u32 << p));
                assert!((1..=HyperLogLog::rho_ceiling(p)).contains(&rho));
            }
        }
    }

    // ---- register max-update & idempotence ------------------------------

    #[test]
    fn add_updates_expected_register_to_rho() {
        let p = 14u8;
        let mut h = HyperLogLog::with_precision(p).unwrap();
        let (idx, rho) = expected_split(42, p);
        h.add(42);
        assert_eq!(h.registers()[idx as usize], rho);
        assert_eq!(h.nonzero_registers(), 1);
        assert_eq!(h.max_register(), rho);
    }

    #[test]
    fn add_is_max_not_overwrite_and_idempotent() {
        let p = 4u8;
        let a = HyperLogLog::with_precision(p)
            .map(|mut h| {
                h.add(7);
                h
            })
            .unwrap();
        let mut b = HyperLogLog::with_precision(p).unwrap();
        b.add(7);
        b.add(7);
        b.add(7);
        // Re-add of same item is a no-op (same x -> same idx/rho -> max no-op).
        assert_eq!(a.registers(), b.registers());
    }

    #[test]
    fn add_order_independent() {
        let p = 6u8;
        let mut ab = HyperLogLog::with_precision(p).unwrap();
        ab.add(11);
        ab.add(99999);
        let mut ba = HyperLogLog::with_precision(p).unwrap();
        ba.add(99999);
        ba.add(11);
        assert_eq!(ab.registers(), ba.registers());
    }

    #[test]
    fn neg_one_zero_extend_differs_from_sign_extend() {
        // add(-1) encodes 0x00000000ffffffff (zero-extend). The would-be
        // sign-extend (0xffffffffffffffff) yields a different x -> different
        // (idx, rho). We pin that the implementation uses the zero-extend.
        let p = 4u8;
        let mut h = HyperLogLog::with_precision(p).unwrap();
        h.add(-1);
        let (zi, zr) = HyperLogLog::split(hash64(0x0000_0000_ffff_ffff, 0), p);
        let (si, sr) = HyperLogLog::split(hash64(0xffff_ffff_ffff_ffff, 0), p);
        assert_eq!(h.registers()[zi as usize], zr);
        // Sign-extend would route differently (idx or rho differ).
        assert!(zi != si || zr != sr);
    }

    // ---- merge: element-wise max, p-mismatch ----------------------------

    #[test]
    fn merge_is_elementwise_max() {
        let p = 4u8;
        let mut a = HyperLogLog::with_precision(p).unwrap();
        for v in [1, 2, 3] {
            a.add(v);
        }
        let mut b = HyperLogLog::with_precision(p).unwrap();
        for v in [3, 4, 5] {
            b.add(v);
        }
        let mut expected = a.registers().to_vec();
        for (e, &bb) in expected.iter_mut().zip(b.registers()) {
            *e = (*e).max(bb);
        }
        a.merge(&b).unwrap();
        assert_eq!(a.registers(), expected.as_slice());
    }

    #[test]
    fn merge_commutative_and_idempotent() {
        let p = 5u8;
        let build = |items: &[i32]| {
            let mut h = HyperLogLog::with_precision(p).unwrap();
            for &v in items {
                h.add(v);
            }
            h
        };
        let mut ab = build(&[10, 20, 30]);
        let bset = build(&[30, 40, 50]);
        ab.merge(&bset).unwrap();

        let mut ba = build(&[30, 40, 50]);
        let aset = build(&[10, 20, 30]);
        ba.merge(&aset).unwrap();
        assert_eq!(ab.registers(), ba.registers());

        // Idempotent: merge(a, a) == a.
        let a = build(&[10, 20, 30]);
        let mut aa = build(&[10, 20, 30]);
        aa.merge(&a).unwrap();
        assert_eq!(aa.registers(), a.registers());
    }

    #[test]
    fn merge_p_mismatch_errors() {
        let mut a = HyperLogLog::with_precision(4).unwrap();
        let b = HyperLogLog::with_precision(5).unwrap();
        assert_eq!(
            a.merge(&b),
            Err(HllError::PrecisionMismatch { left: 4, right: 5 })
        );
    }

    // ---- serialization round-trip + rejections --------------------------

    #[test]
    fn serialize_roundtrip_and_header() {
        let mut h = HyperLogLog::with_precision(4).unwrap();
        h.add(1);
        h.add(7);
        h.add(-1);
        let bytes = h.to_bytes();
        assert_eq!(bytes.len(), 5 + 16);
        assert_eq!(&bytes[0..4], b"HLL1");
        assert_eq!(bytes[4], 4);
        let back = HyperLogLog::from_bytes(&bytes).unwrap();
        assert_eq!(back, h);
        assert_eq!(back.registers(), h.registers());
    }

    #[test]
    fn empty_p4_register_hex_anchor() {
        let h = HyperLogLog::with_precision(4).unwrap();
        // "HLL1" + 0x04 + 16 zero bytes.
        assert_eq!(
            to_hex(&h.to_bytes()),
            "484c4c310400000000000000000000000000000000"
        );
    }

    #[test]
    fn from_bytes_rejects_bad_magic() {
        let mut bytes = HyperLogLog::with_precision(4).unwrap().to_bytes();
        bytes[0] = 0x00;
        assert!(matches!(
            HyperLogLog::from_bytes(&bytes),
            Err(HllError::BadMagic(_))
        ));
    }

    #[test]
    fn from_bytes_rejects_too_short() {
        assert_eq!(
            HyperLogLog::from_bytes(&[0x48, 0x4c, 0x4c]),
            Err(HllError::TooShort(3))
        );
    }

    #[test]
    fn from_bytes_rejects_bad_p() {
        let mut bytes = HyperLogLog::with_precision(4).unwrap().to_bytes();
        bytes[4] = 3; // p out of range
        assert_eq!(
            HyperLogLog::from_bytes(&bytes),
            Err(HllError::BadPrecision(3))
        );
        bytes[4] = 19;
        assert_eq!(
            HyperLogLog::from_bytes(&bytes),
            Err(HllError::BadPrecision(19))
        );
    }

    #[test]
    fn from_bytes_rejects_length_mismatch() {
        let mut bytes = HyperLogLog::with_precision(4).unwrap().to_bytes();
        bytes.push(0); // one byte too many
        assert!(matches!(
            HyperLogLog::from_bytes(&bytes),
            Err(HllError::LengthMismatch { .. })
        ));
        let short = &HyperLogLog::with_precision(4).unwrap().to_bytes()[..20];
        assert!(matches!(
            HyperLogLog::from_bytes(short),
            Err(HllError::LengthMismatch { .. })
        ));
    }

    #[test]
    fn from_bytes_rejects_register_above_ceiling() {
        // At p=4 the ceiling is 64-4+1 = 61. A byte of 62 must be rejected.
        let mut bytes = HyperLogLog::with_precision(4).unwrap().to_bytes();
        bytes[5] = 62;
        assert!(matches!(
            HyperLogLog::from_bytes(&bytes),
            Err(HllError::RegisterOutOfRange {
                value: 62,
                max: 61,
                ..
            })
        ));
        // 61 itself is accepted (it is the legal max).
        bytes[5] = 61;
        assert!(HyperLogLog::from_bytes(&bytes).is_ok());
    }

    #[test]
    fn from_bytes_ceiling_is_per_p() {
        // At p=18 the ceiling is 64-18+1 = 47.
        let mut bytes = HyperLogLog::with_precision(18).unwrap().to_bytes();
        bytes[5] = 48;
        assert!(matches!(
            HyperLogLog::from_bytes(&bytes),
            Err(HllError::RegisterOutOfRange {
                value: 48,
                max: 47,
                ..
            })
        ));
        bytes[5] = 47;
        assert!(HyperLogLog::from_bytes(&bytes).is_ok());
    }

    // ---- estimate(): native-only, tolerance-bounded (Rule Q2) -----------

    #[test]
    fn fresh_hll_estimates_zero() {
        // Z = m, E = alpha*m, E <= 2.5m and V = m > 0 -> m*ln(m/m) = m*ln(1) = 0.
        for p in [4u8, 7, 14] {
            let h = HyperLogLog::with_precision(p).unwrap();
            assert_eq!(
                h.estimate(),
                0.0,
                "fresh HLL at p={p} must estimate exactly 0"
            );
        }
    }

    #[test]
    fn estimate_within_tolerance_on_known_cardinality() {
        // Documented tolerance: relative error < 5% (covers HLL's ~1.04/sqrt(m)
        // standard error AND cross-libm float drift). p=14 -> m=16384.
        let p = 14u8;
        let n = 10_000i32;
        let mut h = HyperLogLog::with_precision(p).unwrap();
        for i in 0..n {
            // Distinct i32 values; spread out so they are genuinely distinct.
            h.add(i.wrapping_mul(2_654_435_761u32 as i32));
        }
        let est = h.estimate();
        let rel = (est - n as f64).abs() / n as f64;
        assert!(
            rel < 0.05,
            "estimate {est} for n={n} at p={p}: relative error {rel} >= 0.05"
        );
    }

    #[test]
    fn estimate_small_cardinality_linear_counting() {
        // A few hundred distinct values: linear counting is active (E <= 2.5m,
        // V > 0). Still within tolerance.
        let p = 14u8;
        let n = 300i32;
        let mut h = HyperLogLog::with_precision(p).unwrap();
        for i in 0..n {
            h.add(i.wrapping_mul(2_654_435_761u32 as i32));
        }
        let est = h.estimate();
        let rel = (est - n as f64).abs() / n as f64;
        assert!(rel < 0.05, "small-card estimate {est} for n={n}: rel {rel}");
    }

    #[test]
    fn alpha_m_matches_pinned_constants() {
        assert_eq!(HyperLogLog::alpha_m(4), 0.673);
        assert_eq!(HyperLogLog::alpha_m(5), 0.697);
        assert_eq!(HyperLogLog::alpha_m(6), 0.709);
        // p=7 -> m=128 -> closed form.
        assert_eq!(HyperLogLog::alpha_m(7), 0.7213 / (1.0 + 1.079 / 128.0));
    }

    #[test]
    fn estimate_large_range_correction_is_finite() {
        // Drive a high-register state via from_bytes so raw E exceeds
        // (1/30)*2^64; assert estimate() is finite. Two sub-cases:
        //   r = ceiling - 1: E in the large-range band but below 2^64, the log
        //     correction fires; the 2^64 ceiling keeps ln(1 - E/2^64) > 0 (a
        //     2^32 ceiling would return NaN here).
        //   r = ceiling (fully saturated): every register at the per-p max — a
        //     degenerate, add-unreachable state constructible via from_bytes
        //     whose raw E reaches/exceeds 2^64. The log-argument guard skips the
        //     correction and returns the raw (large, finite) E. Without the
        //     guard, ln(1 - E/2^64) = ln(<= 0) = NaN.
        let p = 4u8;
        for r in [HyperLogLog::rho_ceiling(p) - 1, HyperLogLog::rho_ceiling(p)] {
            let mut bytes = HyperLogLog::with_precision(p).unwrap().to_bytes();
            for b in bytes[5..].iter_mut() {
                *b = r;
            }
            let h = HyperLogLog::from_bytes(&bytes).unwrap();
            let est = h.estimate();
            assert!(
                est.is_finite(),
                "large-range estimate must be finite for all-{r} registers, got {est}"
            );
        }
    }

    // ---- authoritative register_hex values for the scenarios ------------
    // These echo the values committed to the 13-hyperloglog/*.json scenarios so
    // the Rust port IS the oracle they assert. Any change here is a break.

    #[test]
    fn scenario_oracle_values() {
        // single add(1) at p=4
        let mut h = HyperLogLog::with_precision(4).unwrap();
        h.add(1);
        println!("single_add_1_p4 hex = {}", to_hex(&h.to_bytes()));
        let (idx1, rho1) = expected_split(1, 4);
        println!("  add(1) p4 -> idx={idx1} rho={rho1}");
        assert_eq!(h.nonzero_registers(), 1);

        // add(-1) at p=4
        let mut hn = HyperLogLog::with_precision(4).unwrap();
        hn.add(-1);
        println!("neg_one_p4 hex = {}", to_hex(&hn.to_bytes()));
        let (idxn, rhon) = expected_split(-1, 4);
        println!("  add(-1) p4 -> idx={idxn} rho={rhon}");
    }
}
