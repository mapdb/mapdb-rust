// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Count-Min Sketch — a `d×w` integer counter matrix giving a one-sided
//! **over**-estimate of an element's frequency, riding the deterministic hash
//! pipeline (see `spec/features/count-min.md`).
//!
//! This is the **reference port**. The counter matrix after a given
//! add-sequence is the cross-language oracle: because the `d` column indices are
//! exactly [`hash::positions`]`(encode_i32(item), w, d)` — bit-identical across
//! all five ports — the entire matrix, every `estimate`, and `total` are
//! bit-identical too. **No floating point** appears in the deterministic
//! surface (the only float, [`CountMin::optimal`], is native-test-only and
//! never used by the shared scenarios).
//!
//! Pinned rulings:
//! - **Row-hash derivation:** the column touched in row `r` is the `r`-th of
//!   `positions(encode_i32(item), m = w, k = d)` in derivation order
//!   (`c_r = (h1 + r*h2) mod w`). Repeated column numbers across rows touch
//!   **distinct** counters (one counter array per row) — NOT de-duplicated.
//! - **`estimate` = MIN over the `d` rows** (never average/sum/median/row-0).
//!   The empty MIN (`d = 0`) is `u64::MAX`.
//! - **Overflow SATURATES at `u64::MAX`** (does NOT wrap) — a deliberate
//!   departure from the collections' wrapping contract, required by the
//!   no-under-estimate guarantee.
//! - **`add(item, count)` increments by `count`** (plain CMS, no conservative
//!   update); `add_one` ≡ `add(item, 1)`.
//! - **Element encoding:** `i32` → reinterpret `u32` → 4 LE bytes → the byte
//!   `positions` path (length fold applied), identical to Bloom.

use crate::hash;

/// Euler's number `e`, used only by the native-only [`CountMin::optimal`].
const EULER_E: f64 = std::f64::consts::E;

/// A Count-Min Sketch over a flat row-major `Vec<u64>` matrix of `d*w` counters.
///
/// Construct with [`CountMin::with_params`] (the only constructor the
/// cross-language scenarios use) or the native-only [`CountMin::optimal`].
#[derive(Clone, Debug)]
pub struct CountMin {
    d: u32,
    w: u32,
    /// Flat row-major matrix: counter `matrix[r*w + col]` is row `r`, column
    /// `col`. Length is exactly `d*w`.
    matrix: Vec<u64>,
    /// Running sum of every `count` argument (the stream length `N`), saturating.
    total: u64,
}

impl CountMin {
    /// Construct a `d×w` sketch with all counters zero. `d` is the depth (rows /
    /// hash functions = the `k` argument to `positions`); `w` is the width
    /// (columns per row = the `m` argument to `positions`).
    ///
    /// # Panics
    /// `w == 0` is invalid (a zero-column row holds nothing and every modulo
    /// would divide by zero) and traps — identical to Bloom's `m = 0` ruling.
    /// `d == 0` is legal and degenerate (an empty matrix; `estimate` returns
    /// `u64::MAX`).
    pub fn with_params(d: u32, w: u32) -> CountMin {
        assert!(w != 0, "CountMin width w must be non-zero");
        let len = (d as usize)
            .checked_mul(w as usize)
            .expect("CountMin d*w overflows usize (native allocation limit)");
        CountMin {
            d,
            w,
            matrix: vec![0u64; len],
            total: 0,
        }
    }

    /// Native-only convenience constructor sizing the sketch from a target
    /// additive error `epsilon` (relative to the total) and failure probability
    /// `delta` using the standard Count-Min formulas
    /// `w = ceil(e/epsilon)`, `d = ceil(ln(1/delta))`, then delegating to
    /// [`CountMin::with_params`].
    ///
    /// **Float-quarantined: never used by the cross-language scenarios** (the
    /// `ln`/`e`/`ceil` derivation can drift across libm implementations). Each
    /// port native-tests it against the pinned integer table.
    ///
    /// # Panics
    /// Requires `0 < epsilon < 1` and `0 < delta < 1`; values `<= 0`, `>= 1`,
    /// `NaN`, or `±Infinity` are invalid and trap (they would divide by zero,
    /// take `ln` of a non-positive value, or yield a non-finite `(d, w)`).
    pub fn optimal(epsilon: f64, delta: f64) -> CountMin {
        assert!(
            epsilon > 0.0 && epsilon < 1.0,
            "CountMin::optimal requires 0 < epsilon < 1, got {epsilon}"
        );
        assert!(
            delta > 0.0 && delta < 1.0,
            "CountMin::optimal requires 0 < delta < 1, got {delta}"
        );
        let w = (EULER_E / epsilon).ceil();
        let d = (1.0f64 / delta).ln().ceil();
        // Range-check before the `f64 as u32` casts: `as` saturates to
        // `u32::MAX` for out-of-range floats (Java's `(int)` would clamp to
        // `2^31-1`), so an extreme `epsilon`/`delta` would silently produce a
        // giant table. Mirror `Bloom::optimal`'s guard and reject instead.
        assert!(
            w.is_finite() && d.is_finite() && w >= 1.0 && d >= 1.0,
            "CountMin::optimal produced a non-finite (d, w)"
        );
        assert!(
            w <= u32::MAX as f64 && d <= u32::MAX as f64,
            "CountMin::optimal: derived (d, w) out of u32 range (d={d}, w={w})"
        );
        CountMin::with_params(d as u32, w as u32)
    }

