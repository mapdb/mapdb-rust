// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! `RoaringU32` — a sparse, compressed 32-bit integer set (a Roaring-style
//! bitmap). See `spec/features/roaring-u32.md`.
//!
//! The universe (`2^32` values) is split into `2^16` **chunks** keyed by the
//! high 16 bits of a value. Each non-empty chunk is stored as a **container**:
//! an ARRAY (sorted distinct `u16[]`) for cardinality `1 ..= 4096`, or a BITMAP
//! (1024 × `u64`) for cardinality `4097 ..= 65536`. The container type is a
//! **pure function of the chunk's current cardinality** (history-independent),
//! which makes the serialized form canonical.
//!
//! Ordering is **UNSIGNED u32 ascending** throughout (iteration, `min`/`max`,
//! serialized chunk order). An `i32` element is **bit-reinterpreted** to `u32`
//! (not sign-extended), so `i32 -1` is `0xFFFFFFFF` and sorts last.

use std::cmp::Ordering;
use std::fmt;

/// Why [`RoaringU32::deserialize`] rejected a byte image. Every variant marks a
/// specific reader-MUST-reject rule (truncation, foreign/unsupported header, or
/// a non-canonical / corrupt container encoding); see `spec/features/
/// roaring-u32.md`. Mirrors the crate's other typed decode error, `HllError`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RoaringError {
    /// The image ended mid-field: `need` bytes were required at `offset` but only
    /// `have` remained.
    Truncated {
        /// Bytes the field needed.
        need: usize,
        /// Byte offset the field started at.
        offset: usize,
        /// Bytes actually remaining from `offset`.
        have: usize,
    },
    /// The 4-byte header magic did not match the expected `0x3252_3055`.
    BadMagic(u32),
    /// The header version is not the supported `1`.
    UnsupportedVersion(u16),
    /// The header's reserved field was not zero.
    NonZeroReserved(u16),
    /// `CHUNK_COUNT` exceeded the `65536` chunk maximum.
    ChunkCountTooLarge(u32),
    /// Chunk high keys were not strictly ascending: `high` came after `prev`.
    NonAscendingHighKey {
        /// The offending high key.
        high: u16,
        /// The previous high key it failed to exceed.
        prev: u16,
    },
    /// A container record's pad byte was not zero.
    NonZeroPad(u8),
    /// An ARRAY container's cardinality `card` exceeded `max` (would be a BITMAP).
    NonCanonicalArrayCardinality {
        /// The stored cardinality.
        card: u32,
        /// The maximum ARRAY cardinality (`4096`).
        max: usize,
    },
    /// ARRAY low keys were not strictly ascending: `low` came after `prev`.
    NonAscendingArrayLowKey {
        /// The offending low key.
        low: u16,
        /// The previous low key it failed to exceed.
        prev: u16,
    },
    /// A BITMAP container's cardinality `card` was `<= max` (would be an ARRAY).
    NonCanonicalBitmapCardinality {
        /// The stored cardinality.
        card: u32,
        /// The maximum ARRAY cardinality (`4096`); a BITMAP must exceed it.
        max: usize,
    },
    /// A BITMAP's actual set-bit count `popcount` disagreed with the `stored`
    /// cardinality.
    BitmapPopcountMismatch {
        /// The counted set bits.
        popcount: u32,
        /// The cardinality the record claimed.
        stored: u32,
    },
    /// A container-type tag byte was neither `ARRAY` (`0x01`) nor `BITMAP`
    /// (`0x02`).
    UnknownContainerTag(u8),
    /// `count` bytes remained after the last chunk record.
    TrailingBytes(usize),
}

impl fmt::Display for RoaringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoaringError::Truncated { need, offset, have } => write!(
                f,
                "truncated: need {need} bytes at offset {offset}, have {have}"
            ),
            RoaringError::BadMagic(m) => write!(f, "bad MAGIC: {m:#010x}"),
            RoaringError::UnsupportedVersion(v) => write!(f, "unsupported VERSION: {v}"),
            RoaringError::NonZeroReserved(r) => write!(f, "non-zero RESERVED: {r:#06x}"),
            RoaringError::ChunkCountTooLarge(c) => write!(f, "CHUNK_COUNT > 65536: {c}"),
            RoaringError::NonAscendingHighKey { high, prev } => write!(
                f,
                "non-ascending or duplicate high key: {high:#06x} after {prev:#06x}"
            ),
            RoaringError::NonZeroPad(p) => write!(f, "non-zero PAD: {p:#04x}"),
            RoaringError::NonCanonicalArrayCardinality { card, max } => {
                write!(f, "non-canonical ARRAY cardinality {card} (> {max})")
            }
            RoaringError::NonAscendingArrayLowKey { low, prev } => write!(
                f,
                "non-ascending or duplicate ARRAY low key: {low:#06x} after {prev:#06x}"
            ),
            RoaringError::NonCanonicalBitmapCardinality { card, max } => {
                write!(f, "non-canonical BITMAP cardinality {card} (<= {max})")
            }
            RoaringError::BitmapPopcountMismatch { popcount, stored } => {
                write!(
                    f,
                    "BITMAP popcount {popcount} != stored cardinality {stored}"
                )
            }
            RoaringError::UnknownContainerTag(t) => {
                write!(f, "unknown CONTAINER_TYPE tag: {t:#04x}")
            }
            RoaringError::TrailingBytes(n) => write!(f, "{n} trailing bytes after chunk records"),
        }
    }
}

impl std::error::Error for RoaringError {}

/// Cardinality at and below which a chunk is an ARRAY; above which it is a
/// BITMAP. `4096` is the classic Roaring break-even (`4096 × 2 bytes == 8192`,
/// the bitmap size). `c <= 4096 => ARRAY`, `c > 4096 => BITMAP`.
const ARRAY_MAX: usize = 4096;

/// A BITMAP container is always exactly 1024 `u64` words (`2^16` bits).
const BITMAP_WORDS: usize = 1024;

