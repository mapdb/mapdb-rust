// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Bloom filter — approximate set membership on the deterministic hash pipeline
//! (see `spec/features/bloom.md`).
//!
//! This is the **reference port** of the first end-user collection of the
//! probabilistic wave. It rides directly on the just-shipped deterministic hash
//! pipeline ([`crate::hash`]): it uses [`crate::hash::positions`]
//! (Kirsch–Mitzenmacher double-hashing) to pick `k` bit indices in an `m`-bit
//! array. Because `positions()` is **bit-identical across all five ports**, the
//! bit array after a given add-sequence is bit-identical too — that bit array is
//! the cross-language oracle.
//!
//! ## Element encoding (critical)
//!
//! An `i32` element `v` is reinterpreted to `u32`, encoded to **4 little-endian
//! bytes**, and fed to the hash pipeline's **byte-input** `positions(bytes, m,
//! k)` path — the exact path the `12-hash-pipeline/positions_*` scenarios drive,
//! which **folds in the byte length**. This is NOT the scalar `hash32_i32` path:
//! for `v = 7` the byte-path input word is `0x07 ^ 4 = 0x03`. Worked example:
//! `with_params(16, 4)` then `add(7)` lights bits `{0, 2, 7, 9}` →
//! `to_bytes() = [0x85, 0x02]` → `"0x8502"`, `bit_count() == 4`.
//!
//! ## Guarantees
//!
//! - **No false negative.** `add(v)` then `might_contain(v)` is always `true`.
//! - **Idempotent / order-independent.** The bit array depends only on the *set*
//!   of added elements.
//! - **Deterministic.** Identical `(m, k)` + add-sequence ⇒ identical bits on all
//!   five ports.

use crate::hash;

/// A Bloom filter over `i32` elements with `m` bits and `k` hash functions, both
/// fixed at construction. The bit array is stored as a `Vec<u64>` word array; the
/// internal word width is not observable — only [`Bloom::to_bytes`] (LSB-first,
/// ascending bytes) is the cross-language form.
#[derive(Clone, Debug)]
pub struct Bloom {
    /// Number of bits in the array (`m`, `1 ..= 2^32-1`).
    m_bits: u32,
    /// Number of hash functions / positions set per element.
    k: u32,
    /// The bit array, `ceil(m / 64)` words; bit `i` lives in word `i / 64` at
    /// bit position `i % 64` (LSB-first within a word).
    words: Vec<u64>,
}

impl Bloom {
    /// Canonical, fully-deterministic constructor: explicit bit count `m_bits`
    /// and hash count `k`. The filter starts empty (all bits `0`).
    ///
    /// # Panics
    ///
    /// `m_bits == 0` is **invalid** and traps (a 0-bit array can hold nothing and
    /// every `positions` modulo would be by zero). `k == 0` is degenerate but
    /// **legal** (see [`Bloom::might_contain`]).
    pub fn with_params(m_bits: u32, k: u32) -> Bloom {
        assert!(m_bits != 0, "Bloom::with_params: m_bits must be >= 1");
        let n_words = m_bits.div_ceil(64) as usize;
        Bloom {
            m_bits,
            k,
            words: vec![0u64; n_words],
        }
    }

    /// Convenience constructor sizing the filter from an expected element count
    /// `n` and a target false-positive probability `p`, using the standard Bloom
    /// formulas:
    ///
    /// ```text
    /// m = ceil( -n * ln(p) / (ln 2)^2 )
    /// k = max( 1, round( (m / n) * ln 2 ) )      # round-half-away-from-zero
    /// ```
    ///
    /// then delegates to [`Bloom::with_params`]. This is **native-test-only**: it
    /// never appears in the shared cross-language scenarios (the float derivation
    /// could drift by a ULP across libms — quarantined to native tests against the
    /// pinned integer table in `spec/features/bloom.md`).
    ///
    /// # Panics
    ///
    /// Requires `n >= 1` and `0 < p < 1`. `n == 0`, `p <= 0`, `p >= 1`, `NaN`, and
    /// `±Infinity` are invalid and trap (they would divide by zero, take `ln` of a
    /// non-positive value, or yield a non-finite `m`).
    pub fn optimal(n_expected: u64, p: f64) -> Bloom {
        assert!(n_expected >= 1, "Bloom::optimal: n_expected must be >= 1");
        assert!(
            p.is_finite() && p > 0.0 && p < 1.0,
            "Bloom::optimal: p must be finite and in (0, 1), got {p}"
        );
        let n = n_expected as f64;
        let ln2 = std::f64::consts::LN_2;
        let m_f = (-n * p.ln() / (ln2 * ln2)).ceil();
        assert!(
            m_f.is_finite() && m_f >= 1.0 && m_f <= u32::MAX as f64,
            "Bloom::optimal: derived m out of range: {m_f}"
        );
        let m = m_f as u32;
        // round-half-away-from-zero (the common `round()`), clamped to >= 1.
        let k_f = ((m as f64 / n) * ln2).round();
        let k = (k_f as i64).max(1) as u32;
        Bloom::with_params(m, k)
    }

