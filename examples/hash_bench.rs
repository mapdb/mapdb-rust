// Copyright (c) 2026 Jan Kotek.
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Micro-benchmarks for the grouped Swiss-table `OpenHashMap` / `OpenHashSet`.
//!
//! Run with:
//! ```text
//! RUSTFLAGS="-C target-cpu=native" cargo run --offline --release --example hash_bench
//! ```
//!
//! Each op reports best-of-N wall time and a derived throughput (Mitem/s). A
//! running checksum is printed so the optimizer cannot elide the work. We bench:
//!
//!   * OpenHashMap<i64,i64> / OpenHashSet<i64>: build, lookup-hit, lookup-miss,
//!     remove, iterate at N = 1M, 8M, 100M.
//!   * OpenHashMap<String,u64>: build, hit, miss at 1M, 8M.
//!   * std::collections::HashMap/HashSet (hashbrown) as a reference ceiling at
//!     1M, 8M, printed side by side.
//!   * The perf_suite scenario-3 dedup+filter+aggregate pipeline at 1M, 8M.
//!
//! Dependency-free (`std::time` + an xorshift PRNG), matching `perf_suite.rs`.

use mapdb_collections::object::ArrayList;
use mapdb_collections::{BitSet, OpenHashMap, OpenHashSet};
use std::collections::{HashMap as StdMap, HashSet as StdSet};
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

/// Like `bench`, but `black_box`es the checksum immediately and returns only
/// the time. Used for the reference (std) variants whose checksum would
/// otherwise be dropped by the caller — without this the optimizer is free to
/// elide read-only loops (e.g. iterate) entirely, yielding bogus "infinite"
/// throughput.
fn std_bench(rounds: u32, f: impl FnMut() -> u64) -> Duration {
    let (d, sink) = bench(rounds, f);
    black_box(sink);
    d
}

fn mps(d: Duration, items: usize) -> f64 {
    items as f64 / d.as_secs_f64() / 1e6
}

/// One row: label + our-table time/throughput + (optional) std side-by-side.
fn row2(label: &str, ours: Duration, std: Option<Duration>, items: usize) {
    let o = mps(ours, items);
    match std {
        Some(s) => {
            let sm = mps(s, items);
            println!(
                "    {label:<14} {:>9.3?} {o:>8.1} M/s   | std {:>9.3?} {sm:>8.1} M/s   ({:.2}× of std)",
                ours, s, o / sm
            );
        }
        None => println!("    {label:<14} {:>9.3?} {o:>8.1} M/s", ours),
    }
}