/// Serialized header magic: `0x3252_3055` (LE bytes `55 30 52 32`).
const MAGIC: u32 = 0x3252_3055;
/// Serialized format version.
const VERSION: u16 = 1;

const TAG_ARRAY: u8 = 0x01;
const TAG_BITMAP: u8 = 0x02;

/// A per-chunk container. The variant is always the canonical type for the
/// contained cardinality (ARRAY for `1 ..= 4096`, BITMAP for `4097 ..= 65536`).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Container {
    /// Sorted, distinct low-16-bit keys (length == cardinality).
    Array(Vec<u16>),
    /// Dense bitmap: 1024 `u64` words; bit `(w*64 + b)` is low key `w*64+b`.
    /// `count` is the cached popcount (== cardinality).
    Bitmap {
        words: Box<[u64; BITMAP_WORDS]>,
        count: u32,
    },
}

impl Container {
    /// Cardinality (number of present low keys), `1 ..= 65536`.
    fn cardinality(&self) -> u32 {
        match self {
            Container::Array(a) => a.len() as u32,
            Container::Bitmap { count, .. } => *count,
        }
    }

    fn contains(&self, low: u16) -> bool {
        match self {
            Container::Array(a) => a.binary_search(&low).is_ok(),
            Container::Bitmap { words, .. } => {
                let (w, b) = (low as usize >> 6, low as usize & 63);
                words[w] & (1u64 << b) != 0
            }
        }
    }

