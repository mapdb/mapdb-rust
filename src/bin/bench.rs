// Benchmark and stress test for Rust collections.
// Run: cd mapdb-rust && rustc -O --edition 2021 -L target/release/deps \
//      --extern mapdb_collections=target/release/libmapdb_collections.rlib \
//      ../benchmarks/bench_rust.rs -o bench_rust && ./bench_rust
// Or simpler: copy into src/bin/bench.rs and run: cargo run --release --bin bench

use std::time::Instant;

// When run as src/bin/bench.rs, use crate imports
use mapdb_collections::arraylist::i32_array_list::I32ArrayList;
use mapdb_collections::bag::i32_hash_bag::I32HashBag;
use mapdb_collections::bitset::bit_set::BitSet;
use mapdb_collections::deque::i32_array_deque::I32ArrayDeque;
use mapdb_collections::hashmap::i32_i32_hash_map::I32I32HashMap;
use mapdb_collections::hashset::i32_hash_set::I32HashSet;
use mapdb_collections::priority_queue::i32_priority_queue::I32PriorityQueue;

const N: usize = 100_000;
const WARM: usize = 3;

fn main() {
    println!("=== Rust Benchmark ===");
    println!("N={}\n", N);

    bench_hashmap_insert();
    bench_hashmap_get();
    bench_hashmap_delete();
    bench_hashmap_iterate();
    bench_hashset_insert();
    bench_hashset_contains();
    bench_arraylist_push();
    bench_bag_add();
    bench_array_deque_insert();
    bench_array_deque_get();
    bench_array_deque_delete();
    bench_priority_queue_insert();
    bench_priority_queue_get();
    bench_priority_queue_delete();
    bench_bitset_insert();
    bench_bitset_get();
    bench_bitset_delete();

    println!("\n=== Stress Tests ===");
    stress_collision_keys();
    stress_delete_heavy();
    stress_resize_cycles();
    stress_float_keys();
    stress_edge_keys();
}

