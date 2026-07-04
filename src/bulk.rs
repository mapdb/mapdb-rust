// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Shared contract for the **data pump** (bulk import): insert-only,
//! single-pass construction of a *fresh* collection from prepared input.
//!
//! See `spec/features/data-pump.md`. The two families of entry points are:
//!
//! - **One-shot constructors** — the primary API everywhere:
//!   - ordered: [`crate::object::TreeMap::from_sorted`] /
//!     [`crate::object::TreeSet::from_sorted`] and the multimap
//!     `from_sorted_*` constructors.
//!   - hash/list: [`crate::OpenHashMap::bulk_load`] /
//!     [`crate::OpenHashMap::bulk_load_exact`] and friends.
//! - **Streaming `Sink`** — only for ordered + multimap builders
//!   ([`crate::object::TreeMapSink`], [`crate::object::TreeSetSink`]).
//!
//! Fallible constructors and `try_create` return `Result<_, BulkError>`;
//! data-shape problems (out-of-order, duplicate) are reported through that
//! result. Infallible sink `create()` methods panic if called after the sink
//! has already been poisoned by an earlier error. A failed pump leaks nothing
//! (Rust `Drop` runs on the partially-built buffer) and never returns a
//! half-built collection.

/// Policy for how a bulk builder treats a duplicate key/element.
///
/// There is deliberately **no** `Overwrite` variant: "last value wins" is a
/// read-modify-write and contradicts the pump's insert-only contract (use the
/// normal `insert` loop for that). Bags do not consult this policy — equal
/// keys increment a count instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuplicatePolicy {
    /// A duplicate key/element aborts the build with [`BulkError::Duplicate`].
    Error,
    /// The first occurrence is kept; later duplicates are skipped.
    IgnoreDuplicates,
}

/// Error returned by every data-pump entry point.
///
/// `Duplicate` and `OutOfOrder` carry the **input index** (0-based position in
/// the consumed iterator) of the offending element, mirroring the Kotlin 3.x
/// `DBException.PumpSourceDuplicate` / `PumpSourceNotSorted`. `Alloc` wraps a
/// `TryReserveError` for entry points that explicitly pre-reserve fallibly.
#[derive(Debug)]
#[non_exhaustive]
pub enum BulkError {
    /// A duplicate key/element was found at input index `index` while the
    /// policy was [`DuplicatePolicy::Error`] (or, for BiMap, a duplicate value).
    Duplicate {
        /// 0-based index of the duplicate element in the consumed input.
        index: usize,
    },
    /// An ordered builder saw an element at input index `index` that was not
    /// strictly greater than its predecessor under the collection's comparator.
    OutOfOrder {
        /// 0-based index of the out-of-order element in the consumed input.
        index: usize,
    },
    /// The backing allocation could not be satisfied. Carries the std error so
    /// callers can inspect/propagate it.
    Alloc(std::collections::TryReserveError),
    /// A counted collection (`HashBag`/`TreeBag`) would overflow its `usize`
    /// occurrence count while compressing a run of equal keys.
    CountOverflow {
        /// 0-based index of the element whose addition overflowed the count.
        index: usize,
    },
    /// A `bulk_load_exact` source produced more than the declared `n` elements,
    /// which would force a rehash and break the zero-rehash contract.
    ExactSizeExceeded {
        /// The declared exact size `n`.
        expected: usize,
    },
    /// An index-addressed builder (e.g. [`crate::BitSet::from_sorted_indices`])
    /// saw an index at input position `index` too large to represent the
    /// collection's length convention (`max_index + 1` would overflow `usize`).
    IndexOverflow {
        /// 0-based position in the consumed input of the oversized index.
        index: usize,
    },
}

impl std::fmt::Display for BulkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BulkError::Duplicate { index } => {
                write!(f, "duplicate key at input index {index}")
            }
            BulkError::OutOfOrder { index } => {
                write!(f, "input not strictly ascending at index {index}")
            }
            BulkError::Alloc(e) => write!(f, "allocation failed during bulk load: {e}"),
            BulkError::CountOverflow { index } => {
                write!(f, "occurrence count overflowed at input index {index}")
            }
            BulkError::ExactSizeExceeded { expected } => {
                write!(
                    f,
                    "bulk_load_exact source exceeded declared size {expected}"
                )
            }
            BulkError::IndexOverflow { index } => {
                write!(
                    f,
                    "index at input position {index} too large to represent length"
                )
            }
        }
    }
}

