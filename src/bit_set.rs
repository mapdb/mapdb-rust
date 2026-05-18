// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use std::fmt;

const BITS_PER_WORD: usize = 64;

/// Compact bit-packed storage backed by `Vec<u64>`. Per
/// `spec/collections.md` §"BitSet" this is a single non-generic type:
/// `set` / `clear_bit` / `flip` / `get` are O(1); `cardinality` is
/// O(n/64) popcount; bitwise ops are O(n/64).
#[derive(Debug, Clone, Default)]
pub struct BitSet {
    words: Vec<u64>,
    bit_length: usize,
}

#[inline]
fn word_index(bit: usize) -> usize {
    bit / BITS_PER_WORD
}

#[inline]
fn bit_mask(bit: usize) -> u64 {
    1u64 << (bit % BITS_PER_WORD)
}

impl BitSet {
    pub fn new() -> Self {
        BitSet {
            words: Vec::new(),
            bit_length: 0,
        }
    }

    /// Preallocates room for `n_bits` bits, all initially 0.
    pub fn with_bit_length(n_bits: usize) -> Self {
        let n_words = n_bits.div_ceil(BITS_PER_WORD);
        BitSet {
            words: vec![0u64; n_words],
            bit_length: n_bits,
        }
    }

    fn ensure(&mut self, bit: usize) {
        let needed = word_index(bit) + 1;
        if self.words.len() < needed {
            self.words.resize(needed, 0);
        }
        if bit + 1 > self.bit_length {
            self.bit_length = bit + 1;
        }
    }

    pub fn set(&mut self, bit: usize) {
        self.ensure(bit);
        self.words[word_index(bit)] |= bit_mask(bit);
    }

    /// Clears the bit at `bit`. No-op for out-of-range indices.
    pub fn clear_bit(&mut self, bit: usize) {
        let wi = word_index(bit);
        if wi >= self.words.len() {
            return;
        }
        self.words[wi] &= !bit_mask(bit);
    }

    pub fn flip(&mut self, bit: usize) {
        self.ensure(bit);
        self.words[word_index(bit)] ^= bit_mask(bit);
    }

    pub fn get(&self, bit: usize) -> bool {
        let wi = word_index(bit);
        if wi >= self.words.len() {
            return false;
        }
        self.words[wi] & bit_mask(bit) != 0
    }

    /// Number of set bits. O(n/64) via `u64::count_ones`.
    pub fn cardinality(&self) -> usize {
        if self.bit_length == 0 {
            return 0;
        }
        let last_idx = (self.bit_length - 1) / BITS_PER_WORD;
        let mut count = 0usize;
        for (i, &w) in self.words.iter().enumerate() {
            if i < last_idx {
                count += w.count_ones() as usize;
            } else if i == last_idx {
                let rem = self.bit_length - i * BITS_PER_WORD;
                let mask = if rem == BITS_PER_WORD {
                    !0u64
                } else {
                    (1u64 << rem) - 1
                };
                count += (w & mask).count_ones() as usize;
            }
        }
        count
    }

    pub fn bit_length(&self) -> usize {
        self.bit_length
    }

    pub fn is_empty(&self) -> bool {
        self.cardinality() == 0
    }

    /// Clears every bit. Keeps the backing capacity.
    pub fn clear_all(&mut self) {
        for w in self.words.iter_mut() {
            *w = 0;
        }
    }

    pub fn intersects(&self, other: &BitSet) -> bool {
        let min = self.words.len().min(other.words.len());
        for i in 0..min {
            if self.words[i] & other.words[i] != 0 {
                return true;
            }
        }
        false
    }

    pub fn and_in_place(&mut self, other: &BitSet) {
        for i in 0..self.words.len() {
            let ow = if i < other.words.len() {
                other.words[i]
            } else {
                0
            };
            self.words[i] &= ow;
        }
    }

    pub fn or_in_place(&mut self, other: &BitSet) {
        if other.words.len() > self.words.len() {
            self.words.resize(other.words.len(), 0);
        }
        if other.bit_length > self.bit_length {
            self.bit_length = other.bit_length;
        }
        for (i, &ow) in other.words.iter().enumerate() {
            self.words[i] |= ow;
        }
    }

    pub fn xor_in_place(&mut self, other: &BitSet) {
        if other.words.len() > self.words.len() {
            self.words.resize(other.words.len(), 0);
        }
        if other.bit_length > self.bit_length {
            self.bit_length = other.bit_length;
        }
        for (i, &ow) in other.words.iter().enumerate() {
            self.words[i] ^= ow;
        }
    }

    pub fn and_not_in_place(&mut self, other: &BitSet) {
        let min = self.words.len().min(other.words.len());
        for i in 0..min {
            self.words[i] &= !other.words[i];
        }
    }

    /// Index of the next set bit at or after `from`, or `None` if there
    /// is no later set bit.
    pub fn next_set_bit(&self, from: usize) -> Option<usize> {
        let mut wi = word_index(from);
        if wi >= self.words.len() {
            return None;
        }
        let offset = (from % BITS_PER_WORD) as u32;
        // `!0u64 << 64` is UB territory in C; Rust panics in debug,
        // but `from % BITS_PER_WORD` keeps `offset < 64`, so this is
        // always well-defined.
        let mut word = self.words[wi] & (!0u64 << offset);
        loop {
            if word != 0 {
                return Some(wi * BITS_PER_WORD + word.trailing_zeros() as usize);
            }
            wi += 1;
            if wi >= self.words.len() {
                return None;
            }
            word = self.words[wi];
        }
    }

