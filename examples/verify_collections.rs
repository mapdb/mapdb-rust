// Structural verification suite for the Swiss-table OpenHashMap / OpenHashSet.
//
// Dependency-free (deterministic xorshift PRNG, NOT the `rand` crate). Proves
// the *internal structure* of the constructed tables stays valid under large
// randomized workloads, beyond black-box behavior:
//
//   1. Differential fuzz of OpenHashMap<i64,i64> vs std HashMap and
//      OpenHashSet<i64> vs std HashSet, lockstep, multiple seeds, tens of
//      millions of ops, with periodic assert_invariants().
//   2. String-key differential run (Borrow / &str lookup path).
//   3. Drop / leak correctness via a live-instance-counting payload type.
//   4. Invariant stress across randomized phases.
//
// Run:
//   RUSTFLAGS="-C target-cpu=native" \
//     cargo run --offline --release --example verify_collections

use std::collections::{HashMap as StdMap, HashSet as StdSet};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

use mapdb_collections::{OpenHashMap, OpenHashSet};

// ---------------------------------------------------------------------------
// Deterministic PRNG: xorshift64*. Reproducible, no external crates.
// ---------------------------------------------------------------------------
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid the zero fixed point.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }
    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    #[inline]
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

// ===========================================================================
// 1. Differential fuzz — OpenHashMap<i64,i64> vs std::HashMap
// ===========================================================================
//
// Keys are drawn from a bounded range so removes/overwrites actually hit. We
// alternate the active key range to create overlap (some ops target keys that
// are likely present, some likely absent) and run a heavy churn phase at a
// fixed live-size band to stress tombstones + same-capacity rehash.

fn fuzz_map(seed: u64, ops: u64, key_range: u64) -> u64 {
    let mut rng = Rng::new(seed);
    let mut ours: OpenHashMap<i64, i64> = OpenHashMap::new();
    let mut std: StdMap<i64, i64> = StdMap::new();

    let invariant_every = 50_000u64;
    let mut performed = 0u64;

    for i in 0..ops {
        // Slide the active key window over time so churn hits the same keys.
        let window = key_range.max(1);
        let base = (i / 4096) % window;
        let k = ((base + rng.below(window)) % window) as i64;
        let op = rng.below(100);

        match op {
            // insert / overwrite (45%)
            0..=44 => {
                let v = rng.next_u64() as i64;
                let a = ours.insert(k, v);
                let b = std.insert(k, v);
                assert_eq!(a, b, "map insert mismatch seed={seed} i={i} k={k}");
            }
            // get (25%)
            45..=69 => {
                let a = ours.get(&k).copied();
                let b = std.get(&k).copied();
                assert_eq!(a, b, "map get mismatch seed={seed} i={i} k={k}");
            }
            // contains_key (10%)
            70..=79 => {
                assert_eq!(
                    ours.contains_key(&k),
                    std.contains_key(&k),
                    "map contains mismatch seed={seed} i={i} k={k}"
                );
            }
            // remove (17%)
            80..=96 => {
                let a = ours.remove(&k);
                let b = std.remove(&k);
                assert_eq!(a, b, "map remove mismatch seed={seed} i={i} k={k}");
            }
            // reserve (2%)
            97..=98 => {
                let extra = rng.below(4096) as usize;
                ours.try_reserve(extra).expect("try_reserve");
                std.reserve(extra);
                // reserve must not change contents.
                assert_eq!(ours.len(), std.len(), "len after reserve seed={seed} i={i}");
            }
            // clear (1%)
            _ => {
                ours.clear();
                std.clear();
                assert_eq!(ours.len(), 0, "len after clear seed={seed} i={i}");
            }
        }
        performed += 1;

        assert_eq!(
            ours.len(),
            std.len(),
            "map len mismatch seed={seed} i={i} (op={op})"
        );

        if i % invariant_every == invariant_every - 1 {
            ours.assert_invariants();
            // Full content cross-check on the smaller side.
            cross_check_map(&ours, &std, seed, i);
        }
    }

    ours.assert_invariants();
    cross_check_map(&ours, &std, seed, ops);
    performed
}

