# Review brief — mapdb-rust conformance runner: std oracles → production types (todo/iso2 E17)

You are reviewing a single commit in the Rust port of MapDB collections.
Repo: `/home/play2/mapdb/mapdb-rust` (branch `main`, commit `4c8b2b7`, local only, not pushed).
Full diff: `/tmp/iso2-e17.diff` (also `git -C /home/play2/mapdb/mapdb-rust show 4c8b2b7`).
Scenario corpus: `/home/play2/mapdb/mapdb-collection-spec/cross-language-validation/scenarios`
Harness: `/home/play2/mapdb/mapdb-collection-spec/cross-language-validation/validate.sh`

## Background / the defect

`src/bin/validate.rs` is the Rust runner for a 298-scenario cross-language
conformance suite shared by five ports (Java, Go, Rust, Zig, TypeScript). A
review (`/home/play2/mapdb/todo/iso2/fable-review.md`, item E17 and section 2)
found that three runners drove the Rust **standard library** rather than the
mapdb production collections, so 57 of the 298 scenarios (19%) executed no
mapdb code at all and were green for free:

- `run_treeset` used `std::collections::BTreeSet<i32>` plus runner-local
  `set_rank` / `set_select` / `set_nav` oracle helpers (18 scenarios).
- `run_treemap` used `std::collections::BTreeMap<i32,i32>`; its `fromSorted`
  branch built the production `object::TreeMap::from_sorted` tree and then
  **copied the pairs into the std map**, so the bulk builder's subtree-size
  augmentation (which `rank`/`select` assertions exist to test) was never
  exercised (21 scenarios).
- `run_arraylist` used `Vec<i32>` with iterator loops (18 scenarios).

The review also names a discipline rule (its "G5"): *a runner MUST obtain every
assertion value by calling the production method the assertion names; a
runner-local loop over the collection's contents is a bug even when the
comparator/element type is production.*

The f32 runners (`run_f32_treeset`, `run_f32_arraylist`) were already correct
and served as the model.

## What the commit does