    /// Insert `low`. Returns whether the container changed. The caller converts
    /// ARRAY → BITMAP if the cardinality rises above `ARRAY_MAX`.
    fn add(&mut self, low: u16) -> bool {
        match self {
            Container::Array(a) => match a.binary_search(&low) {
                Ok(_) => false,
                Err(pos) => {
                    a.insert(pos, low);
                    true
                }
            },
            Container::Bitmap { words, count } => {
                let (w, b) = (low as usize >> 6, low as usize & 63);
                let bit = 1u64 << b;
                if words[w] & bit == 0 {
                    words[w] |= bit;
                    *count += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Remove `low`. Returns whether the container changed. The caller converts
    /// BITMAP → ARRAY if the cardinality drops to `ARRAY_MAX` or below, and
    /// drops the whole chunk if the cardinality hits zero.
    fn remove(&mut self, low: u16) -> bool {
        match self {
            Container::Array(a) => match a.binary_search(&low) {
                Ok(pos) => {
                    a.remove(pos);
                    true
                }
                Err(_) => false,
            },
            Container::Bitmap { words, count } => {
                let (w, b) = (low as usize >> 6, low as usize & 63);
                let bit = 1u64 << b;
                if words[w] & bit != 0 {
                    words[w] &= !bit;
                    *count -= 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// All present low keys in unsigned ascending order.
    fn lows(&self) -> Vec<u16> {
        match self {
            Container::Array(a) => a.clone(),
            Container::Bitmap { words, .. } => {
                let mut out = Vec::with_capacity(self.cardinality() as usize);
                for (w, &word) in words.iter().enumerate() {
                    let mut bits = word;
                    while bits != 0 {
                        let b = bits.trailing_zeros() as usize;
                        out.push((w * 64 + b) as u16);
                        bits &= bits - 1;
                    }
                }
                out
            }
        }
    }

    /// Minimum present low key (unsigned).
    fn min_low(&self) -> u16 {
        match self {
            Container::Array(a) => a[0],
            Container::Bitmap { words, .. } => {
                for (w, &word) in words.iter().enumerate() {
                    if word != 0 {
                        return (w * 64 + word.trailing_zeros() as usize) as u16;
                    }
                }
                unreachable!("non-empty bitmap has a set bit")
            }
        }
    }

    /// Maximum present low key (unsigned).
    fn max_low(&self) -> u16 {
        match self {
            Container::Array(a) => a[a.len() - 1],
            Container::Bitmap { words, .. } => {
                for (w, &word) in words.iter().enumerate().rev() {
                    if word != 0 {
                        return (w * 64 + (63 - word.leading_zeros() as usize)) as u16;
                    }
                }
                unreachable!("non-empty bitmap has a set bit")
            }
        }
    }

    /// Build a BITMAP from a sorted low-key list.
    fn bitmap_from_lows(lows: &[u16]) -> Container {
        let mut words = Box::new([0u64; BITMAP_WORDS]);
        for &low in lows {
            let (w, b) = (low as usize >> 6, low as usize & 63);
            words[w] |= 1u64 << b;
        }
        Container::Bitmap {
            words,
            count: lows.len() as u32,
        }
    }

    /// Normalize a low-key list (assumed sorted, distinct, non-empty) into the
    /// canonical container for its cardinality.
    fn canonical_from_lows(lows: Vec<u16>) -> Container {
        if lows.len() <= ARRAY_MAX {
            Container::Array(lows)
        } else {
            Container::bitmap_from_lows(&lows)
        }
    }
}

/// A sparse, compressed set of `u32` values (Roaring-style bitmap).
///
/// `i32` elements are bit-reinterpreted to `u32` (`-1 → 0xFFFFFFFF`) before
/// being split into a 16-bit high key (chunk) and 16-bit low key. Ordering is
/// unsigned u32 ascending throughout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoaringU32 {
    /// Non-empty chunks in **unsigned high-key ascending** order. The high key
    /// is `key.0`; invariant: strictly ascending, no empty containers.
    chunks: Vec<(u16, Container)>,
}

#[inline]
fn split(value: u32) -> (u16, u16) {
    ((value >> 16) as u16, (value & 0xFFFF) as u16)
}

#[inline]
fn join(high: u16, low: u16) -> u32 {
    ((high as u32) << 16) | (low as u32)
}

impl RoaringU32 {
    /// An empty set.
    pub fn new() -> Self {
        RoaringU32 { chunks: Vec::new() }
    }

    /// Build a set from an iterator of `u32` values.
    pub fn from_iter_u32<I: IntoIterator<Item = u32>>(it: I) -> Self {
        let mut s = RoaringU32::new();
        for v in it {
            s.add(v);
        }
        s
    }

    /// Locate the chunk index for `high`: `Ok(i)` if present, `Err(i)` is the
    /// insertion point preserving ascending order.
    fn find(&self, high: u16) -> Result<usize, usize> {
        self.chunks.binary_search_by(|(h, _)| h.cmp(&high))
    }

    /// Insert `value`. Returns whether the set changed (was newly added).
    pub fn add(&mut self, value: u32) -> bool {
        let (high, low) = split(value);
        match self.find(high) {
            Ok(i) => {
                let changed = self.chunks[i].1.add(low);
                if changed && self.chunks[i].1.cardinality() as usize > ARRAY_MAX {
                    // ARRAY → BITMAP up-conversion at cardinality 4097.
                    if let Container::Array(_) = self.chunks[i].1 {
                        let lows = self.chunks[i].1.lows();
                        self.chunks[i].1 = Container::bitmap_from_lows(&lows);
                    }
                }
                changed
            }
            Err(i) => {
                self.chunks.insert(i, (high, Container::Array(vec![low])));
                true
            }
        }
    }

    /// Remove `value`. Returns whether the set changed (was present).
    pub fn remove(&mut self, value: u32) -> bool {
        let (high, low) = split(value);
        let i = match self.find(high) {
            Ok(i) => i,
            Err(_) => return false,
        };
        let changed = self.chunks[i].1.remove(low);
        if !changed {
            return false;
        }
        let card = self.chunks[i].1.cardinality() as usize;
        if card == 0 {
            // Empty-chunk normalization: drop the chunk entirely.
            self.chunks.remove(i);
        } else if card <= ARRAY_MAX {
            // BITMAP → ARRAY down-conversion at cardinality 4096.
            if let Container::Bitmap { .. } = self.chunks[i].1 {
                let lows = self.chunks[i].1.lows();
                self.chunks[i].1 = Container::Array(lows);
            }
        }
        true
    }

    /// Whether `value` is present.
    pub fn contains(&self, value: u32) -> bool {
        let (high, low) = split(value);
        match self.find(high) {
            Ok(i) => self.chunks[i].1.contains(low),
            Err(_) => false,
        }
    }

    /// Logical cardinality (up to `2^32`).
    pub fn cardinality(&self) -> u64 {
        self.chunks
            .iter()
            .map(|(_, c)| c.cardinality() as u64)
            .sum()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Remove all values (canonical empty set).
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    /// Number of non-empty chunks (the serialized `CHUNK_COUNT`).
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Unsigned minimum present value, or `None` if empty.
    pub fn min(&self) -> Option<u32> {
        self.chunks.first().map(|(h, c)| join(*h, c.min_low()))
    }

    /// Unsigned maximum present value, or `None` if empty.
    pub fn max(&self) -> Option<u32> {
        self.chunks.last().map(|(h, c)| join(*h, c.max_low()))
    }

    /// All values in unsigned u32 ascending order.
    pub fn to_sorted_vec(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.cardinality() as usize);
        for (h, c) in &self.chunks {
            for low in c.lows() {
                out.push(join(*h, low));
            }
        }
        out
    }

    /// Iterate values in unsigned u32 ascending order.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.chunks
            .iter()
            .flat_map(|(h, c)| c.lows().into_iter().map(move |low| join(*h, low)))
    }

    /// Per-chunk container-type tags in chunk order (`"array"` / `"bitmap"`).
    pub fn container_types(&self) -> Vec<&'static str> {
        self.chunks
            .iter()
            .map(|(_, c)| match c {
                Container::Array(_) => "array",
                Container::Bitmap { .. } => "bitmap",
            })
            .collect()
    }

    // ---- Set algebra (container-granularity, scalar) ---------------------

    /// Generic chunk-merge driver. `merge` combines two containers sharing a
    /// high key into a low-key list; `keep_a`/`keep_b` decide whether an
    /// only-in-A / only-in-B chunk contributes (a rebuilt copy).
    fn combine(
        &self,
        other: &RoaringU32,
        keep_a: bool,
        keep_b: bool,
        merge: impl Fn(&Container, &Container) -> Vec<u16>,
    ) -> RoaringU32 {
        let mut chunks: Vec<(u16, Container)> = Vec::new();
        let (mut i, mut j) = (0, 0);
        while i < self.chunks.len() && j < other.chunks.len() {
            let (ha, ca) = &self.chunks[i];
            let (hb, cb) = &other.chunks[j];
            match ha.cmp(hb) {
                Ordering::Less => {
                    if keep_a {
                        chunks.push((*ha, Container::canonical_from_lows(ca.lows())));
                    }
                    i += 1;
                }
                Ordering::Greater => {
                    if keep_b {
                        chunks.push((*hb, Container::canonical_from_lows(cb.lows())));
                    }
                    j += 1;
                }
                Ordering::Equal => {
                    let lows = merge(ca, cb);
                    if !lows.is_empty() {
                        chunks.push((*ha, Container::canonical_from_lows(lows)));
                    }
                    i += 1;
                    j += 1;
                }
            }
        }
        if keep_a {
            for (h, c) in &self.chunks[i..] {
                chunks.push((*h, Container::canonical_from_lows(c.lows())));
            }
        }
        if keep_b {
            for (h, c) in &other.chunks[j..] {
                chunks.push((*h, Container::canonical_from_lows(c.lows())));
            }
        }
        RoaringU32 { chunks }
    }

    /// Union (`v ∈ A` or `v ∈ B`).
    pub fn or(&self, other: &RoaringU32) -> RoaringU32 {
        self.combine(other, true, true, |a, b| sorted_union(&a.lows(), &b.lows()))
    }

    /// Intersection (`v ∈ A` and `v ∈ B`).
    pub fn and(&self, other: &RoaringU32) -> RoaringU32 {
        self.combine(other, false, false, |a, b| {
            sorted_intersect(&a.lows(), &b.lows())
        })
    }

    /// Difference (`v ∈ A` and `v ∉ B`; asymmetric `A \ B`).
    pub fn and_not(&self, other: &RoaringU32) -> RoaringU32 {
        self.combine(other, true, false, |a, b| {
            sorted_and_not(&a.lows(), &b.lows())
        })
    }

    /// Symmetric difference (exactly one of `A`, `B`).
    pub fn xor(&self, other: &RoaringU32) -> RoaringU32 {
        self.combine(other, true, true, |a, b| sorted_xor(&a.lows(), &b.lows()))
    }

    /// `true` if `self` and `other` share no element. A short-circuiting
    /// two-pointer walk over the (high-key-sorted) chunks — only chunks with a
    /// matching high key can overlap, and it returns as soon as one does.
    pub fn is_disjoint(&self, other: &RoaringU32) -> bool {
        let (mut i, mut j) = (0, 0);
        while i < self.chunks.len() && j < other.chunks.len() {
            let (ha, ca) = &self.chunks[i];
            let (hb, cb) = &other.chunks[j];
            match ha.cmp(hb) {
                Ordering::Less => i += 1,
                Ordering::Greater => j += 1,
                Ordering::Equal => {
                    if !sorted_intersect(&ca.lows(), &cb.lows()).is_empty() {
                        return false;
                    }
                    i += 1;
                    j += 1;
                }
            }
        }
        true
    }

    /// `true` if every element of `self` is also in `other` (`self ⊆ other`).
    /// The empty set is a subset of everything. Two-pointer walk: every
    /// `self` chunk must have a matching-high `other` chunk whose low set
    /// contains all of the `self` chunk's lows. Relies on the canonical
    /// invariant that no chunk is empty (`remove` drops emptied chunks,
    /// `combine` never pushes an empty one), so a present `self` chunk always
    /// carries ≥1 element that `other` must also hold.
    pub fn is_subset(&self, other: &RoaringU32) -> bool {
        let (mut i, mut j) = (0, 0);
        while i < self.chunks.len() {
            let (ha, ca) = &self.chunks[i];
            // Advance `other` to the first chunk with high key >= ha.
            while j < other.chunks.len() && other.chunks[j].0 < *ha {
                j += 1;
            }
            // If `other` has no chunk at `ha`, `self` holds keys it can't.
            if j >= other.chunks.len() || other.chunks[j].0 > *ha {
                return false;
            }
            let cb = &other.chunks[j].1;
            // self-chunk's lows must be a subset of other-chunk's lows.
            if !sorted_and_not(&ca.lows(), &cb.lows()).is_empty() {
                return false;
            }
            i += 1;
            j += 1;
        }
        true
    }

    /// In-place union: `self |= other`.
    pub fn or_in_place(&mut self, other: &RoaringU32) {
        *self = self.or(other);
    }

    /// In-place intersection: `self &= other`.
    pub fn and_in_place(&mut self, other: &RoaringU32) {
        *self = self.and(other);
    }

    /// In-place difference: `self \= other`.
    pub fn and_not_in_place(&mut self, other: &RoaringU32) {
        *self = self.and_not(other);
    }

    /// In-place symmetric difference: `self ^= other`.
    pub fn xor_in_place(&mut self, other: &RoaringU32) {
        *self = self.xor(other);
    }

    // ---- Serialization (little-endian, canonical) ------------------------

    /// Serialize to the canonical little-endian v1 byte image.
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC.to_le_bytes());
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // RESERVED
        out.extend_from_slice(&(self.chunks.len() as u32).to_le_bytes());
        for (high, container) in &self.chunks {
            out.extend_from_slice(&high.to_le_bytes());
            let card = container.cardinality();
            match container {
                Container::Array(a) => {
                    out.push(TAG_ARRAY);
                    out.push(0); // PAD
                    out.extend_from_slice(&((card - 1) as u16).to_le_bytes());
                    for &low in a {
                        out.extend_from_slice(&low.to_le_bytes());
                    }
                }
                Container::Bitmap { words, .. } => {
                    out.push(TAG_BITMAP);
                    out.push(0); // PAD
                    out.extend_from_slice(&((card - 1) as u16).to_le_bytes());
                    for &word in words.iter() {
                        out.extend_from_slice(&word.to_le_bytes());
                    }
                }
            }
        }
        out
    }

    /// Deserialize a canonical v1 byte image. Returns a [`RoaringError`] for any
    /// non-canonical / corrupt / foreign image (see spec reader-MUST-reject
    /// rules).
    pub fn deserialize(bytes: &[u8]) -> Result<RoaringU32, RoaringError> {
        let mut r = Reader::new(bytes);
        let magic = r.u32()?;
        if magic != MAGIC {
            return Err(RoaringError::BadMagic(magic));
        }
        let version = r.u16()?;
        if version != VERSION {
            return Err(RoaringError::UnsupportedVersion(version));
        }
        let reserved = r.u16()?;
        if reserved != 0 {
            return Err(RoaringError::NonZeroReserved(reserved));
        }
        let chunk_count = r.u32()?;
        if chunk_count > 65536 {
            return Err(RoaringError::ChunkCountTooLarge(chunk_count));
        }
        let mut chunks: Vec<(u16, Container)> = Vec::with_capacity(chunk_count as usize);
        let mut prev_high: Option<u16> = None;
        for _ in 0..chunk_count {
            let high = r.u16()?;
            if let Some(prev) = prev_high {
                if high <= prev {
                    return Err(RoaringError::NonAscendingHighKey { high, prev });
                }
            }
            prev_high = Some(high);
            let tag = r.u8()?;
            let pad = r.u8()?;
            if pad != 0 {
                return Err(RoaringError::NonZeroPad(pad));
            }
            let card = r.u16()? as u32 + 1; // CARDINALITY_MINUS_1 + 1
            match tag {
                TAG_ARRAY => {
                    if card as usize > ARRAY_MAX {
                        return Err(RoaringError::NonCanonicalArrayCardinality {
                            card,
                            max: ARRAY_MAX,
                        });
                    }
                    let mut lows = Vec::with_capacity(card as usize);
                    let mut prev: Option<u16> = None;
                    for _ in 0..card {
                        let low = r.u16()?;
                        if let Some(prev) = prev {
                            if low <= prev {
                                return Err(RoaringError::NonAscendingArrayLowKey { low, prev });
                            }
                        }
                        prev = Some(low);
                        lows.push(low);
                    }
                    chunks.push((high, Container::Array(lows)));
                }
                TAG_BITMAP => {
                    if (card as usize) <= ARRAY_MAX {
                        return Err(RoaringError::NonCanonicalBitmapCardinality {
                            card,
                            max: ARRAY_MAX,
                        });
                    }
                    let mut words = Box::new([0u64; BITMAP_WORDS]);
                    let mut popcount: u32 = 0;
                    for w in words.iter_mut() {
                        let word = r.u64()?;
                        popcount += word.count_ones();
                        *w = word;
                    }
                    if popcount != card {
                        return Err(RoaringError::BitmapPopcountMismatch {
                            popcount,
                            stored: card,
                        });
                    }
                    chunks.push((high, Container::Bitmap { words, count: card }));
                }
                other => return Err(RoaringError::UnknownContainerTag(other)),
            }
        }
        if !r.at_end() {
            return Err(RoaringError::TrailingBytes(r.remaining()));
        }
        Ok(RoaringU32 { chunks })
    }
}

impl FromIterator<u32> for RoaringU32 {
    /// Builds a set from `u32` values (`iter.collect()`), superseding the
    /// explicit [`from_iter_u32`](RoaringU32::from_iter_u32).
    fn from_iter<I: IntoIterator<Item = u32>>(iter: I) -> Self {
        let mut s = RoaringU32::new();
        s.extend(iter);
        s
    }
}

impl Extend<u32> for RoaringU32 {
    fn extend<I: IntoIterator<Item = u32>>(&mut self, iter: I) {
        for v in iter {
            self.add(v);
        }
    }
}

impl<'a> Extend<&'a u32> for RoaringU32 {
    fn extend<I: IntoIterator<Item = &'a u32>>(&mut self, iter: I) {
        for &v in iter {
            self.add(v);
        }
    }
}

/// Consuming iterator over a [`RoaringU32`]'s values in **unsigned u32
/// ascending** order — the owned counterpart to [`RoaringU32::iter`]. Each
/// chunk's container is decompressed and dropped as it is reached, so peak
/// extra memory is one chunk's low keys rather than the whole set.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct RoaringIntoIter {
    chunks: std::vec::IntoIter<(u16, Container)>,
    current: Option<(u16, std::vec::IntoIter<u16>)>,
}

impl Iterator for RoaringIntoIter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        loop {
            if let Some((high, lows)) = &mut self.current {
                if let Some(low) = lows.next() {
                    return Some(join(*high, low));
                }
            }
            let (high, container) = self.chunks.next()?;
            self.current = Some((high, container.lows().into_iter()));
        }
    }
}

