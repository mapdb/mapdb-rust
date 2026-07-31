// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Workload-level performance suite (not micro-benchmarks).
//!
//! Run with:
//! ```text
//! cargo run --release --example perf_suite
//! # or pin the CPU so kernels can use the widest available ISA:
//! RUSTFLAGS="-C target-cpu=native" cargo run --release --example perf_suite
//! ```
//!
//! Each scenario combines several collection operations into one realistic task
//! and reports best-of-N wall time plus a derived throughput. Where an
//! optimization applies, the suite runs the *old* and *new* shapes side by side
//! so the effect is visible on a real workload rather than a tight micro-loop:
//!
//!   * Scenario 1 — BitSet membership/sieve pipeline. Contrasts the streaming
//!     set-bit iterator (`for i in &bitset`, `word &= word - 1`) against the
//!     previous repeated-`next_set_bit(b + 1)` scan.
//!   * Scenario 2 — ArrayList analytics "query". Contrasts the slice-backed
//!     `Collection` bulk methods against the boxed `dyn Iterator` path (both an
//!     inlinable and an opaque/non-inlinable factory, to show the devirt cliff).
//!   * Scenario 3 — end-to-end dedup + membership-filter + aggregate across
//!     ArrayList + OpenHashSet + BitSet.
//!
//! Dependency-free (`std::time` only), matching the crate's ethos.

use mapdb_collections::object::{ArrayList, Collection};
use mapdb_collections::{BitSet, OpenHashSet};
use std::hint::black_box;
use std::time::{Duration, Instant};

// ── harness ──────────────────────────────────────────────────────────────────

/// Best-of-`rounds` wall time for `f` (after a warm-up), plus a checksum so the
/// optimizer cannot elide the work.
fn bench(rounds: u32, mut f: impl FnMut() -> u64) -> (Duration, u64) {
    let mut sink = f();
    let mut best = Duration::MAX;
    for _ in 0..rounds {
        let t = Instant::now();
        sink = sink.wrapping_add(f());
        best = best.min(t.elapsed());
    }
    (best, sink)
}

fn row(label: &str, d: Duration, items: usize) {
    let mps = items as f64 / d.as_secs_f64() / 1e6;
    println!("    {label:<34} {:>10.3?}   {mps:>8.1} Mitem/s", d);
}

fn speedup(label: &str, slow: Duration, fast: Duration) {
    println!(
        "    => {label}: {:.2}× ({:.3?} -> {:.3?})",
        slow.as_secs_f64() / fast.as_secs_f64(),
        slow,
        fast
    );
}

/// Tiny deterministic PRNG (xorshift64*) — no `rand` dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

// ── scenario 1: BitSet sieve + iterate + algebra ────────────────────────────

/// Reference iteration, replicating the previous `next_set_bit(b + 1)` scan.
fn rescan_sum(bs: &BitSet) -> u64 {
    let mut s = 0u64;
    let mut b = bs.next_set_bit(0);
    while let Some(i) = b {
        s = s.wrapping_add(i as u64);
        b = bs.next_set_bit(i + 1);
    }
    s
}

/// Streaming iteration via the optimized `IntoIterator` (`word &= word - 1`).
fn stream_sum(bs: &BitSet) -> u64 {
    let mut s = 0u64;
    for i in bs {
        s = s.wrapping_add(i as u64);
    }
    s
}

fn scenario_bitset(n: usize, rounds: u32) {
    println!("\n[1] BitSet sieve + iterate + set algebra  (n = {n})");
    // Build a sieve of Eratosthenes: a realistic dense write pattern.
    let mut sieve = BitSet::new();
    sieve.set(0); // we'll treat set bits as "composite" then iterate them
    let mut i = 2usize;
    while i * i < n {
        if !sieve.get(i) {
            let mut j = i * i;
            while j < n {
                sieve.set(j);
                j += i;
            }
        }
        i += 1;
    }
    let composites = sieve.cardinality();
    println!(
        "    sieve built: {composites} composite bits set ({:.1}%)",
        100.0 * composites as f64 / n as f64
    );

    let (d_rescan, c1) = bench(rounds, || rescan_sum(black_box(&sieve)));
    let (d_stream, c2) = bench(rounds, || stream_sum(black_box(&sieve)));
    assert_eq!(c1, c2, "iteration variants must agree");
    row("iterate: rescan next_set_bit", d_rescan, composites);
    row("iterate: streaming (BLSR)", d_stream, composites);
    speedup("streaming set-bit iterator", d_rescan, d_stream);

    // set algebra: intersect the sieve with a strided mask, take cardinality.
    let mut mask = BitSet::new();
    let mut k = 0;
    while k < n {
        mask.set(k);
        k += 3;
    }
    let (d_alg, c3) = bench(rounds, || {
        let mut a = sieve.clone();
        a.and_in_place(black_box(&mask));
        a.cardinality() as u64
    });
    row("intersect + cardinality", d_alg, n);
    black_box(c3);
}

