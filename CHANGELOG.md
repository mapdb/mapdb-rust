# Changelog

All notable changes to `mapdb-collections` are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/); this crate is pre-1.0,
so a breaking change is a **minor** version bump.

## [Unreleased] — v3 Stage A (additive) + review-ledger bug fixes

Stage A of the v3 blueprint (`todo/fable-rust`): all **additive** — no removals,
no type-identity changes. Existing code compiles unchanged. Deprecated items
warn but still work; they are scheduled for removal in the breaking Stage C.

### Added

- **`RichIterator`** — a blanket extension trait over `Iterator` carrying the
  Eclipse-Collections vocabulary (`select`/`reject` lazy adapters, `detect`,
  `inject_into`, `group_by`/`group_by_each` → crate `OpenHashMap`, `to_bag`,
  `partition_into`, `top_n`/`bottom_n`, `join_display`, `count_where`, …).
  Available on every iterator, unboxed. Re-exported at the crate root.
- **`OpenHashMap::entry`** — full `Entry`/`OccupiedEntry`/`VacantEntry` API
  (`or_insert`/`or_insert_with`/`or_default`/`and_modify`/`remove`). Resolves a
  pending resize before probing, so an entry-built table is byte-identical to an
  insert-built one. Wired into `Multimap`/`SetMultimap::insert` and
  `RichIterator::group_by` (was a double probe).
- **`Compare<K>` comparator type parameter** on `TreeMap`/`TreeSet` (default
  `C = Comparator<K>`, so unchanged by default): `Natural` (zero-sized, inlined),
  `Reverse<C>`, `FnCmp<F>`, and a `Comparator<K>` bridge. New `with_comparator`
  / `natural` constructors; `Default`/`FromIterator`/`Extend` on the `Natural`
  instantiation; `DynTreeMap`/`DynTreeSet` aliases for the runtime-comparator case.
- **`TreeMap::range` / `TreeSet::range`** taking any `RangeBounds<K>`, returning
  a lazy **double-ended, exact-size** iterator whose bounds are compared through
  the map's own comparator (so range selection can never disagree with tree
  order). `.rev()` gives descending. Inverted/empty bounds yield nothing (no
  panic — documented divergence from `BTreeMap::range`).
- **`retain`** on `OpenHashMap`/`OpenHashSet`; **owned `IntoIterator`** +
  `into_keys`/`into_values` on `TreeMap`, owned `IntoIterator` on `TreeSet`
  (all `DoubleEnded` + `ExactSize`); **`FromIterator`/`Extend`** on `RoaringU32`.

### Fixed (confirmed review-ledger findings)

- `HashBag::insert`/`add_occurrences` now overflow-check the count and size
  (were unchecked `+=`, wrapping in release).
- `SetMultimap::from_sorted_key_values` dedupes a value by `Eq` against the whole
  bucket, not just the last element (a non-adjacent `Eq`-duplicate of a
  comparator-equal value no longer breaks the set invariant / `len`).
- `RangeMap::put_coalescing` merges an already-emitted equal-valued left entry
  that a later entry bridges (was two entries instead of one).
- `BitSet::from_sorted_indices([usize::MAX])` returns `BulkError::IndexOverflow`
  (new variant) instead of overflowing.
- `BitSet` `PartialEq` is now logical-bits-only (`java.util.BitSet.equals`
  semantics); capacity/history no longer observable.
- `CountMin::optimal` range-checks `(d, w)` before the `f64 as u32` casts.
- `PriorityQueue::Display` documents its non-canonical heap-array order;
  `TreeMap`/`TreeSet` legacy `Range<K>` methods documented as natural-order-only;
  `sub_map`/`sub_range_set`/`sub_range_map` documented as snapshots, not views.

### Deprecated

- `stream::collectors::*` and `stream::generators::*` free functions —
  superseded by `RichIterator` and `std` iterator constructors (see the `stream`
  module docs for the mapping).

## [0.2.0] — breaking idiom pass

This release renames the remaining Java-isms to standard-library vocabulary,
removes redundant constructors, makes the open-addressing map/set generic over
the hasher, repacks the probe slot for cache locality, and redesigns the
synchronized wrapper around a lock guard. All changes are **source-breaking**.

### Migration table