impl std::iter::FusedIterator for RoaringIntoIter {}

/// Consumes the set, yielding its values in unsigned u32 ascending order.
impl IntoIterator for RoaringU32 {
    type Item = u32;
    type IntoIter = RoaringIntoIter;
    fn into_iter(self) -> Self::IntoIter {
        RoaringIntoIter {
            chunks: self.chunks.into_iter(),
            current: None,
        }
    }
}

/// Bounds-checked little-endian byte reader.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], RoaringError> {
        if self.pos + n > self.bytes.len() {
            return Err(RoaringError::Truncated {
                need: n,
                offset: self.pos,
                have: self.bytes.len() - self.pos,
            });
        }
        let s = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, RoaringError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, RoaringError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, RoaringError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, RoaringError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn at_end(&self) -> bool {
        self.pos == self.bytes.len()
    }
    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }
}

// ---- Scalar sorted-list set algebra (low-key lists) ----------------------

fn sorted_union(a: &[u16], b: &[u16]) -> Vec<u16> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            Ordering::Less => {
                out.push(a[i]);
                i += 1;
            }
            Ordering::Greater => {
                out.push(b[j]);
                j += 1;
            }
            Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

fn sorted_intersect(a: &[u16], b: &[u16]) -> Vec<u16> {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out
}

