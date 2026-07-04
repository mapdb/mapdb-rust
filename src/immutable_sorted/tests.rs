// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Native tests for the compact immutable sorted map / set. These cover the
//! obligations the cross-language JSON suite cannot express (panics, snapshot
//! independence, iterator key-order pairing, signed-edge brackets, round-trip
//! identity) per `spec/features/sorted-table-map.md` §"Native-only".

use super::{ImmutableSortedMap, ImmutableSortedSet};
use crate::range::Range;

// ── Construction traps (native-only; RESERVED expect_panic) ──────────

#[test]
#[should_panic(expected = "strictly ascending")]
fn map_unsorted_input_traps() {
    // keys[2] < keys[1]: out-of-order -> trap (never silently sort).
    ImmutableSortedMap::from_sorted(&[10, 30, 20], &[1, 3, 2]);
}

#[test]
#[should_panic(expected = "strictly ascending")]
fn map_duplicate_key_traps() {
    // Adjacent equal keys -> trap (no last-wins / dedup).
    ImmutableSortedMap::from_sorted(&[10, 20, 20, 30], &[1, 2, 99, 3]);
}

#[test]
#[should_panic(expected = "length mismatch")]
fn map_length_mismatch_traps() {
    ImmutableSortedMap::from_sorted(&[10, 20, 30], &[1, 2]);
}

#[test]
#[should_panic(expected = "strictly ascending")]
fn set_unsorted_input_traps() {
    ImmutableSortedSet::from_sorted(&[10, 30, 20]);
}

#[test]
#[should_panic(expected = "strictly ascending")]
fn set_duplicate_traps() {
    ImmutableSortedSet::from_sorted(&[10, 20, 20]);
}

#[test]
#[should_panic(expected = "strictly ascending")]
fn from_sorted_iter_unsorted_traps() {
    ImmutableSortedMap::from_sorted_iter([(10, 1), (5, 2)]);
}

// ── Empty + single (valid, not a trap) ───────────────────────────────

#[test]
fn empty_map_is_valid_and_all_absence() {
    let m: ImmutableSortedMap<i32, i32> = ImmutableSortedMap::from_sorted(&[], &[]);
    assert_eq!(m.len(), 0);
    assert!(m.is_empty());
    assert_eq!(m.get(&5), None);
    assert!(!m.contains_key(&5));
    assert_eq!(m.first_key(), None);
    assert_eq!(m.last_key(), None);
    assert_eq!(m.floor_key(&5), None);
    assert_eq!(m.ceiling_key(&5), None);
    assert_eq!(m.lower_key(&5), None);
    assert_eq!(m.higher_key(&5), None);
    assert_eq!(m.rank(&5), 0);
    assert_eq!(m.select_key(0), None);
    assert!(m.keys().next().is_none());
    assert!(m.descending_keys().is_empty());
    assert!(m.range_keys(Range::all()).is_empty());
}

#[test]
fn empty_set_is_valid() {
    let s: ImmutableSortedSet<i32> = ImmutableSortedSet::from_sorted(&[]);
    assert!(s.is_empty());
    assert_eq!(s.first(), None);
    assert_eq!(s.floor(&0), None);
    assert_eq!(s.rank(&0), 0);
    assert_eq!(s.select(0), None);
}

#[test]
fn single_element_is_valid() {
    let m = ImmutableSortedMap::from_sorted(&[7], &[700]);
    assert_eq!(m.get(&7), Some(&700));
    assert_eq!(m.floor_key(&7), Some(&7));
    assert_eq!(m.ceiling_key(&7), Some(&7));
    assert_eq!(m.lower_key(&7), None);
    assert_eq!(m.higher_key(&7), None);
    assert_eq!(m.rank(&6), 0);
    assert_eq!(m.rank(&7), 0);
    assert_eq!(m.rank(&8), 1);
    assert_eq!(m.select_key(0), Some(&7));
    assert_eq!(m.select_key(1), None);
}

// ── values() / entries() key-order pairing (native-only obligation) ──

