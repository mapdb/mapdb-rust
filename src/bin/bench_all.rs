// Comprehensive benchmark for ALL Rust collection classes and ALL their methods.
// Run: cd mapdb-rust && cargo run --release --bin bench_all

use std::time::Instant;

use mapdb_collections::arraylist::i32_array_list::I32ArrayList;
use mapdb_collections::bag::i32_hash_bag::I32HashBag;
use mapdb_collections::bag::i32_tree_bag::I32TreeBag;
use mapdb_collections::hashmap::i32_i32_hash_map::I32I32HashMap;
use mapdb_collections::hashset::i32_hash_set::I32HashSet;
use mapdb_collections::multimap::i32_i32_list_multimap::I32I32ListMultimap;
use mapdb_collections::stack::i32_array_stack::I32ArrayStack;
use mapdb_collections::treemap::i32_i32_tree_map::I32I32TreeMap;
use mapdb_collections::treeset::i32_tree_set::I32TreeSet;

const N: usize = 1_000_000;
const SECS: u64 = 10;

/// Sustained benchmark: runs `f` repeatedly for SECS seconds and reports throughput.
fn bench(name: &str, f: impl Fn()) {
    let mut ops = 0u64;
    let deadline = Instant::now() + std::time::Duration::from_secs(SECS);
    while Instant::now() < deadline {
        f();
        ops += 1;
    }
    println!(
        "  {:<45} {:>8} iters  {:>8.0} ops/s",
        name,
        ops,
        ops as f64 / SECS as f64
    );
}

/// Single-shot benchmark: runs `f` once and reports ns/op for n operations.
fn bench_n(name: &str, n: usize, f: impl FnOnce()) {
    let start = Instant::now();
    f();
    let d = start.elapsed();
    println!(
        "  {:<45} {:?}  ({} ns/op)",
        name,
        d,
        d.as_nanos() / n as u128
    );
}

/// Pseudo-random hash for scrambled insertion order.
#[inline]
fn scramble(i: usize) -> i32 {
    ((i as u64).wrapping_mul(2654435761) % N as u64) as i32
}

// ---------------------------------------------------------------------------
// Helper: build a pre-populated HashMap with N entries
// ---------------------------------------------------------------------------
fn make_hashmap() -> I32I32HashMap {
    let mut m = I32I32HashMap::with_capacity(N * 2);
    for i in 0..N as i32 {
        m.insert(i, i * 10);
    }
    m
}

// ---------------------------------------------------------------------------
// Helper: build a pre-populated HashSet with N entries
// ---------------------------------------------------------------------------
fn make_hashset() -> I32HashSet {
    let mut s = I32HashSet::new();
    for i in 0..N as i32 {
        s.add(i);
    }
    s
}

// ---------------------------------------------------------------------------
// Helper: build a pre-populated ArrayList with N entries
// ---------------------------------------------------------------------------
fn make_arraylist() -> I32ArrayList {
    let mut a = I32ArrayList::with_capacity(N);
    for i in 0..N as i32 {
        a.push(i);
    }
    a
}

// ---------------------------------------------------------------------------
// Helper: build a pre-populated HashBag (i % 10000 => ~100 occurrences each)
// ---------------------------------------------------------------------------
fn make_hashbag() -> I32HashBag {
    let mut b = I32HashBag::new();
    for i in 0..N as i32 {
        b.add(i % 10000);
    }
    b
}

// ---------------------------------------------------------------------------
// Helper: build a pre-populated TreeMap with N entries (sorted order)
// ---------------------------------------------------------------------------
fn make_treemap_sorted() -> I32I32TreeMap {
    let mut m = I32I32TreeMap::new();
    for i in 0..N as i32 {
        m.insert(i, i * 10);
    }
    m
}

// ---------------------------------------------------------------------------
// Helper: build a pre-populated TreeSet with N entries
// ---------------------------------------------------------------------------
fn make_treeset() -> I32TreeSet {
    let mut s = I32TreeSet::new();
    for i in 0..N as i32 {
        s.add(i);
    }
    s
}

// ---------------------------------------------------------------------------
// Helper: build a pre-populated ListMultimap (N/10 keys, ~10 values each)
// ---------------------------------------------------------------------------
fn make_multimap() -> I32I32ListMultimap {
    let mut m = I32I32ListMultimap::new();
    let num_keys = N / 10;
    for i in 0..N as i32 {
        m.put(i % num_keys as i32, i);
    }
    m
}

// ---------------------------------------------------------------------------
// Helper: build a pre-populated TreeBag (i % 10000 => ~100 occurrences each)
// ---------------------------------------------------------------------------
fn make_treebag() -> I32TreeBag {
    let mut b = I32TreeBag::new();
    for i in 0..N as i32 {
        b.add(i % 10000);
    }
    b
}

// ---------------------------------------------------------------------------
// Helper: build a small (1K) version for Display benchmarks
// ---------------------------------------------------------------------------
fn make_hashmap_1k() -> I32I32HashMap {
    let mut m = I32I32HashMap::new();
    for i in 0..1000i32 {
        m.insert(i, i * 10);
    }
    m
}
fn make_hashset_1k() -> I32HashSet {
    let mut s = I32HashSet::new();
    for i in 0..1000i32 {
        s.add(i);
    }
    s
}
fn make_arraylist_1k() -> I32ArrayList {
    let mut a = I32ArrayList::new();
    for i in 0..1000i32 {
        a.push(i);
    }
    a
}
fn make_treemap_1k() -> I32I32TreeMap {
    let mut m = I32I32TreeMap::new();
    for i in 0..1000i32 {
        m.insert(i, i * 10);
    }
    m
}
fn make_treeset_1k() -> I32TreeSet {
    let mut s = I32TreeSet::new();
    for i in 0..1000i32 {
        s.add(i);
    }
    s
}