// ── scenario 2: ArrayList analytics query ───────────────────────────────────

/// Opaque boxed-iterator factory: models the cross-crate / non-LTO trait-object
/// boundary the slice override removes the dependence on.
#[inline(never)]
fn opaque_boxed(list: &ArrayList<i64>) -> Box<dyn Iterator<Item = &i64> + '_> {
    black_box(list.iter())
}

fn scenario_arraylist(n: usize, rounds: u32) {
    println!("\n[2] ArrayList analytics query  (n = {n})");
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let list: ArrayList<i64> = (0..n).map(|_| (rng.next() % 1000) as i64).collect();
    let threshold = 500i64;

    // The query: count over threshold, sum, and locate the first big value.
    // (a) slice fast-path Collection methods (the optimized path)
    let (d_fast, cf) = bench(rounds, || {
        let cnt = list.count_where(|v| *v >= threshold) as u64;
        let sum = list.inject_into(0i64, |a, v| a + *v) as u64;
        let det = list.detect(|v| *v == 999).copied().unwrap_or(-1) as u64;
        cnt ^ sum ^ det
    });
    // (b) same query via the boxed dyn-iterator, factory visible to optimizer
    let (d_boxed, cb) = bench(rounds, || {
        let cnt = list.iter().filter(|v| **v >= threshold).count() as u64;
        let sum = list.iter().fold(0i64, |a, v| a + *v) as u64;
        let det = list.iter().find(|v| **v == 999).copied().unwrap_or(-1) as u64;
        cnt ^ sum ^ det
    });
    // (c) same query via an OPAQUE boxed factory (non-inlinable boundary)
    let (d_opaque, co) = bench(rounds, || {
        let cnt = opaque_boxed(&list).filter(|v| **v >= threshold).count() as u64;
        let sum = opaque_boxed(&list).fold(0i64, |a, v| a + *v) as u64;
        let det = opaque_boxed(&list)
            .find(|v| **v == 999)
            .copied()
            .unwrap_or(-1) as u64;
        cnt ^ sum ^ det
    });
    assert_eq!(cf, cb, "fast vs boxed must agree");
    assert_eq!(cf, co, "fast vs opaque must agree");
    // 3 passes over the list per round.
    row("query: slice fast-path", d_fast, n * 3);
    row("query: boxed dyn (inlinable)", d_boxed, n * 3);
    row("query: boxed dyn (opaque factory)", d_opaque, n * 3);
    speedup("slice fast-path vs opaque-boxed", d_opaque, d_fast);

    // select() materializes a filtered Vec — realistic "give me the matches".
    let (d_sel, cs) = bench(rounds, || list.select(|v| *v >= threshold).len() as u64);
    row("select (materialize matches)", d_sel, n);
    black_box(cs);
}

// ── scenario 3: end-to-end dedup + membership filter + aggregate ─────────────

fn scenario_pipeline(n: usize, rounds: u32) {
    println!("\n[3] end-to-end: dedup -> membership filter -> aggregate  (n = {n})");
    let mut rng = Rng(0xD1B54A32D192ED03);
    // A stream of ids with heavy duplication (mod keeps the key space small).
    let ids: ArrayList<i64> = (0..n)
        .map(|_| (rng.next() % (n as u64 / 4).max(1)) as i64)
        .collect();
    // A membership mask: ids that pass some upstream filter.
    let mut allowed = BitSet::new();
    let mut r2 = Rng(0x123456789);
    let keyspace = (n / 4).max(1);
    for _ in 0..(keyspace / 2) {
        allowed.set((r2.next() as usize) % keyspace);
    }

    let (d, sink) = bench(rounds, || {
        // 1. dedup via OpenHashSet
        let mut seen: OpenHashSet<i64> = OpenHashSet::new();
        for v in black_box(&ids) {
            seen.insert(*v);
        }
        // 2. membership filter via BitSet + 3. aggregate
        let mut sum = 0u64;
        let mut kept = 0u64;
        for v in seen.iter() {
            if allowed.get(*v as usize) {
                sum = sum.wrapping_add(*v as u64);
                kept += 1;
            }
        }
        sum ^ kept ^ seen.len() as u64
    });
    row("dedup+filter+aggregate", d, n);
    black_box(sink);
}

fn main() {
    println!(
        "perf_suite — mapdb-collections workload benchmarks\n\
         cores={}  (build with RUSTFLAGS=\"-C target-cpu=native\" for widest SIMD)",
        std::thread::available_parallelism()
            .map(|x| x.get())
            .unwrap_or(0)
    );

    for &n in &[1_000_000usize, 8_000_000] {
        scenario_bitset(n, 25);
        scenario_arraylist(n, 25);
        scenario_pipeline(n, 15);
    }
    println!("\ndone.");
}
