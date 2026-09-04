# Review of `4c8b2b7`

1. **MUST-FIX — `fromSorted` silently drops all range assertions.** `src/bin/validate.rs:2104-2115` calls `emit_treemap_assertions(..., None, ...)` and returns before the normal path builds the query at `src/bin/validate.rs:2150-2156`. Consequently the known `range_keys`, `range_keys_desc`, and `range_size` arms at `src/bin/validate.rs:2186-2199` turn into `UNKNOWN_ASSERTION` and `emit` silently suppresses them (`src/bin/validate.rs:51-58`). This is a regression from the parent runner, which built `query` after both construction paths. The current bulk scenario (`cross-language-validation/scenarios/17-bulk-load/treemap_i32_from_sorted.json:1-30`) happens not to carry a query, so 298/298 does not expose it. I confirmed it with a synthetic `fromSorted` scenario containing a closed `[1,2]` query and assertions `range_keys: [1,2]`, `range_keys_desc: [2,1]`, `range_size: 2`: the runner printed only `size`. This is a **runner mistake**, not a production bug. Pass the query/bounds into `emit_treemap_assertions` for both construction paths and answer these assertions from `map.range(bounds)`; a `(std::ops::Bound<i32>, std::ops::Bound<i32>)` implements `RangeBounds`, so a small conversion from the production `Range`'s public endpoint/bound-type accessors avoids any runner-side membership loop and works for both `Natural` and runtime comparators. `range(...).len()` also makes `range_size` exercise the tree's augmented range implementation. Add a `fromSorted` + `query` regression scenario.

2. **MUST-FIX — the new CI guard cannot currently reach its validation step on the configured toolchain, and the commit message's “CI pins” claim is false.** `.github/workflows/ci.yml:27` uses the floating `dtolnay/rust-toolchain@stable`; it does not pin an older compiler. As of this review, stable is 1.98.1, and the brief already records that Clippy 1.98 reports `manual_slice_fill` at `src/index_table.rs:138-140`. Because the existing step at `.github/workflows/ci.yml:40-41` runs with `-D warnings`, CI fails there before the added validation step at `.github/workflows/ci.yml:51-52`; the latter would fail on the same library lint as well. That the parent also fails does not make the new guard operational or the commit-message explanation accurate. Prefer the straightforward one-line compatible fix, `self.slots.fill(IdxSlot::Empty);`, then run both exact CI commands. Alternatively pin a toolchain deliberately and document it, but the current workflow is not pinned.

3. **SHOULD-FIX — the Clippy guard is real for direct uses in `validate`, but its comments overstate and misdescribe its scope.** `Cargo.toml:27-30`, `src/bin/validate.rs:7-9`, and `.github/workflows/ci.yml:51-52` together ensure that a reintroduced direct `BTreeSet`/`BTreeMap` use in `validate.rs` is compiled with `clippy::disallowed_types` denied, so that specific guard is sound. The crate-wide allow at `src/lib.rs:52-56`, however, masks every listed type throughout production library modules and their inline unit-test modules. It does **not** mask `src/bin/nanprobe.rs` or integration-test crates: those are separate crate targets, and the configured lint is warn-by-default, so `.github/workflows/ci.yml:40-41` upgrades their violations to errors. Thus `clippy.toml:6-8` (“only the binary ... is bound”) is inaccurate. Tighter scoping exists: remove the crate-level allow and place `#[allow(clippy::disallowed_types)]` only on the small set of modules/items with intentional implementation use (and separately on test-oracle modules where desired). That would catch a future oracle helper smuggled into unrelated library code. Also narrow `src/bin/validate.rs:16-21`: the list bans six selected collection types, not all standard-library containers, and necessarily does not ban `Vec`, so it cannot prevent a return of the original `ArrayList`-on-`Vec` defect or enforce G5 generally.

