# Changelog

All notable changes to `mapdb-collections` are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/); this crate is pre-1.0,
so a breaking change is a **minor** version bump.

## [Unreleased] — additive: `entry` + `retain` + `drain` + mutable access + owned `IntoIterator` + `BoundedMap` across the collections

- **BREAKING: `RoaringU32::deserialize` now returns `Result<_, RoaringError>`**
  (a typed enum) instead of `Result<_, String>`, mirroring the crate's other
  typed decode error `HllError`. Each of the 13 variants names a specific
  reader-MUST-reject rule (truncation, foreign/unsupported header, non-canonical
  or corrupt container encoding) and carries the offending values as fields.
  `RoaringError` is `Clone`/`PartialEq`/`Eq`/`Debug` + `Display` + `std::error::Error`;
  its `Display` text is byte-for-byte the previous error strings, so
  string-consuming callers are unaffected — only the `Err` *type* changed.
- **`BoundedMap<K, V, P: EvictionPolicy>`** — a new capacity-bounded map, generic
  in its value type and in a pluggable **eviction policy**, the value-generic
  successor to the frozen, `i32`-specialised `BoundedLruMap` (which is untouched).
  The map owns its values in an `Option<(K, V)>` slot arena, so eviction is
  ownership transfer, not garbage collection: `evict()` **returns** the victim
  `(K, V)` to the caller, and an implicit size eviction **drops** the value
  synchronously (its `Drop` runs *right then* — the point of a generic `V` such
  as a buffer or file handle). *Which* resident entry is evicted is delegated to a
  `P: EvictionPolicy` (a slot-index trait — `on_insert`/`on_access`/`on_remove`/
  `victim`/`clear`), monomorphised in like `OpenHashMap`'s hasher; `Lru`
  (intrusive recency list over slot indices) and `Fifo` (insertion order) ship,
  and a new policy is a new type — the map does not change. Surface: `put`
  (evict-before-insert, returns the previous value), `get`/`get_mut` (refresh
  recency — hence `&mut self`), `peek` (`&self`, no recency touch), `remove`,
  `evict`, `clear`, `contains_key`, `iter`/`keys`/`values`, owned + borrowed
  `IntoIterator`, and an optional `on_evict` **observer** (`&K, &V, cause`; fired
  for size and TTL-expiry evictions — `remove`/`evict`/`clear` are not evictions).
  Optional after-write **TTL** (`with_ttl(ticks)` + `put_at(k, v, now)` +
  `expire_entries(now)`), orthogonal to the eviction policy (time vs space) and
  matching `BoundedLruMap`'s logical-tick model; `u64::MAX` is the never-expire
  sentinel. Aliased constructors: `BoundedMap::with_capacity(n)` (LRU) and
  `BoundedMap::fifo(n)`. Requires `K: Clone` (transitional `OpenHashMap<K, usize>`
  index; a later revision can move to the key-owning-free `IndexTable` kernel to
  drop it).
- **Owned `IntoIterator` across the remaining containers** — `RangeSet<T>`,
  `RangeMap<T, V>`, `RoaringU32`, `HashBag<T>`, `Multimap<K, V>`, and
  `SetMultimap<K, V>` now support consuming iteration (`for x in value`, and
  `value.into_iter()`), matching the borrowing `IntoIterator` they already had.
  Each yields in the same order as its borrowing iterator: `RangeSet`/`RangeMap`
  hand out their normal-form entries ascending by lower cut (double-ended,
  exact-size, fused, since they wrap the backing `Vec`); `RoaringU32` yields
  `u32` values in unsigned-ascending order, decompressing one chunk container at
  a time so peak extra memory is a single chunk; `HashBag` yields each element
  once **per occurrence**, and `Multimap`/`SetMultimap` yield one flattened
  `(K, V)` pair per stored value. The bag/multimap owned iterators require
  `T: Clone` / `K: Clone` respectively (unavoidable: one stored element is handed
  out as several owned values — it is cloned for every occurrence/value but the
  last of its group, which moves out); the bound stays on the owned iterator only
  and does not touch the borrowing iterator or any other method.