fn bench_hashmap_insert() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut m = I32I32HashMap::new();
        let start = Instant::now();
        for i in 0..N as i32 {
            m.insert(i, i * 10);
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
        std::hint::black_box(&m);
    }
    println!(
        "HashMap.insert      {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_hashmap_get() {
    let mut m = I32I32HashMap::new();
    for i in 0..N as i32 {
        m.insert(i, i * 10);
    }
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let start = Instant::now();
        for i in 0..N as i32 {
            std::hint::black_box(m.get(i));
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "HashMap.get         {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_hashmap_delete() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut m = I32I32HashMap::new();
        for i in 0..N as i32 {
            m.insert(i, i * 10);
        }
        let start = Instant::now();
        for i in 0..N as i32 {
            m.remove(i);
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "HashMap.remove      {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_hashmap_iterate() {
    let mut m = I32I32HashMap::new();
    for i in 0..N as i32 {
        m.insert(i, i * 10);
    }
    let mut best = u128::MAX;
    let mut sum: i64 = 0;
    for _ in 0..WARM {
        sum = 0;
        let start = Instant::now();
        for (_, v) in m.iter() {
            sum += v as i64;
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "HashMap.iter        {} entries  {:.3}ms  ({} ns/entry) sum={}",
        N,
        best as f64 / 1e6,
        best / N as u128,
        sum
    );
}

fn bench_hashset_insert() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut s = I32HashSet::new();
        let start = Instant::now();
        for i in 0..N as i32 {
            s.add(i);
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "HashSet.add         {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_hashset_contains() {
    let mut s = I32HashSet::new();
    for i in 0..N as i32 {
        s.add(i);
    }
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let start = Instant::now();
        for i in 0..N as i32 {
            std::hint::black_box(s.contains(i));
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "HashSet.contains    {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_arraylist_push() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut a = I32ArrayList::new();
        let start = Instant::now();
        for i in 0..N as i32 {
            a.push(i);
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "ArrayList.add       {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_bag_add() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut b = I32HashBag::new();
        let start = Instant::now();
        for i in 0..N as i32 {
            b.add(i % 1000);
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "Bag.add             {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_array_deque_insert() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut d = I32ArrayDeque::new();
        let start = Instant::now();
        for i in 0..N as i32 {
            d.add_last(i);
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "ArrayDeque.add_last {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_array_deque_get() {
    let mut d = I32ArrayDeque::new();
    for i in 0..N as i32 {
        d.add_last(i);
    }
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let start = Instant::now();
        for _ in 0..N {
            std::hint::black_box(d.peek_first());
            std::hint::black_box(d.peek_last());
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best {
            best = elapsed;
        }
    }
    println!(
        "ArrayDeque.peek     {} ops  {:.3}ms  ({} ns/op)",
        N * 2,
        best as f64 / 1e6,
        best / (N * 2) as u128
    );
}

fn bench_array_deque_delete() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut d = I32ArrayDeque::new();
        for i in 0..N as i32 {
            d.add_last(i);
        }
        let start = Instant::now();
        for _ in 0..N {
            d.remove_first();
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best {
            best = elapsed;
        }
    }
    println!(
        "ArrayDeque.remove   {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_priority_queue_insert() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut q = I32PriorityQueue::new();
        let start = Instant::now();
        for i in 0..N as i32 {
            q.push(i);
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "PriorityQueue.push  {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_priority_queue_get() {
    let mut q = I32PriorityQueue::new();
    for i in 0..N as i32 {
        q.push(i);
    }
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let start = Instant::now();
        for _ in 0..N {
            std::hint::black_box(q.peek());
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best {
            best = elapsed;
        }
    }
    println!(
        "PriorityQueue.peek  {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_priority_queue_delete() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut q = I32PriorityQueue::new();
        for i in 0..N as i32 {
            q.push(i);
        }
        let start = Instant::now();
        for _ in 0..N {
            q.pop();
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best {
            best = elapsed;
        }
    }
    println!(
        "PriorityQueue.pop   {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_bitset_insert() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut b = BitSet::new();
        let start = Instant::now();
        for i in 0..N {
            b.set(i);
        }
        let d = start.elapsed().as_nanos();
        if d < best {
            best = d;
        }
    }
    println!(
        "BitSet.set          {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_bitset_get() {
    let mut b = BitSet::new();
    for i in 0..N {
        b.set(i);
    }
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let start = Instant::now();
        for i in 0..N {
            std::hint::black_box(b.get(i));
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best {
            best = elapsed;
        }
    }
    println!(
        "BitSet.get          {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

fn bench_bitset_delete() {
    let mut best = u128::MAX;
    for _ in 0..WARM {
        let mut b = BitSet::new();
        for i in 0..N {
            b.set(i);
        }
        let start = Instant::now();
        for i in 0..N {
            b.clear_bit(i);
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed < best {
            best = elapsed;
        }
    }
    println!(
        "BitSet.clear_bit    {} ops  {:.3}ms  ({} ns/op)",
        N,
        best as f64 / 1e6,
        best / N as u128
    );
}

// --- Stress Tests ---

fn stress_collision_keys() {
    let mut m = I32I32HashMap::new();
    let start = Instant::now();
    for i in 0..10_000i32 {
        m.insert(i * 16, i); // multiples of 16 = default capacity
    }
    let d = start.elapsed();
    let mut ok = true;
    for i in 0..10_000i32 {
        if m.get(i * 16).is_none() {
            ok = false;
            break;
        }
    }
    println!(
        "STRESS collision_keys   10000 ops  {:.3}ms  all_found={}",
        d.as_secs_f64() * 1000.0,
        ok
    );
}

fn stress_delete_heavy() {
    let mut m = I32I32HashMap::new();
    for i in 0..50_000i32 {
        m.insert(i, i);
    }
    let start = Instant::now();
    for i in (0..50_000i32).step_by(2) {
        m.remove(i);
    } // remove even
    for i in 50_000..75_000i32 {
        m.insert(i, i);
    } // insert new
    for i in 0..75_000i32 {
        m.remove(i);
    } // remove all
    let d = start.elapsed();
    println!(
        "STRESS delete_heavy    125000 ops  {:.3}ms  size={} (expect 0)",
        d.as_secs_f64() * 1000.0,
        m.len()
    );
}

fn stress_resize_cycles() {
    let mut m = I32I32HashMap::new();
    let start = Instant::now();
    for _ in 0..10 {
        for i in 0..10_000i32 {
            m.insert(i, i);
        }
        m.clear();
    }
    let d = start.elapsed();
    println!(
        "STRESS resize_cycles   100000 ops  {:.3}ms  size={} (expect 0)",
        d.as_secs_f64() * 1000.0,
        m.len()
    );
}

fn stress_float_keys() {
    use mapdb_collections::hashmap::f32_f32_hash_map::F32F32HashMap;
    let mut m = F32F32HashMap::new();
    m.insert(1.0, 10.0);
    m.insert(-0.0, 20.0);
    m.insert(f32::INFINITY, 30.0);
    m.insert(f32::NAN, 40.0);
    m.insert(f32::NAN, 50.0); // NaN overwrite (same bits)
    let nan_val = m.get(f32::NAN);
    let neg_zero_val = m.get(-0.0);
    let inf_val = m.get(f32::INFINITY);
    println!(
        "STRESS float_keys      NaN={:?} -0.0={:?} Inf={:?} size={} (expect 4)",
        nan_val,
        neg_zero_val,
        inf_val,
        m.len()
    );
}

fn stress_edge_keys() {
    let mut m = I32I32HashMap::new();
    m.insert(0, 100);
    m.insert(-1, 200);
    m.insert(i32::MAX, 300);
    m.insert(i32::MIN, 400);
    let ok = m.len() == 4
        && m.get(0) == Some(100)
        && m.get(i32::MAX) == Some(300)
        && m.get(i32::MIN) == Some(400);
    println!("STRESS edge_keys       boundary values  ok={}", ok);
}
