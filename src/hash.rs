// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Deterministic, byte-exact, cross-language hash pipeline (see
//! `spec/features/hash-pipeline.md`).
//!
//! This is the **reference port** of a small, frozen primitive whose entire
//! contract is *bit-exactness across all five language ports*: every
//! `(input, seed)` produces the identical `hash32` / `hash64` / `positions`
//! bits in Rust, Go, TypeScript, Zig and Java. It is a separate, additive
//! module — it does NOT touch the collections' bucket hash
//! (`algorithms.md` §"Hash function"), which keeps its native-hash carve-out
//! (Rust uses SipHash there). This module has **no carve-out**.
//!
//! - [`hash32`] — MurmurHash3 32-bit finalizer (`fmix32`) over the input word
//!   XOR'd with a 32-bit fold of the 64-bit seed.
//! - [`hash64`] — MurmurHash3 64-bit finalizer (`fmix64`, constants
//!   `0xff51afd7ed558ccd` / `0xc4ceb9fe1a85ec53`, shifts `33/33/33` — NOT the
//!   SplitMix64 generator's final mix) over the input word XOR'd with the seed.
//! - [`positions`] — Kirsch–Mitzenmacher double hashing (`h1 + i*h2 mod m`),
//!   all 32-bit wrapping, unsigned modulo.
//! - [`hll_split`] — pre-stated `(register_index, leading_zero_run)` split for
//!   the later HyperLogLog feature.
//!
//! All multiplies are two's-complement **wrapping** at their declared width;
//! all right shifts are **logical** (unsigned). On `u32`/`u64` Rust gives both
//! natively (`wrapping_mul`, logical `>>` on unsigned types).

/// MurmurHash3 `fmix32` finalizer constants (the published values).
const FMIX32_C1: u32 = 0x85eb_ca6b;
const FMIX32_C2: u32 = 0xc2b2_ae35;

/// MurmurHash3 `fmix64` finalizer constants (the published values). NOTE: these
/// are the MurmurHash3 `fmix64` constants with three `33`-bit shifts, NOT the
/// SplitMix64 *generator's* final mix (`0xbf58476d1ce4e5b9` /
/// `0x94d049bb133111eb`, shifts `30/27/31`) — a different function.
const FMIX64_C1: u64 = 0xff51_afd7_ed55_8ccd;
const FMIX64_C2: u64 = 0xc4ce_b9fe_1a85_ec53;

/// The fixed 32-bit salt for the second base hash of the double-hashing
/// position scheme (the 32-bit golden-ratio prime). Distinct from the 64-bit
/// collection Fibonacci constant `0x9E3779B97F4A7C15`.
pub const SALT2: u64 = 0x9e37_79b1;

/// 32-bit named hash: the MurmurHash3 `fmix32` finalizer applied to one 32-bit
/// lane derived from `input_word` and a 32-bit fold of the 64-bit `seed`.
///
/// The seed is folded with `seed ^ (seed >> 32)` so two seeds differing only in
/// their high 32 bits still produce different hashes. Seed `0` is an ordinary
/// seed (XOR'd in; no special case).
#[inline]
pub fn hash32(input_word: u32, seed: u64) -> u32 {
    // Fold the full 64-bit seed into one 32-bit lane (low XOR high).
    let seed32: u32 = (seed ^ (seed >> 32)) as u32;
    let mut h: u32 = input_word ^ seed32;
    h ^= h >> 16;
    h = h.wrapping_mul(FMIX32_C1);
    h ^= h >> 13;
    h = h.wrapping_mul(FMIX32_C2);
    h ^= h >> 16;
    h
}

/// 64-bit named hash: the MurmurHash3 `fmix64` finalizer applied to
/// `input_word ^ seed`. The seed is mixed in first as a 64-bit integer (no
/// endianness, no special case for seed `0`).
#[inline]
pub fn hash64(input_word: u64, seed: u64) -> u64 {
    let mut h: u64 = input_word ^ seed;
    h ^= h >> 33;
    h = h.wrapping_mul(FMIX64_C1);
    h ^= h >> 33;
    h = h.wrapping_mul(FMIX64_C2);
    h ^= h >> 33;
    h
}

/// The high 32-bit lane of [`hash64`] (pins the TypeScript hi/lo lane split).
#[inline]
pub fn hash64_hi(input_word: u64, seed: u64) -> u32 {
    (hash64(input_word, seed) >> 32) as u32
}