4. **SHOULD-FIX — the code no longer shadows an existing production method in the reviewed functions, but the G5/documentation claim is literally too strong.** In `run_arraylist`/`eval_list_assertion` (`src/bin/validate.rs:1601-1733`), predicates are correctly passed into production `select`, `reject`, `detect`, `count_where`, `any_satisfy`, `all_satisfy`, and `none_satisfy`; reducers are correctly passed into production `inject_into`; lookup/mutation calls are production calls. `sorted_copy` at `src/bin/validate.rs:1636-1642` uses iteration only to copy and production `ArrayList::sort` to establish order; it is not a local comparison oracle. Likewise, the reviewed tree runners have no remaining local navigation/rank/select/filter loop shadowing an available production method. However, `min`, `max`, `sum`, `product`, and `max_minus_min` do not call methods with those names: they are compositions over `sort`/`first`/`last` or `inject_into` (`src/bin/validate.rs:1649-1680`), because `ArrayList` exposes no such methods (`src/object/arraylist.rs:136-250`). Therefore “each assertion value [comes] from the production method the assertion names” at `src/bin/validate.rs:16-19` and “Every assertion ... [uses] the list method the assertion names” at `src/bin/validate.rs:1606-1609` are false under G5's literal wording. Either (a) add production `ArrayList::min`/`max` (and settle which aggregate methods are actually part of the Rust surface) and call them, or (b) explicitly codify a narrow adapter exception for assertion vocabulary that has no corresponding Rust method, requiring composition solely from production operations. For this commit I do not classify the current sorted-copy implementation as a runtime runner bug—the absent method cannot be falsely tested—but the policy and comments must stop claiming exact name-to-method correspondence. The same overclaim already exists in the f32 runner at `src/bin/validate.rs:2423-2426,2450-2461`; precedent does not make it satisfy literal G5.

5. **SHOULD-FIX — add scenarios for newly wired mutation branches that the 57 scenarios do not exercise.** No `ArrayList<i32>` scenario invokes `remove`, so the switch at `src/bin/validate.rs:1619-1622` is unprotected. The old loop and production `ArrayList::remove` have the same first-occurrence semantics (`src/object/arraylist.rs:237-245`); a scenario `add 1, add 2, add 1, remove 1` asserting `[2,1]` would pin that fact. Likewise, `treemap_range_open_no_integer.json:4-13` and its TreeSet counterpart test open-range **queries**, while the shared `remove_range` scenarios only use `closed_open`; add open `(1,2)` mutation scenarios asserting count `0` and unchanged contents for `src/bin/validate.rs:1976-1978,2142-2144`. I ran both shapes through the new runner and found no divergence. Production TreeMap already has a native check at `src/object/treemap.rs:2030-2043`, so this is shared-runner coverage debt, not a discovered production bug.

6. **OK — the remaining highlighted semantic choices and covered edge behavior are correct.** Natural-comparator `TreeSet<i32>`/`TreeMap<i32,i32>` is the right trade for the production `sub_set`/`sub_map` and `remove_range` APIs, whose natural-order-only impls are at `src/object/treeset.rs:387-412` and `src/object/treemap.rs:690-737`. The range snapshot's internal filtering is production code, so it does not violate the runner-local-oracle rule, although direct `range` is stronger and fixes finding 1. Signed ranks and out-of-range select are covered by `treemap_rank_select_signed.json:5-18` and the analogous set scenario, matching production checks at `src/object/treemap.rs:2100-2108` and `src/object/treeset.rs:872-880`. Empty first/last polls and empty `remove_range` are covered by `treeset_nav_empty.json:4-21` and its map analogue. `DuplicatePolicy::Error` at `src/bin/validate.rs:2108-2113` matches the strict bulk-input contract and native duplicate test at `src/object/treemap.rs:2316-2331`. Finally, `sum` widens through an `i64` production fold while `inject_into_sum` and both product spellings explicitly wrap in `i32` (`src/bin/validate.rs:1649-1658,1672-1680`), matching the overflow scenarios. I found **no production collection bug** in these paths. The blanket conclusion should be revised to: no production bug found, but one uncovered runner regression exists (finding 1).

