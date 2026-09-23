// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Requested-byte measurement for five collections. Not a library API.
//!
//! The counting allocator is installed only for this binary
//! (`#[global_allocator]` here). The library crate forbids `unsafe` and does
//! not name this type.
//!
//! `String → Vec<u8>` (and the 8M-entry string maps in
//! `todo/cloudflare-dns-cache/03-improvements.md` §0) is deferred until this
//! integer table exists. This file does not build that corpus.
//!
//! Counts are requested `Layout` sizes forwarded to the system allocator.
//! `realloc` is forwarded (so growth-in-place still happens) and accounted as
//! one free of the old size plus one alloc of the new size. Usable size after
//! malloc size-class rounding is not visible. `live_bytes_if_known` is
//! alloc bytes minus free bytes at the end of the build, before lookups.
//!
//! ```text
//! cargo run --release --example bytes_per_collection
//! cargo run --release --example bytes_per_collection -- --smoke
//! cargo run --release --example bytes_per_collection -- --full
//! ```
//!
//! Stdout is one tab-separated row per id. Stderr is notes and checksums.

use mapdb_collections::object::TreeMap;
use mapdb_collections::{ImmutableSortedMap, Multimap, OpenHashMap, RoaringU32};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

const LOOKUPS: usize = 10_000;
const WARMUP: usize = 64;
const VALUES_PER_KEY: usize = 8;
const ROARING_STRIDE: u32 = 100;
/// Non-zero. Same constant as the Zig harness.
const XORSHIFT_SEED: u64 = 0xA11C_E5EE_D000_0001;

struct BytesPerCollectionAlloc {
    inner: System,
}

static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static FREE_BYTES: AtomicU64 = AtomicU64::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);

#[global_allocator]
static GLOBAL: BytesPerCollectionAlloc = BytesPerCollectionAlloc { inner: System };

