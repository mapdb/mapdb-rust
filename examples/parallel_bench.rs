// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Dependency-free micro-benchmark for the parallel executors.
//!
//! Run with:
//! ```text
//! cargo run --release --example parallel_bench --features parallel
//! ```
//!
//! Uses only `std::time` — no `criterion`/`rand`, in keeping with the crate's
//! minimal-dependency ethos. It compares sequential, fixed-chunk batch (no work
//! stealing), and rayon work-stealing execution on a deliberately heavy
//! per-element workload, and sweeps input sizes around `DEFAULT_MIN_FORK_SIZE`
//! to show where parallelism starts paying off.

use mapdb_collections::parallel::{batch, iterable::as_parallel};
use std::time::{Duration, Instant};

/// A CPU-bound, side-effect-free transform so the cost is real work, not memory
/// traffic. Integer hashing iterated a few times.
#[inline]
fn work(x: u64) -> u64 {
    let mut h = x;
    for _ in 0..64 {
        h = h
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        h ^= h >> 31;
    }
    h
}

fn time_it(label: &str, rounds: u32, mut f: impl FnMut() -> u64) {
    // Warm up once, then take the best of `rounds` to reduce noise.
    let mut sink = f();
    let mut best = Duration::MAX;
    for _ in 0..rounds {
        let t = Instant::now();
        sink = sink.wrapping_add(f());
        best = best.min(t.elapsed());
    }
    // Print the sink so the optimizer can't elide the computation.
    println!("    {label:<28} {best:>10.3?}  (checksum {sink:016x})");
}

fn main() {
    let threshold = batch::DEFAULT_MIN_FORK_SIZE;
    println!(
        "cores={}  default_task_count={}  DEFAULT_MIN_FORK_SIZE={}",
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0),
        batch::default_task_count(),
        threshold,
    );

    for &n in &[1_000usize, threshold, 100_000, 1_000_000, 10_000_000] {
        let data: Vec<u64> = (0..n as u64).collect();
        println!("\nn = {n}");

        time_it("sequential map+sum", 5, || {
            data.iter().map(|v| work(*v)).sum()
        });

        time_it("batch (no steal)", 5, || {
            batch::collect(&data, |v| work(*v)).iter().sum()
        });

        time_it("rayon (work stealing)", 5, || {
            as_parallel(&data).sum_by(|v| work(*v))
        });
    }
}
