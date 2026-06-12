# BREAKING-V2 — idiom changes (DONE, shipped in 0.2.0)

These source-breaking idiom changes were batched out of the additive v1 idiom
pass because each removes or alters an existing public API. They are now
**implemented** and released in `0.2.0`. One line each, with rationale. The
consumer migration table lives in [`CHANGELOG.md`](CHANGELOG.md).

## Java-ism removals — DONE
- **Removed `size()` (use `len()`).** The `size()` Java-ism duplicated the
  idiomatic `len()`; it is gone from `Multimap`, `SetMultimap`, `Interval`,
  `PriorityQueue`, and the `BatchIterable` trait. The distinct-count method is
  now `distinct_len()` (was `size_distinct()`) on `HashBag`/`Bag` and the
  multimaps.
- **`add` → `insert` / `push`.** `MutableSet::add` / `MutableBag::add` and every
  concrete set/bag (`OpenHashSet`, object `HashSet`, `LinkedHashSet`, `TreeSet`,
  `HashSetWithStrategy`, `HashBag`) now use `insert` to match `std`. Stack/list
  growth already used `push`.
- **Removed `of()` constructors (use `FromIterator` / `from_iter` / `collect`).**
  `of()` was an Eclipse/Java idiom fully covered by the `FromIterator` impls
  added in v1. The `PriorityQueue` O(n) Floyd heapify now lives in its
  `FromIterator` impl.
- **`put` → `insert`.** `HashBiMap::put` and both multimaps' `put` now use the
  std map vocabulary `insert` (the `MutableMap` trait already did).

## Generic hasher parameter — DONE
- **`S: BuildHasher = RandomState` type parameter.** `OpenHashMap<K, V, S =
  RandomState>` and `OpenHashSet<K, S = RandomState>` carry a hasher type
  parameter. `::new()` / `::with_capacity()` default to `RandomState` (HashDoS
  resistance, matching `std`), so ordinary call sites still compile via the
  default. New `with_hasher` / `with_capacity_and_hasher` constructors plus a
  `hasher()` accessor enable FxHash/AHash opt-in.

## Layout / internals — DONE
- **`MapEntry` repacked for cache locality.** The probe array is a `Vec<Slot>`
  with a two-variant `Slot` enum (`Empty` / `Occupied { key, value }`). The
  occupancy flag is the discriminant (no `bool`) and key/value are inline (no
  `Option`), removing ~40 invariant `unwrap()`s and shrinking the probe
  footprint. Backward-shift deletion keeps the table tombstone-free, so two
  variants suffice. Internal/ABI change (touches iterator/`IntoIter` internals);
  no public method signature changed.

## Concurrency — DONE
- **`Synchronized<C>` redesigned around a guard.** `lock()` returns a
  `SyncGuard<'_, C>` implementing `Deref`/`DerefMut` (standard Rust `Mutex`
  ergonomics); the `with` / `with_mut` closure methods were dropped. The type is
  kept (lower-risk than dropping it for a bare `Mutex<C>`); `Synchronized::inner()`
  exposes the underlying `Arc<Mutex<C>>` for `try_lock` / `PoisonError` recovery.

## Signature relaxations shipped in v1 (minor, recorded for completeness)
These were applied additively in the v1 idiom pass because they follow `std`
exactly and keep ordinary call sites compiling, but they DO alter public
signatures, so they are recorded here (they were never deferred to v2):
- **`Borrow<Q>` on `get`/`contains_key`/`remove`** — generalized from `&K` to
  `&Q where K: Borrow<Q>, Q: Hash + Eq + ?Sized` on `OpenHashMap`/`OpenHashSet`,
  object `HashMap`/`HashSet`, and `HashBiMap` (fwd + inverse). `K: Borrow<K>`
  keeps `map.get(&k)` compiling; rare inference caveat at turbofish / `.into()`
  call sites.
- **Inherent `iter()` vs the `MapIterable::iter` trait method** — the inherent
  `iter()` on the object maps returns a concrete iterator and shadows the trait
  method. `for` loops and unannotated calls are unaffected; an explicit
  `Box<dyn Iterator<…>>` annotation must call the trait method by path.
