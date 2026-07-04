// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! [`BoundedMap`] — a capacity-bounded map, generic in its value type and in its
//! **eviction policy** (`P: EvictionPolicy`). It is the modern, value-generic
//! successor to the frozen, `i32`-specialised [`BoundedLruMap`](crate::BoundedLruMap)
//! (which stays pinned for its cross-language suite — see `doc 04 §4.4`).
//!
//! ## Eviction is ownership transfer, not garbage collection
//!
//! The map **owns** its values (they live in an arena of `Option<(K, V)>`
//! slots). Eviction is therefore just the map letting go of a value: it either
//! **moves it back to the caller** ([`evict`](BoundedMap::evict) →
//! `Option<(K, V)>`) or **drops it**, running `V`'s destructor *synchronously,
//! right then* (RAII / `try-with-resources` promoted to a language invariant).
//! There is no pending-garbage state. For an owning `V` (`String`, `Vec<u8>`, a
//! file handle, an off-heap buffer) that deterministic drop-on-eviction is the
//! whole point.
//!
//! ## Policy as a type parameter (composition, not subclassing)
//!
//! *Which* resident entry to evict under size pressure is delegated to a
//! pluggable [`EvictionPolicy`], monomorphised into the map (zero vtable cost),
//! exactly as [`OpenHashMap`](crate::OpenHashMap) parameterises its hasher. The
//! map owns the *mechanism* (arena storage, the key index, the eviction
//! listener); the policy owns the *decision*. [`Lru`] (least-recently-used) and
//! [`Fifo`] (insertion order) ship here; a new policy is a new type implementing
//! the same five methods — the map does not change.
//!
//! The policy works purely in terms of the arena **slot index** (`usize`); it
//! owns no keys or values. This keeps it simple and reusable but means a v1
//! policy cannot see key/value metadata — weighted eviction and admission
//! policies (TinyLFU-style) would need a richer trait; that is a deliberate
//! future extension, not part of this transitional design (`doc 04` rev-note 2).
//!
//! ## Transitional backing
//!
//! The key index is an [`OpenHashMap`](crate::OpenHashMap)`<K, usize>` (so
//! `K: Clone` — the key is stored once in the index and once in its arena slot,
//! matching [`BoundedLruMap`](crate::BoundedLruMap)). A later revision can move
//! the index to the crate's key-owning-free `IndexTable` + `SlotList` kernels to
//! drop the `Clone` bound (`doc 04` rev-note 4); the public API here is designed
//! not to change when that happens.

use crate::bounded_lru::EvictionCause;
use crate::OpenHashMap;
use std::collections::VecDeque;
use std::fmt;
use std::hash::Hash;

/// Sentinel slot index meaning "no node" (list end).
const NIL: usize = usize::MAX;

/// The pluggable eviction **policy**: it decides recency/frequency bookkeeping
/// and which resident slot to evict under size pressure. It owns **no** keys or
/// values — it works purely in terms of the arena slot index the [`BoundedMap`]
/// hands it. Composition over subclassing: a `BoundedMap<K, V, P>` is
/// parameterised by its policy `P`.
///
/// A slot index is stable while a slot is resident and is **reused** after the
/// slot is freed; the map always calls [`on_remove`](EvictionPolicy::on_remove)
/// for a slot before that slot can be handed back to
/// [`on_insert`](EvictionPolicy::on_insert) for a new occupant, so a policy may
/// assume a slot it is told to insert is not one it currently tracks.
///
/// # Correctness contract
///
/// This is a **logical contract**, like [`Ord`]/[`Hash`]: [`BoundedMap`]'s
/// capacity guarantee holds only if the policy honours it. Specifically,
/// [`victim`](EvictionPolicy::victim) **must** return the slot of a currently
/// resident entry whenever the map is non-empty (in particular when the map is
/// at capacity and must make room). A policy that returns `None`, or a stale /
/// non-resident slot, at capacity causes the map to skip the eviction and insert
/// anyway — exceeding `max_size`. That is a logic error, never memory-unsafe.
pub trait EvictionPolicy {
    /// A brand-new entry became resident in `slot`. The policy should start
    /// tracking it (LRU: link it as most-recently-used; FIFO: enqueue it).
    fn on_insert(&mut self, slot: usize);

    /// An existing entry in `slot` was read ([`get`](BoundedMap::get)) or updated
    /// (re-`put`). This is where "get mutates recency" lives — LRU touches it to
    /// most-recently-used, FIFO ignores it.
    fn on_access(&mut self, slot: usize);

    /// The entry in `slot` was removed (evicted or explicitly removed); the
    /// policy must forget it so the slot can be safely reused.
    fn on_remove(&mut self, slot: usize);

    /// Choose the next victim's slot when the map is at capacity and must make
    /// room, or `None` when the policy tracks nothing (the map is empty).
    fn victim(&self) -> Option<usize>;

    /// Forget everything ([`BoundedMap::clear`]).
    fn clear(&mut self);
}

/// The eviction **observer** listener: it is shown each evicted entry by
/// reference (`&K`, `&V`) with the [`EvictionCause`], synchronously, *after* the
/// map is already in a consistent post-eviction state and *before* the value is
/// dropped. It observes; it does not take ownership (use
/// [`evict`](BoundedMap::evict) for one-shot ownership transfer) and it must not
/// mutate the map (Rust's borrow rules prevent re-entrant mutation at compile
/// time).
type EvictObserver<K, V> = Box<dyn FnMut(&K, &V, EvictionCause)>;