impl std::error::Error for BulkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BulkError::Alloc(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::collections::TryReserveError> for BulkError {
    fn from(e: std::collections::TryReserveError) -> Self {
        BulkError::Alloc(e)
    }
}

/// Open-addressing pre-size capacity for `n` items at the 0.75 load factor.
///
/// Returns the power-of-two table capacity that holds `n` entries without ever
/// hitting the strict growth predicate `cap*3 <= (size+1)*4` mid-load (see
/// `algorithms.md` §"Open-addressing layout"). The required-slot formula is
///
/// ```text
/// required = floor(4*n / 3) + 1     // == ceil((4n + 1) / 3)
/// cap      = nextPow2(required)
/// ```
///
/// computed overflow-safely (no `4*n` intermediate), and clamped to a minimum
/// of `min_cap` (the empty-table sentinel). `n = 0` returns `min_cap`.
///
/// Verified against [`crate::hash_table`]'s own `needs_resize` predicate by the
/// `zero rehash at n = 3·2^k` unit tests.
pub(crate) fn open_addressing_capacity(n: usize, min_cap: usize) -> usize {
    // floor(4n/3) + 1 without overflowing 4*n: decompose 4n/3 as
    // n + floor(n/3) + carry, where carry accounts for the (n mod 3) part of
    // the extra n.  4n/3 = n + n/3, and floor(4n/3) = n + floor(n/3) when
    // viewed over the integers? Not exactly — do it via 128-bit-free math:
    //   4n = 3n + n  =>  4n/3 = n + n/3  exactly in rationals.
    //   floor(4n/3) = n + floor(n/3) + ((n%3)+ (extra carry))? Keep it simple
    //   and correct: floor(4n/3) = (4n - (4n mod 3)) / 3. Compute 4n mod 3 =
    //   (n mod 3) since 4 ≡ 1 (mod 3). So:
    //     q = n/3, r = n%3  =>  4n = 12q + 4r,  4n/3 = 4q + (4r)/3.
    //   floor(4n/3) = 4q + (4r)/3 with r in {0,1,2}: r=0->0, r=1->1, r=2->2.
    //   i.e. floor(4n/3) = 4*(n/3) + ( (4*(n%3)) / 3 ).
    let q = n / 3;
    let r = n % 3;
    let floor_4n_3 = q.saturating_mul(4).saturating_add((4 * r) / 3);
    let required = floor_4n_3.saturating_add(1);
    let floor = required.max(min_cap);
    floor.checked_next_power_of_two().unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_formula_matches_spec_examples() {
        // n = 0 -> empty-table sentinel (min_cap), never 1.
        assert_eq!(open_addressing_capacity(0, 16), 16);
        // required = floor(4n/3)+1; cap = nextPow2(required), clamped to 16.
        // n=3 -> floor(12/3)+1 = 5 -> nextPow2 = 8 -> clamp 16.
        assert_eq!(open_addressing_capacity(3, 16), 16);
        // n=12 -> floor(48/3)+1 = 17 -> nextPow2 = 32.
        assert_eq!(open_addressing_capacity(12, 16), 32);
        // n=24 -> floor(96/3)+1 = 33 -> nextPow2 = 64.
        assert_eq!(open_addressing_capacity(24, 16), 64);
        // n=48 -> floor(192/3)+1 = 65 -> nextPow2 = 128.
        assert_eq!(open_addressing_capacity(48, 16), 128);
    }

    #[test]
    fn floor_4n_3_is_exact() {
        // Brute-force the decomposed floor(4n/3) against the naive form for a
        // range of n, including the n = 3·2^k spec points.
        for n in 0..10_000usize {
            let q = n / 3;
            let r = n % 3;
            let decomposed = q * 4 + (4 * r) / 3;
            assert_eq!(decomposed, (4 * n) / 3, "mismatch at n={n}");
        }
    }

    #[test]
    fn capacity_never_overflows() {
        // Huge n must saturate, not panic.
        assert_eq!(open_addressing_capacity(usize::MAX, 16), usize::MAX);
    }
}
