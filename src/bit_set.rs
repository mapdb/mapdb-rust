// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use std::cmp::Ordering;
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

    /// Bulk-loads a fresh `BitSet` from **strictly-ascending** bit indices in a
    /// single allocation: the largest index fixes the word count, so all words
    /// are reserved once and every bit set directly (no incremental `ensure`
    /// growth). A non-ascending or repeated index is a
    /// [`BulkError::OutOfOrder`](crate::BulkError::OutOfOrder) /
    /// [`BulkError::Duplicate`](crate::BulkError::Duplicate) per `dup`.
    pub fn from_sorted_indices<I: IntoIterator<Item = usize>>(
        iter: I,
        dup: crate::bulk::DuplicatePolicy,
    ) -> Result<Self, crate::bulk::BulkError> {
        use crate::bulk::{BulkError, DuplicatePolicy};
        // Buffer once so we can size from the max index in a single allocation.
        let bits: Vec<usize> = iter.into_iter().collect();
        // Validate strict ascending order under the natural ordering of indices.
        for (i, w) in bits.windows(2).enumerate() {
            match w[0].cmp(&w[1]) {
                Ordering::Less => {}
                Ordering::Equal => match dup {
                    DuplicatePolicy::IgnoreDuplicates => {}
                    DuplicatePolicy::Error => return Err(BulkError::Duplicate { index: i + 1 }),
                },
                Ordering::Greater => return Err(BulkError::OutOfOrder { index: i + 1 }),
            }
        }
        // `max_index + 1` is the length convention; `usize::MAX` cannot be
        // represented (debug: `b + 1` panics; release: wraps to 0 then indexes
        // out of bounds). Report it as a structured error instead.
        let bit_length = match bits.last() {
            None => 0,
            Some(&b) => b
                .checked_add(1)
                .ok_or(BulkError::IndexOverflow { index: bits.len() - 1 })?,
        };
        let n_words = bit_length.div_ceil(BITS_PER_WORD);
        let mut words = vec![0u64; n_words];
        for &b in &bits {
            words[word_index(b)] |= bit_mask(b);
        }
        Ok(BitSet { words, bit_length })
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
    /// Logical-bit equality, matching `java.util.BitSet.equals`: two bit sets
    /// are equal iff they have exactly the same set bits. Capacity and history
    /// are ignored — `BitSet::new()` equals `BitSet::with_bit_length(100)`, and
    /// `set(10); clear_bit(10)` equals a never-touched empty set. (Words past a
    /// set's populated range read as 0; bits above the highest set index are
    /// never populated, so word-wise comparison is exactly logical equality.)
    fn eq(&self, other: &Self) -> bool {
        let n = self.words.len().max(other.words.len());
        (0..n).all(|i| {
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = other.words.get(i).copied().unwrap_or(0);
            a == b
        })
    }
}

impl Eq for BitSet {}

/// Iterator over the indices of the set bits, ascending.
pub struct BitSetIter<'a> {
    bitset: &'a BitSet,
    next: Option<usize>,
}

impl Iterator for BitSetIter<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        let bit = self.next?;
        self.next = self.bitset.next_set_bit(bit + 1);
        Some(bit)
    }
}

/// Iterates the indices of the set bits, ascending: `for i in &bitset`.
impl<'a> IntoIterator for &'a BitSet {
    type Item = usize;
    type IntoIter = BitSetIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        BitSetIter {
            bitset: self,
            next: self.next_set_bit(0),
        }
    }
}

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
    fn from_sorted_indices_usize_max_is_structured_error() {
        // Regression: `max_index + 1` overflows for `usize::MAX` (debug panic /
        // release out-of-bounds). Must surface a structured error instead.
        use crate::bulk::{BulkError, DuplicatePolicy};
        let err = BitSet::from_sorted_indices([usize::MAX], DuplicatePolicy::Error).unwrap_err();
        assert!(matches!(err, BulkError::IndexOverflow { index: 0 }));
        // A large-but-representable index still works.
        let b = BitSet::from_sorted_indices([10usize, 100], DuplicatePolicy::Error).unwrap();
        assert!(b.get(10) && b.get(100));
    }

    #[test]
    fn eq_is_logical_bits_only() {
        // Regression: `PartialEq` was capacity/history sensitive. It must match
        // `java.util.BitSet.equals` — only the set bits matter.
        assert_eq!(BitSet::new(), BitSet::with_bit_length(100));
        let mut b = BitSet::new();
        b.set(10);
        b.clear_bit(10);
        assert_eq!(b, BitSet::new()); // history erased
        let mut x = BitSet::with_bit_length(8);
        x.set(3);
        let mut y = BitSet::with_bit_length(500);
        y.set(3);
        assert_eq!(x, y); // same set bit, different capacity
        y.set(4);
        assert_ne!(x, y);
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

    #[test]
    fn into_iter_yields_set_bit_indices() {
        let mut b = BitSet::new();
        for i in [0usize, 5, 63, 64, 200] {
            b.set(i);
        }
        let collected: Vec<usize> = (&b).into_iter().collect();
        assert_eq!(collected, vec![0, 5, 63, 64, 200]);
        // Works in a for loop too.
        let mut sum = 0;
        for i in &b {
            sum += i;
        }
        assert_eq!(sum, 5 + 63 + 64 + 200);
    }

    #[test]
    fn from_sorted_indices_equals_incremental() {
        use crate::bulk::DuplicatePolicy;
        let idx = [0usize, 5, 63, 64, 200];
        let bulk = BitSet::from_sorted_indices(idx, DuplicatePolicy::Error).unwrap();
        let mut inc = BitSet::new();
        for &i in &idx {
            inc.set(i);
        }
        assert_eq!(bulk, inc);
        assert_eq!(bulk.cardinality(), 5);
    }

    #[test]
    fn from_sorted_indices_order_and_dup_errors() {
        use crate::bulk::{BulkError, DuplicatePolicy};
        let err = BitSet::from_sorted_indices([3usize, 1], DuplicatePolicy::Error).unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 1 }));
        let err = BitSet::from_sorted_indices([3usize, 3], DuplicatePolicy::Error).unwrap_err();
        assert!(matches!(err, BulkError::Duplicate { index: 1 }));
        // IgnoreDuplicates tolerates the repeat.
        let b =
            BitSet::from_sorted_indices([3usize, 3, 7], DuplicatePolicy::IgnoreDuplicates).unwrap();
        assert_eq!(b.cardinality(), 2);
        // empty.
        let e = BitSet::from_sorted_indices(Vec::<usize>::new(), DuplicatePolicy::Error).unwrap();
        assert_eq!(e.cardinality(), 0);
    }
}