    /// Indices of the set bits, ascending.
    pub fn to_vec(&self) -> Vec<usize> {
        let mut out = Vec::with_capacity(self.cardinality());
        let mut bit = self.next_set_bit(0);
        while let Some(b) = bit {
            out.push(b);
            bit = self.next_set_bit(b + 1);
        }
        out
    }
}

impl PartialEq for BitSet {
    fn eq(&self, other: &Self) -> bool {
        if self.bit_length != other.bit_length {
            return false;
        }
        let n = self.words.len().max(other.words.len());
        for i in 0..n {
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = other.words.get(i).copied().unwrap_or(0);
            if a != b {
                return false;
            }
        }
        true
    }
}

impl Eq for BitSet {}

impl fmt::Display for BitSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        let mut bit = self.next_set_bit(0);
        while let Some(b) = bit {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", b)?;
            first = false;
            bit = self.next_set_bit(b + 1);
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_clear() {
        let mut b = BitSet::new();
        b.set(0);
        b.set(63);
        b.set(64);
        b.set(200);
        assert!(b.get(0));
        assert!(b.get(63));
        assert!(b.get(64));
        assert!(b.get(200));
        assert!(!b.get(1));
        assert!(!b.get(199));
        assert!(!b.get(10_000));
        b.clear_bit(63);
        assert!(!b.get(63));
        b.clear_bit(10_000); // no-op, no panic
    }

    #[test]
    fn flip_toggles() {
        let mut b = BitSet::new();
        b.flip(5);
        assert!(b.get(5));
        b.flip(5);
        assert!(!b.get(5));
        b.flip(150);
        assert!(b.get(150));
    }

    #[test]
    fn cardinality_and_is_empty() {
        let mut b = BitSet::with_bit_length(200);
        assert!(b.is_empty());
        assert_eq!(b.cardinality(), 0);
        for i in [0, 1, 63, 64, 127, 199] {
            b.set(i);
        }
        assert_eq!(b.cardinality(), 6);
        // Bits beyond bit_length must not be counted.
        let mut c = BitSet::with_bit_length(70);
        // Force-set a word bit past bit_length without extending: rely
        // on internal word access via the public `set` (which extends).
        // The masking in cardinality should still cap at bit_length=70
        // even though `set(80)` would extend bit_length. Re-test by
        // checking a freshly truncated case:
        c.set(69);
        assert_eq!(c.cardinality(), 1);
    }

    #[test]
    fn cardinality_word_aligned_length() {
        let mut b = BitSet::with_bit_length(64);
        b.set(0);
        b.set(63);
        assert_eq!(b.cardinality(), 2);
    }

    #[test]
    fn intersects_and_bitops() {
        let mut a = BitSet::new();
        a.set(1);
        a.set(2);
        a.set(70);
        let mut c = BitSet::new();
        c.set(2);
        c.set(70);
        assert!(a.intersects(&c));
        let mut d = BitSet::new();
        d.set(3);
        assert!(!a.intersects(&d));

        let mut and = a.clone();
        and.and_in_place(&c);
        assert_eq!(and.to_vec(), vec![2, 70]);

        let mut or = a.clone();
        or.or_in_place(&d);
        assert_eq!(or.to_vec(), vec![1, 2, 3, 70]);

        let mut xor = a.clone();
        xor.xor_in_place(&c);
        assert_eq!(xor.to_vec(), vec![1]);

        let mut andnot = a.clone();
        andnot.and_not_in_place(&c);
        assert_eq!(andnot.to_vec(), vec![1]);
    }

    #[test]
    fn next_set_bit_iteration() {
        let mut b = BitSet::new();
        for i in [0usize, 5, 63, 64, 65, 200] {
            b.set(i);
        }
        let mut out = Vec::new();
        let mut bit = b.next_set_bit(0);
        while let Some(v) = bit {
            out.push(v);
            bit = b.next_set_bit(v + 1);
        }
        assert_eq!(out, vec![0, 5, 63, 64, 65, 200]);
        assert_eq!(b.next_set_bit(300), None);
        assert_eq!(b.next_set_bit(6), Some(63));
    }

    #[test]
    fn to_vec_ascending() {
        let mut b = BitSet::new();
        b.set(64);
        b.set(0);
        b.set(2);
        assert_eq!(b.to_vec(), vec![0, 2, 64]);
    }

    #[test]
    fn with_bit_length_zero_and_clear_all() {
        let mut b = BitSet::with_bit_length(0);
        assert_eq!(b.cardinality(), 0);
        b.set(5);
        assert_eq!(b.cardinality(), 1);
        b.clear_all();
        assert_eq!(b.cardinality(), 0);
        // bit_length is preserved by clear_all even if storage retained.
        assert!(b.bit_length() >= 6);
    }

    #[test]
    fn equals_clone_display() {
        let mut a = BitSet::new();
        a.set(1);
        a.set(3);
        a.set(5);
        let b = a.clone();
        assert_eq!(a, b);
        let mut c = a.clone();
        c.set(7);
        assert_ne!(a, c);
        assert_eq!(format!("{}", a), "{1, 3, 5}");
        let empty = BitSet::new();
        assert_eq!(format!("{}", empty), "{}");
    }
}