fn cross_check_map(ours: &OpenHashMap<i64, i64>, std: &StdMap<i64, i64>, seed: u64, i: u64) {
    assert_eq!(ours.len(), std.len(), "cross len seed={seed} i={i}");
    // Every std entry present in ours with same value.
    for (k, v) in std.iter() {
        assert_eq!(
            ours.get(k),
            Some(v),
            "cross: ours missing/wrong for k={k} seed={seed} i={i}"
        );
    }
    // Every ours entry present in std (catches phantom/duplicate live slots).
    for (k, v) in ours.iter() {
        assert_eq!(
            std.get(k),
            Some(v),
            "cross: std missing/wrong for k={k} seed={seed} i={i}"
        );
    }
}

// Heavy churn at fixed size: keep ~N keys live, repeatedly remove a random one
// and insert a fresh one. Maximizes tombstone production / same-cap rehash.
fn churn_map(seed: u64, rounds: u64, live_target: u64) -> u64 {
    let mut rng = Rng::new(seed ^ 0xABCD);
    let mut ours: OpenHashMap<i64, i64> = OpenHashMap::new();
    let mut std: StdMap<i64, i64> = StdMap::new();
    let mut next_key: i64 = 0;

    // Fill to the target.
    while (ours.len() as u64) < live_target {
        let k = next_key;
        next_key += 1;
        let v = rng.next_u64() as i64;
        ours.insert(k, v);
        std.insert(k, v);
    }
    ours.assert_invariants();

    let mut performed = 0u64;
    for r in 0..rounds {
        // Remove a key in the live window, insert a new one (size oscillates).
        if !std.is_empty() && rng.below(2) == 0 {
            let lo = (next_key as u64).saturating_sub(live_target * 2);
            let span = (next_key as u64).saturating_sub(lo).max(1);
            let k = (lo + rng.below(span)) as i64;
            let a = ours.remove(&k);
            let b = std.remove(&k);
            assert_eq!(a, b, "churn remove mismatch seed={seed} r={r} k={k}");
        } else {
            let k = next_key;
            next_key += 1;
            let v = rng.next_u64() as i64;
            let a = ours.insert(k, v);
            let b = std.insert(k, v);
            assert_eq!(a, b, "churn insert mismatch seed={seed} r={r} k={k}");
        }
        performed += 1;
        assert_eq!(ours.len(), std.len(), "churn len mismatch seed={seed} r={r}");

        if r % 100_000 == 99_999 {
            ours.assert_invariants();
        }
    }
    ours.assert_invariants();
    cross_check_map(&ours, &std, seed, rounds);
    performed
}

// ===========================================================================
// 1b. Differential fuzz — OpenHashSet<i64> vs std::HashSet
// ===========================================================================

fn fuzz_set(seed: u64, ops: u64, key_range: u64) -> u64 {
    let mut rng = Rng::new(seed ^ 0x5151_5151);
    let mut ours: OpenHashSet<i64> = OpenHashSet::new();
    let mut std: StdSet<i64> = StdSet::new();

    let invariant_every = 50_000u64;
    let mut performed = 0u64;

    for i in 0..ops {
        let window = key_range.max(1);
        let base = (i / 4096) % window;
        let k = ((base + rng.below(window)) % window) as i64;
        let op = rng.below(100);

        match op {
            // insert (45%)
            0..=44 => {
                let a = ours.insert(k);
                let b = std.insert(k);
                assert_eq!(a, b, "set insert mismatch seed={seed} i={i} k={k}");
            }
            // contains (35%)
            45..=79 => {
                assert_eq!(
                    ours.contains(&k),
                    std.contains(&k),
                    "set contains mismatch seed={seed} i={i} k={k}"
                );
            }
            // remove (18%)
            80..=97 => {
                let a = ours.remove(&k);
                let b = std.remove(&k);
                assert_eq!(a, b, "set remove mismatch seed={seed} i={i} k={k}");
            }
            // reserve (1%)
            98 => {
                let extra = rng.below(4096) as usize;
                ours.try_reserve(extra).expect("set try_reserve");
                std.reserve(extra);
            }
            // clear (1%)
            _ => {
                ours.clear();
                std.clear();
            }
        }
        performed += 1;
        assert_eq!(ours.len(), std.len(), "set len mismatch seed={seed} i={i}");

        if i % invariant_every == invariant_every - 1 {
            ours.assert_invariants();
            for k in std.iter() {
                assert!(ours.contains(k), "set cross missing k={k} seed={seed} i={i}");
            }
            for k in ours.iter() {
                assert!(std.contains(k), "set phantom k={k} seed={seed} i={i}");
            }
        }
    }

    ours.assert_invariants();
    assert_eq!(ours.len(), std.len());
    for k in std.iter() {
        assert!(ours.contains(k), "set final missing k={k} seed={seed}");
    }
    for k in ours.iter() {
        assert!(std.contains(k), "set final phantom k={k} seed={seed}");
    }
    performed
}