/// Tiny deterministic PRNG (xorshift64*) — no `rand` dependency.
struct Rng(u64);
impl Rng {
    #[inline]
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

/// N distinct, well-scrambled i64 keys (present set) and N absent keys.
/// Present keys are even, absent keys are odd ⇒ disjoint by construction.
fn make_keys(n: usize) -> (Vec<i64>, Vec<i64>) {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let present: Vec<i64> = (0..n).map(|i| (i as i64) << 1).collect();
    // shuffle-ish: scramble order with the PRNG (Fisher–Yates).
    let mut present = present;
    for i in (1..n).rev() {
        let j = (rng.next() as usize) % (i + 1);
        present.swap(i, j);
    }
    let absent: Vec<i64> = (0..n).map(|i| ((i as i64) << 1) | 1).collect();
    (present, absent)
}

// ── i64 map: our table vs std, all ops ───────────────────────────────────────

fn bench_i64(n: usize, rounds: u32, with_std: bool) {
    println!("\n[OpenHashMap<i64,i64> / OpenHashSet<i64>]  N = {n}");
    let (present, absent) = make_keys(n);

    // ---- MAP ----
    // build
    let (b_ours, c1) = bench(rounds, || {
        let mut m: OpenHashMap<i64, i64> = OpenHashMap::new();
        for &k in black_box(&present) {
            m.insert(k, k ^ 0x55);
        }
        m.len() as u64
    });
    let b_std = with_std.then(|| {
        std_bench(rounds, || {
            let mut m: StdMap<i64, i64> = StdMap::new();
            for &k in black_box(&present) {
                m.insert(k, k ^ 0x55);
            }
            m.len() as u64
        })
    });

    // Prebuilt instances for the read/iterate ops.
    let mut m: OpenHashMap<i64, i64> = OpenHashMap::new();
    for &k in &present {
        m.insert(k, k ^ 0x55);
    }
    let mut sm: StdMap<i64, i64> = StdMap::new();
    for &k in &present {
        sm.insert(k, k ^ 0x55);
    }

    // lookup hit
    let (h_ours, c2) = bench(rounds, || {
        let mut s = 0u64;
        for &k in black_box(&present) {
            s = s.wrapping_add(*m.get(&k).unwrap() as u64);
        }
        s
    });
    let h_std = with_std.then(|| {
        std_bench(rounds, || {
            let mut s = 0u64;
            for &k in black_box(&present) {
                s = s.wrapping_add(*sm.get(&k).unwrap() as u64);
            }
            s
        })
    });

    // lookup miss
    let (mi_ours, c3) = bench(rounds, || {
        let mut s = 0u64;
        for &k in black_box(&absent) {
            s = s.wrapping_add(m.get(&k).is_none() as u64);
        }
        s
    });
    let mi_std = with_std.then(|| {
        std_bench(rounds, || {
            let mut s = 0u64;
            for &k in black_box(&absent) {
                s = s.wrapping_add(sm.get(&k).is_none() as u64);
            }
            s
        })
    });

    // iterate (sum values)
    let (it_ours, c4) = bench(rounds, || {
        let mut s = 0u64;
        for (k, v) in m.iter() {
            s = s.wrapping_add(*k as u64 ^ *v as u64);
        }
        s
    });
    let it_std = with_std.then(|| {
        std_bench(rounds, || {
            let mut s = 0u64;
            for (k, v) in sm.iter() {
                s = s.wrapping_add(*k as u64 ^ *v as u64);
            }
            s
        })
    });

    // remove (rebuild each round, then remove all -> empty). Build cost is shared
    // by both, so this is a fair relative number though it includes a build.
    let (rm_ours, c5) = bench(rounds, || {
        let mut mm: OpenHashMap<i64, i64> = OpenHashMap::with_capacity(n);
        for &k in &present {
            mm.insert(k, k);
        }
        let mut removed = 0u64;
        for &k in black_box(&present) {
            removed += mm.remove(&k).is_some() as u64;
        }
        removed ^ mm.len() as u64
    });
    let rm_std = with_std.then(|| {
        std_bench(rounds, || {
            let mut mm: StdMap<i64, i64> = StdMap::with_capacity(n);
            for &k in &present {
                mm.insert(k, k);
            }
            let mut removed = 0u64;
            for &k in black_box(&present) {
                removed += mm.remove(&k).is_some() as u64;
            }
            removed ^ mm.len() as u64
        })
    });

    println!("  map:");
    row2("build", b_ours, b_std, n);
    row2("lookup hit", h_ours, h_std, n);
    row2("lookup miss", mi_ours, mi_std, n);
    row2("remove", rm_ours, rm_std, n);
    row2("iterate", it_ours, it_std, n);

    // ---- SET ----
    let (sb_ours, c6) = bench(rounds, || {
        let mut s: OpenHashSet<i64> = OpenHashSet::new();
        for &k in black_box(&present) {
            s.insert(k);
        }
        s.len() as u64
    });
    let sb_std = with_std.then(|| {
        std_bench(rounds, || {
            let mut s: StdSet<i64> = StdSet::new();
            for &k in black_box(&present) {
                s.insert(k);
            }
            s.len() as u64
        })
    });

    let mut set: OpenHashSet<i64> = OpenHashSet::new();
    for &k in &present {
        set.insert(k);
    }
    let mut sset: StdSet<i64> = StdSet::new();
    for &k in &present {
        sset.insert(k);
    }

    let (sh_ours, c7) = bench(rounds, || {
        let mut s = 0u64;
        for &k in black_box(&present) {
            s += set.contains(&k) as u64;
        }
        s
    });
    let sh_std = with_std.then(|| {
        std_bench(rounds, || {
            let mut s = 0u64;
            for &k in black_box(&present) {
                s += sset.contains(&k) as u64;
            }
            s
        })
    });

    let (sm_ours, c8) = bench(rounds, || {
        let mut s = 0u64;
        for &k in black_box(&absent) {
            s += (!set.contains(&k)) as u64;
        }
        s
    });
    let sm_std = with_std.then(|| {
        std_bench(rounds, || {
            let mut s = 0u64;
            for &k in black_box(&absent) {
                s += (!sset.contains(&k)) as u64;
            }
            s
        })
    });

    let (si_ours, c9) = bench(rounds, || {
        let mut s = 0u64;
        for k in set.iter() {
            s = s.wrapping_add(*k as u64);
        }
        s
    });
    let si_std = with_std.then(|| {
        std_bench(rounds, || {
            let mut s = 0u64;
            for k in &sset {
                s = s.wrapping_add(*k as u64);
            }
            s
        })
    });

    println!("  set:");
    row2("build", sb_ours, sb_std, n);
    row2("contains hit", sh_ours, sh_std, n);
    row2("contains miss", sm_ours, sm_std, n);
    row2("iterate", si_ours, si_std, n);

    black_box(
        c1 ^ c2 ^ c3 ^ c4 ^ c5 ^ c6 ^ c7 ^ c8 ^ c9,
    );
}

// ── String-key map ───────────────────────────────────────────────────────────

fn bench_string(n: usize, rounds: u32) {
    println!("\n[OpenHashMap<String,u64>]  N = {n}");
    // distinct present keys; absent keys share the prefix but never collide.
    let present: Vec<String> = (0..n).map(|i| format!("key-{:016x}", i as u64)).collect();
    let absent: Vec<String> = (0..n).map(|i| format!("zzz-{:016x}", i as u64)).collect();

    let (b, c1) = bench(rounds, || {
        let mut m: OpenHashMap<String, u64> = OpenHashMap::with_capacity(n);
        for (i, k) in black_box(&present).iter().enumerate() {
            m.insert(k.clone(), i as u64);
        }
        m.len() as u64
    });
    let b_std = std_bench(rounds, || {
        let mut m: StdMap<String, u64> = StdMap::with_capacity(n);
        for (i, k) in black_box(&present).iter().enumerate() {
            m.insert(k.clone(), i as u64);
        }
        m.len() as u64
    });

    let mut m: OpenHashMap<String, u64> = OpenHashMap::with_capacity(n);
    for (i, k) in present.iter().enumerate() {
        m.insert(k.clone(), i as u64);
    }
    let mut sm: StdMap<String, u64> = StdMap::with_capacity(n);
    for (i, k) in present.iter().enumerate() {
        sm.insert(k.clone(), i as u64);
    }

    let (h, c2) = bench(rounds, || {
        let mut s = 0u64;
        for k in black_box(&present) {
            s = s.wrapping_add(*m.get(k).unwrap());
        }
        s
    });
    let h_std = std_bench(rounds, || {
        let mut s = 0u64;
        for k in black_box(&present) {
            s = s.wrapping_add(*sm.get(k).unwrap());
        }
        s
    });

    let (mi, c3) = bench(rounds, || {
        let mut s = 0u64;
        for k in black_box(&absent) {
            s += m.get(k).is_none() as u64;
        }
        s
    });
    let mi_std = std_bench(rounds, || {
        let mut s = 0u64;
        for k in black_box(&absent) {
            s += sm.get(k).is_none() as u64;
        }
        s
    });

    row2("build", b, Some(b_std), n);
    row2("lookup hit", h, Some(h_std), n);
    row2("lookup miss", mi, Some(mi_std), n);
    black_box(c1 ^ c2 ^ c3);
}

// ── scenario 3: dedup + membership filter + aggregate (from perf_suite) ───────

fn scenario_pipeline(n: usize, rounds: u32) {
    let mut rng = Rng(0xD1B54A32D192ED03);
    let ids: ArrayList<i64> = (0..n)
        .map(|_| (rng.next() % (n as u64 / 4).max(1)) as i64)
        .collect();
    let mut allowed = BitSet::new();
    let mut r2 = Rng(0x123456789);
    let keyspace = (n / 4).max(1);
    for _ in 0..(keyspace / 2) {
        allowed.set((r2.next() as usize) % keyspace);
    }

    let (d, sink) = bench(rounds, || {
        let mut seen: OpenHashSet<i64> = OpenHashSet::new();
        for v in black_box(&ids) {
            seen.insert(*v);
        }
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
    println!(
        "    dedup+filter+aggregate  n={n:>9}   {:>9.3?}   {:>8.1} Mitem/s",
        d,
        mps(d, n)
    );
    black_box(sink);
}

fn main() {
    println!(
        "hash_bench — Swiss-table OpenHashMap/OpenHashSet micro-benchmarks\n\
         cores={}  (build with RUSTFLAGS=\"-C target-cpu=native\")",
        std::thread::available_parallelism()
            .map(|x| x.get())
            .unwrap_or(0)
    );

    println!("\n=== i64 keys: our table vs std (hashbrown) ceiling ===");
    bench_i64(1_000_000, 15, true);
    bench_i64(8_000_000, 7, true);

    println!("\n=== i64 keys: 100M (our table only) ===");
    bench_i64(100_000_000, 2, false);

    println!("\n=== String keys: our table vs std ===");
    bench_string(1_000_000, 10);
    bench_string(8_000_000, 4);

    println!("\n=== scenario-3 pipeline (compare vs old baseline 42.5 / 13.8 Mitem/s) ===");
    scenario_pipeline(1_000_000, 15);
    scenario_pipeline(8_000_000, 7);

    println!("\ndone.");
}
