// Copyright (c) 2026 Jan Kotek.
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Rebuild benchmark: one table's iteration fed into a fresh table with the
//! same hasher, by `collect`, `extend` and a plain `insert` loop. With a fixed
//! hasher the source is in slot order, which used to cluster the growing
//! target quadratically. Also times ordinary random insert and lookup.
//!
//! `cargo run --release --example rehash_bench`

use mapdb_collections::hash_table::{OpenHashMap, OpenHashSet};
use std::collections::hash_map::{DefaultHasher, RandomState};
use std::collections::HashMap;
use std::hash::{BuildHasher, BuildHasherDefault, Hasher};
use std::time::Instant;

/// FxHash-style multiply hasher (rustc's), inlined to stay dependency-free.
#[derive(Default)]
struct Fx(u64);

impl Hasher for Fx {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

type Sip = BuildHasherDefault<DefaultHasher>;
type FxB = BuildHasherDefault<Fx>;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

/// Pseudo-random keys (splitmix64), so the source table is not trivially ordered.
fn keys(n: usize) -> Vec<u64> {
    let mut s = 0x9e37_79b9_7f4a_7c15u64;
    (0..n)
        .map(|_| {
            s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        })
        .collect()
}

fn run<S: BuildHasher + Default + Clone>(name: &str, n: usize) {
    let ks = keys(n);
    let mut a: OpenHashMap<u64, u64, S> = OpenHashMap::default();
    let t = Instant::now();
    for &k in &ks {
        a.insert(k, k);
    }
    let insert = ms(t);
    let t = Instant::now();
    let mut hits = 0u64;
    for &k in &ks {
        hits += *a.get(&k).unwrap() & 1;
    }
    let get = ms(t);

    let t = Instant::now();
    let b: OpenHashMap<u64, u64, S> = a.iter().map(|(k, v)| (*k, *v)).collect();
    let collect = ms(t);
    let t = Instant::now();
    let mut c: OpenHashMap<u64, u64, S> = OpenHashMap::default();
    c.extend(a.iter().map(|(k, v)| (*k, *v)));
    let extend = ms(t);
    let t = Instant::now();
    let mut d: OpenHashMap<u64, u64, S> = OpenHashMap::default();
    for (k, v) in a.iter() {
        d.insert(*k, *v);
    }
    let loop_ = ms(t);
    let s: OpenHashSet<u64, S> = ks.iter().copied().collect();
    let t = Instant::now();
    let s2: OpenHashSet<u64, S> = s.iter().copied().collect();
    let set_collect = ms(t);

    let t = Instant::now();
    let std_a: HashMap<u64, u64, S> = a.iter().map(|(k, v)| (*k, *v)).collect();
    let std_collect = ms(t);
    assert_eq!(b.len() + c.len() + d.len() + s2.len() + std_a.len(), 5 * n);
    println!(
        "{name:5} n={n:8} insert {insert:7.1} get {get:6.1} | collect {collect:8.1} extend {extend:8.1} \
         loop {loop_:8.1} set-collect {set_collect:8.1} | std-collect {std_collect:6.1} ms  ({hits})"
    );
}

fn main() {
    for &n in &[175_000usize, 350_000, 700_000, 1_400_000] {
        run::<Sip>("sip", n);
        run::<FxB>("fx", n);
        run::<RandomState>("rand", n);
    }
}