    /// The `d` column indices for `item`, one per row, in derivation order:
    /// `positions(encode_i32(item), m = w, k = d)`. `c_r` (the `r`-th element)
    /// is the column touched in row `r`.
    #[inline]
    fn columns(&self, item: i32) -> Vec<u32> {
        // Element encoding: i32 -> reinterpret u32 -> 4 LE bytes -> byte
        // positions path (length fold applied), identical to Bloom.
        let bytes = (item as u32).to_le_bytes();
        hash::positions(&bytes, self.w, self.d)
    }

    /// Increment the `d` selected counters (one per row) by `count`, saturating
    /// at `u64::MAX`. `add(item, count)` is **not** observably five `add_one`
    /// calls but yields the identical counters (increments are commutative).
    /// `count = 0` is legal: a no-op on the counters that still updates `total`
    /// (by 0). Plain CMS — increments **all** `d` counters (no conservative
    /// update).
    pub fn add(&mut self, item: i32, count: u64) {
        let cols = self.columns(item);
        for (r, &c) in cols.iter().enumerate() {
            let idx = r * (self.w as usize) + (c as usize);
            self.matrix[idx] = self.matrix[idx].saturating_add(count);
        }
        self.total = self.total.saturating_add(count);
    }

    /// Convenience for `add(item, 1)`; identical bits.
    #[inline]
    pub fn add_one(&mut self, item: i32) {
        self.add(item, 1);
    }

    /// The frequency estimate for `item`: the **MIN** over the `d` rows of the
    /// selected counter. Never under-estimates (within the `u64` domain). For
    /// `d = 0` the MIN over zero rows is the empty-min identity `u64::MAX`.
    pub fn estimate(&self, item: i32) -> u64 {
        let cols = self.columns(item);
        let mut min = u64::MAX;
        for (r, &c) in cols.iter().enumerate() {
            let idx = r * (self.w as usize) + (c as usize);
            min = min.min(self.matrix[idx]);
        }
        min
    }

    /// The running sum of every `count` argument ever added (the stream length
    /// `N`), saturating at `u64::MAX`.
    #[inline]
    pub fn total(&self) -> u64 {
        self.total
    }

    /// The depth `d` (number of rows / hash functions).
    #[inline]
    pub fn depth(&self) -> u32 {
        self.d
    }

    /// The width `w` (number of columns per row).
    #[inline]
    pub fn width(&self) -> u32 {
        self.w
    }