/// A capacity-bounded map that delegates victim selection to an
/// [`EvictionPolicy`] `P` (default [`Lru`]).
///
/// Holds at most `max_size` entries; a **new-key** insert that would exceed the
/// capacity evicts one victim **first** (evict-before-insert), so the inserted
/// key is never its own victim. Updating an existing key never evicts.
///
/// See the [module docs](crate::bounded_map) for the ownership-transfer eviction
/// model and the policy-as-type-parameter design.
pub struct BoundedMap<K: Hash + Eq + Clone, V, P: EvictionPolicy = Lru> {
    /// Key -> arena slot index.
    index: OpenHashMap<K, usize>,
    /// The value arena; `None` marks a free slot. A live slot holds `(key, value)`
    /// — the key is duplicated here so eviction can report/return it without a
    /// reverse lookup.
    slots: Vec<Option<(K, V)>>,
    /// Free-list of reusable slot indices.
    free: Vec<usize>,
    /// Per-slot logical expiry tick, parallel to `slots` (`u64::MAX` = never).
    /// An entry expires when `now >= expiry[slot]`; only meaningful for live
    /// slots (a freed slot's stale value is ignored — it is overwritten on
    /// reuse). Kept length-synced with `slots`.
    expiry: Vec<u64>,
    /// The eviction policy (victim selection + recency/order bookkeeping).
    policy: P,
    /// Capacity: at most this many resident entries (`0` ⇒ every insert drops).
    max_size: usize,
    /// After-write TTL in logical ticks, or `None` for a pure max-size map.
    /// `expiry[slot] = saturating(now + ttl)` on each write.
    ttl: Option<u64>,
    /// Optional eviction observer, fired for `Size` and `Expired` evictions
    /// (explicit [`evict`](BoundedMap::evict) returns the value instead).
    on_evict: Option<EvictObserver<K, V>>,
}

impl<K: Hash + Eq + Clone, V, P: EvictionPolicy> BoundedMap<K, V, P> {
    /// A bounded map of capacity `max_size` using the given `policy`.
    pub fn with_policy(max_size: usize, policy: P) -> Self {
        BoundedMap {
            index: OpenHashMap::new(),
            slots: Vec::new(),
            free: Vec::new(),
            expiry: Vec::new(),
            policy,
            max_size,
            ttl: None,
            on_evict: None,
        }
    }

    /// Set the after-write TTL in logical ticks (builder-style). Each write
    /// (`put`/`put_at`) then stamps the entry with `expiry = saturating(now +
    /// ttl)`, and [`expire_entries`](BoundedMap::expire_entries)`(now)` removes
    /// every entry whose expiry tick has passed. TTL is **orthogonal** to the
    /// eviction policy — it is about *time*, the policy about *space*.
    #[must_use]
    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Install an eviction **observer** (builder-style; replaces any previous
    /// one). It is called `(&key, &value, cause)` for every entry evicted under
    /// size pressure ([`Size`](EvictionCause::Size)) or by TTL expiry
    /// ([`Expired`](EvictionCause::Expired)), after the map is already consistent
    /// and before the value drops. It must not attempt to mutate the map.
    #[must_use]
    pub fn on_evict<F: FnMut(&K, &V, EvictionCause) + 'static>(mut self, cb: F) -> Self {
        self.on_evict = Some(Box::new(cb));
        self
    }

    /// The number of resident entries.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the map holds no entries.
    pub fn is_empty(&self) -> bool {
        self.index.len() == 0
    }