    /// The bit count `m`.
    #[inline]
    pub fn m_bits(&self) -> u32 {
        self.m_bits
    }

    /// The hash count `k`.
    #[inline]
    pub fn k(&self) -> u32 {
        self.k
    }

    /// Add an `i32` element: set the `k` bits for `v` (idempotent). With `k == 0`
    /// this sets no bits.
    pub fn add(&mut self, v: i32) {
        let bytes = (v as u32).to_le_bytes();
        for p in hash::positions(&bytes, self.m_bits, self.k) {
            self.set_bit(p);
        }
    }

    /// `might_contain` — the canonical name. Returns `false` ⇒ definitely absent;
    /// `true` ⇒ possibly present (may be a false positive). **Never** returns
    /// `false` for an element that was added (no false negative).
    ///
    /// With `k == 0` the AND over zero positions is **vacuously true**, so this
    /// returns `true` for every element (an all-false-positive filter).
    pub fn might_contain(&self, v: i32) -> bool {
        let bytes = (v as u32).to_le_bytes();
        for p in hash::positions(&bytes, self.m_bits, self.k) {
            if !self.get_bit(p) {
                return false;
            }
        }
        true
    }

    /// Idiomatic alias for [`Bloom::might_contain`] (what the JSON suite's
    /// `contains_<v>` keys probe). The result is **approximate** membership.
    #[inline]
    pub fn contains(&self, v: i32) -> bool {
        self.might_contain(v)
    }

    /// `true` iff no bit is set (equivalently: nothing has been added, or only
    /// `k == 0` adds). Equal to `bit_count() == 0`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    /// The number of set bits (popcount of the whole bit array). The zeroed tail
    /// bits never contribute (no `positions` index reaches them).
    pub fn bit_count(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }

    /// Bitwise OR of two filters with **identical `(m, k)`**, returning a new
    /// filter. The result's membership is the union of the two filters'
    /// membership (no false negatives lost).
    ///
    /// # Panics
    ///
    /// Mismatched `(m, k)` traps (a filter built with different parameters has an
    /// incompatible bit array; ORing them is meaningless).
    pub fn union(&self, other: &Bloom) -> Bloom {
        assert!(
            self.m_bits == other.m_bits && self.k == other.k,
            "Bloom::union: parameter mismatch ({}, {}) vs ({}, {})",
            self.m_bits,
            self.k,
            other.m_bits,
            other.k
        );
        let words = self
            .words
            .iter()
            .zip(other.words.iter())
            .map(|(a, b)| a | b)
            .collect();
        Bloom {
            m_bits: self.m_bits,
            k: self.k,
            words,
        }
    }