// ===========================================================================
//  1. I32I32HashMap
// ===========================================================================
fn bench_hashmap() {
    println!("\n--- I32I32HashMap (N={}) ---", N);

    // Constructor: new
    bench("new()", || {
        std::hint::black_box(I32I32HashMap::new());
    });

    // Constructor: with_capacity
    bench("with_capacity(N*2)", || {
        std::hint::black_box(I32I32HashMap::with_capacity(N * 2));
    });

    // insert(N)
    bench_n("insert(N)", N, || {
        let mut m = I32I32HashMap::with_capacity(N * 2);
        for i in 0..N as i32 {
            m.insert(i, i * 10);
        }
        std::hint::black_box(&m);
    });

    // insert(overwrite)
    {
        let mut m = make_hashmap();
        bench_n("insert(overwrite N)", N, || {
            for i in 0..N as i32 {
                m.insert(i, i * 20);
            }
            std::hint::black_box(&m);
        });
    }

    // get(hit)
    {
        let m = make_hashmap();
        bench_n("get(hit N)", N, || {
            for i in 0..N as i32 {
                std::hint::black_box(m.get(i));
            }
        });
    }

    // get(miss)
    {
        let m = make_hashmap();
        bench_n("get(miss N)", N, || {
            for i in N as i32..2 * N as i32 {
                std::hint::black_box(m.get(i));
            }
        });
    }

    // get_or_default
    {
        let m = make_hashmap();
        bench_n("get_or_default(N)", N, || {
            for i in 0..N as i32 {
                std::hint::black_box(m.get_or_default(i, -1));
            }
        });
    }

    // contains_key
    {
        let m = make_hashmap();
        bench_n("contains_key(hit N)", N, || {
            for i in 0..N as i32 {
                std::hint::black_box(m.contains_key(i));
            }
        });
    }

    // contains_value (O(n) scan -- benchmark with smaller subset)
    {
        let m = make_hashmap();
        bench_n("contains_value(1 lookup)", 1, || {
            std::hint::black_box(m.contains_value(500_000 * 10));
        });
    }

    // for_each
    {
        let m = make_hashmap();
        bench_n("for_each(N)", N, || {
            let mut sum = 0i64;
            m.for_each(|_k, v| {
                sum += v as i64;
            });
            std::hint::black_box(sum);
        });
    }

    // select(50%)
    {
        let m = make_hashmap();
        bench_n("select(50%)", N, || {
            let r = m.select(|_k, v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // reject(50%)
    {
        let m = make_hashmap();
        bench_n("reject(50%)", N, || {
            let r = m.reject(|_k, v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // detect
    {
        let m = make_hashmap();
        bench_n("detect", N, || {
            // worst case: element near end of iteration
            std::hint::black_box(m.detect(|_k, v| v == (N as i32 - 1) * 10));
        });
    }

    // any_satisfy
    {
        let m = make_hashmap();
        bench_n("any_satisfy", N, || {
            std::hint::black_box(m.any_satisfy(|_k, v| v == (N as i32 - 1) * 10));
        });
    }

    // all_satisfy
    {
        let m = make_hashmap();
        bench_n("all_satisfy", N, || {
            std::hint::black_box(m.all_satisfy(|_k, v| v >= 0));
        });
    }

    // none_satisfy
    {
        let m = make_hashmap();
        bench_n("none_satisfy", N, || {
            std::hint::black_box(m.none_satisfy(|_k, v| v < 0));
        });
    }

    // count
    {
        let m = make_hashmap();
        bench_n("count(50%)", N, || {
            std::hint::black_box(m.count(|_k, v| v % 2 == 0));
        });
    }

    // inject_into
    {
        let m = make_hashmap();
        bench_n("inject_into(sum values)", N, || {
            let r = m.inject_into(0i64, |acc, _k, v| acc + v as i64);
            std::hint::black_box(r);
        });
    }

    // sum_of_values (manual via inject_into -- no dedicated method)
    {
        let m = make_hashmap();
        bench_n("sum_of_values(via inject_into)", N, || {
            let r = m.inject_into(0i64, |acc, _k, v| acc + v as i64);
            std::hint::black_box(r);
        });
    }

    // keys_to_vec
    {
        let m = make_hashmap();
        bench_n("keys_to_vec", N, || {
            std::hint::black_box(m.keys_to_vec());
        });
    }

    // values_to_vec
    {
        let m = make_hashmap();
        bench_n("values_to_vec", N, || {
            std::hint::black_box(m.values_to_vec());
        });
    }

    // remove(all)
    bench_n("remove(all N)", N, || {
        let mut m = make_hashmap();
        for i in 0..N as i32 {
            m.remove(i);
        }
        std::hint::black_box(&m);
    });

    // clear
    {
        let mut m = make_hashmap();
        bench_n("clear", 1, || {
            m.clear();
            std::hint::black_box(&m);
        });
    }

    // with_key_value (fluent, single op)
    bench("with_key_value(chain 100)", || {
        let mut m = I32I32HashMap::new();
        for i in 0..100i32 {
            m = m.with_key_value(i, i * 10);
        }
        std::hint::black_box(&m);
    });

    // without_key (fluent, single op)
    bench("without_key(chain 100)", || {
        let mut m = I32I32HashMap::new();
        for i in 0..100i32 {
            m.insert(i, i);
        }
        for i in 0..100i32 {
            m = m.without_key(i);
        }
        std::hint::black_box(&m);
    });

    // Display(1K)
    {
        let m = make_hashmap_1k();
        bench("Display(1K)", || {
            std::hint::black_box(format!("{}", m));
        });
    }
}

// ===========================================================================
//  2. I32HashSet
// ===========================================================================
fn bench_hashset() {
    println!("\n--- I32HashSet (N={}) ---", N);

    // Constructor: new
    bench("new()", || {
        std::hint::black_box(I32HashSet::new());
    });

    // Constructor: of
    bench("of(&[0..100])", || {
        let data: Vec<i32> = (0..100).collect();
        std::hint::black_box(I32HashSet::of(&data));
    });

    // add(insert N)
    bench_n("add(insert N)", N, || {
        let mut s = I32HashSet::new();
        for i in 0..N as i32 {
            s.add(i);
        }
        std::hint::black_box(&s);
    });

    // add(duplicate)
    {
        let mut s = make_hashset();
        bench_n("add(duplicate N)", N, || {
            for i in 0..N as i32 {
                std::hint::black_box(s.add(i));
            }
        });
    }

    // contains(hit)
    {
        let s = make_hashset();
        bench_n("contains(hit N)", N, || {
            for i in 0..N as i32 {
                std::hint::black_box(s.contains(i));
            }
        });
    }

    // contains(miss)
    {
        let s = make_hashset();
        bench_n("contains(miss N)", N, || {
            for i in N as i32..2 * N as i32 {
                std::hint::black_box(s.contains(i));
            }
        });
    }

    // remove(all)
    bench_n("remove(all N)", N, || {
        let mut s = make_hashset();
        for i in 0..N as i32 {
            s.remove(i);
        }
        std::hint::black_box(&s);
    });

    // for_each
    {
        let s = make_hashset();
        bench_n("for_each(N)", N, || {
            let mut sum = 0i64;
            s.for_each(|v| {
                sum += v as i64;
            });
            std::hint::black_box(sum);
        });
    }

    // iter
    {
        let s = make_hashset();
        bench_n("iter(N)", N, || {
            let mut sum = 0i64;
            for v in s.iter() {
                sum += v as i64;
            }
            std::hint::black_box(sum);
        });
    }

    // select(50%)
    {
        let s = make_hashset();
        bench_n("select(50%)", N, || {
            let r = s.select(|v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // reject(50%)
    {
        let s = make_hashset();
        bench_n("reject(50%)", N, || {
            let r = s.reject(|v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // detect
    {
        let s = make_hashset();
        bench_n("detect", N, || {
            std::hint::black_box(s.detect(|v| v == N as i32 - 1));
        });
    }

    // any_satisfy
    {
        let s = make_hashset();
        bench_n("any_satisfy", N, || {
            std::hint::black_box(s.any_satisfy(|v| v == N as i32 - 1));
        });
    }

    // all_satisfy
    {
        let s = make_hashset();
        bench_n("all_satisfy", N, || {
            std::hint::black_box(s.all_satisfy(|v| v >= 0));
        });
    }

    // none_satisfy
    {
        let s = make_hashset();
        bench_n("none_satisfy", N, || {
            std::hint::black_box(s.none_satisfy(|v| v < 0));
        });
    }

    // union
    {
        let a = make_hashset();
        let mut b = I32HashSet::new();
        for i in (N / 2) as i32..(N + N / 2) as i32 {
            b.add(i);
        }
        bench_n("union(N, N)", N, || {
            let r = a.union(&b);
            std::hint::black_box(&r);
        });
    }

    // intersect
    {
        let a = make_hashset();
        let mut b = I32HashSet::new();
        for i in (N / 2) as i32..(N + N / 2) as i32 {
            b.add(i);
        }
        bench_n("intersect(N, N)", N, || {
            let r = a.intersect(&b);
            std::hint::black_box(&r);
        });
    }

    // difference
    {
        let a = make_hashset();
        let mut b = I32HashSet::new();
        for i in (N / 2) as i32..(N + N / 2) as i32 {
            b.add(i);
        }
        bench_n("difference(N, N)", N, || {
            let r = a.difference(&b);
            std::hint::black_box(&r);
        });
    }

    // symmetric_difference
    {
        let a = make_hashset();
        let mut b = I32HashSet::new();
        for i in (N / 2) as i32..(N + N / 2) as i32 {
            b.add(i);
        }
        bench_n("symmetric_difference(N, N)", N, || {
            let r = a.symmetric_difference(&b);
            std::hint::black_box(&r);
        });
    }

    // to_vec
    {
        let s = make_hashset();
        bench_n("to_vec", N, || {
            std::hint::black_box(s.to_vec());
        });
    }

    // with (fluent)
    bench("with(chain 100)", || {
        let mut s = I32HashSet::new();
        for i in 0..100i32 {
            s = s.with(i);
        }
        std::hint::black_box(&s);
    });

    // without (fluent)
    bench("without(chain 100)", || {
        let s = I32HashSet::of(&(0..100).collect::<Vec<i32>>());
        let mut s = s;
        for i in 0..100i32 {
            s = s.without(i);
        }
        std::hint::black_box(&s);
    });

    // Display(1K)
    {
        let s = make_hashset_1k();
        bench("Display(1K)", || {
            std::hint::black_box(format!("{}", s));
        });
    }
}

// ===========================================================================
//  3. I32ArrayList
// ===========================================================================
fn bench_arraylist() {
    println!("\n--- I32ArrayList (N={}) ---", N);

    // Constructor: new
    bench("new()", || {
        std::hint::black_box(I32ArrayList::new());
    });

    // Constructor: with_capacity
    bench("with_capacity(N)", || {
        std::hint::black_box(I32ArrayList::with_capacity(N));
    });

    // add(N) = push(N)
    bench_n("push(N)", N, || {
        let mut a = I32ArrayList::with_capacity(N);
        for i in 0..N as i32 {
            a.push(i);
        }
        std::hint::black_box(&a);
    });

    // get
    {
        let a = make_arraylist();
        bench_n("get(N)", N, || {
            for i in 0..N {
                std::hint::black_box(a.get(i));
            }
        });
    }

    // set
    {
        let mut a = make_arraylist();
        bench_n("set(N)", N, || {
            for i in 0..N {
                a.set(i, i as i32 * 2);
            }
            std::hint::black_box(&a);
        });
    }

    // contains(hit)
    {
        let a = make_arraylist();
        // Only test a few to avoid O(n^2)
        bench_n("contains(hit, 1000 lookups)", 1000, || {
            for i in (0..N as i32).step_by(N / 1000) {
                std::hint::black_box(a.contains(i));
            }
        });
    }

    // contains(miss)
    {
        let a = make_arraylist();
        bench_n("contains(miss, 1000 lookups)", 1000, || {
            for i in (N as i32..N as i32 + 1000) {
                std::hint::black_box(a.contains(i));
            }
        });
    }

    // index_of
    {
        let a = make_arraylist();
        bench_n("index_of(1000 lookups)", 1000, || {
            for i in (0..N as i32).step_by(N / 1000) {
                std::hint::black_box(a.index_of(i));
            }
        });
    }

    // remove_at_index (remove from end to avoid O(n) shifts)
    {
        bench_n("remove_at_index(last, 10000)", 10000, || {
            let mut a = make_arraylist();
            for _ in 0..10000 {
                a.remove_at_index(a.len() - 1);
            }
            std::hint::black_box(&a);
        });
    }

    // for_each
    {
        let a = make_arraylist();
        bench_n("for_each(N)", N, || {
            let mut sum = 0i64;
            a.for_each(|v| {
                sum += v as i64;
            });
            std::hint::black_box(sum);
        });
    }

    // for_each_with_index (manual via iter + enumerate since no dedicated method)
    {
        let a = make_arraylist();
        bench_n("iter().enumerate()(N)", N, || {
            let mut sum = 0i64;
            for (i, v) in a.iter().enumerate() {
                sum += (i as i64) + (v as i64);
            }
            std::hint::black_box(sum);
        });
    }

    // select(50%)
    {
        let a = make_arraylist();
        bench_n("select(50%)", N, || {
            let r = a.select(|v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // reject(50%)
    {
        let a = make_arraylist();
        bench_n("reject(50%)", N, || {
            let r = a.reject(|v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // detect
    {
        let a = make_arraylist();
        bench_n("detect(last element)", N, || {
            std::hint::black_box(a.detect(|v| v == N as i32 - 1));
        });
    }

    // any_satisfy
    {
        let a = make_arraylist();
        bench_n("any_satisfy", N, || {
            std::hint::black_box(a.any_satisfy(|v| v == N as i32 - 1));
        });
    }

    // all_satisfy
    {
        let a = make_arraylist();
        bench_n("all_satisfy", N, || {
            std::hint::black_box(a.all_satisfy(|v| v >= 0));
        });
    }

    // none_satisfy
    {
        let a = make_arraylist();
        bench_n("none_satisfy", N, || {
            std::hint::black_box(a.none_satisfy(|v| v < 0));
        });
    }

    // count
    {
        let a = make_arraylist();
        bench_n("count(50%)", N, || {
            std::hint::black_box(a.count(|v| v % 2 == 0));
        });
    }

    // inject_into
    {
        let a = make_arraylist();
        bench_n("inject_into(sum)", N, || {
            let r = a.inject_into(0i64, |acc, v| acc + v as i64);
            std::hint::black_box(r);
        });
    }

    // sum
    {
        let a = make_arraylist();
        bench_n("sum", N, || {
            std::hint::black_box(a.sum());
        });
    }

    // min
    {
        let a = make_arraylist();
        bench_n("min", N, || {
            std::hint::black_box(a.min());
        });
    }

    // max
    {
        let a = make_arraylist();
        bench_n("max", N, || {
            std::hint::black_box(a.max());
        });
    }

    // sort
    bench_n("sort(N)", N, || {
        let mut a = make_arraylist();
        // reverse so sort has work to do
        a = a.reversed();
        a.sort();
        std::hint::black_box(&a);
    });

    // reversed
    {
        let a = make_arraylist();
        bench_n("reversed", N, || {
            std::hint::black_box(a.reversed());
        });
    }

    // distinct (all unique => full scan)
    {
        let a = make_arraylist();
        bench_n("distinct(all unique)", N, || {
            std::hint::black_box(a.distinct());
        });
    }

    // to_vec
    {
        let a = make_arraylist();
        bench_n("to_vec", N, || {
            std::hint::black_box(a.to_vec());
        });
    }

    // Display(1K)
    {
        let a = make_arraylist_1k();
        bench("Display(1K)", || {
            std::hint::black_box(format!("{}", a));
        });
    }
}

// ===========================================================================
//  4. I32HashBag
// ===========================================================================
fn bench_hashbag() {
    println!("\n--- I32HashBag (N={}) ---", N);

    // Constructor
    bench("new()", || {
        std::hint::black_box(I32HashBag::new());
    });

    // add
    bench_n("add(N, i%10000)", N, || {
        let mut b = I32HashBag::new();
        for i in 0..N as i32 {
            b.add(i % 10000);
        }
        std::hint::black_box(&b);
    });

    // add_occurrences (no dedicated method; simulate with repeated add)
    bench_n("add(bulk 100 occurrences x 10000)", N, || {
        let mut b = I32HashBag::new();
        for key in 0..10000i32 {
            for _ in 0..100 {
                b.add(key);
            }
        }
        std::hint::black_box(&b);
    });

    // remove (one occurrence)
    bench_n("remove(one occ, 10000 distinct)", 10000, || {
        let mut b = make_hashbag();
        for i in 0..10000i32 {
            b.remove(i);
        }
        std::hint::black_box(&b);
    });

    // remove_occurrences (no dedicated method; simulate with repeated remove)
    bench_n("remove(50 occ each, 1000 keys)", 50000, || {
        let mut b = make_hashbag();
        for key in 0..1000i32 {
            for _ in 0..50 {
                b.remove(key);
            }
        }
        std::hint::black_box(&b);
    });

    // remove_all
    bench_n("remove_all(10000 distinct)", 10000, || {
        let mut b = make_hashbag();
        for i in 0..10000i32 {
            b.remove_all(i);
        }
        std::hint::black_box(&b);
    });

    // occurrences_of
    {
        let b = make_hashbag();
        bench_n("occurrences_of(10000 lookups)", 10000, || {
            for i in 0..10000i32 {
                std::hint::black_box(b.occurrences_of(i));
            }
        });
    }

    // contains
    {
        let b = make_hashbag();
        bench_n("contains(10000 lookups)", 10000, || {
            for i in 0..10000i32 {
                std::hint::black_box(b.contains(i));
            }
        });
    }

    // size
    {
        let b = make_hashbag();
        bench("size()", || {
            std::hint::black_box(b.size());
        });
    }

    // size_distinct
    {
        let b = make_hashbag();
        bench("size_distinct()", || {
            std::hint::black_box(b.size_distinct());
        });
    }

    // for_each (via for_each_with_occurrences, since no plain for_each)
    {
        let b = make_hashbag();
        bench_n("for_each_with_occurrences(N)", N, || {
            let mut sum = 0i64;
            b.for_each_with_occurrences(|v, c| {
                sum += v as i64 * c as i64;
            });
            std::hint::black_box(sum);
        });
    }

    // for_each_with_occurrences (same as above, explicit name)
    {
        let b = make_hashbag();
        bench_n("for_each_with_occurrences(distinct)", 10000, || {
            let mut count = 0usize;
            b.for_each_with_occurrences(|_v, c| {
                count += c;
            });
            std::hint::black_box(count);
        });
    }

    // select(50%)
    {
        let b = make_hashbag();
        bench_n("select(50%)", N, || {
            let r = b.select(|v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // reject (no dedicated method; use select with negated predicate)
    {
        let b = make_hashbag();
        bench_n("reject(50% via select(!p))", N, || {
            let r = b.select(|v| v % 2 != 0);
            std::hint::black_box(&r);
        });
    }

    // detect (via iter_distinct, since no dedicated method)
    {
        let b = make_hashbag();
        bench_n("detect(via iter_distinct)", 10000, || {
            let r = b.iter_distinct().find(|&v| v == 9999);
            std::hint::black_box(r);
        });
    }

    // any_satisfy (via iter_distinct)
    {
        let b = make_hashbag();
        bench_n("any_satisfy(via iter_distinct)", 10000, || {
            let r = b.iter_distinct().any(|v| v == 9999);
            std::hint::black_box(r);
        });
    }

    // all_satisfy (via iter_distinct)
    {
        let b = make_hashbag();
        bench_n("all_satisfy(via iter_distinct)", 10000, || {
            let r = b.iter_distinct().all(|v| v >= 0);
            std::hint::black_box(r);
        });
    }

    // none_satisfy (via iter_distinct)
    {
        let b = make_hashbag();
        bench_n("none_satisfy(via iter_distinct)", 10000, || {
            let r = !b.iter_distinct().any(|v| v < 0);
            std::hint::black_box(r);
        });
    }

    // top_occurrences (no dedicated method; simulate via collect+sort)
    {
        let b = make_hashbag();
        bench_n("top_occurrences(10, manual)", 10000, || {
            let mut pairs: Vec<(i32, usize)> = Vec::new();
            b.for_each_with_occurrences(|v, c| pairs.push((v, c)));
            pairs.sort_by(|a, b_| b_.1.cmp(&a.1));
            pairs.truncate(10);
            std::hint::black_box(&pairs);
        });
    }

    // clear
    {
        let mut b = make_hashbag();
        bench_n("clear", 1, || {
            b.clear();
            std::hint::black_box(&b);
        });
    }
}

// ===========================================================================
//  5. I32ArrayStack
// ===========================================================================
fn bench_arraystack() {
    println!("\n--- I32ArrayStack (N={}) ---", N);

    // Constructor: new
    bench("new()", || {
        std::hint::black_box(I32ArrayStack::new());
    });

    // Constructor: of
    bench("of(&[0..100])", || {
        let data: Vec<i32> = (0..100).collect();
        std::hint::black_box(I32ArrayStack::of(&data));
    });

    // push+pop(N)
    bench_n("push(N)+pop(N)", N * 2, || {
        let mut s = I32ArrayStack::new();
        for i in 0..N as i32 {
            s.push(i);
        }
        for _ in 0..N {
            std::hint::black_box(s.pop());
        }
    });

    // peek
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench("peek()", || {
            std::hint::black_box(s.peek());
        });
    }

    // len
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench("len()", || {
            std::hint::black_box(s.len());
        });
    }

    // is_empty
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench("is_empty()", || {
            std::hint::black_box(s.is_empty());
        });
    }

    // contains
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench_n("contains(hit, 1000 lookups)", 1000, || {
            for i in (0..N as i32).step_by(N / 1000) {
                std::hint::black_box(s.contains(i));
            }
        });
    }

    // for_each
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench_n("for_each(N)", N, || {
            let mut sum = 0i64;
            s.for_each(|v| {
                sum += v as i64;
            });
            std::hint::black_box(sum);
        });
    }

    // iter
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench_n("iter(N)", N, || {
            let mut sum = 0i64;
            for v in s.iter() {
                sum += v as i64;
            }
            std::hint::black_box(sum);
        });
    }

    // select
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench_n("select(50%)", N, || {
            let r = s.select(|v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // detect
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench_n("detect", N, || {
            std::hint::black_box(s.detect(|v| v == N as i32 - 1));
        });
    }

    // any_satisfy
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench_n("any_satisfy", N, || {
            std::hint::black_box(s.any_satisfy(|v| v == N as i32 - 1));
        });
    }

    // all_satisfy
    {
        let s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
        bench_n("all_satisfy", N, || {
            std::hint::black_box(s.all_satisfy(|v| v >= 0));
        });
    }

    // clear
    {
        bench_n("clear(N)", 1, || {
            let mut s = I32ArrayStack::of(&(0..N as i32).collect::<Vec<i32>>());
            s.clear();
            std::hint::black_box(&s);
        });
    }
}

// ===========================================================================
//  6. I32I32TreeMap
// ===========================================================================
fn bench_treemap() {
    println!("\n--- I32I32TreeMap (N={}) ---", N);

    // Constructor
    bench("new()", || {
        std::hint::black_box(I32I32TreeMap::new());
    });

    // insert(sorted N)
    bench_n("insert(sorted N)", N, || {
        let mut m = I32I32TreeMap::new();
        for i in 0..N as i32 {
            m.insert(i, i * 10);
        }
        std::hint::black_box(&m);
    });

    // insert(random N)
    bench_n("insert(random N)", N, || {
        let mut m = I32I32TreeMap::new();
        for i in 0..N {
            m.insert(scramble(i), scramble(i) * 10);
        }
        std::hint::black_box(&m);
    });

    // get(hit)
    {
        let m = make_treemap_sorted();
        bench_n("get(hit N)", N, || {
            for i in 0..N as i32 {
                std::hint::black_box(m.get(i));
            }
        });
    }

    // get(miss)
    {
        let m = make_treemap_sorted();
        bench_n("get(miss N)", N, || {
            for i in N as i32..2 * N as i32 {
                std::hint::black_box(m.get(i));
            }
        });
    }

    // contains_key
    {
        let m = make_treemap_sorted();
        bench_n("contains_key(hit N)", N, || {
            for i in 0..N as i32 {
                std::hint::black_box(m.contains_key(i));
            }
        });
    }

    // contains_value (no dedicated method; iterate to find)
    {
        let m = make_treemap_sorted();
        bench_n("contains_value(1 lookup, via iter)", 1, || {
            let target = (N as i32 / 2) * 10;
            let r = m.iter().any(|(_, v)| v == target);
            std::hint::black_box(r);
        });
    }

    // for_each
    {
        let m = make_treemap_sorted();
        bench_n("for_each(N)", N, || {
            let mut sum = 0i64;
            m.for_each(|_k, v| {
                sum += v as i64;
            });
            std::hint::black_box(sum);
        });
    }

    // select
    {
        let m = make_treemap_sorted();
        bench_n("select(50%)", N, || {
            let r = m.select(|_k, v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // reject (no dedicated method; use select with negated predicate)
    {
        let m = make_treemap_sorted();
        bench_n("reject(50% via select(!p))", N, || {
            let r = m.select(|_k, v| v % 2 != 0);
            std::hint::black_box(&r);
        });
    }

    // detect (via iter)
    {
        let m = make_treemap_sorted();
        bench_n("detect(via iter.find)", N, || {
            let r = m.iter().find(|&(_, v)| v == (N as i32 - 1) * 10);
            std::hint::black_box(r);
        });
    }

    // any_satisfy
    {
        let m = make_treemap_sorted();
        bench_n("any_satisfy", N, || {
            std::hint::black_box(m.any_satisfy(|_k, v| v == (N as i32 - 1) * 10));
        });
    }

    // all_satisfy
    {
        let m = make_treemap_sorted();
        bench_n("all_satisfy", N, || {
            std::hint::black_box(m.all_satisfy(|_k, v| v >= 0));
        });
    }

    // min
    {
        let m = make_treemap_sorted();
        bench("min()", || {
            std::hint::black_box(m.min());
        });
    }

    // max
    {
        let m = make_treemap_sorted();
        bench("max()", || {
            std::hint::black_box(m.max());
        });
    }

    // floor (no dedicated method; approximate via BTreeMap range -- use iter)
    {
        let m = make_treemap_sorted();
        bench_n("floor(via iter, 1000 lookups)", 1000, || {
            for i in (0..N as i32).step_by(N / 1000) {
                // floor: largest key <= i
                let r = m.iter().take_while(|&(k, _)| k <= i).last();
                std::hint::black_box(r);
            }
        });
    }

    // ceiling (no dedicated method; approximate via iter)
    {
        let m = make_treemap_sorted();
        bench_n("ceiling(via iter, 1000 lookups)", 1000, || {
            for i in (0..N as i32).step_by(N / 1000) {
                // ceiling: smallest key >= i
                let r = m.iter().find(|&(k, _)| k >= i);
                std::hint::black_box(r);
            }
        });
    }

    // remove(all)
    bench_n("remove(all N)", N, || {
        let mut m = make_treemap_sorted();
        for i in 0..N as i32 {
            m.remove(i);
        }
        std::hint::black_box(&m);
    });

    // keys_to_vec (via keys().collect())
    {
        let m = make_treemap_sorted();
        bench_n("keys().collect::<Vec>()", N, || {
            let r: Vec<i32> = m.keys().collect();
            std::hint::black_box(&r);
        });
    }

    // values_to_vec (via values().collect())
    {
        let m = make_treemap_sorted();
        bench_n("values().collect::<Vec>()", N, || {
            let r: Vec<i32> = m.values().collect();
            std::hint::black_box(&r);
        });
    }

    // clear
    {
        let mut m = make_treemap_sorted();
        bench_n("clear", 1, || {
            m.clear();
            std::hint::black_box(&m);
        });
    }

    // Display(1K)
    {
        let m = make_treemap_1k();
        bench("Display(1K)", || {
            std::hint::black_box(format!("{}", m));
        });
    }
}

// ===========================================================================
//  7. I32TreeSet
// ===========================================================================
fn bench_treeset() {
    println!("\n--- I32TreeSet (N={}) ---", N);

    // Constructor
    bench("new()", || {
        std::hint::black_box(I32TreeSet::new());
    });

    // add(sorted)
    bench_n("add(sorted N)", N, || {
        let mut s = I32TreeSet::new();
        for i in 0..N as i32 {
            s.add(i);
        }
        std::hint::black_box(&s);
    });

    // add(random)
    bench_n("add(random N)", N, || {
        let mut s = I32TreeSet::new();
        for i in 0..N {
            s.add(scramble(i));
        }
        std::hint::black_box(&s);
    });

    // contains(hit)
    {
        let s = make_treeset();
        bench_n("contains(hit N)", N, || {
            for i in 0..N as i32 {
                std::hint::black_box(s.contains(i));
            }
        });
    }

    // contains(miss)
    {
        let s = make_treeset();
        bench_n("contains(miss N)", N, || {
            for i in N as i32..2 * N as i32 {
                std::hint::black_box(s.contains(i));
            }
        });
    }

    // for_each
    {
        let s = make_treeset();
        bench_n("for_each(N)", N, || {
            let mut sum = 0i64;
            s.for_each(|v| {
                sum += v as i64;
            });
            std::hint::black_box(sum);
        });
    }

    // iter
    {
        let s = make_treeset();
        bench_n("iter(N)", N, || {
            let mut sum = 0i64;
            for v in s.iter() {
                sum += v as i64;
            }
            std::hint::black_box(sum);
        });
    }

    // select(50%)
    {
        let s = make_treeset();
        bench_n("select(50%)", N, || {
            let r = s.select(|v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // reject (no dedicated method; use select with negated predicate)
    {
        let s = make_treeset();
        bench_n("reject(50% via select(!p))", N, || {
            let r = s.select(|v| v % 2 != 0);
            std::hint::black_box(&r);
        });
    }

    // detect (via iter)
    {
        let s = make_treeset();
        bench_n("detect(via iter.find)", N, || {
            let r = s.iter().find(|&v| v == N as i32 - 1);
            std::hint::black_box(r);
        });
    }

    // any_satisfy
    {
        let s = make_treeset();
        bench_n("any_satisfy", N, || {
            std::hint::black_box(s.any_satisfy(|v| v == N as i32 - 1));
        });
    }

    // all_satisfy
    {
        let s = make_treeset();
        bench_n("all_satisfy", N, || {
            std::hint::black_box(s.all_satisfy(|v| v >= 0));
        });
    }

    // min
    {
        let s = make_treeset();
        bench("min()", || {
            std::hint::black_box(s.min());
        });
    }

    // max
    {
        let s = make_treeset();
        bench("max()", || {
            std::hint::black_box(s.max());
        });
    }

    // floor (no dedicated method; approximate via iter)
    {
        let s = make_treeset();
        bench_n("floor(via iter, 1000 lookups)", 1000, || {
            for i in (0..N as i32).step_by(N / 1000) {
                let r = s.iter().take_while(|&v| v <= i).last();
                std::hint::black_box(r);
            }
        });
    }

    // ceiling (no dedicated method; approximate via iter)
    {
        let s = make_treeset();
        bench_n("ceiling(via iter, 1000 lookups)", 1000, || {
            for i in (0..N as i32).step_by(N / 1000) {
                let r = s.iter().find(|&v| v >= i);
                std::hint::black_box(r);
            }
        });
    }

    // union
    {
        let a = make_treeset();
        let mut b = I32TreeSet::new();
        for i in (N / 2) as i32..(N + N / 2) as i32 {
            b.add(i);
        }
        bench_n("union(N, N)", N, || {
            let r = a.union(&b);
            std::hint::black_box(&r);
        });
    }

    // intersect
    {
        let a = make_treeset();
        let mut b = I32TreeSet::new();
        for i in (N / 2) as i32..(N + N / 2) as i32 {
            b.add(i);
        }
        bench_n("intersect(N, N)", N, || {
            let r = a.intersect(&b);
            std::hint::black_box(&r);
        });
    }

    // difference
    {
        let a = make_treeset();
        let mut b = I32TreeSet::new();
        for i in (N / 2) as i32..(N + N / 2) as i32 {
            b.add(i);
        }
        bench_n("difference(N, N)", N, || {
            let r = a.difference(&b);
            std::hint::black_box(&r);
        });
    }

    // remove(all)
    bench_n("remove(all N)", N, || {
        let mut s = make_treeset();
        for i in 0..N as i32 {
            s.remove(i);
        }
        std::hint::black_box(&s);
    });

    // to_vec
    {
        let s = make_treeset();
        bench_n("to_vec", N, || {
            std::hint::black_box(s.to_vec());
        });
    }

    // clear
    {
        let mut s = make_treeset();
        bench_n("clear", 1, || {
            s.clear();
            std::hint::black_box(&s);
        });
    }

    // Display(1K)
    {
        let s = make_treeset_1k();
        bench("Display(1K)", || {
            std::hint::black_box(format!("{}", s));
        });
    }
}

// ===========================================================================
//  8. I32I32ListMultimap
// ===========================================================================
fn bench_multimap() {
    println!("\n--- I32I32ListMultimap (N={}) ---", N);

    // Constructor
    bench("new()", || {
        std::hint::black_box(I32I32ListMultimap::new());
    });

    // put(N with ~10 values per key)
    bench_n("put(N, ~10 vals/key)", N, || {
        let mut m = I32I32ListMultimap::new();
        let num_keys = N / 10;
        for i in 0..N as i32 {
            m.put(i % num_keys as i32, i);
        }
        std::hint::black_box(&m);
    });

    // get
    {
        let m = make_multimap();
        let num_keys = (N / 10) as i32;
        bench_n("get(all keys)", N / 10, || {
            for i in 0..num_keys {
                std::hint::black_box(m.get(i));
            }
        });
    }

    // get_all
    {
        let m = make_multimap();
        let num_keys = (N / 10) as i32;
        bench_n("get_all(all keys)", N / 10, || {
            for i in 0..num_keys {
                std::hint::black_box(m.get_all(i));
            }
        });
    }

    // remove_all
    bench_n("remove_all(all keys)", N / 10, || {
        let mut m = make_multimap();
        let num_keys = (N / 10) as i32;
        for i in 0..num_keys {
            m.remove_all(i);
        }
        std::hint::black_box(&m);
    });

    // contains_key
    {
        let m = make_multimap();
        let num_keys = (N / 10) as i32;
        bench_n("contains_key(all keys)", N / 10, || {
            for i in 0..num_keys {
                std::hint::black_box(m.contains_key(i));
            }
        });
    }

    // contains_key_value
    {
        let m = make_multimap();
        bench_n("contains_key_value(1000 lookups)", 1000, || {
            for i in 0..1000i32 {
                std::hint::black_box(m.contains_key_value(i, i));
            }
        });
    }

    // keys_count
    {
        let m = make_multimap();
        bench("keys_count()", || {
            std::hint::black_box(m.keys_count());
        });
    }

    // size
    {
        let m = make_multimap();
        bench("size()", || {
            std::hint::black_box(m.size());
        });
    }

    // for_each
    {
        let m = make_multimap();
        bench_n("for_each(N pairs)", N, || {
            let mut sum = 0i64;
            m.for_each(|_k, v| {
                sum += v as i64;
            });
            std::hint::black_box(sum);
        });
    }

    // for_each_key_value
    {
        let m = make_multimap();
        bench_n("for_each_key_value(all keys)", N / 10, || {
            let mut total_vals = 0usize;
            m.for_each_key_value(|_k, vals| {
                total_vals += vals.len();
            });
            std::hint::black_box(total_vals);
        });
    }

    // select(50%)
    {
        let m = make_multimap();
        bench_n("select(50%)", N, || {
            let r = m.select(|_k, v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // reject(50%)
    {
        let m = make_multimap();
        bench_n("reject(50%)", N, || {
            let r = m.reject(|_k, v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // keys_to_vec
    {
        let m = make_multimap();
        bench_n("keys_to_vec", N / 10, || {
            std::hint::black_box(m.keys_to_vec());
        });
    }

    // values_to_vec
    {
        let m = make_multimap();
        bench_n("values_to_vec", N, || {
            std::hint::black_box(m.values_to_vec());
        });
    }

    // clear
    {
        let mut m = make_multimap();
        bench_n("clear", 1, || {
            m.clear();
            std::hint::black_box(&m);
        });
    }
}

// ===========================================================================
//  9. I32TreeBag
// ===========================================================================
fn bench_treebag() {
    println!("\n--- I32TreeBag (N={}) ---", N);

    // Constructor
    bench("new()", || {
        std::hint::black_box(I32TreeBag::new());
    });

    // add
    bench_n("add(N, i%10000)", N, || {
        let mut b = I32TreeBag::new();
        for i in 0..N as i32 {
            b.add(i % 10000);
        }
        std::hint::black_box(&b);
    });

    // add_occurrences (no dedicated method; simulate with repeated add)
    bench_n("add(bulk 100 occurrences x 10000)", N, || {
        let mut b = I32TreeBag::new();
        for key in 0..10000i32 {
            for _ in 0..100 {
                b.add(key);
            }
        }
        std::hint::black_box(&b);
    });

    // remove (one occurrence)
    bench_n("remove(one occ, 10000 distinct)", 10000, || {
        let mut b = make_treebag();
        for i in 0..10000i32 {
            b.remove(i);
        }
        std::hint::black_box(&b);
    });

    // remove_occurrences (no dedicated method; simulate with repeated remove)
    bench_n("remove(50 occ each, 1000 keys)", 50000, || {
        let mut b = make_treebag();
        for key in 0..1000i32 {
            for _ in 0..50 {
                b.remove(key);
            }
        }
        std::hint::black_box(&b);
    });

    // remove_all
    bench_n("remove_all(10000 distinct)", 10000, || {
        let mut b = make_treebag();
        for i in 0..10000i32 {
            b.remove_all(i);
        }
        std::hint::black_box(&b);
    });

    // occurrences_of
    {
        let b = make_treebag();
        bench_n("occurrences_of(10000 lookups)", 10000, || {
            for i in 0..10000i32 {
                std::hint::black_box(b.occurrences_of(i));
            }
        });
    }

    // contains
    {
        let b = make_treebag();
        bench_n("contains(10000 lookups)", 10000, || {
            for i in 0..10000i32 {
                std::hint::black_box(b.contains(i));
            }
        });
    }

    // size
    {
        let b = make_treebag();
        bench("size()", || {
            std::hint::black_box(b.size());
        });
    }

    // size_distinct
    {
        let b = make_treebag();
        bench("size_distinct()", || {
            std::hint::black_box(b.size_distinct());
        });
    }

    // min
    {
        let b = make_treebag();
        bench("min()", || {
            std::hint::black_box(b.min());
        });
    }

    // max
    {
        let b = make_treebag();
        bench("max()", || {
            std::hint::black_box(b.max());
        });
    }

    // for_each (via for_each_with_occurrences expanding all)
    {
        let b = make_treebag();
        bench_n("for_each(via for_each_with_occ)", N, || {
            let mut sum = 0i64;
            b.for_each_with_occurrences(|v, c| {
                sum += v as i64 * c as i64;
            });
            std::hint::black_box(sum);
        });
    }

    // for_each_with_occurrences
    {
        let b = make_treebag();
        bench_n("for_each_with_occurrences(distinct)", 10000, || {
            let mut count = 0usize;
            b.for_each_with_occurrences(|_v, c| {
                count += c;
            });
            std::hint::black_box(count);
        });
    }

    // select(50%)
    {
        let b = make_treebag();
        bench_n("select(50%)", N, || {
            let r = b.select(|v| v % 2 == 0);
            std::hint::black_box(&r);
        });
    }

    // detect (via iter_distinct)
    {
        let b = make_treebag();
        bench_n("detect(via iter_distinct)", 10000, || {
            let r = b.iter_distinct().find(|&v| v == 9999);
            std::hint::black_box(r);
        });
    }

    // any_satisfy (via iter_distinct)
    {
        let b = make_treebag();
        bench_n("any_satisfy(via iter_distinct)", 10000, || {
            let r = b.iter_distinct().any(|v| v == 9999);
            std::hint::black_box(r);
        });
    }

    // all_satisfy (via iter_distinct)
    {
        let b = make_treebag();
        bench_n("all_satisfy(via iter_distinct)", 10000, || {
            let r = b.iter_distinct().all(|v| v >= 0);
            std::hint::black_box(r);
        });
    }

    // none_satisfy (via iter_distinct)
    {
        let b = make_treebag();
        bench_n("none_satisfy(via iter_distinct)", 10000, || {
            let r = !b.iter_distinct().any(|v| v < 0);
            std::hint::black_box(r);
        });
    }

    // clear
    {
        let mut b = make_treebag();
        bench_n("clear", 1, || {
            b.clear();
            std::hint::black_box(&b);
        });
    }
}

// ===========================================================================
//  Main
// ===========================================================================
fn main() {
    println!("=== Comprehensive Rust Benchmark: ALL Collections, ALL Methods ===");
    println!("N={}, SECS={} (for sustained benchmarks)", N, SECS);
    println!("Pattern: bench_n = single-shot with ns/op, bench = sustained ops/s");

    bench_hashmap();
    bench_hashset();
    bench_arraylist();
    bench_hashbag();
    bench_arraystack();
    bench_treemap();
    bench_treeset();
    bench_multimap();
    bench_treebag();

    println!("\n=== Done ===");
}