Overall: the conversion is substantially correct and removes the original 57-scenario std-oracle bypass. I would not land it unchanged because the bulk-map range regression is a real silent-skip bug and the stated CI configuration is presently red; the other items are scope/maintenance hardening rather than evidence of a production collection defect.

---

# Round 2 (HashBag + HashSet set-algebra oracles)

## Brief

# Review brief (round 2, short) — mapdb-rust validator: HashBag + HashSet set-algebra oracles

Follow-up to your review of `4c8b2b7`/`778aff6` (archived at
`/home/play2/mapdb/mapdb-rust/reviews/iso2-e17-codex.md`). The coordinator asked
for two same-class leftovers in `/home/play2/mapdb/mapdb-rust/src/bin/validate.rs`
to be fixed. This round reviews ONLY the uncommitted working-tree diff:
`/tmp/iso2-e17b.diff` (or `git -C /home/play2/mapdb/mapdb-rust diff`).

## What changed

1. **`run_hashbag` drove a hand-rolled bag.** It was
   `OpenHashMap<i32, usize>` plus a runner-maintained `total: usize` counter,
   with add/remove occurrence bookkeeping written in the runner — so `size`,
   `size_distinct`, `occurrences_*`, `is_empty` tested the runner's arithmetic,
   not `mapdb_collections::object::HashBag`. Now it drives production `HashBag`:
   `insert` / `add_occurrences` / `remove_one` / `clear` for ops, and
   `len` / `distinct_len` / `is_empty` / `occurrences_of` / `contains` /
   `for_each_with_occurrences` / `iter` for assertions. `eval_bag_assertion` lost
   its `total` parameter.
2. **`eval_set_assertion` computed set algebra locally.** `union_sorted`,
   `intersect_sorted`, `difference_sorted`, `symmetric_difference_sorted` and
   the four `*_size` keys were runner-local `iter().chain()/filter()/dedup()`
   chains, even though the production kernel has
   `OpenHashSet::union/intersection/difference/symmetric_difference`
   (`src/hash_table.rs:1032,1046,1067,1082`). They now call those methods; a new
   `sorted_render()` helper does the ascending render only.
   I deliberately kept the runner on `OpenHashSet` rather than switching to
   `object::HashSet<i32>`: `object::HashSet` is a thin newtype that delegates
   every one of these to exactly the same kernel methods
   (`src/object/hashset.rs:131-155`), and the spec repo's new G1 manifest names
   `OpenHashSet` as the required production symbol for `HashSet<i32>`.
   Is that the right call, or should the runner use the `object::` wrapper?

## Deliberately left (please challenge)

- Sorting the output of an unordered production collection (hash map/set/bag/
  multimap `sorted_keys` / `sorted_values` / `to_sorted_array` /
  `sorted_distinct`, and the new `sorted_render`). These types have no order, so
  I treat the sort as presentation, not computation.
- `eval_map_assertion`'s `"min"` / `"max"` over `OpenHashMap<i32,i32>` keys
  (`src/bin/validate.rs:1409-1422`): computed by sorting the key render. Neither
  `OpenHashMap` nor `object::HashMap` has `min`/`max` (I grepped). Same shape as
  the list `min`/`max` you already assessed as "not a runtime runner bug".
- Everything else I scanned: the remaining `.filter(`/`.any(`/`.count()` chains
  in the file are JSON-operation parsing or assertion-key grammar, and
  `eval_roaring_assertion` / `run_f32_treeset` `min`/`max` already call
  production `min()`/`max()`.

## Verification

- `cargo build --release --bin validate --features validation`, `cargo test`
  (811 + 4 doctests green), `cargo fmt --all --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo clippy --features validation --bins -- -D warnings` — all clean.