#[test]
fn values_and_entries_pair_with_keys_not_value_sorted() {
    // Deliberately NON-monotonic values: a port that sorts values independently
    // would mis-pair. keys ascending {10,20,30}; values {300,100,200}.
    let m = ImmutableSortedMap::from_sorted(&[10, 20, 30], &[300, 100, 200]);

    // values() iterates in ascending-KEY order, paired with keys().
    let keys: Vec<i32> = m.keys().copied().collect();
    let values: Vec<i32> = m.values().copied().collect();
    assert_eq!(keys, vec![10, 20, 30]);
    assert_eq!(values, vec![300, 100, 200]); // NOT [100,200,300]

    // Zip-and-assert: values[i] is the value of keys[i].
    for (k, v) in keys.iter().zip(values.iter()) {
        assert_eq!(m.get(k), Some(v));
    }

    // entries() carries the same pairing.
    let entries: Vec<(i32, i32)> = m.entries().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(entries, vec![(10, 300), (20, 100), (30, 200)]);

    // get_<k> sees the right value (the cross-language oracle for misalignment).
    assert_eq!(m.get(&10), Some(&300));
    assert_eq!(m.get(&20), Some(&100));
    assert_eq!(m.get(&30), Some(&200));
}

// ── Snapshot independence from a mutated source buffer ───────────────

#[test]
fn construction_takes_an_independent_snapshot() {
    let mut keys = vec![10, 20, 30];
    let mut values = vec![100, 200, 300];
    let m = ImmutableSortedMap::from_sorted(&keys, &values);

    // Mutate the caller's source buffers AFTER construction.
    keys[0] = 999;
    values[1] = -1;
    keys.push(40);

    // The built map is unaffected.
    assert_eq!(m.len(), 3);
    assert_eq!(m.get(&10), Some(&100));
    assert_eq!(m.get(&20), Some(&200));
    assert_eq!(m.first_key(), Some(&10));
    assert!(!m.contains_key(&999));
    assert!(!m.contains_key(&40));
}

// ── select(rank(k)) == k round-trip identity ─────────────────────────

#[test]
fn select_rank_round_trip() {
    let keys = [-100, -1, 0, 1, 42, 1000];
    let m = ImmutableSortedMap::from_sorted(&keys, &[1, 2, 3, 4, 5, 6]);
    for k in keys {
        let r = m.rank(&k);
        assert_eq!(m.select_key(r), Some(&k), "select(rank({k})) must be {k}");
        assert_eq!(m.rank(m.select_key(r).unwrap()), r);
    }
    // rank on absent keys is the lower-bound index.
    assert_eq!(m.rank(&-101), 0);
    assert_eq!(m.rank(&500), 5);
    assert_eq!(m.rank(&100_000), 6);
}

// ── Sortedness / parallel-array invariants post-build ────────────────

#[test]
fn stored_arrays_are_strictly_ascending_and_aligned() {
    let m = ImmutableSortedMap::from_sorted(&[10, 20, 30, 40, 50], &[1, 2, 3, 4, 5]);
    let keys: Vec<i32> = m.keys().copied().collect();
    for w in keys.windows(2) {
        assert!(w[0] < w[1], "stored keys must be strictly ascending");
    }
    // Parallel-array alignment: get(keys[i]) == values[i].
    for (i, k) in keys.iter().enumerate() {
        assert_eq!(m.select_entry(i).map(|(_, v)| *v), m.get(k).copied());
    }
}

// ── Signed extremes (INT_MIN / INT_MAX) ──────────────────────────────

#[test]
fn signed_extremes_lookup_nav_rank_select() {
    let keys = [i32::MIN, -1, 0, 1, i32::MAX];
    let m = ImmutableSortedMap::from_sorted(&keys, &[10, 20, 30, 40, 50]);

    assert_eq!(m.get(&i32::MIN), Some(&10));
    assert_eq!(m.get(&i32::MAX), Some(&50));

    assert_eq!(m.floor_key(&i32::MIN), Some(&i32::MIN));
    assert_eq!(m.lower_key(&i32::MIN), None);
    assert_eq!(m.higher_key(&(-1)), Some(&0));
    assert_eq!(m.ceiling_key(&i32::MAX), Some(&i32::MAX));
    assert_eq!(m.higher_key(&i32::MAX), None);

    assert_eq!(m.rank(&0), 2);
    assert_eq!(m.rank(&i32::MIN), 0);
    assert_eq!(m.rank(&i32::MAX), 4);
    assert_eq!(m.select_key(0), Some(&i32::MIN));
    assert_eq!(m.select_key(4), Some(&i32::MAX));
    assert_eq!(m.select_key(5), None);
    assert_eq!(m.descending_keys(), vec![i32::MAX, 1, 0, -1, i32::MIN]);
}

