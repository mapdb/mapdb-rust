# mapdb-collections (Rust)

High-performance primitive-specialized and generic collections for Rust, inspired by [Eclipse Collections](https://eclipse.dev/collections/).

## Why?

Each primitive collection type is specialized per primitive type (`i8`, `i16`, `i32`, `i64`, `f32`, `f64`, `bool`, `char`) — no boxing, no trait objects, contiguous memory layout. Generic object collections (`ArrayList<T>`, `HashSet<T>`, etc.) complement the primitive types for general-purpose use.

## Primitive Collections

| Type | Mutable | Immutable | Variants |
|------|---------|-----------|----------|
| **ArrayList** | `I32ArrayList` | `ImmutableI32ArrayList` | 8 types |
| **HashSet** | `I32HashSet` | `ImmutableI32HashSet` | 8 types |
| **HashBag** | `I32HashBag` | `ImmutableI32HashBag` | 8 types |
| **ArrayStack** | `I32ArrayStack` | `ImmutableI32ArrayStack` | 8 types |
| **HashMap** | `I32I64HashMap` | `ImmutableI32I64HashMap` | 64 pairs (8x8) |
| **TreeSet** | `I32TreeSet` | — | 8 types |
| **TreeMap** | `I32I64TreeMap` | — | 64 pairs |
| **Pair** | `I32I64Pair` | — | 64 pairs |
| **Multimap** | `Multimap<K, V>` | — | Generic |
| **Stream** | generators + collectors | — | Generic |

## Object Collections

Generic collections that work with any type:

| Type | Description |
|------|-------------|
| `ArrayList<T>` | Ordered list backed by `Vec<T>` |
| `HashSet<T>` | Unordered set backed by `std::collections::HashSet` |
| `HashMap<K, V>` | Key-value map backed by `std::collections::HashMap` |
| `HashBag<T>` | Counting bag backed by `HashMap<T, usize>` |
| `ArrayStack<T>` | LIFO stack backed by `Vec<T>` |
| `HashBiMap<K, V>` | Bidirectional map with unique keys and values |

## Quick Start

```rust
use mapdb_collections::arraylist::i32_array_list::I32ArrayList;
use mapdb_collections::hashmap::i32_i64_hash_map::I32I64HashMap;
use mapdb_collections::object::{ArrayList, HashBag, Collection, MutableList, MutableBag};

// Primitive ArrayList
let mut list = I32ArrayList::of(&[3, 1, 4, 1, 5]);
list.sort();
assert_eq!(list.to_vec(), vec![1, 1, 3, 4, 5]);
assert_eq!(list.select(|v| v > 2).len(), 3);

// Primitive HashMap
let mut map = I32I64HashMap::new();
map.insert(1, 100);
map.insert(2, 200);
assert_eq!(map.get(1), Some(100));

// Generic ArrayList
let names = ArrayList::of(vec!["Alice", "Bob", "Charlie"]);
assert_eq!(names.detect(|n| n.starts_with("B")), Some(&"Bob"));

// Generic HashBag
let mut bag = HashBag::new();
bag.add("apple");
bag.add_occurrences("apple", 3);
assert_eq!(bag.occurrences_of(&"apple"), 4);
```

## Stream API

```rust
use mapdb_collections::stream::generators;
use mapdb_collections::stream::collectors;

let squares: Vec<i32> = generators::range(1, 6).map(|x| x * x).collect();
assert_eq!(squares, vec![1, 4, 9, 16, 25]);

let groups = collectors::group_by(vec![1, 2, 3, 4, 5, 6].into_iter(), |v| v % 2);
assert_eq!(groups[&0].len(), 3);
```

## Immutable Collections

Immutable variants use `Arc<[T]>` for O(1) clone. Modifications return new instances.

```rust
use mapdb_collections::immutable::immutable_i32_array_list::ImmutableI32ArrayList;

let im = ImmutableI32ArrayList::of(&[1, 2, 3]);
let im2 = im.clone(); // O(1) — shared Arc
let mut mutable = im.to_mutable();
mutable.push(4);
assert_eq!(im.len(), 3); // unchanged
```

## Float Handling

Float types (`f32`, `f64`) use `to_bits()` for equality and hashing — NaN-safe.
`HashableF32` / `HashableF64` wrappers available for use as HashMap keys.

## Stats

- **580 source files**, **4,517 tests** passing
- **0 clippy warnings**
- All 8 Rust primitive types + generic object types
- Zero external dependencies (library crate)
