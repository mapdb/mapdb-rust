# Astra review: retain panic safety, round 1

**Verdict: SHIP.** No blocking correctness findings in the reviewed uncommitted changes on `codex/fix-retain-panic`. Scope: `src/hash_table.rs` and retain documentation in `src/object/hashmap.rs` / `src/object/hashset.rs`. The original dirty checkout was not modified.

## Correctness

- **Predicate and hashing unwind are safe.** Map lines 409–416 and set lines 1008–1015 collect only indexes and hashes while the original entries and size stay installed. A callback or survivor hash panic drops this scalar scratch buffer without removing any entries. Map value mutations already performed survive. Hashing is completed before structural mutation, including user-defined BuildHasher/Hasher execution. This guarantee assumes the ordinary hash-table contract that keys' hash/equality behavior remains stable while stored; interior mutation of keys or stateful hash behavior can invalidate any such table.
- **The move phase has no user callbacks.** Map lines 418–428 and set lines 1017–1027 allocate and initialize replacement storage first, then move each survivor exactly once using `insert_hashed_no_resize` (lines 494 and 1183). The helper neither hashes nor compares keys. Replacing an Empty destination cannot run a key/value destructor. Original unique indexes remain valid, capacity is unchanged and nonzero, and the original load bound leaves an empty probe slot. Thus indexing, probe termination, and size increments are valid for a valid input table. No Clone bound or unsafe code is introduced.
- **Destructor unwind happens after commit.** Rejected keys/values remain in `old` until every survivor has been installed and counted (map lines 429–431; set line 1028). A single rejected destructor panic leaves a complete, reachable survivor table. This covers set keys and map keys as well as map values by the same ownership argument. As usual, a second destructor panic during unwinding may abort; this is not a recoverable guarantee.
- **Collision behavior is preserved.** Cached hashes use the same hasher and unchanged capacity mask; insertion uses the original linear probe algorithm and increments size once per survivor. The operation does not need duplicate detection because it moves an already unique population. Backward-shift deletion remains compatible with the rebuilt table.

## Cost and documentation

The stated O(capacity) extra space and O(capacity + probe work) structural cost are accurate, with callback/hash execution costs additional. Worst-case collision probing remains quadratic, as in the previous rebuild. The new scratch allocation reserves `len` pairs of `(usize, u64)`, even when every predicate returns false: typically an additional 16 × original len bytes on a 64-bit target, on top of the old and replacement slot arrays. It also adds a decision scan and survivor-index reads; peak ownership of rejected values lasts until commit. These are concrete constant-factor costs of the stronger guarantee, not asymptotic regressions. No benchmark was run, so this review makes no throughput claim.

The wrapper documentation correctly delegates the detailed contract to the kernel. The map's direct `hasher.hash_one(key)` is equivalent to its hash helper and avoids borrowing all of self during mutable entry iteration.

## Nonblocking coverage suggestion

**P3 — extend destructor coverage when maintaining this area.** `src/hash_table.rs:2391` exercises only a removed map value, with one rejected entry. Add a set-key destructor case and multiple rejected entries with drop counters to pin both post-panic survivor reachability and cleanup of remaining rejected objects. The current implementation is correct by inspection; this is defense against later regressions, not a shipping gate.

The added predicate tests (`src/hash_table.rs:2268`, `2303`) are meaningful: deterministic collisions force a cluster, false decisions precede the panic, every entry is checked afterward, and a removal checks continued cluster reachability. The map also checks retained value mutations. The success test at line 2329 covers all/some/none under collisions. The non-Clone hash-panic key at line 2363 tests both kernels and disables the injected failure before querying contents. The removed-value test at line 2391 distinguishes committed survivors from merely avoiding a crash.

## Independent checks

- `cargo test --all-features retain -- --nocapture`: **33 passed**, zero failed; expected caught-panic messages appeared.
- `git diff --check`: passed.
- Toolchain used: rustc 1.98.0 / cargo 1.98.0. No local blocker affected these checks. This review did not independently rerun the full suite, Clippy, or the declared Rust 1.82 MSRV; it does not certify those gates. No newly used API appears newer than the project's already-used `BuildHasher::hash_one`.

Only this review file was written; production and test sources were left unchanged.