// ===========================================================================
// 2. String keys — exercises non-primitive K and the Borrow/&str path
// ===========================================================================

fn fuzz_string_map(seed: u64, ops: u64, key_range: u64) -> u64 {
    let mut rng = Rng::new(seed ^ 0x57_5249_4E47);
    let mut ours: OpenHashMap<String, i64> = OpenHashMap::new();
    let mut std: StdMap<String, i64> = StdMap::new();
    let mut performed = 0u64;

    let make_key = |n: u64| -> String {
        // Mix of lengths so hashing isn't uniform; reproducible.
        format!("key-{:x}-{}", n, "padpadpad".repeat((n % 4) as usize))
    };

    for i in 0..ops {
        let n = rng.below(key_range.max(1));
        let key = make_key(n);
        let op = rng.below(100);
        match op {
            0..=44 => {
                let v = rng.next_u64() as i64;
                let a = ours.insert(key.clone(), v);
                let b = std.insert(key.clone(), v);
                assert_eq!(a, b, "str insert mismatch seed={seed} i={i}");
            }
            45..=74 => {
                // Borrowed &str lookup path.
                let a = ours.get(key.as_str()).copied();
                let b = std.get(key.as_str()).copied();
                assert_eq!(a, b, "str get(&str) mismatch seed={seed} i={i}");
            }
            75..=84 => {
                assert_eq!(
                    ours.contains_key(key.as_str()),
                    std.contains_key(key.as_str()),
                    "str contains(&str) mismatch seed={seed} i={i}"
                );
            }
            85..=98 => {
                let a = ours.remove(key.as_str());
                let b = std.remove(key.as_str());
                assert_eq!(a, b, "str remove(&str) mismatch seed={seed} i={i}");
            }
            _ => {
                ours.clear();
                std.clear();
            }
        }
        performed += 1;
        assert_eq!(ours.len(), std.len(), "str len mismatch seed={seed} i={i}");
        if i % 20_000 == 19_999 {
            ours.assert_invariants();
        }
    }
    ours.assert_invariants();
    assert_eq!(ours.len(), std.len());
    for (k, v) in std.iter() {
        assert_eq!(ours.get(k.as_str()), Some(v), "str final cross seed={seed}");
    }
    for (k, v) in ours.iter() {
        assert_eq!(std.get(k), Some(v), "str final phantom seed={seed}");
    }
    performed
}

// ===========================================================================
// 3. Drop / leak correctness
// ===========================================================================
//
// `Tracked` bumps a global atomic on construction (incl. clone) and decrements
// on Drop. A leak leaves the count > 0; a double-drop underflows past 0 (and
// the post-condition assert catches it). The Eq/Hash use only the inner id so
// the type is a valid key.

static LIVE: AtomicI64 = AtomicI64::new(0);

#[derive(Debug)]
struct Tracked {
    id: i64,
}

impl Tracked {
    fn new(id: i64) -> Self {
        LIVE.fetch_add(1, Ordering::SeqCst);
        Tracked { id }
    }
}
impl Clone for Tracked {
    fn clone(&self) -> Self {
        LIVE.fetch_add(1, Ordering::SeqCst);
        Tracked { id: self.id }
    }
}
impl Drop for Tracked {
    fn drop(&mut self) {
        let prev = LIVE.fetch_sub(1, Ordering::SeqCst);
        assert!(prev > 0, "double-drop / drop of untracked Tracked id={}", self.id);
    }
}
impl PartialEq for Tracked {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Tracked {}
impl std::hash::Hash for Tracked {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.id.hash(h);
    }
}