    /// The capacity (`max_size`) fixed at construction.
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    /// Whether `key` is resident (does **not** count as an access — recency is
    /// untouched; it takes `&self`).
    pub fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    /// Insert or update `key` at logical time `0` — shorthand for
    /// [`put_at(key, value, 0)`](BoundedMap::put_at). Use `put_at` when a TTL is
    /// configured and you need real expiry timestamps.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        self.put_at(key, value, 0)
    }

    /// Insert or update `key` at logical time `now`. Returns the previous value
    /// if `key` was resident.
    ///
    /// On a **new** key when the map is already at capacity, exactly one victim
    /// (chosen by the policy) is evicted first — firing the [`on_evict`] observer
    /// with cause [`Size`](EvictionCause::Size) — so the map never exceeds
    /// `max_size`. A capacity of `0` drops every insert (returns `None`).
    /// Updating an existing key refreshes its policy access and never evicts.
    ///
    /// If a TTL is configured ([`with_ttl`](BoundedMap::with_ttl)), the (new or
    /// updated) entry's expiry is (re)stamped to `saturating(now + ttl)` — TTL is
    /// after-write, so every write extends the entry's life.
    ///
    /// [`on_evict`]: BoundedMap::on_evict
    pub fn put_at(&mut self, key: K, value: V, now: u64) -> Option<V> {
        let expire_at = self.ttl.map_or(u64::MAX, |t| now.saturating_add(t));
        if let Some(&slot) = self.index.get(&key) {
            // Update in place: refresh recency + expiry, swap the value.
            self.policy.on_access(slot);
            self.expiry[slot] = expire_at;
            let cell = self.slots[slot]
                .as_mut()
                .expect("indexed slot must be occupied");
            return Some(std::mem::replace(&mut cell.1, value));
        }
        if self.max_size == 0 {
            return None; // capacity 0: value drops here.
        }
        self.insert_absent(key, value, expire_at);
        None
    }

    /// Insert a **known-absent** `key` (evict-before-insert when at capacity),
    /// stamping `expire_at`, and return its slot. Caller must have already
    /// checked `key` is not resident and `max_size >= 1`.
    ///
    /// Ordered so a panicking `Clone`, or a panicking `Hash`/`Eq` during the
    /// index *probe*, leaves the index, arena, free-list, expiry, and policy all
    /// untouched — not even a leaked cell: the slot is peeked (not committed), the
    /// only fallible user code (key clone + index insert) runs first, and the
    /// arena allocation is committed afterwards. (The one residual is a `Hash`
    /// that panics during a kernel resize-rehash, which can leave `index` partial
    /// — a pre-existing `OpenHashMap::insert` property shared with `BoundedLruMap`,
    /// not addressed here.)
    fn insert_absent(&mut self, key: K, value: V, expire_at: u64) -> usize {
        if self.index.len() >= self.max_size {
            if let Some(victim) = self.policy.victim() {
                self.evict_slot(victim, EvictionCause::Size);
            }
        }
        let slot = self.free.last().copied().unwrap_or(self.slots.len());
        self.index.insert(key.clone(), slot);
        // Commit the allocation (infallible from here on): reuse the peeked free
        // slot, or grow the arena (both `slots` and the parallel `expiry`) to make
        // `slot` valid.
        if self.free.pop().is_none() {
            self.slots.push(None);
            self.expiry.push(u64::MAX);
        }
        self.slots[slot] = Some((key, value));
        self.expiry[slot] = expire_at;
        self.policy.on_insert(slot);
        slot
    }

    /// Remove every entry whose logical expiry tick has passed (`expiry <= now`),
    /// firing the [`on_evict`] observer with cause
    /// [`Expired`](EvictionCause::Expired) for each and dropping its value.
    /// Returns the number expired. A no-op when no TTL is configured. Order of
    /// removal is unspecified (arena order).
    ///
    /// [`on_evict`]: BoundedMap::on_evict
    pub fn expire_entries(&mut self, now: u64) -> usize {
        // Two-phase: collect the expired slots under a read-only scan, then evict
        // them. If a value `Drop` or the observer panics mid-eviction, each
        // already-processed slot is fully consistent and the rest simply remain
        // (a later call re-collects them) — no divergence.
        let expired: Vec<usize> = (0..self.slots.len())
            .filter(|&i| {
                // `u64::MAX` is the "never expires" sentinel — excluded even at
                // `now == u64::MAX`.
                self.slots[i].is_some() && self.expiry[i] != u64::MAX && self.expiry[i] <= now
            })
            .collect();
        let count = expired.len();
        for slot in expired {
            self.evict_slot(slot, EvictionCause::Expired);
        }
        count
    }

    /// Return a mutable borrow of `key`'s value, computing and inserting `f()`
    /// first if `key` is absent (compute-if-absent) — shorthand for
    /// [`get_or_insert_with_at(key, 0, f)`](BoundedMap::get_or_insert_with_at).
    ///
    /// # Panics
    ///
    /// Panics if [`capacity`](BoundedMap::capacity) is `0` on a miss — a
    /// zero-capacity map cannot hold the computed value while returning a borrow
    /// of it. (A hit never allocates, so it never panics.)
    pub fn get_or_insert_with(&mut self, key: K, f: impl FnOnce() -> V) -> &mut V {
        self.get_or_insert_with_at(key, 0, f)
    }

    /// Return a mutable borrow of `key`'s value at logical time `now`, computing
    /// and inserting `f()` first if `key` is absent.
    ///
    /// A **hit** refreshes the policy access (recency) but — because the map's
    /// TTL is *after-write* — does **not** extend the entry's expiry (it was not
    /// written). A **miss** is an insert: it evicts one victim first if the map is
    /// at capacity, stamps the new entry's expiry to `saturating(now + ttl)`, and
    /// only calls `f` when the key is genuinely absent. If `f` panics nothing is
    /// committed.
    ///
    /// # Panics
    ///
    /// Panics if [`capacity`](BoundedMap::capacity) is `0` on a miss.
    pub fn get_or_insert_with_at(&mut self, key: K, now: u64, f: impl FnOnce() -> V) -> &mut V {
        if let Some(&slot) = self.index.get(&key) {
            self.policy.on_access(slot);
            return &mut self.slots[slot].as_mut().expect("indexed slot occupied").1;
        }
        assert!(
            self.max_size > 0,
            "get_or_insert_with on a zero-capacity BoundedMap"
        );
        let expire_at = self.ttl.map_or(u64::MAX, |t| now.saturating_add(t));
        let slot = self.insert_absent(key, f(), expire_at);
        &mut self.slots[slot]
            .as_mut()
            .expect("just-inserted slot occupied")
            .1
    }

    /// Read `key`, refreshing its policy access (for [`Lru`] this is the "get
    /// mutates recency" step — hence `&mut self`). Returns a borrow that pins the
    /// map, so nothing can evict the entry while the borrow lives (enforced by
    /// the borrow checker). Use [`peek`](BoundedMap::peek) for a non-touching
    /// `&self` read.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let slot = *self.index.get(key)?;
        self.policy.on_access(slot);
        self.slots[slot].as_ref().map(|(_, v)| v)
    }

    /// Read `key` **without** touching recency/order (`&self`). A `peek` never
    /// changes which entry the policy would evict next.
    pub fn peek(&self, key: &K) -> Option<&V> {
        let slot = *self.index.get(key)?;
        self.slots[slot].as_ref().map(|(_, v)| v)
    }

    /// Mutable access to `key`'s value, refreshing its policy access. Keys are
    /// never handed out mutably (a changed key would desync the index).
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let slot = *self.index.get(key)?;
        self.policy.on_access(slot);
        self.slots[slot].as_mut().map(|(_, v)| v)
    }

    /// Remove `key`, returning its value. This is **not** an eviction — the
    /// observer is not fired.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let slot = *self.index.get(key)?;
        self.take_slot(slot).map(|(_, v)| v)
    }

    /// Drop every entry for which `keep(&k, &mut v)` returns `false`, allowing
    /// in-place value mutation of the survivors (the key stays shared). Dropped
    /// entries are removed like [`remove`](BoundedMap::remove) — **not** evicted,
    /// so the observer is not fired and the policy simply forgets them. Two-phase
    /// (decide over all live slots, then remove the rejects), so if `keep` panics
    /// no entry has been removed yet — the map is left valid with whatever
    /// survivors' values `keep` had already mutated.
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut keep: F) {
        let mut reject: Vec<usize> = Vec::new();
        for slot in 0..self.slots.len() {
            if let Some((k, v)) = self.slots[slot].as_mut() {
                if !keep(k, v) {
                    reject.push(slot);
                }
            }
        }
        for slot in reject {
            self.take_slot(slot);
        }
    }

    /// Explicitly evict one entry (the policy's current victim) and **return** it
    /// to the caller (`Option<(K, V)>`) — ownership transfer, the observer is not
    /// fired. `None` if the map is empty.
    pub fn evict(&mut self) -> Option<(K, V)> {
        let victim = self.policy.victim()?;
        self.take_slot(victim)
    }

    /// Remove every entry. Values are dropped synchronously (their `Drop` runs
    /// now); this is **not** eviction, so the observer is not fired.
    ///
    /// Panic-safe: the map is reset to a valid empty state *before* any key or
    /// value destructor runs, so if a `Drop` panics (and the unwind is caught),
    /// the map is still a consistent, usable empty map — only the not-yet-dropped
    /// entries leak.
    pub fn clear(&mut self) {
        self.policy.clear();
        self.free.clear();
        self.expiry.clear();
        let old_index = std::mem::replace(&mut self.index, OpenHashMap::new());
        let old_slots = std::mem::take(&mut self.slots);
        // `self` is now a valid empty map; dropping the drained keys/values below
        // cannot corrupt it even if a destructor panics.
        drop(old_index);
        drop(old_slots);
    }

    /// Borrowing iterator over the resident `(&K, &V)` entries. **Order is
    /// unspecified** (arena order, not recency/insertion order).
    pub fn iter(&self) -> BoundedMapIter<'_, K, V> {
        BoundedMapIter {
            inner: self.slots.iter().flatten(),
        }
    }

    /// Borrowing iterator over the resident keys (unspecified order).
    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.iter().map(|(k, _)| k)
    }

    /// Borrowing iterator over the resident values (unspecified order).
    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.iter().map(|(_, v)| v)
    }

    /// Evict `slot` under `cause`, firing the observer (if any) after the map is
    /// already consistent, then dropping the value.
    fn evict_slot(&mut self, slot: usize, cause: EvictionCause) {
        if let Some((k, v)) = self.take_slot(slot) {
            if let Some(cb) = self.on_evict.as_mut() {
                cb(&k, &v, cause);
            }
            // k, v drop here — deterministically, after the observer has seen them.
        }
    }

    /// Move `(K, V)` out of `slot`, unindex it, tell the policy to forget it, and
    /// recycle the slot. Ordered so a panicking user `Hash`/`Eq` during the
    /// unindex leaves the map fully consistent (the value is still in its slot
    /// and still indexed): the index removal — the only step that runs user code
    /// — happens *before* any structural change, and the backing kernel's
    /// backward-shift deletion runs no user code once the cell is found.
    fn take_slot(&mut self, slot: usize) -> Option<(K, V)> {
        // Occupancy + bounds check without holding the borrow across the removal.
        self.slots.get(slot)?.as_ref()?;
        // Unindex first (borrows the still-resident key; disjoint from `index`).
        let key_ref = &self.slots[slot].as_ref().unwrap().0;
        self.index.remove(key_ref);
        // From here on nothing runs user code: pure moves and index arithmetic.
        let (k, v) = self.slots[slot].take().unwrap();
        self.policy.on_remove(slot);
        self.free.push(slot);
        Some((k, v))
    }
}