unsafe impl GlobalAlloc for BytesPerCollectionAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() && ENABLED.load(Ordering::Relaxed) {
            ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ENABLED.load(Ordering::Relaxed) {
            FREE_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { self.inner.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { self.inner.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() && ENABLED.load(Ordering::Relaxed) {
            FREE_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            ALLOC_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        new_ptr
    }
}

fn reset_counters() {
    ALLOC_BYTES.store(0, Ordering::Relaxed);
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    FREE_BYTES.store(0, Ordering::Relaxed);
}

fn snapshot() -> (u64, u64, i64) {
    let alloc_bytes = ALLOC_BYTES.load(Ordering::Relaxed);
    let alloc_count = ALLOC_COUNT.load(Ordering::Relaxed);
    let free_bytes = FREE_BYTES.load(Ordering::Relaxed);
    let live = alloc_bytes as i64 - free_bytes as i64;
    (alloc_bytes, alloc_count, live)
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Fisher–Yates shuffle of `0..n` driven by `xorshift64`. Deterministic.
fn permutation(n: usize) -> Vec<i64> {
    let mut keys: Vec<i64> = (0..n as i64).collect();
    let mut state = XORSHIFT_SEED;
    let mut i = n;
    while i > 1 {
        i -= 1;
        let r = xorshift64(&mut state);
        let j = (r as usize) % (i + 1);
        keys.swap(i, j);
    }
    keys
}

struct Corpus {
    map_n: usize,
    mm_keys: usize,
    /// Insertion order for the hash map: a xorshift permutation of `0..map_n`.
    hash_keys: Vec<i64>,
    /// Strictly ascending keys for `ImmutableSortedMap::from_sorted`.
    sorted_keys: Vec<i64>,
    sorted_vals: Vec<i64>,
    /// `LOOKUPS` indices in `0..map_n`, from the same xorshift64 and seed.
    /// Covers the key domain (not a prefix) for the tree, immutable, and roaring rows.
    lookup_map: Vec<usize>,
    /// `LOOKUPS` indices in `0..mm_keys`, same generator, for the multimap.
    lookup_mm: Vec<usize>,
}

/// `count` draws in `0..n` from xorshift64 restarted at `XORSHIFT_SEED`.
/// Restarting (rather than continuing the Fisher–Yates stream) keeps the
/// draw independent of `n` and identical in the Zig harness.
fn draw(n: usize, count: usize) -> Vec<usize> {
    let mut state = XORSHIFT_SEED;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let r = xorshift64(&mut state);
        out.push((r as usize) % n);
    }
    out
}

fn corpus(map_n: usize, mm_keys: usize) -> Corpus {
    let sorted_keys: Vec<i64> = (0..map_n as i64).collect();
    let sorted_vals = sorted_keys.clone();
    Corpus {
        map_n,
        mm_keys,
        hash_keys: permutation(map_n),
        sorted_keys,
        sorted_vals,
        lookup_map: draw(map_n, LOOKUPS),
        lookup_mm: draw(mm_keys, LOOKUPS),
    }
}

fn expect_len(id: &str, got: usize, want: usize) {
    if got != want {
        eprintln!("{id}: len {got} != {want}");
        std::process::exit(1);
    }
}

fn begin_build() {
    ENABLED.store(true, Ordering::Relaxed);
}

fn end_build() -> (u64, u64, i64) {
    let snap = snapshot();
    ENABLED.store(false, Ordering::Relaxed);
    snap
}

fn ns_per(elapsed_ns: u128) -> u64 {
    (elapsed_ns / LOOKUPS as u128) as u64
}

fn print_row(id: &str, n: usize, snap: (u64, u64, i64), lookup_ns: u64) {
    println!(
        "{id}\t{n}\t{}\t{}\t{}\t{lookup_ns}",
        snap.0, snap.1, snap.2
    );
}

fn run_ohm(c: &Corpus) {
    let warm_n = WARMUP.min(c.map_n);
    begin_build();
    {
        let mut warm = OpenHashMap::new();
        for &k in &c.hash_keys[..warm_n] {
            warm.insert(k, k);
        }
    }
    reset_counters();
    let mut map = OpenHashMap::new();
    for &k in &c.hash_keys {
        map.insert(k, k);
    }
    expect_len("ohm-i64", map.len(), c.map_n);
    let snap = end_build();
    let start = Instant::now();
    let mut sum: u64 = 0;
    for i in 0..LOOKUPS {
        let k = c.hash_keys[i % c.map_n];
        sum = sum.wrapping_add(map.get(&k).copied().unwrap_or(0) as u64);
    }
    black_box(sum);
    let lookup_ns = ns_per(start.elapsed().as_nanos());
    eprintln!("checksum ohm-i64 {sum}");
    print_row("ohm-i64", c.map_n, snap, lookup_ns);
}

fn run_tm(c: &Corpus) {
    let warm_n = WARMUP.min(c.map_n);
    begin_build();
    {
        let mut warm = TreeMap::new();
        for k in 0..warm_n as i64 {
            warm.insert(k, k);
        }
    }
    reset_counters();
    let mut map = TreeMap::new();
    for k in 0..c.map_n as i64 {
        map.insert(k, k);
    }
    expect_len("tm-i64", map.len(), c.map_n);
    let snap = end_build();
    let start = Instant::now();
    let mut sum: u64 = 0;
    for &idx in &c.lookup_map {
        let k = idx as i64;
        sum = sum.wrapping_add(map.get(&k).copied().unwrap_or(0) as u64);
    }
    black_box(sum);
    let lookup_ns = ns_per(start.elapsed().as_nanos());
    eprintln!("checksum tm-i64 {sum}");
    print_row("tm-i64", c.map_n, snap, lookup_ns);
}

fn run_ism(c: &Corpus) {
    let warm_n = WARMUP.min(c.map_n);
    begin_build();
    {
        let _warm = ImmutableSortedMap::from_sorted(&c.sorted_keys[..warm_n], &c.sorted_vals[..warm_n]);
    }
    reset_counters();
    let map = ImmutableSortedMap::from_sorted(&c.sorted_keys, &c.sorted_vals);
    expect_len("ism", map.len(), c.map_n);
    let snap = end_build();
    let start = Instant::now();
    let mut sum: u64 = 0;
    for &idx in &c.lookup_map {
        let k = idx as i64;
        sum = sum.wrapping_add(map.get(&k).copied().unwrap_or(0) as u64);
    }
    black_box(sum);
    let lookup_ns = ns_per(start.elapsed().as_nanos());
    eprintln!("checksum ism {sum}");
    print_row("ism", c.map_n, snap, lookup_ns);
}

fn run_mm(c: &Corpus) {
    let warm_keys = WARMUP.min(c.mm_keys);
    begin_build();
    {
        let mut warm = Multimap::new();
        for k in 0..warm_keys as i64 {
            for v in 0..VALUES_PER_KEY as i32 {
                warm.insert(k, v);
            }
        }
    }
    reset_counters();
    let mut map = Multimap::new();
    for k in 0..c.mm_keys as i64 {
        for v in 0..VALUES_PER_KEY as i32 {
            map.insert(k, v);
        }
    }
    expect_len("mm", map.len(), c.mm_keys * VALUES_PER_KEY);
    let snap = end_build();
    let start = Instant::now();
    let mut sum: u64 = 0;
    for &idx in &c.lookup_mm {
        let k = idx as i64;
        sum = sum.wrapping_add(map.get(&k).len() as u64);
    }
    black_box(sum);
    let lookup_ns = ns_per(start.elapsed().as_nanos());
    eprintln!("checksum mm {sum}");
    print_row("mm", c.mm_keys, snap, lookup_ns);
}

fn run_roar(c: &Corpus) {
    let warm_n = WARMUP.min(c.map_n);
    begin_build();
    {
        let mut warm = RoaringU32::new();
        for i in 0..warm_n as u32 {
            warm.add(i * ROARING_STRIDE);
        }
    }
    reset_counters();
    let mut set = RoaringU32::new();
    for i in 0..c.map_n as u32 {
        set.add(i * ROARING_STRIDE);
    }
    expect_len("roar", set.cardinality() as usize, c.map_n);
    let snap = end_build();
    let start = Instant::now();
    let mut sum: u64 = 0;
    for &idx in &c.lookup_map {
        let v = (idx as u32) * ROARING_STRIDE;
        if set.contains(v) {
            sum = sum.wrapping_add(1);
        }
    }
    black_box(sum);
    let lookup_ns = ns_per(start.elapsed().as_nanos());
    eprintln!("checksum roar {sum}");
    print_row("roar", c.map_n, snap, lookup_ns);
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mode = match args.next().as_deref() {
        None => "default",
        Some("--smoke") => "smoke",
        Some("--full") => "full",
        Some(other) => {
            eprintln!("unknown argument: {other}");
            eprintln!("usage: bytes_per_collection [--smoke|--full]");
            return ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: bytes_per_collection [--smoke|--full]");
        return ExitCode::from(2);
    }
    let (map_n, mm_keys) = match mode {
        "smoke" => (1_000, 1_000),
        "full" => (1_000_000, 100_000),
        _ => (100_000, 10_000),
    };
    eprintln!(
        "mode {mode} map_n {map_n} mm_keys {mm_keys} values_per_key {VALUES_PER_KEY} lookups {LOOKUPS}"
    );
    eprintln!(
        "bytes are requested Layout sizes. malloc size-class rounding is not measured."
    );
    // Corpus vectors are allocated with counting disabled so they are not
    // part of a collection's build total.
    let c = corpus(map_n, mm_keys);
    println!("id\tn\talloc_bytes\talloc_count\tlive_bytes_if_known\tlookup_ns");
    run_ohm(&c);
    run_tm(&c);
    run_ism(&c);
    run_mm(&c);
    run_roar(&c);
    ExitCode::SUCCESS
}
