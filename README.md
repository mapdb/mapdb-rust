# mapdb-collections (Rust)

Generic collections for Rust, ported from [Eclipse Collections](https://eclipse.dev/collections/).

## Why generic, not per-primitive?

The Java port has `IntIntHashMap`, `LongObjectHashMap`, … as hand-written per-primitive classes because Java's `List<Integer>` boxes every `int` into a heap `Integer` object. There is no way to get a contiguous `int[]`-backed map in Java without code duplication.

Rust doesn't have this problem. **Monomorphisation** specialises `OpenHashMap<i32, i32>` and `OpenHashMap<f32, f32>` at compile time — no boxing, no indirection, identical performance to a hand-written `IntIntHashMap`. So we ship one algorithm body and let the compiler do the specialisation.

| Java needs | Rust gets via |
|---|---|
| `IntArrayList` | `Vec<i32>` |
| `IntIntHashMap` | `OpenHashMap<i32, i32>` |
| `IntHashSet` | `OpenHashSet<i32>` |
| `FloatFloatHashMap` | `OpenHashMap<HashableF32, f32>` |
| `IntIntPair` | `Pair<i32, i32>` |
| `ImmutableIntHashSet` | `ImmutableHashSet<i32>` (= `Arc<OpenHashSet<i32>>`) |
| `Collections.synchronizedList(l)` | `synchronized(l)` returning `Synchronized<L>` |

## Core types

```rust
use mapdb_collections::{
    OpenHashMap, OpenHashSet,           // ported from Java's hashmap algorithm
    HashableF32, HashableF64,           // newtype for f32/f64 as Hash + Eq + Ord
    Synchronized, synchronized,         // Java-style sync wrapper
    ImmutableHashMap, ImmutableHashSet, // frozen via Arc, O(1) lookup
    ImmutableList,                      // frozen Arc<[T]>
    Pair,                               // generic 2-tuple
};
```

The Java-side algorithm port (open-addressing, linear probing, Robin Hood backward-shift deletion, interleaved `{occupied, key, value}` entries for cache locality) lives in `src/hash_table.rs`.

## Quick start

```rust
use mapdb_collections::{OpenHashMap, OpenHashSet, HashableF32, synchronized};

let mut m: OpenHashMap<i32, i32> = OpenHashMap::new();
m.insert(1, 100);
m.insert(2, 200);
assert_eq!(m.get(&1), Some(&100));

// V can be any type — including non-Copy:
let mut by_id: OpenHashMap<i32, String> = OpenHashMap::new();
by_id.insert(7, "seven".to_string());

// Float keys via the HashableF32/F64 newtype — NaN-aware, ±0 distinct:
let mut prices: OpenHashMap<HashableF32, &str> = OpenHashMap::new();
prices.insert(HashableF32(1.99), "soda");
prices.insert(HashableF32(f32::NAN), "missing");
assert!(prices.contains_key(&HashableF32(f32::NAN))); // NaN-keyed lookups work

// Thread-safe view, Java-style factory:
let sync_map = synchronized(OpenHashMap::<i32, i32>::new());
sync_map.with_mut(|m| { m.insert(1, 10); });
sync_map.with(|m| assert_eq!(m.get(&1), Some(&10)));

// Set:
let mut s: OpenHashSet<i32> = OpenHashSet::new();
s.add(1); s.add(2); s.add(3);
assert!(s.contains(&2));
```

## Float handling

`HashableF32` / `HashableF64` are `#[repr(transparent)]` newtypes around `f32`/`f64`. They implement:
- `Hash` + `Eq` via the IEEE bit pattern — same NaN bits → equal, `+0.0 ≠ -0.0`.
- `Ord` via `total_cmp` — total ordering even with NaN (NaN sorts at the extremes).

There is zero runtime cost: a `HashableF32` is bit-identical to an `f32`.

## Cross-language algorithm parity

This crate uses our ported `OpenHashMap` (not `std::collections::HashMap`) for everything that needs a hash map, including:
- The primary `OpenHashMap<K, V>` public type
- `object::HashMap<K, V>` / `object::HashSet<T>` wrappers
- `Multimap<K, V>` (one-key-to-many-values) backing store
- `ImmutableHashMap<K, V>` / `ImmutableHashSet<T>` frozen views

This preserves the cache-locality interleaved-entry layout that Eclipse Collections uses on the Java side. `std::BTreeMap`/`BTreeSet`/`BinaryHeap` are used where the algorithm matches (sorted maps, priority queues).

## Layout

```
src/
├── hash_table.rs        — OpenHashMap<K, V>, OpenHashSet<T> (the algorithm)
├── hashable_float.rs    — HashableF32, HashableF64
├── synchronized.rs      — Synchronized<C> + factory
├── immutable.rs         — ImmutableHashMap / ImmutableHashSet / ImmutableList
├── pair.rs              — Pair<A, B>
├── traits.rs            — PrimitiveCollection<T>, PrimitiveList, PrimitiveMap, …
├── multimap/            — Multimap<K, V>
├── object/              — HashMap / HashSet / ArrayList / TreeMap / …
├── stream/              — Stream-style generators + collectors
└── bin/
    ├── nanprobe.rs      — NaN-semantics probe (cross-language harness)
    └── validate.rs      — JSON scenario runner (cross-language harness)
```