impl<K: Hash + Eq + Clone, V> BoundedMap<K, V, Lru> {
    /// A least-recently-used bounded map of capacity `max_size`.
    pub fn with_capacity(max_size: usize) -> Self {
        Self::with_policy(max_size, Lru::new())
    }
}

impl<K: Hash + Eq + Clone, V> BoundedMap<K, V, Fifo> {
    /// A first-in-first-out (insertion-order) bounded map of capacity `max_size`.
    pub fn fifo(max_size: usize) -> Self {
        Self::with_policy(max_size, Fifo::new())
    }
}

/// Consuming iterator over a [`BoundedMap`]'s resident `(K, V)` entries in
/// unspecified (arena) order.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct BoundedMapIntoIter<K, V> {
    inner: std::iter::Flatten<std::vec::IntoIter<Option<(K, V)>>>,
}

impl<K, V> Iterator for BoundedMapIntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<(K, V)> {
        self.inner.next()
    }
}

impl<K, V> std::iter::FusedIterator for BoundedMapIntoIter<K, V> {}

/// Consumes the map, yielding its resident `(K, V)` entries (unspecified order).
impl<K: Hash + Eq + Clone, V, P: EvictionPolicy> IntoIterator for BoundedMap<K, V, P> {
    type Item = (K, V);
    type IntoIter = BoundedMapIntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        BoundedMapIntoIter {
            inner: self.slots.into_iter().flatten(),
        }
    }
}

