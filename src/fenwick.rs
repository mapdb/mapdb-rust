// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Fenwick tree / Binary Indexed Tree (prefix & range sums).
//!
//! A fixed-size index structure with O(log n) point-update and O(log n)
//! prefix/range sum over signed `i32` element values accumulated in a wrapping
//! `i64` accumulator. See `spec/features/fenwick.md` for the pinned design.
//!
//! Pinned invariants realized here:
//! - **Indexing**: the public API is 0-based (`0 ..= n-1`); the BIT is
//!   classically 1-based internally (`internal = public + 1`). The 1-based index
//!   is never observable. The backing array is length `n + 1` with slot 0 unused.
//! - **Ranges**: `prefix_sum(i)` is the INCLUSIVE prefix `[0..=i]`;
//!   `range_sum(lo, hi)` is the INCLUSIVE closed range `[lo..=hi]`;
//!   `total() == prefix_sum(n-1)` (and `0` for the empty tree).
//! - **Accumulator**: each slot and every sum is a wrapping two's-complement
//!   `i64` (`wrapping_add` / `wrapping_sub`). The per-element value widens to
//!   `i64` and does NOT re-wrap at `i32`, so `get` returns `i64`.
//! - **Out-of-range**: mutators (`update`/`set`), `get`, and `prefix_sum` panic
//!   on an out-of-domain index. `range_sum` validates BOTH endpoints first
//!   (out-of-domain endpoint panics), THEN returns `0` for an empty `lo > hi`
//!   range. (Rust indexes with `usize`, so only `i >= n` is representable —
//!   `i < 0` is a type error, not a runtime trap.)

/// A Fenwick tree (Binary Indexed Tree) over `i32` element values with a
/// wrapping `i64` accumulator. Fixed size; no resize.
///
/// The backing array `tree` has length `n + 1`: slot `0` is the unused BIT
/// terminator and `tree[1 ..= n]` are the 1-based partial sums.
#[derive(Clone, Debug)]
pub struct FenwickTree {
    /// 1-based partial sums; `tree[0]` unused. Length is `n + 1`.
    tree: Vec<i64>,
    /// Public size `n` (number of valid 0-based indices).
    n: usize,
}

impl FenwickTree {
    /// Construct an all-zero tree of size `n`. `with_size(0)` is a valid empty
    /// tree (`total() == 0`, `is_empty() == true`).
    pub fn with_size(n: usize) -> Self {
        FenwickTree {
            tree: vec![0i64; n + 1],
            n,
        }
    }

    /// Build from an initial `i32` array; the tree has `size == values.len()`
    /// and `get(i) == values[i]`. Uses the O(n) in-place build (it produces the
    /// identical tree as `with_size(len)` then `update(i, values[i])`).
    pub fn from_values(values: &[i32]) -> Self {
        let n = values.len();
        let mut tree = vec![0i64; n + 1];
        // Seed each 1-based slot with the (widened) element value.
        for (i, &v) in values.iter().enumerate() {
            tree[i + 1] = v as i64;
        }
        // O(n) in-place build: push each slot's running sum to its parent.
        // Over the 1-based array: parent = i + (i & -i).
        for i in 1..=n {
            let parent = i + lowbit(i);
            if parent <= n {
                tree[parent] = tree[parent].wrapping_add(tree[i]);
            }
        }
        FenwickTree { tree, n }
    }

    /// Number of valid 0-based indices.
    pub fn len(&self) -> usize {
        self.n
    }

    /// True iff the tree is empty (`n == 0`).
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Add `delta` (`i32`, widened to `i64`) to the value at 0-based index `i`.
    ///
    /// # Panics
    /// Panics if `i >= n` (out of the fixed `0 ..= n-1` domain).
    pub fn update(&mut self, i: usize, delta: i32) {
        self.add_internal(i, delta as i64);
    }

    /// Point-assign: make the value at `i` equal `value` (`i32`).
    ///
    /// Implemented as a Fenwick difference-add computed in wrapping `i64`
    /// (`delta = (value as i64) - get(i)`), NOT routed through the `i32`
    /// `update` signature — so the internal delta stays exact even when the
    /// current slot value already exceeds `i32`.
    ///
    /// # Panics
    /// Panics if `i >= n`.
    pub fn set(&mut self, i: usize, value: i32) {
        let delta = (value as i64).wrapping_sub(self.get(i));
        self.add_internal(i, delta);
    }