    /// The serialized bit array (`spec/features/bloom.md` §"Serialized bit-array
    /// form"): length exactly `ceil(m / 8)` bytes; **LSB-first** bit order within
    /// each byte (bit `i` ⇒ `byte[i / 8] |= 1 << (i % 8)`); ascending byte order;
    /// **little-endian on every host**; unused tail bits `0`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let n_bytes = self.m_bits.div_ceil(8) as usize;
        let mut out = vec![0u8; n_bytes];
        for (wi, &w) in self.words.iter().enumerate() {
            // Each word holds bits [wi*64 .. wi*64 + 64). Emit its 8 bytes
            // little-endian so bit (wi*64 + b*8 + j) lands at out[...]&(1<<j).
            let word_bytes = w.to_le_bytes();
            for (bi, &byte) in word_bytes.iter().enumerate() {
                let out_idx = wi * 8 + bi;
                if out_idx < n_bytes {
                    out[out_idx] = byte;
                }
                // out_idx >= n_bytes can only be a fully-zero tail byte (no
                // `positions` index reaches >= m), so dropping it is exact.
            }
        }
        out
    }

    // ---- internal bit ops ------------------------------------------------

    #[inline]
    fn set_bit(&mut self, i: u32) {
        debug_assert!(
            i < self.m_bits,
            "bit index {i} out of range {}",
            self.m_bits
        );
        let idx = i as usize;
        self.words[idx / 64] |= 1u64 << (idx % 64);
    }

    #[inline]
    fn get_bit(&self, i: u32) -> bool {
        let idx = i as usize;
        (self.words[idx / 64] >> (idx % 64)) & 1 == 1
    }

    /// The sorted-ascending indices of the set bits — a human-legible alternate
    /// oracle to [`Bloom::to_bytes`] (drives the `set_bits` scenario assertion).
    pub fn set_bits(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.bit_count() as usize);
        for (wi, &w) in self.words.iter().enumerate() {
            let mut bits = w;
            while bits != 0 {
                let j = bits.trailing_zeros();
                out.push(wi as u32 * 64 + j);
                bits &= bits - 1; // clear lowest set bit
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        let mut s = String::from("0x");
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    // ---- The worked example (spec §"Serialized bit-array form") ----------

    #[test]
    fn worked_example_add_7() {
        // with_params(16, 4); add(7). positions(7, 16, 4) = [7, 0, 9, 2] (from
        // 12-hash-pipeline/positions_basic.json). Bits {0, 2, 7, 9} set.
        let mut b = Bloom::with_params(16, 4);
        assert!(b.is_empty());
        b.add(7);
        assert_eq!(b.set_bits(), vec![0, 2, 7, 9]);
        assert_eq!(b.bit_count(), 4);
        assert!(!b.is_empty());
        // Byte 0: bits 0,2,7 -> 0x01|0x04|0x80 = 0x85. Byte 1: bit 9 -> 0x02.
        assert_eq!(b.to_bytes(), vec![0x85, 0x02]);
        assert_eq!(hex(&b.to_bytes()), "0x8502");
        assert!(b.might_contain(7));
        assert!(b.contains(7));
    }

    // ---- optimal() pinned integer table (spec §"Construction") -----------

    #[test]
    fn optimal_integer_table() {
        let cases: [(u64, f64, u32, u32); 5] = [
            (1000, 0.01, 9586, 7),
            (1000, 0.001, 14378, 10),
            (10000, 0.01, 95851, 7),
            (100, 0.1, 480, 3),
            (1, 0.5, 2, 1),
        ];
        for (n, p, em, ek) in cases {
            let b = Bloom::optimal(n, p);
            assert_eq!(b.m_bits(), em, "optimal({n}, {p}) m");
            assert_eq!(b.k(), ek, "optimal({n}, {p}) k");
        }
    }

    #[test]
    #[should_panic(expected = "n_expected must be >= 1")]
    fn optimal_n_zero_traps() {
        Bloom::optimal(0, 0.01);
    }

    #[test]
    #[should_panic(expected = "p must be finite")]
    fn optimal_p_zero_traps() {
        Bloom::optimal(100, 0.0);
    }

    #[test]
    #[should_panic(expected = "p must be finite")]
    fn optimal_p_one_traps() {
        Bloom::optimal(100, 1.0);
    }

    #[test]
    #[should_panic(expected = "p must be finite")]
    fn optimal_p_nan_traps() {
        Bloom::optimal(100, f64::NAN);
    }

    #[test]
    #[should_panic(expected = "p must be finite")]
    fn optimal_p_inf_traps() {
        Bloom::optimal(100, f64::INFINITY);
    }

    // ---- m = 0 trap; k = 0 vacuous-true ----------------------------------

    #[test]
    #[should_panic(expected = "m_bits must be >= 1")]
    fn m_zero_traps() {
        Bloom::with_params(0, 4);
    }

    #[test]
    fn k_zero_vacuous_true() {
        // k=0: add sets no bits; might_contain is vacuously true for everything.
        let mut b = Bloom::with_params(16, 0);
        b.add(5);
        assert_eq!(b.bit_count(), 0);
        assert!(b.is_empty());
        assert_eq!(b.to_bytes(), vec![0x00, 0x00]);
        assert!(b.might_contain(5));
        assert!(b.might_contain(9999));
        assert!(b.might_contain(-1));
    }

    // ---- union OR + mismatch trap ----------------------------------------

    #[test]
    fn union_is_bitwise_or() {
        let mut a = Bloom::with_params(32, 3);
        a.add(1);
        a.add(2);
        let mut c = Bloom::with_params(32, 3);
        c.add(3);
        c.add(4);
        let u = a.union(&c);
        // No false negative for either operand's elements.
        assert!(u.might_contain(1));
        assert!(u.might_contain(2));
        assert!(u.might_contain(3));
        assert!(u.might_contain(4));
        // Union bits are exactly the OR of the two bit arrays.
        for (i, ((ua, ub), uc)) in a
            .words
            .iter()
            .zip(c.words.iter())
            .zip(u.words.iter())
            .enumerate()
        {
            assert_eq!(*uc, ua | ub, "word {i}");
        }
    }

    #[test]
    #[should_panic(expected = "parameter mismatch")]
    fn union_m_mismatch_traps() {
        let a = Bloom::with_params(16, 4);
        let b = Bloom::with_params(32, 4);
        let _ = a.union(&b);
    }

    #[test]
    #[should_panic(expected = "parameter mismatch")]
    fn union_k_mismatch_traps() {
        let a = Bloom::with_params(16, 4);
        let b = Bloom::with_params(16, 3);
        let _ = a.union(&b);
    }

    // ---- idempotent / order-independent ----------------------------------

    #[test]
    fn add_is_idempotent() {
        let mut once = Bloom::with_params(64, 5);
        once.add(7);
        let mut twice = Bloom::with_params(64, 5);
        twice.add(7);
        twice.add(7);
        assert_eq!(once.to_bytes(), twice.to_bytes());
        assert_eq!(once.bit_count(), twice.bit_count());
    }

    #[test]
    fn add_is_order_independent() {
        let mut ab = Bloom::with_params(128, 4);
        ab.add(11);
        ab.add(22);
        ab.add(33);
        let mut ba = Bloom::with_params(128, 4);
        ba.add(33);
        ba.add(11);
        ba.add(22);
        assert_eq!(ab.to_bytes(), ba.to_bytes());
    }

    // ---- signed extremes (reinterpret, not sign-extend) ------------------

    #[test]
    fn signed_extremes() {
        let mut b = Bloom::with_params(256, 4);
        b.add(-1);
        b.add(i32::MIN);
        assert!(b.might_contain(-1));
        assert!(b.might_contain(i32::MIN));
        // -1 encodes to LE bytes ffffffff, distinct from any small positive.
        let mut neg = Bloom::with_params(256, 4);
        neg.add(-1);
        let mut pos = Bloom::with_params(256, 4);
        pos.add(1);
        assert_ne!(neg.to_bytes(), pos.to_bytes());
    }

    // ---- no false negative over a set ------------------------------------

    #[test]
    fn no_false_negative_over_a_set() {
        let mut b = Bloom::with_params(512, 7);
        let elems: Vec<i32> = (-50..50).chain([i32::MIN, i32::MAX, 0]).collect();
        for &e in &elems {
            b.add(e);
        }
        for &e in &elems {
            assert!(b.might_contain(e), "false negative for {e}");
        }
    }

    // ---- tail bits (m not a multiple of 8) -------------------------------

    #[test]
    fn tail_bits_zeroed() {
        // m = 13 -> ceil(13/8) = 2 bytes; bits 13,14,15 are never addressable.
        let mut b = Bloom::with_params(13, 3);
        b.add(7);
        b.add(42);
        let bytes = b.to_bytes();
        assert_eq!(bytes.len(), 2);
        for &p in &b.set_bits() {
            assert!(p < 13, "set bit {p} must be < 13");
        }
    }

    // ---- host-endianness independence of to_bytes ------------------------

    #[test]
    fn to_bytes_lsb_first_independent_of_word_width() {
        // Set bit 8 only -> byte1 bit0 = 0x01; bytes [0x00, 0x01].
        let mut b = Bloom::with_params(16, 1);
        // Directly set bit 8 to make the byte layout unambiguous.
        b.set_bit(8);
        assert_eq!(b.to_bytes(), vec![0x00, 0x01]);
        // Set bit 0 only -> 0x01 in byte 0.
        let mut c = Bloom::with_params(16, 1);
        c.set_bit(0);
        assert_eq!(c.to_bytes(), vec![0x01, 0x00]);
        // Set bit 7 -> 0x80 in byte 0 (LSB-first: bit 7 is the high bit).
        let mut d = Bloom::with_params(16, 1);
        d.set_bit(7);
        assert_eq!(d.to_bytes(), vec![0x80, 0x00]);
        // A bit in a high word emits at the correct byte regardless of word width.
        let mut e = Bloom::with_params(200, 1);
        e.set_bit(130); // byte 130/8 = 16, bit 130%8 = 2 -> 0x04
        let bytes = e.to_bytes();
        assert_eq!(bytes.len(), 25);
        assert_eq!(bytes[16], 0x04);
    }

    #[test]
    fn empty_filter_serializes_to_zero_of_full_length() {
        let b = Bloom::with_params(16, 4);
        assert_eq!(b.to_bytes(), vec![0x00, 0x00]);
        assert_eq!(b.bit_count(), 0);
        assert!(b.is_empty());
        // An empty k>=1 filter reports absent for everything.
        assert!(!b.might_contain(7));
    }
}