fn assert_live(expected: i64, ctx: &str) {
    let n = LIVE.load(Ordering::SeqCst);
    assert_eq!(n, expected, "live-instance leak/double-drop at: {ctx}");
}

fn drop_leak_tests() {
    assert_live(0, "start");

    // --- A. build + drop the whole map (keys AND values tracked) ---
    {
        let mut m: OpenHashMap<Tracked, Tracked> = OpenHashMap::new();
        for i in 0..5000 {
            m.insert(Tracked::new(i), Tracked::new(i + 1_000_000));
        }
        // 5000 keys + 5000 values live.
        assert_live(10_000, "after build map");
        m.assert_invariants();
    }
    assert_live(0, "after drop map");

    // --- B. overwrite-on-reinsert drops old value exactly once ---
    {
        let mut m: OpenHashMap<Tracked, Tracked> = OpenHashMap::new();
        for i in 0..2000 {
            m.insert(Tracked::new(i), Tracked::new(i));
        }
        assert_live(4000, "after build B");
        // Re-insert same keys with new values: old value dropped, new key dropped
        // (existing key kept), net live unchanged.
        for i in 0..2000 {
            let old = m.insert(Tracked::new(i), Tracked::new(i + 7));
            assert!(old.is_some(), "expected overwrite for {i}");
            // `old` (the previous value) drops here at end of loop body.
        }
        assert_live(4000, "after overwrite B");
        m.assert_invariants();
    }
    assert_live(0, "after drop B");

    // --- C. remove drops key+value exactly once; returned value drops on scope ---
    {
        let mut m: OpenHashMap<Tracked, Tracked> = OpenHashMap::new();
        for i in 0..3000 {
            m.insert(Tracked::new(i), Tracked::new(i));
        }
        assert_live(6000, "after build C");
        for i in 0..1500 {
            let v = m.remove(&Tracked::new(i));
            assert!(v.is_some());
            // The probe key Tracked::new(i) drops at end of remove() call;
            // returned value `v` drops at end of loop body.
        }
        assert_live(3000, "after remove half C"); // 1500 keys + 1500 values left
        m.assert_invariants();
    }
    assert_live(0, "after drop C");

    // --- D. clear drops all live entries ---
    {
        let mut m: OpenHashMap<Tracked, Tracked> = OpenHashMap::new();
        for i in 0..4000 {
            m.insert(Tracked::new(i), Tracked::new(i));
        }
        assert_live(8000, "after build D");
        m.clear();
        assert_live(0, "after clear D");
        m.assert_invariants();
        // Reuse after clear works and still drops cleanly.
        for i in 0..100 {
            m.insert(Tracked::new(i), Tracked::new(i));
        }
        assert_live(200, "after refill D");
    }
    assert_live(0, "after drop D");

    // --- E. clone then drop both copies ---
    {
        let mut m: OpenHashMap<Tracked, Tracked> = OpenHashMap::new();
        for i in 0..2000 {
            m.insert(Tracked::new(i), Tracked::new(i));
        }
        assert_live(4000, "after build E");
        let c = m.clone();
        assert_live(8000, "after clone E");
        c.assert_invariants();
        m.assert_invariants();
        drop(m);
        assert_live(4000, "after drop original E");
        drop(c);
        assert_live(0, "after drop clone E");
    }
    assert_live(0, "after E");

    // --- F. into_iter full consumption ---
    {
        let mut m: OpenHashMap<Tracked, Tracked> = OpenHashMap::new();
        for i in 0..3000 {
            m.insert(Tracked::new(i), Tracked::new(i));
        }
        assert_live(6000, "after build F");
        let mut count = 0;
        for (k, v) in m.into_iter() {
            assert_eq!(k.id, v.id);
            count += 1;
            // k, v drop at end of body.
        }
        assert_eq!(count, 3000, "into_iter yielded wrong count");
        assert_live(0, "after full into_iter F");
    }
    assert_live(0, "after F");

    // --- G. into_iter PARTIAL consumption then drop (destructor must drop rest) ---
    {
        let mut m: OpenHashMap<Tracked, Tracked> = OpenHashMap::new();
        for i in 0..4000 {
            m.insert(Tracked::new(i), Tracked::new(i));
        }
        assert_live(8000, "after build G");
        {
            let mut it = m.into_iter();
            // Consume only ~1000 entries, then drop the iterator.
            for _ in 0..1000 {
                let (k, v) = it.next().expect("partial into_iter ran dry");
                let _ = (k.id, v.id);
                // yielded pair drops here.
            }
            // 3000 entries remain (6000 instances) inside the iterator.
            assert_live(6000, "mid partial into_iter G");
            // `it` drops here -> destructor must drop the remaining 3000 entries.
        }
        assert_live(0, "after partial into_iter drop G");
    }
    assert_live(0, "after G");

    // --- H. set variant: build / remove / clear / clone / into_iter ---
    {
        let mut s: OpenHashSet<Tracked> = OpenHashSet::new();
        for i in 0..3000 {
            s.insert(Tracked::new(i));
        }
        assert_live(3000, "after build set H");
        // Re-insert existing key: insert() returns false and drops the arg.
        for i in 0..3000 {
            let added = s.insert(Tracked::new(i));
            assert!(!added, "dup insert should be false {i}");
        }
        assert_live(3000, "after dup inserts H");
        s.assert_invariants();
        for i in 0..1000 {
            assert!(s.remove(&Tracked::new(i)));
        }
        assert_live(2000, "after removes H");
        let c = s.clone();
        assert_live(4000, "after clone set H");
        drop(c);
        assert_live(2000, "after drop set clone H");
        // partial into_iter
        {
            let mut it = s.into_iter();
            for _ in 0..500 {
                let _k = it.next().expect("set partial dry");
            }
            assert_live(1500, "mid set partial H");
        }
        assert_live(0, "after set into_iter drop H");
    }
    assert_live(0, "after H");

    // --- I. randomized drop stress: random ops then drop, count must zero ---
    {
        let mut rng = Rng::new(0xD0D0_D0D0);
        for phase in 0..40 {
            let mut m: OpenHashMap<Tracked, Tracked> = OpenHashMap::new();
            let mut shadow: StdSet<i64> = StdSet::new();
            for _ in 0..20_000 {
                let k = rng.below(3000) as i64;
                if rng.below(3) == 0 {
                    if m.remove(&Tracked::new(k)).is_some() {
                        shadow.remove(&k);
                    }
                } else {
                    let was = m.insert(Tracked::new(k), Tracked::new(k));
                    if was.is_none() {
                        shadow.insert(k);
                    }
                }
            }
            assert_eq!(m.len(), shadow.len(), "drop-stress len phase {phase}");
            // live == 2 * len (key+value), before dropping the map.
            assert_live(
                2 * shadow.len() as i64,
                "drop-stress live before drop",
            );
            m.assert_invariants();
            // half the time drain via into_iter, half drop directly.
            if phase % 2 == 0 {
                for (k, v) in m {
                    let _ = (k.id, v.id);
                }
            } else {
                drop(m);
            }
            assert_live(0, "drop-stress end phase");
        }
    }
    assert_live(0, "after I");
}