    /// The full counter matrix as `d*w` values, **row-major** (row 0 first,
    /// column 0 first within a row). Dense (all cells, including zeros).
    #[inline]
    pub fn to_counters(&self) -> Vec<u64> {
        self.matrix.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The LE-4-byte encoding of an i32, the input the byte `positions` path
    /// consumes (reinterpret, not sign-extend).
    fn encode(item: i32) -> [u8; 4] {
        (item as u32).to_le_bytes()
    }

    #[test]
    #[should_panic(expected = "out of u32 range")]
    fn optimal_rejects_saturating_epsilon() {
        // Regression: `f64 as u32` saturates to `u32::MAX` for an extreme
        // epsilon, silently sizing a giant table. Must trap (mirrors Bloom).
        let _ = CountMin::optimal(1e-12, 0.01);
    }

    #[test]
    fn row_hash_matches_positions() {
        // The d columns are EXACTLY positions(encode_i32(item), w, d) in order.
        let c = CountMin::with_params(4, 16);
        let cols = c.columns(7);
        assert_eq!(cols, hash::positions(&encode(7), 16, 4));
        // Pinned worked example: add(7) over (d=4, w=16) touches [7,0,9,2]
        // (the 12-hash-pipeline/positions_basic.json vector).
        assert_eq!(cols, vec![7, 0, 9, 2]);
    }

    #[test]
    fn add_one_touches_the_four_columns() {
        let mut c = CountMin::with_params(4, 16);
        c.add_one(7);
        // Rows 0..3 touch columns [7,0,9,2] respectively (row r at index r*16+col).
        let m = c.to_counters();
        assert_eq!(m[7], 1); // row 0, col 7
        assert_eq!(m[16], 1); // row 1, col 0
        assert_eq!(m[2 * 16 + 9], 1); // row 2, col 9
        assert_eq!(m[3 * 16 + 2], 1); // row 3, col 2
                                      // Exactly four cells are 1.
        assert_eq!(m.iter().filter(|&&v| v == 1).count(), 4);
        assert_eq!(c.estimate(7), 1);
        assert_eq!(c.total(), 1);
        assert_eq!(m.len(), 64);
    }

    #[test]
    fn add_by_count_equals_repeated_add_one() {
        let mut a = CountMin::with_params(3, 13);
        let mut b = CountMin::with_params(3, 13);
        a.add(42, 5);
        for _ in 0..5 {
            b.add_one(42);
        }
        assert_eq!(a.to_counters(), b.to_counters());
        assert_eq!(a.estimate(42), 5);
        assert_eq!(a.total(), 5);
    }

    #[test]
    fn add_count_accumulates() {
        let mut c = CountMin::with_params(4, 16);
        c.add(7, 5);
        c.add(7, 3);
        assert_eq!(c.estimate(7), 8);
        assert_eq!(c.total(), 8);
    }

    #[test]
    fn count_zero_is_counter_noop_but_updates_total() {
        let mut c = CountMin::with_params(3, 7);
        c.add(1, 0);
        assert!(c.to_counters().iter().all(|&v| v == 0));
        assert_eq!(c.total(), 0); // += 0
        c.add(1, 4);
        c.add(1, 0);
        assert_eq!(c.estimate(1), 4);
        assert_eq!(c.total(), 4);
    }

    #[test]
    fn collision_across_rows_not_deduped() {
        // Find an item + (w, d) whose positions repeat a column across rows;
        // both same-numbered counters in DIFFERENT rows must be incremented.
        // Search small w for a repeated column.
        let d = 3u32;
        for w in 2u32..32 {
            for item in 0i32..256 {
                let cols = hash::positions(&encode(item), w, d);
                // a repeat means two rows share a column number.
                let mut seen = std::collections::HashMap::new();
                let mut repeated = None;
                for (r, &col) in cols.iter().enumerate() {
                    if let Some(&r0) = seen.get(&col) {
                        repeated = Some((r0, r, col));
                        break;
                    }
                    seen.insert(col, r);
                }
                if let Some((r0, r1, col)) = repeated {
                    let mut c = CountMin::with_params(d, w);
                    c.add_one(item);
                    let m = c.to_counters();
                    // Both row r0 col `col` and row r1 col `col` incremented.
                    assert_eq!(m[r0 * w as usize + col as usize], 1);
                    assert_eq!(m[r1 * w as usize + col as usize], 1);
                    // estimate is MIN over rows = 1 (all touched counters are 1).
                    assert_eq!(c.estimate(item), 1);
                    return;
                }
            }
        }
        panic!("no cross-row column collision found in the search space");
    }

    #[test]
    fn estimate_is_min_not_average_or_row0() {
        // Engineer unequal row counters for an item: add the item once, then
        // bump SOME of its rows via colliding other items. We verify estimate ==
        // the MIN selected counter and that the matrix max exceeds it.
        let mut c = CountMin::with_params(4, 8);
        let target = 5i32;
        c.add(target, 1);
        let cols = c.columns(target);
        // Manually inflate one of the target's columns via a direct second add
        // of the SAME item plus distinct colliding items until rows differ.
        // Simplest deterministic route: add other items that happen to land on
        // some of target's columns. Bump every other i32 and check.
        for other in 0i32..200 {
            if other == target {
                continue;
            }
            c.add(other, 7);
        }
        let m = c.to_counters();
        let selected: Vec<u64> = cols
            .iter()
            .enumerate()
            .map(|(r, &col)| m[r * 8 + col as usize])
            .collect();
        let min = *selected.iter().min().unwrap();
        let max = *selected.iter().max().unwrap();
        assert_eq!(c.estimate(target), min);
        // With many colliding adds the rows are very likely unequal; assert MIN
        // is the estimate regardless. If they happened to be equal this still
        // holds (MIN == that value). Guarantee no under-estimate of true count 1.
        assert!(c.estimate(target) >= 1);
        assert!(max >= min);
    }

    #[test]
    fn overflow_saturates_not_wraps() {
        let mut c = CountMin::with_params(2, 4);
        c.add(9, u64::MAX);
        c.add(9, 5);
        // Each selected counter clamps at u64::MAX (not wrap to 4).
        assert_eq!(c.estimate(9), u64::MAX);
        assert_eq!(c.total(), u64::MAX);
        for (r, &col) in c.columns(9).iter().enumerate() {
            assert_eq!(c.to_counters()[r * 4 + col as usize], u64::MAX);
        }
    }

    #[test]
    fn no_under_estimate() {
        let mut c = CountMin::with_params(5, 64);
        c.add(-1, 3);
        c.add(i32::MIN, 10);
        assert!(c.estimate(-1) >= 3);
        assert!(c.estimate(i32::MIN) >= 10);
    }

    #[test]
    fn order_independence() {
        let mut a = CountMin::with_params(4, 16);
        let mut b = CountMin::with_params(4, 16);
        let seq = [(1, 3u64), (2, 5), (1, 2), (-7, 9), (i32::MAX, 1)];
        for &(it, ct) in &seq {
            a.add(it, ct);
        }
        for &(it, ct) in seq.iter().rev() {
            b.add(it, ct);
        }
        assert_eq!(a.to_counters(), b.to_counters());
        assert_eq!(a.total(), b.total());
    }

    #[test]
    fn d_zero_is_legal_vacuous_max() {
        let mut c = CountMin::with_params(0, 16);
        c.add(5, 1);
        assert_eq!(c.to_counters(), Vec::<u64>::new());
        assert_eq!(c.total(), 1);
        // MIN over zero rows = u64::MAX.
        assert_eq!(c.estimate(5), u64::MAX);
    }

    #[test]
    fn empty_matrix_is_all_zero_dense() {
        let c = CountMin::with_params(4, 16);
        let m = c.to_counters();
        assert_eq!(m.len(), 64);
        assert!(m.iter().all(|&v| v == 0));
        assert_eq!(c.estimate(7), 0);
        assert_eq!(c.total(), 0);
    }

    #[test]
    #[should_panic(expected = "width w must be non-zero")]
    fn w_zero_traps() {
        let _ = CountMin::with_params(4, 0);
    }

    #[test]
    fn element_encoding_byte_path() {
        // Reuses the byte positions path (length fold), NOT the scalar word.
        let c = CountMin::with_params(4, 16);
        assert_eq!(c.columns(7), hash::positions(&7u32.to_le_bytes(), 16, 4));
        // -1 reinterprets to 0xffffffff -> LE bytes [ff,ff,ff,ff].
        assert_eq!(encode(-1), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(encode(i32::MIN), [0x00, 0x00, 0x00, 0x80]);
    }

    // ---- optimal() pinned integer table (native-only, float-quarantined) ---

    #[test]
    fn optimal_integer_table() {
        // (epsilon, delta) -> (w, d) per spec/features/count-min.md.
        let cases = [
            (0.01, 0.01, 272u32, 5u32),
            (0.001, 0.001, 2719, 7),
            (0.1, 0.05, 28, 3),
            (0.01, 0.001, 272, 7),
            (0.5, 0.5, 6, 1),
        ];
        for &(eps, delta, w, d) in &cases {
            let c = CountMin::optimal(eps, delta);
            assert_eq!(c.width(), w, "w for ({eps}, {delta})");
            assert_eq!(c.depth(), d, "d for ({eps}, {delta})");
        }
    }

    #[test]
    #[should_panic(expected = "0 < epsilon < 1")]
    fn optimal_rejects_bad_epsilon() {
        let _ = CountMin::optimal(0.0, 0.5);
    }

    #[test]
    #[should_panic(expected = "0 < delta < 1")]
    fn optimal_rejects_bad_delta() {
        let _ = CountMin::optimal(0.5, 1.0);
    }

    #[test]
    #[should_panic(expected = "0 < epsilon < 1")]
    fn optimal_rejects_nan() {
        let _ = CountMin::optimal(f64::NAN, 0.5);
    }
}