#[test]
fn range_brackets_at_signed_extremes_do_not_overflow() {
    let keys = [i32::MIN, -1, 0, 1, i32::MAX];
    let m = ImmutableSortedMap::from_sorted(&keys, &[10, 20, 30, 40, 50]);

    // Open bound at INT_MIN: greater_than(MIN) excludes MIN, no `MIN - 1`.
    assert_eq!(
        m.range_keys(Range::greater_than(i32::MIN)),
        vec![-1, 0, 1, i32::MAX]
    );
    // Open bound at INT_MAX: less_than(MAX) excludes MAX, no `MAX + 1`.
    assert_eq!(
        m.range_keys(Range::less_than(i32::MAX)),
        vec![i32::MIN, -1, 0, 1]
    );
    // Closed both ends spanning the full signed range.
    assert_eq!(
        m.range_keys(Range::closed(i32::MIN, i32::MAX)),
        vec![i32::MIN, -1, 0, 1, i32::MAX]
    );
    // Singleton at the extreme.
    assert_eq!(m.range_keys(Range::singleton(i32::MAX)), vec![i32::MAX]);
}

// ── Range membership == range.contains (discrete-empty is NOT an error) ─

#[test]
fn open_range_over_adjacent_ints_is_empty_not_error() {
    let m = ImmutableSortedMap::from_sorted(&[1, 2], &[10, 20]);
    // open(1,2) over i32 matches NO key, yet is a valid empty result.
    assert!(m.range_keys(Range::open(1, 2)).is_empty());
    // cut-empty range matches nothing.
    assert!(m.range_keys(Range::closed_open(5, 5)).is_empty());
}

#[test]
fn range_query_contiguous_slice() {
    let keys: Vec<i32> = (1..=10).map(|i| i * 10).collect();
    let vals: Vec<i32> = keys.iter().map(|k| k * 10).collect();
    let m = ImmutableSortedMap::from_sorted(&keys, &vals);
    // closed_open(30,70) -> [30,40,50,60].
    assert_eq!(
        m.range_keys(Range::closed_open(30, 70)),
        vec![30, 40, 50, 60]
    );
    assert_eq!(
        m.descending_range_keys(Range::closed_open(30, 70)),
        vec![60, 50, 40, 30]
    );
    assert_eq!(
        m.range_entries(Range::closed(40, 50)),
        vec![(40, 400), (50, 500)]
    );
    // at_least / at_most / all.
    assert_eq!(m.range_keys(Range::at_least(80)), vec![80, 90, 100]);
    assert_eq!(m.range_keys(Range::at_most(30)), vec![10, 20, 30]);
    assert_eq!(m.range_keys(Range::all()).len(), 10);
}

// ── Lazy std-shape `range` (RangeBounds, borrowing, double-ended) ────

#[test]
fn lazy_range_all_bounds_match_snapshot() {
    let keys: Vec<i32> = (1..=10).map(|i| i * 10).collect();
    let vals: Vec<i32> = keys.iter().map(|k| k * 10).collect();
    let m = ImmutableSortedMap::from_sorted(&keys, &vals);

    // half-open a..b, closed a..=b, from a.., to ..b, to-inclusive ..=b, full ..
    let collect = |it: super::SortedRangeIter<'_, i32, i32>| -> Vec<(i32, i32)> {
        it.map(|(k, v)| (*k, *v)).collect()
    };
    assert_eq!(
        collect(m.range(30..70)),
        vec![(30, 300), (40, 400), (50, 500), (60, 600)]
    );
    assert_eq!(collect(m.range(40..=50)), vec![(40, 400), (50, 500)]);
    assert_eq!(
        collect(m.range(80..)),
        vec![(80, 800), (90, 900), (100, 1000)]
    );
    assert_eq!(
        collect(m.range(..=30)),
        vec![(10, 100), (20, 200), (30, 300)]
    );
    assert_eq!(collect(m.range(..30)), vec![(10, 100), (20, 200)]);
    assert_eq!(m.range(..).count(), 10);
    // Excluded start via explicit tuple bound: (30, 50] -> 40,50.
    use std::ops::Bound::{Excluded, Included};
    assert_eq!(
        collect(m.range((Excluded(30), Included(50)))),
        vec![(40, 400), (50, 500)]
    );
}