1. `run_treeset` → production `mapdb_collections::object::TreeSet<i32>` with the
   `Natural` comparator (i32 `Ord`). Assertions call `floor` / `ceiling` /
   `lower` / `higher` / `rank` / `select` / `first` / `last` / `min` / `max` /
   `contains` / `len` / `is_empty`; ops call `insert` / `remove` / `clear` /
   `poll_first` / `poll_last` / `remove_range(Range<i32>)`. Sorted output is
   `set.iter()` (the tree's in-order traversal); descending output is
   `set.range(..).rev()` (the production double-ended range iterator); the
   `range_elements` / `range_elements_desc` / `range_size` assertions read a
   production `set.sub_set(range)` snapshot. `set_rank`/`set_select`/`set_nav`
   deleted.
2. `run_treemap` → production `object::TreeMap`. `fromSorted` keeps the pumped
   tree (no copy). Assertion body factored into `emit_treemap_assertions`,
   generic over the comparator `C: Compare<i32>`, so the pumped tree (which has
   a runtime `Comparator<i32>` because `from_sorted` lives on that form) and the
   incrementally built natural map share one body. Uses `floor_entry` /
   `ceiling_entry` / `lower_entry` / `higher_entry` / `rank` / `select_key` /
   `first_key` / `last_key` / `min` / `max` / `keys` / `values` /
   `range(..).rev()` / `get` / `contains_key`, and ops use
   `poll_first_entry` / `poll_last_entry` / `remove_range`. Range assertions
   read a production `sub_map(range)` snapshot. `map_nav` deleted.
3. `run_arraylist` → production `object::ArrayList<i32>`: `select` / `reject` /
   `detect` / `count_where` / `any_satisfy` / `all_satisfy` / `none_satisfy` /
   `inject_into` / `sort` / `contains` / `get` / `insert` / `remove` / `push`.
4. `use std::collections::{BTreeMap, BTreeSet};` removed; the module doc that
   claimed the runner routes through "Vec, BTreeMap, BTreeSet" rewritten.
5. Guard: new root `clippy.toml` with `disallowed-types` for
   `std::collections::{BTreeSet,BTreeMap,HashMap,HashSet,VecDeque,BinaryHeap}`
   (each with a reason string naming the conformance rule);
   `#![deny(clippy::disallowed_types)]` at the top of `src/bin/validate.rs`;
   `#![allow(clippy::disallowed_types)]` crate-wide in `src/lib.rs` because the
   library legitimately uses std containers internally (49 call sites) and
   `clippy.toml` is repo-global. CI's existing clippy step is
   `cargo clippy --all-targets -- -D warnings`, which does **not** compile
   `validate.rs` (it is behind `required-features = ["validation"]`), so the
   commit adds a `cargo clippy --features validation --bins -- -D warnings`
   step to `.github/workflows/ci.yml`.

## Notable design choices you should challenge if you disagree

- **Natural vs Dyn comparator.** The task text suggested `DynTreeSet<i32>` via
  `TreeSet::with_comparator(natural_comparator::<i32>())`. I used the
  `Natural`-comparator form `TreeSet::<i32>::new()` instead, because
  `remove_range(Range<T>)` and `sub_set(Range<T>)` are only implemented on
  `impl<T: Ord + Copy> TreeSet<T>` (the natural form) — `src/object/treeset.rs`
  around line 387 — and those are exactly the production methods the
  `remove_range` / `range_*` scenarios name. With `DynTreeSet` I would have had
  to rebuild range membership in the runner, which is the very thing E17/G5
  forbids. Same reasoning for `TreeMap::sub_map` / `TreeMap::remove_range`
  (`impl<K: Ord + Copy, V> TreeMap<K, V>`, around line 690 of
  `src/object/treemap.rs`). Is that the right trade?
- **min/max on `ArrayList<i32>`.** `object::ArrayList` has no `min()`/`max()`
  method. I sort a copy with the production `ArrayList::sort` and take
  `first()`/`last()`, which is exactly what the pre-existing (and previously
  reviewed) `run_f32_arraylist` does. Is that acceptable under G5, or should the
  library grow `min()`/`max()`?
- **`select_gt_N` / `reject_gt_N` sorting.** The production `select`/`reject`
  return a `Vec<i32>` in list order; the scenarios expect sorted output, so the
  result is re-wrapped in an `ArrayList` and sorted with the production `sort`.
- **`sub_set` / `sub_map` are internally filter loops.** They are production
  code in the library (so a bug there would now show up), but if you think
  calling `set.range(bounds)` with a `Range<i32>`→`RangeBounds` conversion would
  be a stronger test, say so. Note `Range<T>` does not implement `RangeBounds<T>`
  in this crate.

## Verification performed (all measured, not assumed)

- `cargo build --release --bin validate --features validation` — clean.
- `cargo test` — 811 + 4 doctests, 0 failures.
- `cargo fmt --all --check` — clean (after reformat).
- `cargo clippy --features validation --bins -- -D warnings` — clean **except**
  a pre-existing `clippy::manual_slice_fill` at `src/index_table.rs:138`, which
  is library code untouched by this commit and is a lint from a newer local
  clippy (1.98) than CI pins; it fails on the unmodified parent commit too.
- Guard proof: temporarily inserting
  `fn guard_probe() -> usize { let s: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new(); s.len() }`
  into `src/bin/validate.rs` makes clippy emit
  `error: use of a disallowed type 'std::collections::BTreeSet'` pointing at the
  `#![deny(clippy::disallowed_types)]` line. Reverted afterwards.
- Conformance suite, Rust only
  (`./validate.sh --skip-go --skip-zig --skip-ts --skip-java`):
  **before (stashed working tree, parent commit): 298/298 pass, 0 fail.
  after: 298/298 pass, 0 fail.**
- Extra check because "all green" is exactly what a silently-skipped assertion
  also looks like: for each of the 57 affected scenario files I ran the new
  binary and verified that **every** non-`comment` assertion key appears in the
  emitted output (the runner's `emit` silently skips keys whose evaluator
  returns `UNKNOWN_ASSERTION:`). No key was missing. Script and result are in
  the session transcript; you can re-run it.

So: no scenario went red. The review expected reds ("expect it to turn scenarios
red for real"); it did not happen, i.e. the production TreeSet/TreeMap/ArrayList
agree with the std oracles they replaced on all 57 scenarios. I classify zero
production bugs and zero runner mistakes. **Please try to falsify that.**

## What I want from you

Be adversarial and concrete. Cite file:line.

1. **Did any runner still compute an assertion value locally instead of calling
   the production method the assertion names?** Read the whole of the new
   `run_arraylist`, `eval_list_assertion`, `sorted_copy`, `run_treeset`,
   `run_treemap`, `emit_treemap_assertions` in
   `/home/play2/mapdb/mapdb-rust/src/bin/validate.rs`. Flag every remaining
   runner-local loop/closure that shadows a production method that exists. (Note
   predicates passed *into* `select`/`count_where`/`any_satisfy` are the
   scenario's predicate, not an oracle — those are fine.)
2. **Is the clippy guard real?** Would a reintroduced `BTreeSet` in the bin fail
   *CI* (not just my laptop)? Check `clippy.toml`, the `deny` in the bin, the
   `allow` in `src/lib.rs`, `Cargo.toml`'s `required-features`, and
   `.github/workflows/ci.yml`. Is the crate-level `allow` in `lib.rs` too broad
   — could it mask a future std oracle that someone puts in library code, or in
   `tests/`, or in `src/bin/nanprobe.rs` (which is NOT behind the validation
   feature)? Suggest a tighter scoping if one exists.
3. **Is "no production bug, no runner mistake" correct?** Specifically look for
   behaviour differences between the old std oracle and the new production call
   that the 57 scenarios happen not to cover — e.g.:
   - `Vec::remove(position of value)` vs `ArrayList::remove(&value)` semantics;
   - `rank` at `i32::MIN` / `i32::MAX` and `select` past the end;
   - `poll_first`/`poll_last` on an empty collection (must yield `null`, not panic);
   - `remove_range` counting on open ranges over integers
     (`open(1,2)` matches nothing but is a valid range);
   - the `fromSorted` path: `DuplicatePolicy::Error`, and whether the generic
     `emit_treemap_assertions` silently returns `UNKNOWN_ASSERTION` for a key
     that the natural-map path would have answered (`range_keys` etc. are only
     answered when a `query` block exists — is that the same as before?);
   - i32 overflow assertions (`sum` widens to i64, `inject_into_sum` wraps at
     i32, `product` wraps) — confirm the production `inject_into` fold preserves
     that.
   If you find a real divergence, say whether it is a mapdb production bug or a
   runner mistake, and give the scenario that would catch it (or note that no
   scenario covers it).
4. Anything else in the diff that is wrong, misleading, or a maintenance trap —
   including the commit message and the doc comments (they must not overclaim).

Write your answer to `/tmp/iso2-e17-review.md`. Structure it as a numbered list
of findings, each tagged MUST-FIX / SHOULD-FIX / NIT / OK, with file:line and a
concrete suggested change. If you believe the work is correct, say so plainly
rather than inventing findings.
