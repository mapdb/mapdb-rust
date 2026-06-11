# BREAKING-V2 — deferred idiom changes (do NOT implement in v1)

These are source-breaking idiom changes batched for a future v2. They were
deliberately **not** made during the additive Phase 7c idiom pass because each
removes or alters an existing public API. One line each, with rationale.

## Java-ism removals
- **Remove `size()` (use `len()`)** — `size()` is a Java-ism duplicated by the new
  idiomatic `len()`; removing it forces the std-conventional name. Affects
  `Multimap`, `SetMultimap`, `Interval`, `PriorityQueue`, `HashBag`/`Bag::size_distinct`.
- **`add` → `push`/`insert`** — `MutableSet::add`/`MutableBag::add` should become
  `insert` (sets) to match `std`; stack/list `add`-style helpers should be `push`.
- **Remove `of()` constructors (use `FromIterator`/`from_iter`/`collect`)** — `of()`
  is an Eclipse/Java idiom now fully covered by the `FromIterator` impls added in v1;
  keeping both is redundant.
- **`put` → `insert`** — `HashBiMap::put` and the multimaps' `put` should be `insert`
  to match the std map vocabulary (`MutableMap::insert` already uses it).

## Generic hasher parameter
- **`BuildHasher` / `S = RandomState` type parameter** — adding a hasher type param
  (`HashMap<K, V, S = RandomState>`) to the open-addressing maps/sets changes every
  type signature and every `::new()` call site; defer to v2 where the whole public
  surface can absorb the extra generic at once. Enables FxHash/AHash opt-in.

## Layout / internals
- **`MapEntry` layout change for cache locality** — switching from
  `{ occupied: bool, key: Option<K>, value: Option<V> }` to a struct-of-arrays or a
  niche-packed tombstone representation changes the iterator/`IntoIter` internals and
  any code reaching into `entries`; measurable cache win but ABI/layout breaking.

## Concurrency
- **`Synchronized<C>` redesign (let callers lock)** — the current wrapper exposes
  `with`/`with_mut` closure-scoped access plus a raw `lock()`. A v2 redesign would make
  the guard the primary API (`Deref`/`DerefMut`, drop-`Sync` ergonomics) and likely drop
  the `with`/`with_mut` closure methods, which is source-breaking for existing callers.