// ===========================================================================
// 4. Invariant stress across randomized phases (map + set, big tables)
// ===========================================================================

fn invariant_stress(seed: u64) -> u64 {
    let mut rng = Rng::new(seed ^ 0x1234_5678);
    let mut m: OpenHashMap<i64, i64> = OpenHashMap::new();
    let mut s: OpenHashSet<i64> = OpenHashSet::new();
    let mut ops = 0u64;

    // Phase 1: monotonic growth into the millions.
    for i in 0..3_000_000i64 {
        m.insert(i, i.wrapping_mul(31));
        s.insert(i);
        ops += 2;
    }
    m.assert_invariants();
    s.assert_invariants();

    // Phase 2: delete-heavy (produce tombstones), every 2nd key.
    for i in (0..3_000_000i64).step_by(2) {
        m.remove(&i);
        s.remove(&i);
        ops += 2;
    }
    m.assert_invariants();
    s.assert_invariants();

    // Phase 3: reserve (may rebuild and clear tombstones).
    m.try_reserve(2_000_000).expect("stress reserve map");
    s.try_reserve(2_000_000).expect("stress reserve set");
    m.assert_invariants();
    s.assert_invariants();

    // Phase 4: random churn over a hot range.
    for _ in 0..2_000_000 {
        let k = rng.below(4_000_000) as i64;
        if rng.below(2) == 0 {
            m.insert(k, k);
            s.insert(k);
        } else {
            m.remove(&k);
            s.remove(&k);
        }
        ops += 2;
    }
    m.assert_invariants();
    s.assert_invariants();

    // Phase 5: clear and rebuild.
    m.clear();
    s.clear();
    m.assert_invariants();
    s.assert_invariants();
    for i in 0..500_000i64 {
        m.insert(i, i);
        s.insert(i);
        ops += 2;
    }
    m.assert_invariants();
    s.assert_invariants();

    // Cross-check map vs set agree on membership at the end.
    assert_eq!(m.len(), s.len());
    for (k, _) in m.iter() {
        assert!(s.contains(k), "phase5 map/set divergence k={k}");
    }
    ops
}