    /// The single logical value currently at 0-based index `i`, as `i64`.
    /// Equivalent to `range_sum(i, i)`.
    ///
    /// # Panics
    /// Panics if `i >= n`.
    pub fn get(&self, i: usize) -> i64 {
        assert!(
            i < self.n,
            "FenwickTree::get index {} out of range 0..{}",
            i,
            self.n
        );
        // get(i) == prefix_sum(i) - prefix_sum(i-1); prefix_sum(-1) := 0.
        if i == 0 {
            self.prefix_sum_internal(0)
        } else {
            self.prefix_sum_internal(i)
                .wrapping_sub(self.prefix_sum_internal(i - 1))
        }
    }

    /// Inclusive prefix sum `Σ values[0..=i]`, as wrapping `i64`.
    ///
    /// # Panics
    /// Panics if `i >= n`.
    pub fn prefix_sum(&self, i: usize) -> i64 {
        assert!(
            i < self.n,
            "FenwickTree::prefix_sum index {} out of range 0..{}",
            i,
            self.n
        );
        self.prefix_sum_internal(i)
    }

    /// Inclusive range sum `Σ values[lo..=hi]`, as wrapping `i64`.
    ///
    /// Validates BOTH endpoints first: `lo` and `hi` must be valid public
    /// indices (`< n`). Only after both are valid, if `lo > hi` the range is
    /// empty and returns `0`.
    ///
    /// # Panics
    /// Panics if `lo >= n` or `hi >= n` (out-of-domain endpoint). On the empty
    /// tree every call panics (no valid endpoint exists).
    pub fn range_sum(&self, lo: usize, hi: usize) -> i64 {
        assert!(
            lo < self.n,
            "FenwickTree::range_sum lo {} out of range 0..{}",
            lo,
            self.n
        );
        assert!(
            hi < self.n,
            "FenwickTree::range_sum hi {} out of range 0..{}",
            hi,
            self.n
        );
        // Both endpoints valid; an empty closed range (lo > hi) is a defined 0.
        if lo > hi {
            return 0;
        }
        // range_sum = prefix_sum(hi) - prefix_sum(lo-1); prefix_sum(-1) := 0.
        let upper = self.prefix_sum_internal(hi);
        let lower = if lo == 0 {
            0
        } else {
            self.prefix_sum_internal(lo - 1)
        };
        upper.wrapping_sub(lower)
    }

    /// Grand total `Σ` of all values, `== prefix_sum(n-1)` for `n >= 1`, and
    /// `0` for the empty tree.
    pub fn total(&self) -> i64 {
        if self.n == 0 {
            0
        } else {
            self.prefix_sum_internal(self.n - 1)
        }
    }

    /// The canonical 1-based BIT projection: a length-`n` `i64` array where
    /// element `j-1` (0-based in the returned vec) is the partial sum the tree
    /// stores for the 1-based index `j` — i.e. `tree[1 ..= n]`. This is the
    /// layout-independent secondary determinism oracle.
    pub fn canonical_tree(&self) -> Vec<i64> {
        self.tree[1..=self.n].to_vec()
    }

    // ---- internals (1-based BIT navigation) -------------------------------

    /// Add a wrapping-`i64` `delta` at 0-based index `i` via the low-bit walk.
    fn add_internal(&mut self, i: usize, delta: i64) {
        assert!(
            i < self.n,
            "FenwickTree mutator index {} out of range 0..{}",
            i,
            self.n
        );
        let mut j = i + 1; // public -> 1-based BIT
        while j <= self.n {
            self.tree[j] = self.tree[j].wrapping_add(delta);
            j += lowbit(j);
        }
    }

    /// Inclusive prefix sum for 0-based index `i` (caller guarantees `i < n`).
    fn prefix_sum_internal(&self, i: usize) -> i64 {
        let mut acc: i64 = 0;
        let mut j = i + 1; // public -> 1-based BIT
        while j > 0 {
            acc = acc.wrapping_add(self.tree[j]);
            j -= lowbit(j);
        }
        acc
    }
}