/// The low 32-bit lane of [`hash64`].
#[inline]
pub fn hash64_lo(input_word: u64, seed: u64) -> u32 {
    hash64(input_word, seed) as u32
}

// ---- Per-type input-word encoders ----------------------------------------

/// Encode an `i32` element to the `hash32` input word: a two's-complement bit
/// **reinterpret** to `u32` (NOT a sign-extend).
#[inline]
pub fn encode_i32_word32(value: i32) -> u32 {
    value as u32
}

/// Encode an `i32` element to the `hash64` input word: reinterpret to `u32`
/// then **zero-extend** to `u64` (so the high 32 bits are always `0`; the seed
/// supplies the high-word entropy). NOT a sign-extend.
#[inline]
pub fn encode_i32_word64(value: i32) -> u64 {
    (value as u32) as u64
}

/// Fold a raw byte slice into the `hash32` input word: read 4 bytes at a time as
/// **little-endian** `u32` lanes, XOR-combine, zero-pad a sub-lane tail to the
/// low bytes, then XOR in `len(bytes) mod 2^32`.
#[inline]
pub fn encode_bytes_word32(bytes: &[u8]) -> u32 {
    let mut word: u32 = 0;
    let mut chunks = bytes.chunks_exact(4);
    for c in &mut chunks {
        word ^= u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
    }
    let tail = chunks.remainder();
    if !tail.is_empty() {
        // Tail goes in the LOW bytes of its lane; remaining high bytes are 0.
        let mut buf = [0u8; 4];
        buf[..tail.len()].copy_from_slice(tail);
        word ^= u32::from_le_bytes(buf);
    }
    // Length reduced mod 2^32 before the XOR.
    word ^ (bytes.len() as u32)
}

/// Fold a raw byte slice into the `hash64` input word: read 8 bytes at a time as
/// **little-endian** `u64` lanes, XOR-combine, zero-pad a sub-lane tail to the
/// low bytes, then XOR in `len(bytes) mod 2^64`.
#[inline]
pub fn encode_bytes_word64(bytes: &[u8]) -> u64 {
    let mut word: u64 = 0;
    let mut chunks = bytes.chunks_exact(8);
    for c in &mut chunks {
        word ^= u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
    }
    let tail = chunks.remainder();
    if !tail.is_empty() {
        let mut buf = [0u8; 8];
        buf[..tail.len()].copy_from_slice(tail);
        word ^= u64::from_le_bytes(buf);
    }
    word ^ (bytes.len() as u64)
}

/// `hash32` of an `i32` element (reinterpret encoding).
#[inline]
pub fn hash32_i32(value: i32, seed: u64) -> u32 {
    hash32(encode_i32_word32(value), seed)
}

/// `hash32` of a raw byte slice (little-endian fold encoding).
#[inline]
pub fn hash32_bytes(bytes: &[u8], seed: u64) -> u32 {
    hash32(encode_bytes_word32(bytes), seed)
}

/// `hash64` of an `i32` element (reinterpret + zero-extend encoding).
#[inline]
pub fn hash64_i32(value: i32, seed: u64) -> u64 {
    hash64(encode_i32_word64(value), seed)
}

/// `hash64` of a raw byte slice (little-endian fold encoding).
#[inline]
pub fn hash64_bytes(bytes: &[u8], seed: u64) -> u64 {
    hash64(encode_bytes_word64(bytes), seed)
}

// ---- Derived positions (Kirsch–Mitzenmacher double hashing) --------------

/// Derive `k` array positions over a table of size `m` from two base hashes
/// `h1`/`h2`, combined linearly: `p_i = (h1 + i*h2) mod m`, all 32-bit
/// wrapping, unsigned modulo. Returned in derivation order `p_0 … p_{k-1}`.
///
/// This is the inner function the test-vector oracle is stated on; it is
/// independent of the `hash32` layer and the byte encoding.
pub fn positions_from_hashes(h1: u32, h2: u32, m: u32, k: u32) -> Vec<u32> {
    let mut out = Vec::with_capacity(k as usize);
    for i in 0..k {
        let combined: u32 = h1.wrapping_add(i.wrapping_mul(h2));
        out.push(combined % m);
    }
    out
}