#[test]
fn lazy_range_matches_vec_methods() {
    let keys: Vec<i32> = (1..=10).map(|i| i * 10).collect();
    let vals: Vec<i32> = keys.iter().map(|k| k * 10).collect();
    let m = ImmutableSortedMap::from_sorted(&keys, &vals);
    // closed_open(30,70) is exactly `30..70` under RangeBounds.
    let lazy: Vec<(i32, i32)> = m.range(30..70).map(|(k, v)| (*k, *v)).collect();
    assert_eq!(lazy, m.range_entries(Range::closed_open(30, 70)));
    // descending == .rev().
    let desc: Vec<(i32, i32)> = m.range(30..70).rev().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(desc, m.descending_range_entries(Range::closed_open(30, 70)));
}

#[test]
fn lazy_range_double_ended_and_exact_size() {
    let keys: Vec<i32> = (1..=10).map(|i| i * 10).collect();
    let vals: Vec<i32> = keys.iter().map(|k| k * 10).collect();
    let m = ImmutableSortedMap::from_sorted(&keys, &vals);
    let mut it = m.range(20..=80); // 20,30,40,50,60,70,80 -> len 7
    assert_eq!(it.len(), 7);
    assert_eq!(it.next(), Some((&20, &200)));
    assert_eq!(it.next_back(), Some((&80, &800)));
    assert_eq!(it.len(), 5);
    assert_eq!(it.next_back(), Some((&70, &700)));
    assert_eq!(it.next(), Some((&30, &300)));
    // remaining middle: 40,50,60
    let rest: Vec<i32> = it.map(|(k, _)| *k).collect();
    assert_eq!(rest, vec![40, 50, 60]);
}

#[test]
#[allow(clippy::reversed_empty_ranges)] // intentionally inverted — exercises the empty-bracket path
fn lazy_range_inverted_and_empty_yield_nothing() {
    let keys: Vec<i32> = (1..=5).collect();
    let vals = keys.clone();
    let m = ImmutableSortedMap::from_sorted(&keys, &vals);
    assert_eq!(m.range(4..2).count(), 0); // inverted
    assert_eq!(m.range(3..3).count(), 0); // empty half-open
    assert_eq!(m.range(10..0).count(), 0); // fully out of range + inverted
    assert_eq!(m.range(100..200).count(), 0); // above everything
}

// NB: the lazy `range` lives in an `impl<K: Ord, V>` block (no `Copy`), so it
// is ready for non-`Copy` keys the moment construction drops its `Copy` bound
// (deferred T6-tail work). Until then every constructible instance is `Copy`,
// so a non-`Copy` exercise isn't buildable here.

#[test]
#[allow(clippy::reversed_empty_ranges)] // intentionally inverted at the end
fn lazy_range_set_mirrors_map() {
    let s = ImmutableSortedSet::from_sorted(&[10, 20, 30, 40, 50]);
    let got: Vec<i32> = s.range(20..=40).copied().collect();
    assert_eq!(got, vec![20, 30, 40]);
    let desc: Vec<i32> = s.range(20..=40).rev().copied().collect();
    assert_eq!(desc, vec![40, 30, 20]);
    let mut it = s.range(..); // full, double-ended
    assert_eq!(it.len(), 5);
    assert_eq!(it.next(), Some(&10));
    assert_eq!(it.next_back(), Some(&50));
    assert_eq!(it.len(), 3);
    assert_eq!(s.range(40..20).count(), 0); // inverted -> empty
}

// ── Large flat-array parity (paging-invariance is trivial for flat) ──

#[test]
fn large_flat_lookup_parity() {
    let keys: Vec<i32> = (0..10_000).collect();
    let vals: Vec<i32> = keys.iter().map(|k| k * 7).collect();
    let m = ImmutableSortedMap::from_sorted(&keys, &vals);
    assert_eq!(m.len(), 10_000);
    for probe in [0, 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 9999] {
        assert_eq!(m.get(&probe), Some(&(probe * 7)));
        assert_eq!(m.rank(&probe), probe as usize);
        assert_eq!(m.select_key(probe as usize), Some(&probe));
        assert_eq!(m.floor_key(&probe), Some(&probe));
        assert_eq!(m.ceiling_key(&probe), Some(&probe));
    }
    assert_eq!(m.get(&10_000), None);
    assert_eq!(m.rank(&10_000), 10_000);
    assert_eq!(m.select_key(10_000), None);
    // mid-range query spanning would-be page cuts.
    assert_eq!(m.range_keys(Range::closed_open(4090, 4100)).len(), 10);
}