- **`TreeMap::get_mut` / `iter_mut` / `values_mut`** (+ `IntoIterator for &mut
  TreeMap`, i.e. `for (k, v) in &mut map`) — the mutable-value-access surface,
  previously absent. Keys are handed out as `&K` (shared), so a caller cannot
  change a key and desync the sort order; only values are mutable. `get_mut`
  is an O(log n) recursive comparator descent. `iter_mut`/`values_mut` visit in
  ascending order and are built by disjoint-borrow materialization — each
  `&mut Node` is split into disjoint field borrows (no `unsafe`) to collect
  `(&K, &mut V)` pairs — so `TreeMapIterMut` is double-ended, exact-size, and
  fused. `TreeSet` intentionally gains none of this (mutating an element in
  place would break the ordering invariant).
- **`TreeMap::drain()` / `TreeSet::drain()`** — remove all entries and return
  them as an iterator in ascending comparator order while **keeping the emptied
  map/set (and its comparator) for reuse** — the reuse-friendly counterpart to
  `into_iter`, which consumes the container instead. The container is emptied *immediately*, before the first item is
  yielded, by dismantling the tree up front (no user code runs during teardown);
  so it is left a valid, empty tree even if the iterator is only partially
  consumed, dropped early, or a consuming loop panics — no drop guard needed
  (contrast `retain`). The returned `TreeMapDrain`/`TreeSetDrain` holds a mutable
  borrow of the container for its lifetime (matching `std`'s `Drain`) and is a
  double-ended, exact-size, fused iterator. Works for any comparator `C`.
- **`TreeMap::retain(|&k, &mut v| …)` / `TreeSet::retain(|&t| …)`** — drop the
  entries a predicate rejects, visiting keys in ascending comparator order and
  allowing in-place value mutation (the key, and so the sort order, is
  immutable to the predicate). Works for any comparator `C` (no `K: Clone` /
  `K: Ord`). `O(n log n)`: the tree is dismantled into its sorted entries with
  no user code running during teardown, then the survivors are moved back into
  a fresh tree. Panic-consistent — if the predicate panics, the map is left a
  valid LLRB tree holding exactly the survivors visited before the panic (with
  a correct `len()`), and every not-yet-visited entry is dropped; an O(1) drop
  guard also keeps `len()` consistent if an *adversarial comparator* panics
  during a survivor re-insert (recomputing `size` from the tree's cached root
  subtree size). This completes `retain` across **every** map/set/bag/multimap
  in the crate.
- **`Multimap::retain(|&k, &v| …)` / `SetMultimap::retain(|&k, &v| …)`** —
  per-(key, value)-pair retain: drops rejected values from each key's bucket
  (order preserved), removes a key whose bucket empties out, and keeps the
  total-value `len()` accounting exact. Panic-consistent via the same drop-guard
  as `HashBag::retain` (recomputes `size` from survivors even if the predicate
  panics and the unwind is caught).
- **`retain` across the rest of the object hash family** — `object::HashMap`,
  `object::HashSet` (thin delegations to the `OpenHashMap`/`OpenHashSet` kernel
  retain), and `HashBag::retain(|&elem, occurrences| …)` (multiset retain: drops
  a rejected distinct element with *all* its occurrences and keeps the total-size
  accounting exact by subtracting the dropped counts). Together with the
  `LinkedHash*` retain below, every object hash collection now has `retain`.
- **`LinkedHashMap::retain(|&k, &mut v| …)` / `LinkedHashSet::retain(|&t| …)`** —
  drop the entries a predicate rejects, in a single insertion-order pass, keeping
  survivors' positions and allowing in-place value mutation. O(n): each dropped
  entry is unlinked and its slot recycled in O(1) (no index fix-up sweep), no
  `K: Clone`. Removals go index-cell-first (re-deriving the stored hash from the
  still-live key and matching the exact arena slot — the same cell-location
  argument as `entry`'s `remove_entry`), so no user `Eq` runs during the
  backward-shift. Closes the `retain`-on-insertion-ordered-types M1 gap
  (`OpenHashMap`/`OpenHashSet` already had it).
- **`LinkedHashMap::entry(key)`** — the standard `Entry` API (`or_insert`,
  `or_insert_with`, `or_insert_with_key`, `or_default`, `and_modify`, plus
  `Occupied`/`Vacant` with `key` / `get` / `get_mut` / `into_mut` / `insert` /
  `remove` / `remove_entry` / `into_key`), matching `OpenHashMap::entry`. Does
  the insert-or-update in a **single probe** instead of `contains_key` +
  `insert`. Filling a vacant entry appends in insertion order, exactly like
  `insert`; mutating an occupied entry keeps the key's position. Built on the
  `SlotList` + `IndexTable` kernel: an `OccupiedEntry` holds the entry's stable
  arena slot and key hash, so `remove_entry` locates the index cell by an
  exact-slot match without re-running user `Eq`. Purely additive.

## [Unreleased] — kernel consolidation (T9 / M4–M6 tails)

Internal follow-ups on `feat/rust-v3-hashbag-kernel` that fold the remaining
hand-rolled hash tables onto the crate's shared kernels. Public method surfaces
are unchanged.

- **`HashBag<T>` now stores its occurrence counts in the crate's own
  `OpenHashMap<T, usize>`** (open-addressing, niche-packed slots) instead of
  `std::collections::HashMap`, completing blueprint M5 / the T9 "HashBag on the
  kernel" leftover. No public API or observable-behavior change — `insert` /
  `remove_one` / `add_occurrences` / `occurrences_of` / `distinct_len` / `len` /
  `iter` (each element once per occurrence) / `bulk_load*` / the overflow-checked
  size accounting / count-based multiset `PartialEq` are all unchanged. Only the
  private backing field and `HashBagIter`'s inner iterator type changed.
- **`HashMapWithStrategy` / `HashSetWithStrategy` rebuilt on the shared kernel**
  (`SlotList` arena + `IndexTable`), deleting the ~250-line private Robin-Hood
  probe/resize/backward-shift each carried (blueprint M4/M5). The set is now a
  thin wrapper over `HashMapWithStrategy<T, ()>` (M6). All public methods are
  preserved. Two behavioral notes: (1) iteration is now **insertion-ordered**
  (was arbitrary table order — never a documented guarantee); (2) a panic inside
  a `HashingStrategy` closure can now only happen during the read-only probe,
  never mid backward-shift, since `IndexTable` re-derives ideal positions from
  stored hashes (the old `rehash_from` re-invoked `strategy.hash_code` while
  shifting).

## [Unreleased] — v3 Stage C (breaking, v1.0 cut)

Stage C of the v3 blueprint (`todo/fable-rust`, doc 14 §6): the breaking
removals and the comparator default-flip that change type identity.

### BREAKING-V3

- **`TreeMap` / `TreeSet` default comparator flipped to `Natural`.** The `C`
  type parameter now defaults to the zero-sized `Ord`-based `Natural` instead of
  the runtime `Comparator<K>`. This changes type identity: `TreeMap<K, V>` /
  `TreeSet<T>` now name the **natural-order** type.
  - `new()` / `TreeSet::new()` are now **no-arg** natural-order constructors.
    The `new(cmp)` dynamic constructors are **removed** → use
    `with_comparator(cmp)`, or the `DynTreeMap<K,V>` / `DynTreeSet<T>` aliases.
  - The bulk data pump (`from_sorted`, `TreeMapSink` / `TreeSetSink`, `create`)
    is anchored to the `DynTreeMap` / `DynTreeSet` form (it validates order with
    a runtime `Comparator`).
  - `sub_map` / `sub_set` are natural-order snapshots; comparator-correct slices
    are the job of the lazy `range(bounds)` iterator (T4).
- **Removed the `stream` module** (collectors + generators). Superseded by the
  blanket-impl `RichIterator` (T1) — build pipelines on any `.iter()`.
- **Removed `Vec`-returning range methods** superseded by the lazy `range()`
  iterator (T4):
  - `TreeMap`: `range_keys`, `range_entries`, `descending_keys`,
    `descending_entries`, `descending_range_keys`, `descending_range_entries`.
  - `TreeSet`: `range_elements`, `descending_range_elements`, `descending`.
  - Replace with `range(bounds)` (`.rev()` for descending). `sub_map` /
    `sub_set` / `remove_range` are retained.
- **Removed `keys_to_vec` / `values_to_vec`** on `HashMap` and `LinkedHashMap`
  → use the `keys()` / `values()` iterators + `.collect()`.
- **Deleted the generic trait towers** `crate::traits` (primitive:
  `PrimitiveCollection` / `PrimitiveList` / `PrimitiveSet` / `PrimitiveMap` +
  `Mutable*`) and `crate::object::{Collection, List, Set, Bag, Stack,
  MapIterable, MutableMap, Mutable*}`. Every method they carried is now an
  **inherent** method on the concrete type, so ordinary call sites are
  unchanged; only the `use mapdb_collections::object::{Collection, MutableMap,
  …}` imports go away (they no longer resolve).
  - Re-homed per type: `ArrayList` (full structural + functional set),
    `ArrayStack` (`len`/`is_empty`/`contains`/`iter`/`peek`/`push`/`pop`/`clear`),
    object `HashSet` (core + `any`/`all`/`none_satisfy`, `count_where`, `detect`,
    `select`, `reject`, `to_vec`), `HashBag` (core bag API + `insert`), object
    `HashMap` (core + `for_each`/`any`/`all`/`none_satisfy`), `LinkedHashSet`
    (functional set), `LinkedHashMap` (`for_each`/`any`/`all`/`none_satisfy`).
    `HashBiMap` already had every method inherent.
  - `iter()` now returns a **concrete** iterator (`slice::Iter`,
    `Rev<slice::Iter>`, `OpenHashSetIter`/`OpenHashMapIter`, `HashBagIter`,
    the `LinkedHash*` iterators) instead of the old `Box<dyn Iterator<…>>`.
  - Generic functional helpers that no type-checked caller used
    (e.g. `for_each`/`inject_into`/`select`/`reject` on `ArrayStack` and
    `HashBag`) are **not** re-homed — they are dropped, not moved. The trait
    tower is no longer a shared extension surface.

`ImmutableSortedMap` / `ImmutableSortedSet` **retain** their range/descending
methods: that frozen type has no lazy `range()` replacement, so those methods
are not superseded.

## [Unreleased] — v3 Stage B (arena kernel / T9)

Stage B of the v3 blueprint (`todo/fable-rust`, doc 14 §5). The
insertion-ordered collections are rebuilt on a shared arena kernel. **Public
API and behavior are preserved**; the changes are additive (new methods, a new
optional hasher type parameter) plus internal-representation replacement. No
removals.

### Added

- **`LinkedHashMap` / `LinkedHashSet` rebuilt on an intrusive slot arena.**
  Each entry is now stored **once** (in a `SlotList<(K, V)>` arena) and indexed
  by a key-owning-free open-addressing `IndexTable` — replacing the old
  `Vec` + `std::HashMap` double-storage. Consequences:
  - **`remove` is O(1)** (unlink + recycle a slot; no more O(n) index fix-up
    sweep), fixing the `02-P9` pitfall structurally.
  - **No `K: Clone` / `T: Clone`** bound on the core operations.
  - **`Borrow<Q>` lookups** — `get`/`get_mut`/`contains_key`/`remove` (map) and
    `contains`/`remove` (set) accept any borrowed form of the key, like `std`.
  - New **`get_mut`** on `LinkedHashMap`; new **`with_hasher` /
    `with_capacity_and_hasher`** on both.
  - A **hasher type parameter** `S = RandomState` (matching `OpenHashMap`),
    default unchanged. `LinkedHashSet<T>` is now a thin wrapper over
    `LinkedHashMap<T, ()>` (one implementation instead of two).
  - Named, unboxed iterators (`Iter`/`IterMut`/`IntoIter`) are re-exported from
    `object` as `LinkedHashMap{Iter,IterMut,IntoIter}` /
    `LinkedHashSet{Iter,IntoIter}`.
- Internal kernels `slot_list::SlotList<T>` and `index_table::IndexTable<S>`
  (crate-private). The arena stores values in `Option<T>` slots so a removed
  value's `Drop` runs **at removal time**, not when the slot is later reused
  (fixing `bounded_lru`'s retain-until-reuse for owning payloads). The index
  stores `(hash, slot)` inline, so Robin-Hood backward-shift deletion and resize
  never call a user `Hash`/`Eq` impl.

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