/// Derive `k` array positions for `input` over a table of size `m` using
/// Kirsch–Mitzenmacher double hashing. `h1 = hash32(input, 0)`,
/// `h2 = hash32(input, SALT2)`; then [`positions_from_hashes`].
pub fn positions(input: &[u8], m: u32, k: u32) -> Vec<u32> {
    let h1 = hash32_bytes(input, 0);
    let h2 = hash32_bytes(input, SALT2);
    positions_from_hashes(h1, h2, m, k)
}

// ---- HyperLogLog split (pre-stated for the HLL feature) ------------------

/// Pre-stated HyperLogLog split: from a single 64-bit hash, derive a
/// `(register_index, leading_zero_run)` pair. `p = log2(number of registers)`,
/// `4 <= p <= 18`. Only `hash64(input, 0)` is locked here; HLL itself is a
/// separate later feature.
///
/// - `idx` = the top `p` bits of the hash (the register index).
/// - `rho` = `clz64(w) + 1`, the 1-based leading-zero run of the remaining bits
///   shifted up with a guard bit set at position `p - 1`.
pub fn hll_split(input: &[u8], p: u32) -> (u32, u32) {
    debug_assert!((4..=18).contains(&p), "hll_split requires 4 <= p <= 18");
    let x: u64 = hash64_bytes(input, 0);
    let idx: u32 = (x >> (64 - p)) as u32;
    let w: u64 = (x << p) | (1u64 << (p - 1));
    let rho: u32 = w.leading_zeros() + 1;
    (idx, rho)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Self-consistency anchors (spec §"Self-consistency anchors") -----

    #[test]
    fn anchor_hash32_zero() {
        // hash32(0, 0): seed32=0, h=0, every step 0 -> 0. Universal self-check.
        assert_eq!(hash32(0x0000_0000, 0), 0x0000_0000);
    }

    #[test]
    fn anchor_hash64_zero() {
        assert_eq!(hash64(0, 0), 0);
        assert_eq!(hash64_hi(0, 0), 0x0000_0000);
        assert_eq!(hash64_lo(0, 0), 0x0000_0000);
    }

    #[test]
    fn hash64_all_ones_logical_shift() {
        // h>>33 on all-ones is 0x000000007fffffff (top 33 bits zero): a logical,
        // not arithmetic, shift. The value below is the reference output.
        assert_eq!(hash64(0xffff_ffff_ffff_ffff, 0), 0x64b5_720b_4b82_5f21);
    }

    #[test]
    fn seed_fold_identity() {
        // seed32 = seed ^ (seed>>32). 0x00000000ffffffff -> ffffffff^00000000 =
        // ffffffff. 0xffffffff00000000 -> 00000000 ^ ffffffff = ffffffff. Equal.
        assert_eq!(
            hash32(0x1234_5678, 0x0000_0000_ffff_ffff),
            hash32(0x1234_5678, 0xffff_ffff_0000_0000)
        );
        // Two seeds that fold to DIFFERENT seed32 produce different hashes:
        // 0x...00000001 folds to 0x00000001; 0x...00000002 folds to 0x00000002.
        assert_ne!(
            hash32(0x1234_5678, 0x0000_0000_0000_0001),
            hash32(0x1234_5678, 0x0000_0000_0000_0002)
        );
        // And the high word genuinely affects the fold: high=0x00000001 makes
        // seed32 = 0x00000001 (low=0), differing from a zero seed (seed32=0).
        assert_ne!(
            hash32(0x1234_5678, 0x0000_0001_0000_0000),
            hash32(0x1234_5678, 0x0000_0000_0000_0000)
        );
    }

    // ---- Per-type encoder pins (spec §"Input → input-word derivation") ---

    #[test]
    fn i32_reinterpret_not_sign_extend() {
        // i32(-1) -> u32 0xffffffff (reinterpret).
        assert_eq!(encode_i32_word32(-1), 0xffff_ffff);
        assert_eq!(hash32_i32(-1, 0), hash32(0xffff_ffff, 0));
        // i32 INT_MIN / INT_MAX.
        assert_eq!(encode_i32_word32(i32::MIN), 0x8000_0000);
        assert_eq!(encode_i32_word32(i32::MAX), 0x7fff_ffff);
    }

    #[test]
    fn i32_zero_extend_for_hash64() {
        // i32(-1) -> u64 0x00000000ffffffff (ZERO-extend, not 0xffffffffffffffff).
        assert_eq!(encode_i32_word64(-1), 0x0000_0000_ffff_ffff);
        assert_eq!(hash64_i32(-1, 0), hash64(0x0000_0000_ffff_ffff, 0));
        // The sign-extend trap made observable: zero-extended != all-ones.
        assert_ne!(hash64_i32(-1, 0), hash64(0xffff_ffff_ffff_ffff, 0));
        assert_eq!(encode_i32_word64(i32::MIN), 0x0000_0000_8000_0000);
    }

    #[test]
    fn bytes_le_fold_32() {
        // [01 02 03 04] reads LE to lane 0x04030201 then XOR len(4).
        assert_eq!(
            encode_bytes_word32(&[0x01, 0x02, 0x03, 0x04]),
            0x0403_0201 ^ 4
        );
        // The scenario pins hash32(bytes 01020304) == hash32(word 0x04030201)
        // ONLY when length-fold is considered — so compare encoders directly.
        assert_eq!(
            hash32_bytes(&[0x01, 0x02, 0x03, 0x04], 0),
            hash32(0x0403_0201 ^ 4, 0)
        );
    }

    #[test]
    fn bytes_le_fold_64() {
        // [01..08] reads LE to lane 0x0807060504030201 then XOR len(8).
        assert_eq!(
            encode_bytes_word64(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]),
            0x0807_0605_0403_0201 ^ 8
        );
        assert_eq!(
            hash64_bytes(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], 0),
            hash64(0x0807_0605_0403_0201 ^ 8, 0)
        );
    }

    #[test]
    fn bytes_tail_and_length_distinguish() {
        // Sub-lane tail goes to LOW bytes; length XOR'd in so these all differ.
        let h3 = hash32_bytes(&[0x01, 0x02, 0x03], 0);
        let h2 = hash32_bytes(&[0x01, 0x02], 0);
        let h4 = hash32_bytes(&[0x01, 0x02, 0x03, 0x00], 0);
        assert_ne!(h3, h2);
        assert_ne!(h3, h4);
        // [00] != [00,00] (length XOR distinguishes equal-byte tails).
        assert_ne!(hash32_bytes(&[0x00], 0), hash32_bytes(&[0x00, 0x00], 0));
        // Tail in LOW bytes: [01] folds to lane 0x00000001.
        assert_eq!(encode_bytes_word32(&[0x01]), 0x0000_0001 ^ 1);
    }

    // ---- positions_from_hashes oracle rows (spec §position matrix) -------

    #[test]
    fn positions_vector_rows() {
        assert_eq!(
            positions_from_hashes(0x0000_0000, 0x0000_0001, 16, 4),
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            positions_from_hashes(0x0000_000a, 0x0000_0003, 16, 4),
            vec![10, 13, 0, 3]
        );
        assert_eq!(
            positions_from_hashes(0xffff_ffff, 0x0000_0001, 16, 3),
            vec![15, 0, 1]
        );
        // i*h2 multiply wrap: i=2, 2*0x80000000 = 0x100000000 -> 0.
        assert_eq!(
            positions_from_hashes(0x8000_0000, 0x8000_0000, 7, 3),
            vec![2, 0, 2]
        );
        // addition wrap + unsigned mod with high bit set.
        assert_eq!(
            positions_from_hashes(0xffff_fffd, 0x0000_0002, 1000, 5),
            vec![293, 295, 1, 3, 5]
        );
    }

    #[test]
    fn positions_pow2_equals_modulo() {
        // A power-of-two m must agree with plain % m (no mask shortcut here).
        let v = positions(b"hello", 64, 7);
        let h1 = hash32_bytes(b"hello", 0);
        let h2 = hash32_bytes(b"hello", SALT2);
        for (i, &p) in v.iter().enumerate() {
            let combined = h1.wrapping_add((i as u32).wrapping_mul(h2));
            assert_eq!(p, combined & 63);
            assert_eq!(p, combined % 64);
        }
    }

    #[test]
    fn positions_public_uses_internal_seeds() {
        let h1 = hash32_bytes(b"abc", 0);
        let h2 = hash32_bytes(b"abc", SALT2);
        assert_eq!(
            positions(b"abc", 1000, 5),
            positions_from_hashes(h1, h2, 1000, 5)
        );
    }

    // ---- hll_split sanity (pre-stated) -----------------------------------

    #[test]
    fn hll_split_basic() {
        // idx is the top p bits of hash64(input, 0); rho >= 1.
        let (idx, rho) = hll_split(b"x", 12);
        let x = hash64_bytes(b"x", 0);
        assert_eq!(idx, (x >> (64 - 12)) as u32);
        assert!(rho >= 1);
        assert!(idx < (1u32 << 12));
    }

    // ---- Authoritative numeric test vectors (the committed oracle) -------
    // These rows mirror spec/features/hash-pipeline.md §"Test vectors". They
    // are produced by THIS implementation and are the values committed to the
    // spec. Any change here is a conformance break.

    const HASH32_WORDS: [u32; 6] = [
        0x0000_0000,
        0x0000_0001,
        0xffff_ffff,
        0x8000_0000,
        0x7fff_ffff,
        0x0403_0201,
    ];
    const HASH32_SEEDS: [u64; 4] = [
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x0000_0000_ffff_ffff,
        0xffff_ffff_0000_0000,
    ];

    #[test]
    fn hash32_vectors() {
        // Reference outputs (row-major: word outer, seed inner).
        let expected: [[u32; 4]; 6] = [
            [0x0000_0000, 0x514e_28b7, 0x81f1_6f39, 0x81f1_6f39],
            [0x514e_28b7, 0x0000_0000, 0x7995_c304, 0x7995_c304],
            [0x81f1_6f39, 0x7995_c304, 0x0000_0000, 0x0000_0000],
            [0x6d3c_65a0, 0x8b7f_7a6a, 0xf9cc_0ea8, 0xf9cc_0ea8],
            [0xf9cc_0ea8, 0x551b_50f6, 0x6d3c_65a0, 0x6d3c_65a0],
            [0xd839_eaff, 0x54ec_0422, 0xaf02_bbbc, 0xaf02_bbbc],
        ];
        for (wi, &w) in HASH32_WORDS.iter().enumerate() {
            for (si, &s) in HASH32_SEEDS.iter().enumerate() {
                assert_eq!(
                    hash32(w, s),
                    expected[wi][si],
                    "hash32({:#010x}, {:#018x})",
                    w,
                    s
                );
            }
        }
        // The two high/low-only seeds fold to the SAME seed32 (0xffffffff), so
        // their columns are identical — an intentional, pinned property.
        for &w in &HASH32_WORDS {
            assert_eq!(
                hash32(w, 0x0000_0000_ffff_ffff),
                hash32(w, 0xffff_ffff_0000_0000)
            );
        }
    }

    const HASH64_WORDS: [u64; 6] = [
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x0000_0000_ffff_ffff,
        0x0000_0000_8000_0000,
        0xffff_ffff_ffff_ffff,
        0x0807_0605_0403_0201,
    ];
    const HASH64_SEEDS: [u64; 4] = [
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x0000_0000_ffff_ffff,
        0xffff_ffff_0000_0000,
    ];

    #[test]
    fn hash64_vectors() {
        let expected: [[u64; 4]; 6] = [
            [
                0x0000_0000_0000_0000,
                0xb456_bcfc_34c2_cb2c,
                0xcc71_ecda_2aa8_bcc6,
                0xc921_3cd2_0c52_8300,
            ],
            [
                0xb456_bcfc_34c2_cb2c,
                0x0000_0000_0000_0000,
                0x0789_620c_2ee6_4a3e,
                0x2640_647a_5ca0_376b,
            ],
            [
                0xcc71_ecda_2aa8_bcc6,
                0x0789_620c_2ee6_4a3e,
                0x0000_0000_0000_0000,
                0x64b5_720b_4b82_5f21,
            ],
            [
                0xe3be_ca1f_9a7e_4886,
                0x81b8_7531_8ee0_0b8e,
                0x8a66_2c1a_93a2_6b91,
                0xc4ca_2714_6b0a_922f,
            ],
            [
                0x64b5_720b_4b82_5f21,
                0x3a85_9388_6c55_a02b,
                0xc921_3cd2_0c52_8300,
                0xcc71_ecda_2aa8_bcc6,
            ],
            [
                0x9b57_670c_6024_0a13,
                0xda66_ed8b_c89f_fb5f,
                0xbe7f_6184_4295_15e7,
                0x916b_f52b_f4cf_0681,
            ],
        ];
        for (wi, &w) in HASH64_WORDS.iter().enumerate() {
            for (si, &s) in HASH64_SEEDS.iter().enumerate() {
                assert_eq!(
                    hash64(w, s),
                    expected[wi][si],
                    "hash64({:#018x}, {:#018x})",
                    w,
                    s
                );
            }
        }
    }
}