/// Borrowing iterator over a [`BoundedMap`]'s resident `(&K, &V)` entries in
/// unspecified (arena) order.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct BoundedMapIter<'a, K, V> {
    inner: std::iter::Flatten<std::slice::Iter<'a, Option<(K, V)>>>,
}

impl<'a, K, V> Iterator for BoundedMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        self.inner.next().map(|(k, v)| (k, v))
    }
}

impl<K, V> std::iter::FusedIterator for BoundedMapIter<'_, K, V> {}

/// [`put`](BoundedMap::put) each pair in iterator order — later inserts may evict
/// earlier ones once the map is at capacity (there is no `FromIterator`: a
/// bounded map needs a capacity/policy the iterator cannot supply).
impl<K: Hash + Eq + Clone, V, P: EvictionPolicy> Extend<(K, V)> for BoundedMap<K, V, P> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, entries: I) {
        for (k, v) in entries {
            self.put(k, v);
        }
    }
}

impl<'a, K: Hash + Eq + Clone, V, P: EvictionPolicy> IntoIterator for &'a BoundedMap<K, V, P> {
    type Item = (&'a K, &'a V);
    type IntoIter = BoundedMapIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: Hash + Eq + Clone + fmt::Debug, V: fmt::Debug, P: EvictionPolicy> fmt::Debug
    for BoundedMap<K, V, P>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

// ---- policies --------------------------------------------------------------

/// One node of the [`Lru`] intrusive recency list; `prev`/`next` are slot
/// indices ([`NIL`] at the ends), never pointers.
#[derive(Clone, Copy, Debug)]
struct Link {
    prev: usize,
    next: usize,
}

/// Least-recently-used policy: an intrusive doubly-linked recency list threaded
/// over the arena slot indices (`head` = LRU end = the eviction victim, `tail` =
/// most-recently-used). A recency refresh is an O(1) unlink + push-to-tail;
/// victim selection is an O(1) read of `head`.
#[derive(Debug, Default)]
pub struct Lru {
    /// Parallel to the map's arena, indexed by slot; only live slots' entries
    /// are meaningful.
    links: Vec<Link>,
    head: usize,
    tail: usize,
}

impl Lru {
    /// A new, empty LRU policy.
    pub fn new() -> Self {
        Lru {
            links: Vec::new(),
            head: NIL,
            tail: NIL,
        }
    }

    fn push_tail(&mut self, slot: usize) {
        self.links[slot] = Link {
            prev: self.tail,
            next: NIL,
        };
        if self.tail != NIL {
            self.links[self.tail].next = slot;
        } else {
            self.head = slot;
        }
        self.tail = slot;
    }