- Conformance, rust only: **298/298 pass, 0 fail**, unchanged.
- The spec repo's (still uncommitted) G1 runner-symbol check now reports
  **PASS rust: 26 checked** — it previously failed with
  `FORBIDDEN oracle in run_hashbag: OpenHashMap hand-rolled as a bag`.
  No `--skip-runner-check` needed any more.
- Only one HashBag scenario exists
  (`01-basic-crud/hashbag_add_occurrences.json`, `add` ops only), so the
  `add_occurrences` / `remove_one` / `clear` branches are unexercised by the
  suite. I ran ad-hoc scenarios through the new runner for `remove` down to zero
  (element disappears from `sorted_distinct`, `size` drops, `contains` false)
  and for `clear`.

## Questions

1. Any remaining runner-local computation in the changed code that shadows a
   production method that exists? Cite file:line.
2. Is the `OpenHashSet`-vs-`object::HashSet` choice defensible?
3. Any behaviour divergence between the old hand-rolled bag and production
   `HashBag` that the single shipped scenario would not catch — in particular
   `remove_one` on an absent element, removing the last occurrence, and the
   overflow-checked `insert`? Is `bag.iter()` (yields each element once per
   occurrence) the right source for `to_sorted_array`, and
   `for_each_with_occurrences` the right source for `sorted_distinct`?
4. Anything wrong or overclaiming in the new comments.

Write your answer to `/tmp/iso2-e17b-review.md`: a short numbered list, each item
tagged MUST-FIX / SHOULD-FIX / NIT / OK with file:line. Say plainly if it is
correct.

## Codex answer

1. **OK — `src/bin/validate.rs:1807-1824`.** The changed set assertions now obtain every algebra result and size from `OpenHashSet::{union,intersection,difference,symmetric_difference}`. `sorted_render` at `src/bin/validate.rs:1789-1797` only copies and orders an already-produced unordered result for the wire format; it does not reimplement membership. I found no remaining runner-local computation in the changed code that shadows an available production method.

2. **OK — `src/bin/validate.rs:1756,1769-1778,1813-1824`.** Keeping `OpenHashSet<i32>` is defensible and is the better match for the current G1 manifest, which explicitly requires that symbol. The `object::HashSet` methods at `src/object/hashset.rs:130-155` are direct wrappers over the same kernel methods, so switching would add wrapper coverage but would not exercise different set-algebra logic.

3. **OK — `src/bin/validate.rs:1842-1895`.** The `HashBag` conversion has the right semantics and traversal sources. `remove_one` is a no-op returning `false` when absent and removes the distinct entry at its last occurrence (`src/object/hashbag.rs:187-197`), matching the old runner's observable state. `iter()` yields once per occurrence (`src/object/hashbag.rs:90-95,236-249`), so it is correct for `to_sorted_array`; `for_each_with_occurrences` visits each distinct map entry once (`src/object/hashbag.rs:200-203`), so it is correct for `sorted_distinct`. The only normal-path expansion is that `add_occurrences` is now accepted. At the overflow boundary, production `insert`/`add_occurrences` deliberately panic via checked addition (`src/object/hashbag.rs:141-161,165-184`), whereas the old release runner's unchecked arithmetic could wrap; that divergence is desirable because the validator must expose production behavior.

4. **SHOULD-FIX — `src/bin/validate.rs:1845-1854`.** Add conformance coverage for the newly wired/uncovered bag branches: `add_occurrences` (including zero if it remains runner vocabulary), removal from multiplicity two through the last occurrence, removal when absent, and `clear`. The sole shipped scenario uses only `add`, so 298/298 cannot detect a miswire in any of these branches. This is coverage debt, not a defect found in the implementation.

5. **NIT — `src/bin/validate.rs:1838`.** “The same std/oracle-bypass defect” is slightly inaccurate for the old bag runner: its backing was the project's `OpenHashMap`, not a `std` collection. “The same runner-local/oracle-bypass defect” would be exact. The remaining new comments, including the characterization of sorting as presentation, are accurate and not overclaiming.

Overall: **the patch is correct, with no MUST-FIX finding and no production collection bug found.**