// ── Set surface mirrors the map ──────────────────────────────────────

#[test]
fn set_full_surface() {
    let s = ImmutableSortedSet::from_sorted(&[10, 20, 30, 40, 50]);
    assert_eq!(s.len(), 5);
    assert!(s.contains(&30));
    assert!(!s.contains(&25));
    assert_eq!(s.first(), Some(&10));
    assert_eq!(s.last(), Some(&50));
    assert_eq!(s.floor(&25), Some(&20));
    assert_eq!(s.ceiling(&25), Some(&30));
    assert_eq!(s.lower(&10), None);
    assert_eq!(s.higher(&50), None);
    assert_eq!(s.rank(&30), 2);
    assert_eq!(s.select(0), Some(&10));
    assert_eq!(s.select(5), None);
    assert_eq!(
        s.elements().copied().collect::<Vec<_>>(),
        vec![10, 20, 30, 40, 50]
    );
    assert_eq!(s.descending_elements(), vec![50, 40, 30, 20, 10]);
    assert_eq!(
        s.range_elements(Range::closed_open(20, 50)),
        vec![20, 30, 40]
    );
    assert_eq!(
        s.descending_range_elements(Range::closed_open(20, 50)),
        vec![40, 30, 20]
    );
}

#[test]
fn set_snapshot_independence() {
    let mut elems = vec![1, 2, 3];
    let s = ImmutableSortedSet::from_sorted(&elems);
    elems[0] = 99;
    elems.push(4);
    assert_eq!(s.len(), 3);
    assert!(s.contains(&1));
    assert!(!s.contains(&99));
}

// ── Fallible constructors (try_from_sorted) ──────────────────────────

#[test]
fn try_from_sorted_map_ok_and_errors() {
    use crate::BulkError;

    // Valid strictly-ascending input.
    let m = ImmutableSortedMap::try_from_sorted(&[1, 3, 5], &[10, 30, 50]).unwrap();
    assert_eq!(m.get(&3), Some(&30));
    assert_eq!(m.len(), 3);

    // Length mismatch.
    assert!(matches!(
        ImmutableSortedMap::try_from_sorted(&[1, 2], &[10]),
        Err(BulkError::LengthMismatch { keys: 2, values: 1 })
    ));

    // Duplicate key (equal step) -> Duplicate at the offending index.
    assert!(matches!(
        ImmutableSortedMap::try_from_sorted(&[1, 2, 2], &[1, 2, 3]),
        Err(BulkError::Duplicate { index: 2 })
    ));

    // Out-of-order (descending step) -> OutOfOrder.
    assert!(matches!(
        ImmutableSortedMap::try_from_sorted(&[1, 5, 3], &[1, 2, 3]),
        Err(BulkError::OutOfOrder { index: 2 })
    ));

    // Empty and single are valid.
    assert_eq!(
        ImmutableSortedMap::<i32, i32>::try_from_sorted(&[], &[])
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        ImmutableSortedMap::try_from_sorted(&[7], &[70])
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn try_from_sorted_iter_map_never_length_mismatches() {
    use crate::BulkError;
    let ok = ImmutableSortedMap::try_from_sorted_iter([(1, 1), (2, 2)]).unwrap();
    assert_eq!(ok.len(), 2);
    assert!(matches!(
        ImmutableSortedMap::try_from_sorted_iter([(2, 2), (1, 1)]),
        Err(BulkError::OutOfOrder { index: 1 })
    ));
}

#[test]
fn try_from_sorted_set_ok_and_errors() {
    use crate::BulkError;
    let s = ImmutableSortedSet::try_from_sorted(&[1, 2, 4]).unwrap();
    assert!(s.contains(&2) && !s.contains(&3));
    assert!(matches!(
        ImmutableSortedSet::try_from_sorted(&[1, 1]),
        Err(BulkError::Duplicate { index: 1 })
    ));
    assert!(matches!(
        ImmutableSortedSet::try_from_sorted(&[3, 1]),
        Err(BulkError::OutOfOrder { index: 1 })
    ));
    assert!(matches!(
        ImmutableSortedSet::try_from_sorted_iter([5, 4]),
        Err(BulkError::OutOfOrder { index: 1 })
    ));
}
