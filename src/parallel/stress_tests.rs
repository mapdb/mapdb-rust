// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Stress tests for the parallel executors. These deliberately run with a
//! `min_fork_size` of 1 and high task counts to maximize contention, repeat
//! many rounds to shake out races, and cross-check every parallel result
//! against a sequential oracle. Compiled only under `--features parallel`.

use super::{batch, iterable::as_parallel};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Tiny deterministic LCG so the tests are reproducible without a `rand` dep.
fn lcg(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 16
}

/// Every element must be visited exactly once across all worker threads, over
/// many rounds. An XOR fingerprint catches dropped/duplicated elements that a
/// plain count would miss, and the count catches gross over/under-visiting.
#[test]
fn batch_for_each_visits_each_element_exactly_once() {
    let n: u64 = 200_000;
    let data: Vec<u64> = (0..n).collect();
    let expected_xor = (0..n).fold(0u64, |a, b| a ^ b);

    for round in 0..40 {
        let xor = AtomicU64::new(0);
        let visits = AtomicUsize::new(0);
        // Vary the worker count per round to exercise different sectionings.
        let tasks = 2 + (round % 31) as usize;
        batch::for_each_with(
            &data,
            |v| {
                xor.fetch_xor(*v, Ordering::Relaxed);
                visits.fetch_add(1, Ordering::Relaxed);
            },
            1,
            tasks,
        );
        assert_eq!(visits.load(Ordering::Relaxed), n as usize, "round {round}");
        assert_eq!(xor.load(Ordering::Relaxed), expected_xor, "round {round}");
    }
}

/// The fixed-chunk batch executor, the rayon work-stealing view, and a
/// sequential oracle must all agree — across many randomized predicates.
#[test]
fn batch_rayon_and_sequential_agree() {
    let mut seed = 0x1234_5678_9abc_def0u64;
    let data: Vec<i64> = (0..50_000)
        .map(|_| (lcg(&mut seed) % 1000) as i64)
        .collect();

    for _ in 0..25 {
        let threshold = (lcg(&mut seed) % 1000) as i64;
        let pred = move |v: &i64| *v < threshold;

        let seq_select: Vec<i64> = data.iter().copied().filter(|v| pred(v)).collect();
        let seq_count = seq_select.len();
        let seq_sum: i64 = data.iter().sum();
        let seq_doubled: Vec<i64> = data.iter().map(|v| v * 2).collect();

        // Fixed-chunk batch path (low fork threshold forces parallelism).
        assert_eq!(batch::select_with(&data, pred, 1, 16), seq_select);
        assert_eq!(batch::count_with(&data, pred, 1, 16), seq_count);
        assert_eq!(batch::collect_with(&data, |v| v * 2, 1, 16), seq_doubled);
        assert_eq!(batch::sum_by(&data, |v| *v), seq_sum);
        assert_eq!(batch::any_satisfy(&data, pred), seq_count > 0);
        assert_eq!(batch::all_satisfy(&data, pred), seq_count == data.len());

        // rayon work-stealing path.
        let p = as_parallel(&data);
        assert_eq!(p.select(pred), seq_select);
        assert_eq!(p.count(pred), seq_count);
        assert_eq!(p.collect(|v| v * 2), seq_doubled);
        assert_eq!(p.sum_by(|v| *v), seq_sum);
        assert_eq!(p.any_satisfy(pred), seq_count > 0);
        assert_eq!(p.all_satisfy(pred), seq_count == data.len());
    }
}

/// Parallel summation must be deterministic and equal to the sequential sum
/// every time — a torn or double-counted partial would surface here.
#[test]
fn parallel_sum_is_stable() {
    let data: Vec<i64> = (0..500_000).collect();
    let expected: i64 = data.iter().sum();
    for _ in 0..30 {
        assert_eq!(batch::sum_by(&data, |v| *v), expected);
        assert_eq!(as_parallel(&data).sum_by(|v| *v), expected);
    }
}

/// Hammer `any_satisfy` short-circuiting with a single needle at varying
/// positions: it must always find it, and never report a false positive on a
/// needle-free array, regardless of how sections race to the flag.
#[test]
fn any_satisfy_short_circuit_is_correct_under_contention() {
    let n = 100_000usize;
    for &pos in &[0usize, 1, n / 2, n - 2, n - 1] {
        let mut data = vec![0i32; n];
        data[pos] = 7;
        for _ in 0..20 {
            assert!(batch::any_satisfy(&data, |v| *v == 7), "needle at {pos}");
            assert!(
                as_parallel(&data).any_satisfy(|v| *v == 7),
                "needle at {pos}"
            );
        }
    }
    let clean = vec![0i32; n];
    for _ in 0..20 {
        assert!(!batch::any_satisfy(&clean, |v| *v == 7));
        assert!(!as_parallel(&clean).any_satisfy(|v| *v == 7));
    }
}

/// A non-trivial nested workload: parallel-map then parallel-filter, compared
/// to the sequential pipeline, repeated to expose ordering/aliasing bugs.
#[test]
fn pipeline_collect_then_select_matches_sequential() {
    let data: Vec<u64> = (0..80_000).collect();
    for _ in 0..15 {
        let mapped = batch::collect_with(&data, |v| v.wrapping_mul(2654435761), 1, 12);
        let filtered = batch::select_with(&mapped, |v| v % 3 == 0, 1, 12);

        let seq: Vec<u64> = data
            .iter()
            .map(|v| v.wrapping_mul(2654435761))
            .filter(|v| v % 3 == 0)
            .collect();
        assert_eq!(filtered, seq);
    }
}