/// Low bit `j & -j` over a 1-based index (`j >= 1`). On `usize` the two's
/// complement negation is `j & j.wrapping_neg()`.
#[inline]
fn lowbit(j: usize) -> usize {
    j & j.wrapping_neg()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A brute-force i64 reference: a flat array of per-index i64 values, with
    // the same wrapping arithmetic the Fenwick tree must match.
    struct Brute {
        vals: Vec<i64>,
    }
    impl Brute {
        fn with_size(n: usize) -> Self {
            Brute {
                vals: vec![0i64; n],
            }
        }
        fn update(&mut self, i: usize, delta: i32) {
            self.vals[i] = self.vals[i].wrapping_add(delta as i64);
        }
        fn set(&mut self, i: usize, value: i32) {
            self.vals[i] = value as i64;
        }
        fn get(&self, i: usize) -> i64 {
            self.vals[i]
        }
        fn prefix_sum(&self, i: usize) -> i64 {
            let mut acc = 0i64;
            for k in 0..=i {
                acc = acc.wrapping_add(self.vals[k]);
            }
            acc
        }
        fn range_sum(&self, lo: usize, hi: usize) -> i64 {
            if lo > hi {
                return 0;
            }
            let mut acc = 0i64;
            for k in lo..=hi {
                acc = acc.wrapping_add(self.vals[k]);
            }
            acc
        }
        fn total(&self) -> i64 {
            let mut acc = 0i64;
            for &v in &self.vals {
                acc = acc.wrapping_add(v);
            }
            acc
        }
    }

    // A tiny deterministic LCG so the property tests need no external dep.
    struct Lcg(u64);
    impl Lcg {
        fn next_u64(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0
        }
        fn next_i32(&mut self) -> i32 {
            self.next_u64() as i32
        }
        fn next_usize(&mut self, bound: usize) -> usize {
            (self.next_u64() % bound as u64) as usize
        }
    }

    #[test]
    fn worked_example_from_spec() {
        let mut f = FenwickTree::with_size(8);
        f.update(0, 5);
        f.update(3, 2);
        f.update(7, 9);
        assert_eq!(f.prefix_sum(0), 5);
        assert_eq!(f.prefix_sum(3), 7);
        assert_eq!(f.prefix_sum(6), 7);
        assert_eq!(f.prefix_sum(7), 16);
        assert_eq!(f.total(), 16);
        assert_eq!(f.range_sum(1, 7), 11);
        assert_eq!(f.get(3), 2);
        assert_eq!(f.len(), 8);
        assert!(!f.is_empty());
    }

    #[test]
    fn inclusive_conventions() {
        let f = FenwickTree::from_values(&[3, 1, 4, 1, 5, 9, 2, 6]);
        // prefix_sum(0) is the first value (NOT 0 — inclusive).
        assert_eq!(f.prefix_sum(0), 3);
        // single-element inclusive range == that value (NOT 0).
        assert_eq!(f.range_sum(2, 2), 4);
        assert_eq!(f.get(2), 4);
        assert_eq!(f.prefix_sum(7), 31);
        assert_eq!(f.total(), 31);
        // total == prefix_sum(n-1) == range_sum(0, n-1).
        assert_eq!(f.total(), f.prefix_sum(7));
        assert_eq!(f.total(), f.range_sum(0, 7));
    }

    #[test]
    fn from_values_matches_updates() {
        let cases: &[&[i32]] = &[
            &[],
            &[42],
            &[3, 1, 4, 1, 5, 9, 2, 6],
            &[i32::MIN, i32::MAX, -1, 0, 7],
            &[-5, -5, -5, -5, -5, -5, -5],
        ];
        for vals in cases {
            let built = FenwickTree::from_values(vals);
            let mut updated = FenwickTree::with_size(vals.len());
            for (i, &v) in vals.iter().enumerate() {
                updated.update(i, v);
            }
            assert_eq!(built.canonical_tree(), updated.canonical_tree());
            for i in 0..vals.len() {
                assert_eq!(built.prefix_sum(i), updated.prefix_sum(i));
                assert_eq!(built.get(i), updated.get(i));
            }
            assert_eq!(built.total(), updated.total());
        }
    }

    #[test]
    fn set_replaces_not_adds() {
        let mut f = FenwickTree::with_size(4);
        f.update(1, 5);
        f.set(1, 3); // replace, NOT add: get(1) must be 3, not 8.
        f.update(2, 7);
        assert_eq!(f.get(1), 3);
        assert_eq!(f.get(2), 7);
        assert_eq!(f.prefix_sum(1), 3);
        assert_eq!(f.prefix_sum(3), 10);
        assert_eq!(f.total(), 10);
    }

    #[test]
    fn negative_deltas_cross_zero() {
        let mut f = FenwickTree::with_size(5);
        f.update(0, 10);
        f.update(1, -4);
        f.update(2, -20);
        f.update(3, 7);
        assert_eq!(f.prefix_sum(0), 10);
        assert_eq!(f.prefix_sum(1), 6);
        assert_eq!(f.prefix_sum(2), -14);
        assert_eq!(f.prefix_sum(4), -7);
        assert_eq!(f.total(), -7);
        assert_eq!(f.range_sum(1, 3), -17);
    }

    #[test]
    fn signed_extremes_widen_to_i64() {
        let mut f = FenwickTree::with_size(3);
        f.set(0, i32::MAX); // 2147483647
        f.set(1, i32::MIN); // -2147483648
        f.update(2, i32::MAX);
        f.update(2, 1); // value becomes 2147483648 as i64 (NOT i32-wrapped).
        assert_eq!(f.get(0), 2147483647i64);
        assert_eq!(f.get(1), -2147483648i64);
        assert_eq!(f.get(2), 2147483648i64);
        assert_eq!(f.prefix_sum(1), -1i64);
        assert_eq!(f.total(), 2147483647i64);
    }

    #[test]
    fn large_i64_sum_exceeds_2_53() {
        let mut f = FenwickTree::with_size(4);
        for i in 0..4 {
            f.set(i, i32::MAX);
        }
        assert_eq!(f.total(), 8589934588i64); // 4 * (2^31 - 1)
        assert_eq!(f.prefix_sum(3), 8589934588i64);
        assert_eq!(f.range_sum(1, 2), 4294967294i64);
    }

    #[test]
    fn i64_wrap_is_two_complement_not_saturating() {
        // Seed a slot near i64::MAX via the production path, then add past it.
        // Reaching i64::MAX with i32 deltas is infeasible, so we seed by adding
        // a large i64 directly through the (internal) add path used by set —
        // here we drive it through repeated set/update at the public boundary
        // by exploiting that a slot value is an unbounded i64 accumulator.
        let mut f = FenwickTree::with_size(1);
        // Build i64::MAX - 1 by setting i32::MAX repeatedly via update widening.
        // i64::MAX = 9223372036854775807. We add (2^31-1) chunks: that needs
        // ~2^32 ops, infeasible. Instead use the internal add_internal directly
        // (white-box) to seed exactly, then verify wrap through the public API.
        f.add_internal(0, i64::MAX - 1);
        assert_eq!(f.get(0), i64::MAX - 1);
        f.add_internal(0, 5); // (MAX-1) + 5 wraps two's-complement to negative.
        let expected = (i64::MAX - 1).wrapping_add(5);
        assert!(expected < 0, "expected wrap to negative");
        assert_eq!(f.get(0), expected);
        assert_eq!(f.total(), expected);
        assert_eq!(f.prefix_sum(0), expected);
    }

    #[test]
    fn range_sum_equals_prefix_diff_after_wrap() {
        // Invertibility holds even after the running total has wrapped.
        let mut f = FenwickTree::with_size(3);
        f.add_internal(0, i64::MAX - 10);
        f.add_internal(1, 100); // prefix_sum(1) wraps.
        f.add_internal(2, -7);
        // Concrete brute-force anchors: each per-index logical value is exact
        // (a single value never overflows; only sums wrap), so range_sum over a
        // single index returns that exact value even when prefixes have wrapped.
        assert_eq!(f.get(0), i64::MAX - 10);
        assert_eq!(f.get(1), 100);
        assert_eq!(f.get(2), -7);
        assert_eq!(f.range_sum(1, 1), 100);
        assert_eq!(f.range_sum(2, 2), -7);
        // range_sum(0, 2) == total, which has wrapped to a negative i64.
        let total = (i64::MAX - 10).wrapping_add(100).wrapping_add(-7);
        assert!(total < 0, "expected the running total to have wrapped");
        assert_eq!(f.range_sum(0, 2), total);
        assert_eq!(f.total(), total);
        // Invertibility: range_sum == prefix_sum(hi) - prefix_sum(lo-1) holds
        // for every sub-range even after the wrap (wrapping-sub is exact inverse).
        for lo in 0..3 {
            for hi in lo..3 {
                let direct = f.range_sum(lo, hi);
                let via =
                    f.prefix_sum(hi)
                        .wrapping_sub(if lo == 0 { 0 } else { f.prefix_sum(lo - 1) });
                assert_eq!(direct, via, "lo={} hi={}", lo, hi);
            }
        }
    }

    #[test]
    fn single_element() {
        let mut f = FenwickTree::with_size(1);
        f.update(0, 42);
        assert_eq!(f.len(), 1);
        assert_eq!(f.get(0), 42);
        assert_eq!(f.prefix_sum(0), 42);
        assert_eq!(f.range_sum(0, 0), 42);
        assert_eq!(f.total(), 42);
    }

    #[test]
    fn empty_tree_edges() {
        let f = FenwickTree::with_size(0);
        assert_eq!(f.len(), 0);
        assert!(f.is_empty());
        assert_eq!(f.total(), 0);

        let g = FenwickTree::from_values(&[]);
        assert_eq!(g.len(), 0);
        assert!(g.is_empty());
        assert_eq!(g.total(), 0);
        assert!(g.canonical_tree().is_empty());
    }

    #[test]
    fn lo_gt_hi_returns_zero() {
        let f = FenwickTree::from_values(&[3, 1, 4, 1, 5, 9, 2, 6]);
        assert_eq!(f.range_sum(5, 2), 0); // both endpoints valid, lo > hi.
        assert_eq!(f.range_sum(7, 0), 0);
    }

    #[test]
    #[should_panic]
    fn update_out_of_range_panics() {
        let mut f = FenwickTree::with_size(4);
        f.update(4, 1); // i == n.
    }

    #[test]
    #[should_panic]
    fn set_out_of_range_panics() {
        let mut f = FenwickTree::with_size(4);
        f.set(4, 1);
    }

    #[test]
    #[should_panic]
    fn get_out_of_range_panics() {
        let f = FenwickTree::with_size(4);
        let _ = f.get(4);
    }

    #[test]
    #[should_panic]
    fn prefix_sum_out_of_range_panics() {
        let f = FenwickTree::with_size(4);
        let _ = f.prefix_sum(4);
    }

    #[test]
    #[should_panic]
    fn range_sum_hi_out_of_range_panics() {
        let f = FenwickTree::with_size(4);
        let _ = f.range_sum(0, 4); // hi == n traps (NOT inferred as empty).
    }

    #[test]
    #[should_panic]
    fn range_sum_lo_out_of_range_panics() {
        let f = FenwickTree::with_size(4);
        let _ = f.range_sum(4, 0);
    }

    #[test]
    #[should_panic]
    fn empty_tree_get_panics() {
        let f = FenwickTree::with_size(0);
        let _ = f.get(0);
    }

    #[test]
    #[should_panic]
    fn empty_tree_prefix_sum_panics() {
        let f = FenwickTree::with_size(0);
        let _ = f.prefix_sum(0);
    }

    #[test]
    #[should_panic]
    fn empty_tree_range_sum_panics() {
        let f = FenwickTree::with_size(0);
        let _ = f.range_sum(0, 0);
    }

    #[test]
    fn fenwick_identity_vs_brute_force_randomized() {
        let mut rng = Lcg(0x1234_5678_9abc_def0);
        for trial in 0..200 {
            let n = 1 + rng.next_usize(20);
            let mut f = FenwickTree::with_size(n);
            let mut b = Brute::with_size(n);
            let ops = 5 + rng.next_usize(40);
            for _ in 0..ops {
                let i = rng.next_usize(n);
                // Mix updates (incl. INT_MIN/INT_MAX) and sets.
                let pick = rng.next_u64() % 5;
                let v = match pick {
                    0 => i32::MIN,
                    1 => i32::MAX,
                    _ => rng.next_i32(),
                };
                if rng.next_u64() % 2 == 0 {
                    f.update(i, v);
                    b.update(i, v);
                } else {
                    f.set(i, v);
                    b.set(i, v);
                }
            }
            // Every observable must match the brute-force i64 reference.
            for i in 0..n {
                assert_eq!(f.get(i), b.get(i), "trial {} get {}", trial, i);
                assert_eq!(
                    f.prefix_sum(i),
                    b.prefix_sum(i),
                    "trial {} prefix {}",
                    trial,
                    i
                );
            }
            for lo in 0..n {
                for hi in 0..n {
                    assert_eq!(
                        f.range_sum(lo, hi),
                        b.range_sum(lo, hi),
                        "trial {} range {}..{}",
                        trial,
                        lo,
                        hi
                    );
                }
            }
            assert_eq!(f.total(), b.total(), "trial {} total", trial);
        }
    }

    #[test]
    fn build_determinism_randomized() {
        let mut rng = Lcg(0xdead_beef_cafe_babe);
        for _ in 0..200 {
            let n = rng.next_usize(20);
            let vals: Vec<i32> = (0..n)
                .map(|_| match rng.next_u64() % 4 {
                    0 => i32::MIN,
                    1 => i32::MAX,
                    _ => rng.next_i32(),
                })
                .collect();
            let built = FenwickTree::from_values(&vals);
            let mut updated = FenwickTree::with_size(n);
            for (i, &v) in vals.iter().enumerate() {
                updated.update(i, v);
            }
            assert_eq!(built.canonical_tree(), updated.canonical_tree());
        }
    }
}