    fn unlink(&mut self, slot: usize) {
        let Link { prev, next } = self.links[slot];
        if prev != NIL {
            self.links[prev].next = next;
        } else {
            self.head = next;
        }
        if next != NIL {
            self.links[next].prev = prev;
        } else {
            self.tail = prev;
        }
    }
}

impl EvictionPolicy for Lru {
    fn on_insert(&mut self, slot: usize) {
        if slot >= self.links.len() {
            self.links.resize(
                slot + 1,
                Link {
                    prev: NIL,
                    next: NIL,
                },
            );
        }
        self.push_tail(slot);
    }
    fn on_access(&mut self, slot: usize) {
        if self.tail != slot {
            self.unlink(slot);
            self.push_tail(slot);
        }
    }
    fn on_remove(&mut self, slot: usize) {
        self.unlink(slot);
    }
    fn victim(&self) -> Option<usize> {
        (self.head != NIL).then_some(self.head)
    }
    fn clear(&mut self) {
        self.links.clear();
        self.head = NIL;
        self.tail = NIL;
    }
}

/// First-in-first-out (insertion-order) policy: reads never change the order, so
/// the oldest resident entry is always the victim. `on_remove` of an arbitrary
/// slot is O(n) in the resident count (a rare path — size-pressure eviction only
/// ever removes the front); victim selection and insertion are O(1).
#[derive(Debug, Default)]
pub struct Fifo {
    order: VecDeque<usize>,
}

impl Fifo {
    /// A new, empty FIFO policy.
    pub fn new() -> Self {
        Fifo {
            order: VecDeque::new(),
        }
    }
}

impl EvictionPolicy for Fifo {
    fn on_insert(&mut self, slot: usize) {
        self.order.push_back(slot);
    }
    fn on_access(&mut self, _slot: usize) {
        // FIFO ignores reads.
    }
    fn on_remove(&mut self, slot: usize) {
        self.order.retain(|&s| s != slot);
    }
    fn victim(&self) -> Option<usize> {
        self.order.front().copied()
    }
    fn clear(&mut self) {
        self.order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn sorted_entries<P: EvictionPolicy>(m: &BoundedMap<i32, i32, P>) -> Vec<(i32, i32)> {
        let mut v: Vec<(i32, i32)> = m.iter().map(|(&k, &val)| (k, val)).collect();
        v.sort_unstable();
        v
    }

    #[test]
    fn put_get_update() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(4);
        assert_eq!(m.put(1, 10), None);
        assert_eq!(m.put(2, 20), None);
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&1), Some(&10));
        assert_eq!(m.put(1, 11), Some(10)); // update returns old
        assert_eq!(m.get(&1), Some(&11));
        assert_eq!(m.len(), 2); // update does not grow
        assert_eq!(m.get(&99), None);
    }

    #[test]
    fn lru_evicts_least_recently_used() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(2);
        m.put(1, 10);
        m.put(2, 20);
        assert_eq!(m.get(&1), Some(&10)); // 1 is now MRU, 2 is LRU
        m.put(3, 30); // evicts 2 (the LRU victim)
        assert_eq!(m.len(), 2);
        assert_eq!(m.peek(&2), None);
        assert_eq!(sorted_entries(&m), vec![(1, 10), (3, 30)]);
    }

    #[test]
    fn lru_update_refreshes_recency() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(2);
        m.put(1, 10);
        m.put(2, 20);
        m.put(1, 11); // update touches 1 -> 2 becomes LRU
        m.put(3, 30); // evicts 2
        assert_eq!(sorted_entries(&m), vec![(1, 11), (3, 30)]);
    }

    #[test]
    fn peek_does_not_touch_recency() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(2);
        m.put(1, 10);
        m.put(2, 20);
        assert_eq!(m.peek(&1), Some(&10)); // peek must NOT make 1 the MRU
        m.put(3, 30); // 1 is still LRU -> evicted
        assert_eq!(m.peek(&1), None);
        assert_eq!(sorted_entries(&m), vec![(2, 20), (3, 30)]);
    }

    #[test]
    fn fifo_evicts_oldest_regardless_of_access() {
        let mut m: BoundedMap<i32, i32, Fifo> = BoundedMap::fifo(2);
        m.put(1, 10);
        m.put(2, 20);
        assert_eq!(m.get(&1), Some(&10)); // FIFO ignores the read
        m.put(3, 30); // evicts 1 (oldest), not 2
        assert_eq!(sorted_entries(&m), vec![(2, 20), (3, 30)]);
    }

    #[test]
    fn capacity_zero_drops_everything() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(0);
        assert_eq!(m.put(1, 10), None);
        assert!(m.is_empty());
        assert_eq!(m.get(&1), None);
    }

    #[test]
    fn remove_is_not_an_eviction() {
        let evicted = Rc::new(RefCell::new(Vec::<i32>::new()));
        let ev = evicted.clone();
        let mut m: BoundedMap<i32, i32> =
            BoundedMap::with_capacity(4).on_evict(move |&k, _v, _c| ev.borrow_mut().push(k));
        m.put(1, 10);
        m.put(2, 20);
        assert_eq!(m.remove(&1), Some(10));
        assert_eq!(m.len(), 1);
        assert!(evicted.borrow().is_empty()); // remove did not fire the observer
        assert_eq!(m.remove(&1), None);
    }

    #[test]
    fn on_evict_observer_fires_for_size_eviction() {
        let evicted = Rc::new(RefCell::new(Vec::<(i32, i32, &'static str)>::new()));
        let ev = evicted.clone();
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(1)
            .on_evict(move |&k, &v, c| ev.borrow_mut().push((k, v, c.as_str())));
        m.put(1, 10);
        m.put(2, 20); // evicts 1 under Size
        assert_eq!(*evicted.borrow(), vec![(1, 10, "size")]);
    }

    #[test]
    fn explicit_evict_returns_the_victim() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(3);
        m.put(1, 10);
        m.put(2, 20); // 1 is LRU
        assert_eq!(m.evict(), Some((1, 10)));
        assert_eq!(m.len(), 1);
        assert_eq!(m.evict(), Some((2, 20)));
        assert_eq!(m.evict(), None); // empty
    }

    #[test]
    fn drop_on_eviction_is_deterministic() {
        // A value whose Drop bumps a shared counter proves eviction drops NOW.
        struct Bump(Rc<RefCell<i32>>);
        impl Drop for Bump {
            fn drop(&mut self) {
                *self.0.borrow_mut() += 1;
            }
        }
        let drops = Rc::new(RefCell::new(0));
        let mut m: BoundedMap<i32, Bump> = BoundedMap::with_capacity(1);
        m.put(1, Bump(drops.clone()));
        assert_eq!(*drops.borrow(), 0);
        m.put(2, Bump(drops.clone())); // evicts entry 1 -> its Bump drops now
        assert_eq!(*drops.borrow(), 1);
        drop(m); // entry 2 drops
        assert_eq!(*drops.borrow(), 2);
    }

    #[test]
    fn slot_reuse_does_not_leak_or_alias() {
        // Churn well past capacity; arena must stay bounded and never alias.
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(3);
        for i in 0..1000 {
            m.put(i, i * 10);
            assert!(m.len() <= 3);
        }
        // Only the last three keys survive under LRU.
        assert_eq!(
            sorted_entries(&m),
            vec![(997, 9970), (998, 9980), (999, 9990)]
        );
        // Arena never grew beyond what capacity needs (+ at most transient slack).
        assert!(m.slots.len() <= 4, "arena bloated to {}", m.slots.len());
    }

    #[test]
    fn retain_drops_rejected_and_can_mutate_survivors() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(8);
        for i in 0..6 {
            m.put(i, i * 10);
        }
        // Keep evens, double the survivors' values in place.
        m.retain(|&k, v| {
            if k % 2 == 0 {
                *v *= 2;
                true
            } else {
                false
            }
        });
        assert_eq!(sorted_entries(&m), vec![(0, 0), (2, 40), (4, 80)]);
        assert_eq!(m.len(), 3);
        // Removed keys are gone and their slots reusable (policy forgot them).
        assert_eq!(m.peek(&1), None);
        m.put(100, 1);
        assert!(m.slots.len() <= 8);
    }

    #[test]
    fn retain_does_not_fire_observer() {
        let evicted = Rc::new(RefCell::new(Vec::<i32>::new()));
        let ev = evicted.clone();
        let mut m: BoundedMap<i32, i32> =
            BoundedMap::with_capacity(8).on_evict(move |&k, _v, _c| ev.borrow_mut().push(k));
        for i in 0..4 {
            m.put(i, i);
        }
        m.retain(|&k, _v| k >= 2);
        assert_eq!(m.len(), 2);
        assert!(evicted.borrow().is_empty()); // retain is not an eviction
    }

    #[test]
    fn retain_all_and_none() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(4);
        m.put(1, 1);
        m.put(2, 2);
        m.retain(|_, _| true);
        assert_eq!(m.len(), 2);
        m.retain(|_, _| false);
        assert!(m.is_empty());
        assert!(m.evict().is_none()); // policy is consistent (empty) after clearing all
    }

    #[test]
    fn extend_puts_each_and_honours_capacity() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(2);
        m.extend([(1, 10), (2, 20), (3, 30)]); // 1 evicted (LRU) at the 3rd put
        assert_eq!(m.len(), 2);
        assert_eq!(sorted_entries(&m), vec![(2, 20), (3, 30)]);
    }

    #[test]
    fn into_iter_yields_all_residents() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(4);
        m.put(1, 10);
        m.put(2, 20);
        m.put(3, 30);
        let mut got: Vec<(i32, i32)> = m.into_iter().collect();
        got.sort_unstable();
        assert_eq!(got, vec![(1, 10), (2, 20), (3, 30)]);
    }

    #[test]
    fn get_mut_mutates_in_place() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(2);
        m.put(1, 10);
        *m.get_mut(&1).unwrap() += 5;
        assert_eq!(m.peek(&1), Some(&15));
    }

    #[test]
    fn clear_empties_and_resets_policy() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(3);
        m.put(1, 10);
        m.put(2, 20);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.evict(), None);
        // Reuse after clear works and re-bounds correctly (capacity is 3).
        m.put(3, 30);
        m.put(4, 40);
        m.put(5, 50); // all three fit
        assert_eq!(sorted_entries(&m), vec![(3, 30), (4, 40), (5, 50)]);
        m.put(6, 60); // now at capacity -> evicts 3 (LRU)
        assert_eq!(sorted_entries(&m), vec![(4, 40), (5, 50), (6, 60)]);
    }

    #[test]
    fn get_or_insert_with_computes_only_on_miss() {
        let calls = Rc::new(RefCell::new(0));
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(4);
        let c = calls.clone();
        let v = m.get_or_insert_with(1, || {
            *c.borrow_mut() += 1;
            10
        });
        assert_eq!(*v, 10);
        assert_eq!(*calls.borrow(), 1);
        // Hit: closure must NOT run, existing value returned (and mutable).
        let c = calls.clone();
        let v = m.get_or_insert_with(1, || {
            *c.borrow_mut() += 1;
            999
        });
        assert_eq!(*v, 10);
        *v += 5;
        assert_eq!(*calls.borrow(), 1); // still 1 — no recompute
        assert_eq!(m.peek(&1), Some(&15));
    }

    #[test]
    fn get_or_insert_with_evicts_before_insert() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(2);
        m.put(1, 10);
        m.put(2, 20); // 1 is LRU
        m.get_or_insert_with(3, || 30); // miss -> evict 1
        assert_eq!(m.len(), 2);
        assert_eq!(m.peek(&1), None);
        assert_eq!(sorted_entries(&m), vec![(2, 20), (3, 30)]);
    }

    #[test]
    fn get_or_insert_with_hit_does_not_extend_ttl() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(4).with_ttl(10);
        m.put_at(1, 10, 0); // expiry 10
                            // A hit at t=5 refreshes recency but must NOT extend after-write expiry.
        let _ = m.get_or_insert_with_at(1, 5, || 999);
        assert_eq!(m.expire_entries(10), 1); // still due at 10
        assert!(m.is_empty());
    }

    #[test]
    fn get_or_insert_with_does_not_commit_if_closure_panics() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(4);
        m.put(1, 10);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            m.get_or_insert_with(2, || panic!("boom"));
        }));
        assert!(r.is_err());
        assert_eq!(m.len(), 1); // key 2 not inserted
        assert_eq!(m.peek(&2), None);
        assert_eq!(m.peek(&1), Some(&10)); // untouched, still usable
        m.put(3, 30); // map still works
        assert_eq!(m.len(), 2);
    }

    #[test]
    #[should_panic(expected = "zero-capacity")]
    fn get_or_insert_with_panics_on_zero_capacity_miss() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(0);
        m.get_or_insert_with(1, || 10);
    }

    #[test]
    fn ttl_expires_entries_at_or_past_deadline() {
        // ttl 10, written at now=0 -> expiry 10. expire_entries removes at >= 10.
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(8).with_ttl(10);
        m.put_at(1, 10, 0);
        m.put_at(2, 20, 5);
        assert_eq!(m.expire_entries(9), 0); // nothing due yet (deadlines 10, 15)
        assert_eq!(m.len(), 2);
        assert_eq!(m.expire_entries(10), 1); // key 1 (expiry 10) is due
        assert_eq!(m.peek(&1), None);
        assert_eq!(m.peek(&2), Some(&20));
        assert_eq!(m.expire_entries(15), 1); // key 2 (expiry 15)
        assert!(m.is_empty());
    }

    #[test]
    fn ttl_rewrite_extends_life() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(8).with_ttl(10);
        m.put_at(1, 10, 0); // expiry 10
        m.put_at(1, 11, 7); // after-write TTL: expiry now 17
        assert_eq!(m.expire_entries(10), 0); // no longer due at 10
        assert_eq!(m.peek(&1), Some(&11));
        assert_eq!(m.expire_entries(17), 1);
        assert!(m.is_empty());
    }

    #[test]
    fn ttl_fires_observer_with_expired_cause() {
        let log = Rc::new(RefCell::new(Vec::<(i32, &'static str)>::new()));
        let l = log.clone();
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(8)
            .with_ttl(5)
            .on_evict(move |&k, _v, c| l.borrow_mut().push((k, c.as_str())));
        m.put_at(1, 10, 0);
        m.put_at(2, 20, 0);
        m.expire_entries(5);
        let mut got = log.borrow().clone();
        got.sort_unstable();
        assert_eq!(got, vec![(1, "expired"), (2, "expired")]);
    }

    #[test]
    fn no_ttl_means_expire_entries_is_a_noop() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(4); // no TTL
        m.put(1, 10);
        assert_eq!(m.expire_entries(u64::MAX), 0);
        assert_eq!(m.peek(&1), Some(&10));
    }

    #[test]
    fn ttl_saturates_without_overflow() {
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(4).with_ttl(u64::MAX);
        m.put_at(1, 10, u64::MAX); // now + ttl saturates to u64::MAX, never expires
        assert_eq!(m.expire_entries(u64::MAX - 1), 0);
        assert_eq!(m.peek(&1), Some(&10));
    }

    #[test]
    fn ttl_expiry_slot_reuse_stays_consistent() {
        // An expired-then-reused slot must not carry a stale expiry.
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(2).with_ttl(10);
        m.put_at(1, 10, 0); // expiry 10
        m.expire_entries(10); // remove 1; its slot goes to the free-list
        m.put_at(2, 20, 100); // reuses that slot; expiry must be 110, not 10
        assert_eq!(m.expire_entries(10), 0); // stale 10 must NOT expire key 2
        assert_eq!(m.peek(&2), Some(&20));
        assert_eq!(m.expire_entries(110), 1);
        assert!(m.is_empty());
    }

    #[test]
    fn clear_is_panic_safe_when_a_value_drop_panics() {
        // A value whose Drop panics once. clear() must leave the map a valid,
        // usable, empty map even when the unwind from that Drop is caught — no
        // stale policy links reused, no capacity corruption afterwards.
        thread_local! {
            static ARMED: RefCell<bool> = const { RefCell::new(false) };
        }
        struct PanicOnDrop(i32);
        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                let boom = ARMED.with(|a| {
                    let mut a = a.borrow_mut();
                    if *a {
                        *a = false;
                        true
                    } else {
                        false
                    }
                });
                if boom {
                    panic!("armed drop");
                }
            }
        }

        let mut m: BoundedMap<i32, PanicOnDrop> = BoundedMap::with_capacity(3);
        m.put(1, PanicOnDrop(10));
        m.put(2, PanicOnDrop(20));
        m.put(3, PanicOnDrop(30));
        ARMED.with(|a| *a.borrow_mut() = true);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.clear()));
        assert!(result.is_err(), "the armed drop should have panicked");
        ARMED.with(|a| *a.borrow_mut() = false); // disarm for the rest

        // The map must be a consistent empty map after the caught panic.
        assert!(m.is_empty());
        assert!(m.evict().is_none());
        // And it must still enforce capacity correctly on reuse (no stale LRU
        // links from before the clear).
        m.put(4, PanicOnDrop(40));
        m.put(5, PanicOnDrop(50));
        m.put(6, PanicOnDrop(60));
        assert_eq!(m.len(), 3);
        m.put(7, PanicOnDrop(70)); // evicts LRU (4)
        assert_eq!(m.len(), 3);
        assert_eq!(m.peek(&4).map(|v| v.0), None);
        assert_eq!(m.peek(&7).map(|v| v.0), Some(70));
    }

    #[test]
    fn differential_against_naive_lru() {
        // Cross-check the map against a dead-simple Vec-based LRU oracle over a
        // deterministic op stream.
        let cap = 5;
        let mut m: BoundedMap<i32, i32> = BoundedMap::with_capacity(cap);
        let mut oracle: Vec<(i32, i32)> = Vec::new(); // front = LRU, back = MRU

        let touch = |oracle: &mut Vec<(i32, i32)>, k: i32| {
            if let Some(pos) = oracle.iter().position(|&(ok, _)| ok == k) {
                let e = oracle.remove(pos);
                oracle.push(e);
            }
        };
        let mut seed = 12345u64;
        let mut rng = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as i32
        };

        for _ in 0..5000 {
            let k = rng() % 20;
            if rng() % 3 == 0 {
                // get
                let hit = m.get(&k).copied();
                let oracle_hit = oracle.iter().find(|&&(ok, _)| ok == k).map(|&(_, v)| v);
                assert_eq!(hit, oracle_hit);
                if oracle_hit.is_some() {
                    touch(&mut oracle, k);
                }
            } else {
                // put
                let v = rng();
                m.put(k, v);
                if let Some(e) = oracle.iter_mut().find(|(ok, _)| *ok == k) {
                    e.1 = v;
                    touch(&mut oracle, k);
                } else {
                    if oracle.len() >= cap {
                        oracle.remove(0); // evict LRU (front)
                    }
                    oracle.push((k, v));
                }
            }
            assert_eq!(m.len(), oracle.len());
        }
        assert_eq!(sorted_entries(&m), {
            let mut o = oracle.clone();
            o.sort_unstable();
            o
        });
    }
}