// ===========================================================================
// main
// ===========================================================================

fn main() {
    let t0 = Instant::now();
    let mut total_ops: u64 = 0u64;

    println!("== STRUCTURAL VERIFICATION SUITE ==");
    println!("(deterministic xorshift PRNG; lockstep vs std; periodic assert_invariants)");

    // ---- 1. Differential fuzz: map + set, multiple seeds, large ----
    println!("\n[1] Differential fuzz vs std (map + set, multiple seeds)");
    let seeds: [u64; 5] = [1, 7, 42, 1337, 0xDEAD_BEEF];
    for (n, &seed) in seeds.iter().enumerate() {
        // Mix of key ranges: small (heavy collision/overwrite) and large (growth).
        let key_range = if n % 2 == 0 { 200_000 } else { 4_000_000 };
        let ops = 2_500_000u64;
        let m = fuzz_map(seed, ops, key_range);
        let s = fuzz_set(seed, ops, key_range);
        total_ops += m + s;
        println!(
            "    seed {seed:#x}: map {m} ops + set {s} ops (key_range={key_range})  invariants OK",
        );
    }

    // ---- Churn: fixed-size heavy remove/insert to stress tombstones ----
    println!("\n[1b] Tombstone churn (fixed live-size, heavy remove/insert)");
    for &(seed, live, rounds) in &[
        (11u64, 1_000_000u64, 4_000_000u64),
        (99u64, 250_000u64, 4_000_000u64),
    ] {
        let c = churn_map(seed, rounds, live);
        total_ops += c;
        println!("    seed {seed}: {c} churn ops at ~{live} live entries  invariants OK");
    }

    // ---- 2. String keys ----
    println!("\n[2] String-key differential (Borrow / &str lookup path)");
    for &seed in &[3u64, 314159u64] {
        let n = fuzz_string_map(seed, 1_500_000, 300_000);
        total_ops += n;
        println!("    seed {seed}: {n} string-map ops  invariants OK");
    }

    // ---- 3. Drop / leak correctness ----
    println!("\n[3] Drop / leak correctness (live-instance counting payload)");
    drop_leak_tests();
    println!(
        "    all drop/clear/overwrite/remove/clone/into_iter(partial) cases: LIVE returned to 0"
    );

    // ---- 4. Invariant stress over big randomized phases ----
    println!("\n[4] Invariant stress (multi-phase, millions of live entries)");
    for &seed in &[2024u64, 8675309u64] {
        let n = invariant_stress(seed);
        total_ops += n;
        println!("    seed {seed}: {n} ops across 5 phases  invariants OK");
    }

    let secs = t0.elapsed().as_secs_f64();
    println!(
        "\nTotal operations exercised (differential + stress): {total_ops}"
    );
    println!("Runtime: {secs:.1}s");
    if let Some(kb) = peak_rss_kb() {
        println!("Peak RSS: {:.2} GB ({} kB)", kb as f64 / 1_048_576.0, kb);
    }
    println!("\nALL VERIFICATION PASSED  ({total_ops} ops, 0 leaks, 0 invariant violations)");
}

// Read peak resident set size from /proc/self/status (Linux). Best-effort.
fn peak_rss_kb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb);
        }
    }
    None
}
