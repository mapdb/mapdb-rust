# MERGE NOTE — `putCoalescing` was deleted on `main`

**This branch predates the deletion and still contains `putCoalescing`.**
Do not merge it without reading this.

## What changed on `main` (2026-07-31)

`RangeMap.put` now **coalesces** with connected neighbours holding an equal
value, and `putCoalescing` was **removed entirely** — from the API, from the
tests, and from the `validate` runner's op vocabulary. `RangeMap` is now
symmetric with `RangeSet`: both are always maximally merged, the set by cut
algebra alone, the map by cut algebra plus value equality. A different value is
a barrier.

The spec commit is `mapdb-collection-spec` `00cf2fa`
(`spec/features/range-set-map.md` §"Coalescing, and the `putCoalescing`
divergence"). Guava's `put`/`putCoalescing` split is a compatibility retrofit
(`RangeMap` is `@since 14.0`, `putCoalescing` `@since 22.0`); we have no such
constraint and took the simpler design.

This did not *repair* the direction-dependent coalescing defect — it removed the
conditions the defect needs. With `put` coalescing, a chain of abutting
equal-valued entries is unrepresentable, so at most **one** entry per side is
ever absorbable and there is no direction to get wrong.

## What this branch still has

The **old, pre-fix** `putCoalescing`: a single ascending pass that grows the
merged range while iterating and commits each entry's keep/absorb decision at
visit time. It absorbs a chain on the right transitively and one on the left
exactly one entry deep. It is wrong under every candidate contract. It is also
attached to a `put` that does not coalesce at all.

## Merging this branch

Take `main`'s side for everything under the range map:

1. Keep `main`'s coalescing `put` (bounded one-per-side merge at the insertion
   position — not a scan, not an outward walk, not a fixpoint).
2. Delete `putCoalescing` from the API, the tests, and the `validate` runner's
   op table.
3. Any test on this branch asserting that `put` does **not** coalesce now
   asserts the opposite. If a merged test still expects two entries for
   `put([1,5),A); put([5,9),A)`, the merge went the wrong way.
4. Re-run conformance against spec `main` — 298 scenarios, `20-range-set-map`
   must be 35/35 — with the validator **rebuilt from the merge commit**, never a
   prebuilt binary.

The scenario that catches a bad merge is `rangemap_put_coalesces.json`: it
asserts `put([1,5),100); put([5,9),100)` leaves **one** entry. It is the
inverted descendant of the deleted `rangemap_put_no_coalesce.json`, which
asserted exactly two — so if both files appear after a scenario merge, the
corpus has been made self-contradictory.

## Delete this file as part of the reconciliation

Full checklist: `todo/putcoalescing/07-BRANCH-RECONCILIATION.md`.
Ruling and rationale: `todo/putcoalescing/06-DECISION.md`.