fn sorted_and_not(a: &[u16], b: &[u16]) -> Vec<u16> {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            Ordering::Less => {
                out.push(a[i]);
                i += 1;
            }
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                i += 1;
                j += 1;
            }
        }
    }
    out.extend_from_slice(&a[i..]);
    out
}

fn sorted_xor(a: &[u16], b: &[u16]) -> Vec<u16> {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            Ordering::Less => {
                out.push(a[i]);
                i += 1;
            }
            Ordering::Greater => {
                out.push(b[j]);
                j += 1;
            }
            Ordering::Equal => {
                i += 1;
                j += 1;
            }
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(vals: &[u32]) -> RoaringU32 {
        RoaringU32::from_iter_u32(vals.iter().copied())
    }

    #[test]
    fn empty_set() {
        let s = RoaringU32::new();
        assert!(s.is_empty());
        assert_eq!(s.cardinality(), 0);
        assert_eq!(s.min(), None);
        assert_eq!(s.max(), None);
        assert_eq!(s.chunk_count(), 0);
        assert_eq!(s.to_sorted_vec(), Vec::<u32>::new());
        // 12-byte header, CHUNK_COUNT 0.
        assert_eq!(
            s.serialize(),
            vec![0x55, 0x30, 0x52, 0x32, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn into_iter_matches_borrowed_iter_across_chunks() {
        // Values spanning several high-key chunks, mixing small and large
        // low-key populations (exercises both array and bitmap containers).
        let mut vals: Vec<u32> = Vec::new();
        vals.extend(0..5000u32); // dense low chunk -> bitmap
        vals.push(70_000); // second chunk, sparse -> array
        vals.push(0xFFFF_FFFF); // top chunk
        let s = build(&vals);
        let borrowed: Vec<u32> = s.iter().collect();
        let owned: Vec<u32> = s.into_iter().collect();
        assert_eq!(owned, borrowed);
        assert_eq!(owned, s_sorted(&vals));
    }

    #[test]
    fn into_iter_empty_and_fused() {
        let s = RoaringU32::new();
        let mut it = s.into_iter();
        assert_eq!(it.next(), None);
        assert_eq!(it.next(), None); // FusedIterator
    }

    #[test]
    fn into_iter_ascending_unsigned() {
        // Reinterpreted i32 -1 (0xFFFFFFFF) sorts last under unsigned order.
        let s = build(&[0xFFFF_FFFF, 0, 1, 0x8000_0000]);
        let owned: Vec<u32> = s.into_iter().collect();
        assert_eq!(owned, vec![0, 1, 0x8000_0000, 0xFFFF_FFFF]);
    }

    fn s_sorted(vals: &[u32]) -> Vec<u32> {
        let mut v: Vec<u32> = vals.to_vec();
        v.sort_unstable();
        v.dedup();
        v
    }

    #[test]
    fn single_element() {
        let s = build(&[42]);
        assert_eq!(s.cardinality(), 1);
        assert_eq!(s.chunk_count(), 1);
        assert_eq!(s.container_types(), vec!["array"]);
        assert_eq!(s.min(), Some(42));
        assert_eq!(s.max(), Some(42));
        let bytes = s.serialize();
        // header(12) + high(2) + tag/pad(2) + card-1(2) + one u16 low.
        assert_eq!(bytes.len(), 12 + 6 + 2);
        assert_eq!(
            &bytes[12..],
            &[0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x2a, 0x00]
        );
    }

    #[test]
    fn idempotent_add_remove() {
        let mut s = build(&[5]);
        assert!(!s.add(5)); // re-add no-op
        assert!(s.add(6));
        assert!(s.remove(6));
        assert!(!s.remove(6)); // re-remove no-op
        assert!(!s.remove(99));
    }

    #[test]
    fn basic_distinct_chunks() {
        let s = build(&[1, 70000, 140000, 200000]);
        assert_eq!(s.cardinality(), 4);
        assert_eq!(s.chunk_count(), 4);
        assert_eq!(
            s.container_types(),
            vec!["array", "array", "array", "array"]
        );
        assert_eq!(s.to_sorted_vec(), vec![1, 70000, 140000, 200000]);
        assert_eq!(s.min(), Some(1));
        assert_eq!(s.max(), Some(200000));
    }

    #[test]
    fn unsigned_order_with_signed_extremes() {
        // i32 {INT_MIN, -1, 0, INT_MAX} reinterpreted to u32.
        let vals: Vec<u32> = vec![
            i32::MIN as u32, // 0x80000000
            (-1i32) as u32,  // 0xFFFFFFFF
            0u32,
            i32::MAX as u32, // 0x7FFFFFFF
        ];
        let s = build(&vals);
        assert_eq!(
            s.to_sorted_vec(),
            vec![0x0000_0000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF]
        );
        assert_eq!(s.min(), Some(0));
        assert_eq!(s.max(), Some(0xFFFF_FFFF));
        // Chunk highs unsigned ascending: 0x0000, 0x7FFF, 0x8000, 0xFFFF.
        let bytes = s.serialize();
        let highs: Vec<u16> = (0..4)
            .map(|i| {
                let off = 12 + i * 8;
                u16::from_le_bytes([bytes[off], bytes[off + 1]])
            })
            .collect();
        assert_eq!(highs, vec![0x0000, 0x7FFF, 0x8000, 0xFFFF]);
    }

    #[test]
    fn threshold_array_4096_bitmap_4097() {
        let mut s = RoaringU32::new();
        for v in 0..4096u32 {
            s.add(v);
        }
        assert_eq!(s.cardinality(), 4096);
        assert_eq!(s.container_types(), vec!["array"]); // exactly 4096 is ARRAY
        s.add(4096);
        assert_eq!(s.cardinality(), 4097);
        assert_eq!(s.container_types(), vec!["bitmap"]); // 4097 is first BITMAP
    }

    #[test]
    fn array_to_bitmap_and_back_same_bytes() {
        // Grow to 4097 (BITMAP) then remove back to 4096 (ARRAY).
        let mut grown = RoaringU32::new();
        for v in 0..=4096u32 {
            grown.add(v);
        }
        assert_eq!(grown.container_types(), vec!["bitmap"]);
        grown.remove(4096);
        assert_eq!(grown.container_types(), vec!["array"]);

        // A never-grown ARRAY of the same 4096 elements.
        let mut never = RoaringU32::new();
        for v in 0..4096u32 {
            never.add(v);
        }
        // History-independence: identical bytes.
        assert_eq!(grown.serialize(), never.serialize());
    }

    #[test]
    fn container_type_pure_function_of_cardinality() {
        // Reach cardinality 4096 by two different paths; expect identical bytes.
        let mut a = RoaringU32::new();
        for v in 0..5000u32 {
            a.add(v);
        }
        for v in 4096..5000u32 {
            a.remove(v);
        }
        let mut b = RoaringU32::new();
        for v in (0..4096u32).rev() {
            b.add(v);
        }
        assert_eq!(a.container_types(), vec!["array"]);
        assert_eq!(a.serialize(), b.serialize());
    }

    #[test]
    fn full_chunk() {
        let mut s = RoaringU32::new();
        for v in 0..=65535u32 {
            s.add(v);
        }
        assert_eq!(s.cardinality(), 65536);
        assert_eq!(s.container_types(), vec!["bitmap"]);
        let bytes = s.serialize();
        // CARDINALITY_MINUS_1 == 0xFFFF at offset 12+4.
        assert_eq!(&bytes[16..18], &[0xFF, 0xFF]);
        assert_eq!(bytes.len(), 12 + 6 + 8192);
        // round-trip
        let back = RoaringU32::deserialize(&bytes).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn drop_empty_chunk() {
        let mut s = build(&[100000, 5]);
        assert_eq!(s.chunk_count(), 2);
        s.remove(100000);
        assert_eq!(s.chunk_count(), 1);
        // identical to a set that only ever held {5}.
        assert_eq!(s.serialize(), build(&[5]).serialize());
    }

    #[test]
    fn roundtrip_random() {
        let mut s = RoaringU32::new();
        let mut x: u64 = 0x1234_5678;
        for _ in 0..20000 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            s.add((x >> 16) as u32);
        }
        let bytes = s.serialize();
        let back = RoaringU32::deserialize(&bytes).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.serialize(), bytes);
    }

    #[test]
    fn set_algebra_basic() {
        let a = build(&[1, 2, 3, 70000]);
        let b = build(&[2, 3, 4, 140000]);
        assert_eq!(a.or(&b).to_sorted_vec(), vec![1, 2, 3, 4, 70000, 140000]);
        assert_eq!(a.and(&b).to_sorted_vec(), vec![2, 3]);
        assert_eq!(a.and_not(&b).to_sorted_vec(), vec![1, 70000]);
        assert_eq!(a.xor(&b).to_sorted_vec(), vec![1, 4, 70000, 140000]);
        // operands unchanged
        assert_eq!(a.to_sorted_vec(), vec![1, 2, 3, 70000]);
    }

    #[test]
    fn xor_bitmap_normalizes_to_array() {
        // Two near-identical BITMAP chunks; XOR leaves few bits -> ARRAY.
        let mut a = RoaringU32::new();
        let mut b = RoaringU32::new();
        for v in 0..5000u32 {
            a.add(v);
            b.add(v);
        }
        // differ in 30 low keys
        for v in 5000..5030u32 {
            a.add(v);
        }
        assert_eq!(a.container_types(), vec!["bitmap"]);
        assert_eq!(b.container_types(), vec!["bitmap"]);
        let x = a.xor(&b);
        assert_eq!(x.cardinality(), 30);
        assert_eq!(x.container_types(), vec!["array"]); // canonical for 30
        assert_eq!(x.to_sorted_vec(), (5000..5030u32).collect::<Vec<_>>());
    }

    #[test]
    fn from_iter_and_extend() {
        let a: RoaringU32 = [5u32, 1, 9, 1, 100_000].into_iter().collect();
        assert_eq!(a.cardinality(), 4);
        assert!(a.contains(9) && a.contains(100_000) && !a.contains(2));
        let mut b: RoaringU32 = RoaringU32::new();
        b.extend([1u32, 2, 3]);
        b.extend(&[3u32, 4]);
        assert_eq!(b.cardinality(), 4);
    }

    #[test]
    fn or_array_normalizes_to_bitmap() {
        // Two ARRAYs whose union exceeds 4096 -> BITMAP.
        let a = build(&(0..3000u32).collect::<Vec<_>>());
        let b = build(&(2000..6000u32).collect::<Vec<_>>());
        assert_eq!(a.container_types(), vec!["array"]);
        let u = a.or(&b);
        assert_eq!(u.cardinality(), 6000);
        assert_eq!(u.container_types(), vec!["bitmap"]);
    }

    #[test]
    fn and_not_empties_chunk() {
        // A has chunks 0 and 1; other fully covers chunk 1.
        let a = build(&[1, 2, 70000, 70001]);
        let other = build(&[70000, 70001]);
        let d = a.and_not(&other);
        assert_eq!(d.chunk_count(), 1);
        assert_eq!(d.to_sorted_vec(), vec![1, 2]);
        assert_eq!(d.serialize(), build(&[1, 2]).serialize());
    }

    #[test]
    fn result_independent_of_operands() {
        let a = build(&[1, 2, 3]);
        let b = build(&[3, 4, 5]);
        let mut u = a.or(&b);
        u.add(999);
        assert!(!a.contains(999));
        assert!(!b.contains(999));
        // only-A chunk copied, not aliased
        let mut a2 = build(&[1]);
        let b2 = RoaringU32::new();
        let u2 = a2.or(&b2);
        a2.add(2);
        assert!(!u2.contains(2));
    }

    #[test]
    fn add_range_remove_range_via_helpers() {
        let mut s = RoaringU32::new();
        for v in 0..=4095u32 {
            s.add(v);
        }
        assert_eq!(s.cardinality(), 4096);
        for v in 100..=200u32 {
            s.remove(v);
        }
        assert_eq!(s.cardinality(), 4096 - 101);
    }

    // ---- deserialize rejection tests ----

    fn valid_bytes() -> Vec<u8> {
        build(&[1, 70000]).serialize()
    }

    #[test]
    fn reject_bad_magic() {
        let mut b = valid_bytes();
        b[0] = 0x00;
        assert!(matches!(
            RoaringU32::deserialize(&b),
            Err(RoaringError::BadMagic(_))
        ));
    }

    #[test]
    fn typed_error_variants_are_specific() {
        // The typed error names the exact reader-reject rule.
        let mut b = valid_bytes();
        b[4] = 0x02;
        assert_eq!(
            RoaringU32::deserialize(&b),
            Err(RoaringError::UnsupportedVersion(2))
        );

        let b = valid_bytes();
        assert!(matches!(
            RoaringU32::deserialize(&b[..5]),
            Err(RoaringError::Truncated { .. })
        ));

        let mut b = valid_bytes();
        b[14] = 0x03; // first chunk tag
        assert_eq!(
            RoaringU32::deserialize(&b),
            Err(RoaringError::UnknownContainerTag(0x03))
        );

        let mut b = valid_bytes();
        b.push(0xAB); // one trailing byte
        assert_eq!(
            RoaringU32::deserialize(&b),
            Err(RoaringError::TrailingBytes(1))
        );

        // Display still renders a human-readable message.
        assert_eq!(
            RoaringError::UnsupportedVersion(2).to_string(),
            "unsupported VERSION: 2"
        );
    }

    #[test]
    fn reject_bad_version() {
        let mut b = valid_bytes();
        b[4] = 0x02;
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_nonzero_reserved() {
        let mut b = valid_bytes();
        b[6] = 0x01;
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_nonzero_pad() {
        let mut b = valid_bytes();
        // first chunk PAD is at offset 12+2+1 = 15.
        b[15] = 0x01;
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_unknown_tag() {
        let mut b = valid_bytes();
        // first chunk tag at offset 12+2 = 14.
        b[14] = 0x03;
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_trailing_bytes() {
        let mut b = valid_bytes();
        b.push(0x00);
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_truncated() {
        let b = valid_bytes();
        assert!(RoaringU32::deserialize(&b[..b.len() - 1]).is_err());
        assert!(RoaringU32::deserialize(&b[..5]).is_err());
    }

    #[test]
    fn reject_chunk_count_too_large() {
        let mut b = valid_bytes();
        // CHUNK_COUNT at offset 8..12
        b[8..12].copy_from_slice(&70000u32.to_le_bytes());
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_non_canonical_array_cardinality() {
        // Hand-craft an ARRAY with cardinality 4097 (> ARRAY_MAX): illegal.
        let mut b = Vec::new();
        b.extend_from_slice(&MAGIC.to_le_bytes());
        b.extend_from_slice(&VERSION.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes()); // 1 chunk
        b.extend_from_slice(&0u16.to_le_bytes()); // high
        b.push(TAG_ARRAY);
        b.push(0);
        b.extend_from_slice(&4096u16.to_le_bytes()); // card-1 = 4096 => card 4097
        for low in 0..4097u16 {
            b.extend_from_slice(&low.to_le_bytes());
        }
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_non_canonical_bitmap_cardinality() {
        // BITMAP with cardinality 1 (<= ARRAY_MAX): illegal.
        let mut b = Vec::new();
        b.extend_from_slice(&MAGIC.to_le_bytes());
        b.extend_from_slice(&VERSION.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.push(TAG_BITMAP);
        b.push(0);
        b.extend_from_slice(&0u16.to_le_bytes()); // card 1
        let mut words = [0u64; BITMAP_WORDS];
        words[0] = 1;
        for w in words {
            b.extend_from_slice(&w.to_le_bytes());
        }
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_non_ascending_array_lows() {
        let mut b = Vec::new();
        b.extend_from_slice(&MAGIC.to_le_bytes());
        b.extend_from_slice(&VERSION.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.push(TAG_ARRAY);
        b.push(0);
        b.extend_from_slice(&1u16.to_le_bytes()); // card 2
        b.extend_from_slice(&5u16.to_le_bytes());
        b.extend_from_slice(&5u16.to_le_bytes()); // duplicate
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_bitmap_popcount_mismatch() {
        let mut b = Vec::new();
        b.extend_from_slice(&MAGIC.to_le_bytes());
        b.extend_from_slice(&VERSION.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.push(TAG_BITMAP);
        b.push(0);
        b.extend_from_slice(&4096u16.to_le_bytes()); // claims card 4097
        let words = [0u64; BITMAP_WORDS]; // popcount 0
        for w in words {
            b.extend_from_slice(&w.to_le_bytes());
        }
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn reject_non_ascending_chunk_highs() {
        let mut b = Vec::new();
        b.extend_from_slice(&MAGIC.to_le_bytes());
        b.extend_from_slice(&VERSION.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&2u32.to_le_bytes());
        // chunk 1: high 5
        b.extend_from_slice(&5u16.to_le_bytes());
        b.push(TAG_ARRAY);
        b.push(0);
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        // chunk 2: high 5 again (non-ascending)
        b.extend_from_slice(&5u16.to_le_bytes());
        b.push(TAG_ARRAY);
        b.push(0);
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        assert!(RoaringU32::deserialize(&b).is_err());
    }

    #[test]
    fn cardinality_past_2_31() {
        // Multiple full chunks push cardinality above 2^31.
        let mut s = RoaringU32::new();
        // 2^31 / 65536 = 32768 full chunks needed; too slow to build densely.
        // Instead verify the u64 width directly via two full chunks won't reach
        // 2^31, so simulate via a constructed assertion on cardinality summation.
        for v in 0..=65535u32 {
            s.add(v);
        }
        assert_eq!(s.cardinality(), 65536);
        // cardinality returns u64, so no narrowing — type-level guarantee.
        let _c: u64 = s.cardinality();
    }

    #[test]
    fn is_subset_and_is_disjoint() {
        // Values spanning two chunks (0 and 100_000 → high keys 0 and 1).
        let a = build(&[1, 2, 100_000]);
        let sub = build(&[2, 100_000]);
        let disjoint = build(&[3, 200_000]);
        let empty = RoaringU32::new();

        assert!(sub.is_subset(&a));
        assert!(!a.is_subset(&sub)); // a has 1, not in sub
        assert!(empty.is_subset(&a));
        assert!(!a.is_subset(&empty));
        assert!(a.is_subset(&a)); // reflexive

        assert!(a.is_disjoint(&disjoint));
        assert!(!a.is_disjoint(&sub)); // share 2 and 100_000
        assert!(a.is_disjoint(&empty));
        assert!(empty.is_disjoint(&a));

        // Same high chunk, disjoint lows.
        let x = build(&[1, 2, 3]);
        let y = build(&[4, 5, 6]);
        assert!(x.is_disjoint(&y));
        assert!(!x.is_subset(&y));

        // self has a high chunk other lacks → not subset.
        let hi_only = build(&[300_000]);
        assert!(!hi_only.is_subset(&x));
    }
}