| Area | Old API (0.1.x) | New API (0.2.0) |
|---|---|---|
| Multimap / SetMultimap | `mm.size()` | `mm.len()` |
| Multimap / SetMultimap | `mm.size_distinct()` | `mm.distinct_len()` |
| Multimap / SetMultimap | `mm.put(k, v)` | `mm.insert(k, v)` |
| PriorityQueue | `q.size()` | `q.len()` |
| Interval | `iv.size()` | `iv.len()` |
| HashBag / `Bag` trait | `bag.size_distinct()` | `bag.distinct_len()` |
| `MutableSet` trait + `HashSet`/`LinkedHashSet`/`TreeSet`/`HashSetWithStrategy`/`OpenHashSet` | `set.add(v) -> bool` | `set.insert(v) -> bool` |
| `MutableBag` trait + `HashBag` | `bag.add(v)` | `bag.insert(v)` |
| HashBiMap | `bm.put(k, v) -> Option<V>` | `bm.insert(k, v) -> Option<V>` |
| `BatchIterable` trait | `fn size(&self)` | `fn len(&self)` (+ new default `is_empty`) |
| `HashBag` | `HashBag::of(iter)` | `HashBag::from_iter(iter)` / `iter.collect()` |
| `HashSet` | `HashSet::of(iter)` | `HashSet::from_iter(iter)` / `iter.collect()` |
| `LinkedHashSet` | `LinkedHashSet::of(iter)` | `LinkedHashSet::from_iter(iter)` / `iter.collect()` |
| `ArrayList` | `ArrayList::of(iter)` | `ArrayList::from_iter(iter)` / `iter.collect()` |
| `ArrayStack` | `ArrayStack::of(iter)` | `ArrayStack::from_iter(iter)` / `iter.collect()` |
| `ArrayDeque` | `ArrayDeque::of(iter)` | `ArrayDeque::from_iter(iter)` / `iter.collect()` |
| `PriorityQueue` | `PriorityQueue::of(iter)` | `PriorityQueue::from_iter(iter)` / `iter.collect()` (still O(n) Floyd heapify) |
| `OpenHashMap` / `OpenHashSet` | `OpenHashMap<K, V>` | `OpenHashMap<K, V, S = RandomState>` (default keeps old call sites compiling) |
| `Synchronized<C>` | `s.with(\|c\| …)` | `let g = s.lock(); /* &*g */` (guard `Deref`) |
| `Synchronized<C>` | `s.with_mut(\|c\| …)` | `let mut g = s.lock(); /* &mut *g */` (guard `DerefMut`) |
| `Synchronized<C>` | `s.lock()` → `MutexGuard` | `s.lock()` → `SyncGuard` (Deref/DerefMut); raw handle via `s.inner()` |

### Changed

- **Removed `size()` (Java-ism).** Use `len()` everywhere. The `len()`/`is_empty()`
  pairs shipped additively in v1; v2 deletes the duplicated `size()` on
  `Multimap`, `SetMultimap`, `Interval`, `PriorityQueue`, and the `BatchIterable`
  trait. The distinct-element count on bags and multimaps is now `distinct_len()`
  (was `size_distinct()`).
- **`add` → `insert` / `push`.** Set/bag insertion uses `insert` to match
  `std::collections` sets; this covers the `MutableSet`/`MutableBag` traits and
  every concrete set/bag (`OpenHashSet`, object `HashSet`, `LinkedHashSet`,
  `TreeSet`, `HashSetWithStrategy`, `HashBag`). Stack/list growth already used
  `push`.
- **`put` → `insert`.** `HashBiMap` and both multimaps use the std map verb
  `insert` (the `MutableMap` trait already did).
- **Removed `of()` constructors.** Fully covered by the `FromIterator` impls
  added in v1 — build with `from_iter` / `collect` / `From<[T; N]>`. The
  `PriorityQueue` O(n) Floyd heapify now lives in its `FromIterator` impl.
- **Generic hasher parameter.** `OpenHashMap<K, V, S = RandomState>` and
  `OpenHashSet<K, S = RandomState>` gained a `S: BuildHasher` type parameter.
  `::new()` / `::with_capacity()` still default to `RandomState` (HashDoS-
  resistant, matching `std::collections::HashMap`), so ordinary call sites
  compile unchanged via the default type param. New constructors
  `with_hasher` / `with_capacity_and_hasher` and a `hasher()` accessor opt into
  a faster fixed hasher (FxHash, AHash, …).
- **`MapEntry` layout repacked for cache locality.** The probe array is now a
  `Vec<Slot>` where `Slot` is a two-variant enum (`Empty` / `Occupied { key,
  value }`). The occupancy flag is the enum discriminant (no separate `bool`)
  and the key/value are stored inline (no `Option` wrappers), removing ~40
  invariant `unwrap()`s and shrinking the per-slot footprint. Backward-shift
  deletion keeps the table tombstone-free, so two variants suffice. This is an
  internal/ABI change; no public method signatures changed, but any code that
  reached into the private `entries` field or relied on the old `MapEntry`
  layout is affected.
- **`Synchronized<C>` redesigned around a guard.** `lock()` now returns a
  `SyncGuard<'_, C>` that implements `Deref` / `DerefMut`, so you operate on the
  inner collection directly and the lock releases on drop (standard Rust `Mutex`
  ergonomics). The `with` / `with_mut` closure methods were removed. Use
  `Synchronized::inner()` to reach the underlying `Arc<Mutex<C>>` for `try_lock`
  / `PoisonError` recovery.

### Note — signature relaxations already shipped in v1 (recorded for completeness)

These landed additively in the v1 idiom pass (they follow `std` exactly and keep
ordinary call sites compiling), and were never part of the v2 break:

- `Borrow<Q>` on `get` / `contains_key` / `remove` for `OpenHashMap`/`OpenHashSet`,
  the object `HashMap`/`HashSet`, and `HashBiMap` (forward + inverse). Rare
  inference caveat at turbofish / `.into()` call sites.
- Inherent `iter()` on the object maps shadows the `MapIterable::iter` trait
  method. `for` loops and unannotated calls are unaffected; explicit
  `Box<dyn Iterator<…>>` annotations must call the trait method by path.

## [0.1.0]

Initial release: generic Rust port of Eclipse Collections (open-addressing
`OpenHashMap`/`OpenHashSet`, object collections, multimaps, intervals, priority
queue, parallel batch iteration, immutable views, `Synchronized` wrapper, and
the `HashableF32`/`HashableF64` float newtypes).
