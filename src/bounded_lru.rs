// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Bounded LRU map (max-size v1) — `spec/features/bounded-lru.md`.
//!
//! A fixed-capacity `BoundedLruMap<K, V>` that evicts its least-recently-used
//! entry when an insert would exceed the capacity. The recency order is kept by
//! an **intrusive doubly-linked LRU list over an arena + slot-index** (the
//! Phase-0 arena/slot-index intrusive-list primitive): each live entry owns a
//! slot in a contiguous `Vec<Node>`; nodes carry `{prev, next}` **indices**
//! (never raw pointers) plus a back-reference key, and freed slots are recycled
//! through a free-list. The list head is the LRU end (the eviction victim), the
//! tail is the MRU end. A recency refresh is an O(1) unlink + push-to-tail;
//! eviction is an O(1) pop-from-head.
//!
//! Recency is **position-implicit** (head = least-recently-used): there is no
//! stored `last_use` stamp and therefore nothing to overflow (spec §"`useSeq`
//! width / overflow" — the reference position-implicit form). The observable
//! contract (LRU-order contents, eviction log, results) is what the spec pins;
//! the arena mechanism is non-observable.
//!
//! v1 has **no wall clock**: all time is the caller-supplied logical tick. TTL
//! is an after-write `expire_at = saturating(now + ttl)`; `expire_entries(now)`
//! removes every entry with `expire_at <= now` (inclusive), firing the callback
//! with cause `EXPIRED` in ascending-`expire_at` then ascending-`last_use`
//! order. Plain `put(k, v)` is defined as `put_at(k, v, 0)`.

use crate::OpenHashMap;
use std::hash::Hash;

/// Why an entry left the map (the eviction-callback cause). Only `Size` and
/// `Expired` exist in v1 — `put`-update, `remove`, and `clear` are NOT
/// evictions and never invoke the callback.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EvictionCause {
    /// Evicted because an insert exceeded `maximum_size` (the LRU victim).
    Size,
    /// Removed by `expire_entries(now)` because its logical expiry tick passed.
    Expired,
}

impl EvictionCause {
    /// The lower-case serialized name used by the cross-language suite.
    pub fn as_str(self) -> &'static str {
        match self {
            EvictionCause::Size => "size",
            EvictionCause::Expired => "expired",
        }
    }
}

/// Sentinel index meaning "no node" (list end / free-list end).
const NIL: usize = usize::MAX;

/// The eviction callback type: invoked with `(key, value-at-eviction, cause)`.
type EvictCallback<K> = Box<dyn FnMut(&K, i32, EvictionCause)>;

/// One arena slot: an intrusive doubly-linked-list node. When the slot is live
/// it links into the LRU list (`prev`/`next` are slot indices, `key` is the
/// back-reference into the map). When the slot is free it sits on the free-list
/// (`next` chains the free-list, `prev`/`value`/`expire_at` are dead) and its
/// `key` is `None`: freeing a slot drops the key immediately rather than keeping
/// it alive until the slot is reused.
struct Node<K> {
    prev: usize,
    next: usize,
    /// `Some` while the slot is live, `None` while it is on the free-list.
    key: Option<K>,
    value: i32,
    /// Logical expiry tick: an entry expires when `now >= expire_at`. `u64::MAX`
    /// means "never" (no TTL configured, or `now + ttl` saturated).
    expire_at: u64,
}

/// A fixed-capacity LRU map from `K` to `i32`.
///
/// Constructed with [`BoundedLruMap::with_capacity`] or the
/// [`BoundedLruMap::builder`]. The map holds at most `max_size` entries; a
/// new-key insert that would exceed it evicts the least-recently-used entry
/// first (evict-before-insert), so the inserted key is never its own victim.
pub struct BoundedLruMap<K: Hash + Eq + Clone> {
    /// Key -> arena slot index. The arena slot holds the value + LRU links.
    index: OpenHashMap<K, usize>,
    /// The arena of LRU-list nodes (slot-index addressed).
    arena: Vec<Node<K>>,
    /// Free-list head (a slot index), or `NIL` when no free slot is available.
    free_head: usize,
    /// LRU-list head = least-recently-used (the eviction victim), or `NIL`.
    head: usize,
    /// LRU-list tail = most-recently-used, or `NIL`.
    tail: usize,
    /// Capacity `n` (`0` ⇒ the map is permanently empty; every insert drops).
    max_size: usize,
    /// After-write TTL in logical ticks, or `None` for a pure max-size map.
    ttl: Option<u64>,
    /// Optional recording/eviction callback `(key, value, cause)`.
    on_evict: Option<EvictCallback<K>>,
}

/// Builder for [`BoundedLruMap`] — `max_size` is required; `ttl` and `on_evict`
/// are optional.
pub struct BoundedLruMapBuilder<K: Hash + Eq + Clone> {
    max_size: usize,
    ttl: Option<u64>,
    on_evict: Option<EvictCallback<K>>,
}

impl<K: Hash + Eq + Clone> BoundedLruMapBuilder<K> {
    /// Set the capacity `n` (the maximum number of resident entries).
    pub fn max_size(mut self, n: usize) -> Self {
        self.max_size = n;
        self
    }

    /// Set the after-write TTL (logical ticks). `expire_at = saturating(now + ttl)`.
    pub fn ttl(mut self, ttl: u64) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Install the eviction callback. It is invoked once per evicted entry with
    /// `(key, value-at-eviction, cause)`, synchronously, for causes `Size` and
    /// `Expired` only. It MUST NOT mutate the map (Rust's borrow rules prevent
    /// re-entrant mutation at compile time).
    pub fn on_evict<F: FnMut(&K, i32, EvictionCause) + 'static>(mut self, cb: F) -> Self {
        self.on_evict = Some(Box::new(cb));
        self
    }

    /// Build the map.
    pub fn build(self) -> BoundedLruMap<K> {
        BoundedLruMap {
            index: OpenHashMap::new(),
            arena: Vec::new(),
            free_head: NIL,
            head: NIL,
            tail: NIL,
            max_size: self.max_size,
            ttl: self.ttl,
            on_evict: self.on_evict,
        }
    }
}

impl<K: Hash + Eq + Clone> BoundedLruMap<K> {
    /// A pure max-size LRU map of capacity `n` (no TTL, no callback).
    pub fn with_capacity(n: usize) -> Self {
        Self::builder().max_size(n).build()
    }

    /// Start building a map. `max_size` defaults to `0` (drop-everything) until
    /// set; call `.max_size(n)` to fix the capacity.
    pub fn builder() -> BoundedLruMapBuilder<K> {
        BoundedLruMapBuilder {
            max_size: 0,
            ttl: None,
            on_evict: None,
        }
    }

    /// Current entry count (`0 ..= max_size`).
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// The configured capacity `n`.
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    // --- arena / intrusive-list primitives (non-observable) ---------------

    /// Allocate a slot for a fresh entry (reusing a free slot if available),
    /// returning its index. The node is NOT yet linked into the LRU list.
    fn alloc_node(&mut self, key: K, value: i32, expire_at: u64) -> usize {
        if self.free_head != NIL {
            let idx = self.free_head;
            self.free_head = self.arena[idx].next;
            let node = &mut self.arena[idx];
            node.prev = NIL;
            node.next = NIL;
            node.key = Some(key);
            node.value = value;
            node.expire_at = expire_at;
            idx
        } else {
            let idx = self.arena.len();
            self.arena.push(Node {
                prev: NIL,
                next: NIL,
                key: Some(key),
                value,
                expire_at,
            });
            idx
        }
    }

    /// Return a slot to the free-list (the node must already be unlinked from
    /// the LRU list). Clears the slot's ownership of its key immediately and
    /// returns the key to the caller (e.g. for index removal and the eviction
    /// callback), so a freed slot never keeps a key alive until reuse.
    fn free_node(&mut self, idx: usize) -> Option<K> {
        let node = &mut self.arena[idx];
        node.next = self.free_head;
        node.prev = NIL;
        self.free_head = idx;
        node.key.take()
    }

    /// Unlink a node from the LRU list (O(1)); leaves the slot allocated.
    fn unlink(&mut self, idx: usize) {
        let (prev, next) = {
            let n = &self.arena[idx];
            (n.prev, n.next)
        };
        if prev != NIL {
            self.arena[prev].next = next;
        } else {
            self.head = next;
        }
        if next != NIL {
            self.arena[next].prev = prev;
        } else {
            self.tail = prev;
        }
        let n = &mut self.arena[idx];
        n.prev = NIL;
        n.next = NIL;
    }

    /// Push a (currently unlinked) node onto the MRU end (tail) of the LRU list.
    fn push_tail(&mut self, idx: usize) {
        let old_tail = self.tail;
        self.arena[idx].prev = old_tail;
        self.arena[idx].next = NIL;
        if old_tail != NIL {
            self.arena[old_tail].next = idx;
        } else {
            self.head = idx;
        }
        self.tail = idx;
    }

    /// Move an existing live node to the MRU end (a recency refresh).
    fn touch(&mut self, idx: usize) {
        if self.tail == idx {
            return; // already MRU
        }
        self.unlink(idx);
        self.push_tail(idx);
    }

    // --- map surface ------------------------------------------------------

    /// `put(k, v)` == `put_at(k, v, 0)` (no hidden clock). On a no-TTL map the
    /// `now` is irrelevant; on a TTL map this writes with `now = 0`.
    pub fn put(&mut self, key: K, value: i32) -> Option<i32> {
        self.put_at(key, value, 0)
    }

    /// Insert-or-update with a logical write tick. Refreshes recency of `key`;
    /// a new-key insert at capacity evicts the LRU entry first
    /// (evict-before-insert). Returns the previous value, or `None`.
    pub fn put_at(&mut self, key: K, value: i32, now: u64) -> Option<i32> {
        let expire_at = match self.ttl {
            Some(ttl) => now.saturating_add(ttl),
            None => u64::MAX,
        };

        if let Some(&idx) = self.index.get(&key) {
            // Update: value replaced, expiry reset, recency refreshed; NO evict.
            let old = self.arena[idx].value;
            self.arena[idx].value = value;
            self.arena[idx].expire_at = expire_at;
            self.touch(idx);
            return Some(old);
        }

        // Genuine insertion of a new key.
        if self.max_size == 0 {
            // Capacity 0: the entry is dropped, never resident, no callback.
            return None;
        }

        // Evict-before-insert: the invariant `len() <= max_size` is maintained
        // on every op, so a new-key insert raises size by one and needs AT MOST
        // ONE SIZE eviction when max_size >= 1 (spec §"At most one eviction per
        // put"). The `if` (not a loop) makes that one-eviction contract explicit.
        debug_assert!(self.len() <= self.max_size);
        if self.len() >= self.max_size {
            let victim = self.head; // LRU end; always valid since len() >= 1.
            self.evict_node(victim, EvictionCause::Size);
        }

        // User Clone/Hash/Eq may panic: run them (clone, then the index insert,
        // which may also grow the index) before the arena and list change. The
        // node's slot is peeked here and committed by `alloc_node` below, which
        // runs no user code.
        let node_key = key.clone();
        let idx = if self.free_head != NIL {
            self.free_head
        } else {
            self.arena.len()
        };
        self.index.insert(key, idx);
        let allocated = self.alloc_node(node_key, value, expire_at);
        debug_assert_eq!(allocated, idx);
        self.push_tail(idx);
        None
    }

    /// Lookup. On a hit refreshes recency; on a miss does nothing.
    pub fn get(&mut self, key: &K) -> Option<i32> {
        if let Some(&idx) = self.index.get(key) {
            let v = self.arena[idx].value;
            self.touch(idx);
            Some(v)
        } else {
            None
        }
    }

    /// A `get` that returns `default` on a miss. A hit refreshes recency exactly
    /// like `get`; a miss does NOT refresh recency and does NOT insert `default`.
    pub fn get_or_default(&mut self, key: &K, default: i32) -> i32 {
        self.get(key).unwrap_or(default)
    }

    /// Membership test. Does NOT refresh recency and never evicts.
    pub fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    /// Delete `key`. Does not evict and does NOT invoke the eviction callback
    /// (manual removal is not an eviction). Returns the removed value, or `None`.
    pub fn remove(&mut self, key: &K) -> Option<i32> {
        if let Some(idx) = self.index.remove(key) {
            let v = self.arena[idx].value;
            self.unlink(idx);
            self.free_node(idx);
            Some(v)
        } else {
            None
        }
    }

    /// Remove all entries. Does NOT invoke the eviction callback for the cleared
    /// entries (bulk manual removal is not eviction).
    pub fn clear(&mut self) {
        self.index.clear();
        self.arena.clear();
        self.free_head = NIL;
        self.head = NIL;
        self.tail = NIL;
    }

    /// Remove a victim node entirely (index-remove + unlink + free) and fire the
    /// eviction callback with the given cause and the value-at-eviction.
    fn evict_node(&mut self, idx: usize, cause: EvictionCause) {
        let value = self.arena[idx].value;
        // User Hash/Eq may panic: resolve the lookup before changing links
        // or putting the live node on the free list.
        self.index.remove(Self::live_key(&self.arena[idx]));
        self.unlink(idx);
        let key = self.free_node(idx).expect("live LRU node has a key");
        if let Some(cb) = self.on_evict.as_mut() {
            cb(&key, value, cause);
        }
    }

    /// Logical-time expiry pass: remove every entry with `expire_at <= now`
    /// (inclusive), firing the callback with cause `Expired` in ascending
    /// `expire_at`, then ascending `last_use` (LRU) order. Returns the count
    /// removed. The only time-driven eviction; surviving entries' recency is
    /// unchanged.
    pub fn expire_entries(&mut self, now: u64) -> usize {
        // Collect victims: every live node with expire_at <= now. Order them by
        // (expire_at asc, last_use asc). last_use is position-implicit: walk the
        // LRU list head->tail to get ascending last_use, and use a STABLE sort
        // by expire_at so equal-expire_at entries keep that ascending-LRU order.
        // No-TTL map: expire_at is conceptually +∞ for every entry; nothing
        // expires for any `now` (the map is a pure max-size LRU).
        if self.ttl.is_none() {
            return 0;
        }
        let mut victims: Vec<(u64, usize)> = Vec::new();
        let mut cur = self.head;
        while cur != NIL {
            let n = &self.arena[cur];
            let next = n.next;
            // `u64::MAX` is the saturated "+∞" sentinel: such an entry never
            // expires, even at `now == u64::MAX` (spec §TTL saturation).
            if n.expire_at != u64::MAX && n.expire_at <= now {
                victims.push((n.expire_at, cur));
            }
            cur = next;
        }
        // Stable sort by expire_at preserves the head->tail (ascending last_use)
        // order within each expire_at tie.
        victims.sort_by_key(|&(expire_at, _)| expire_at);
        let count = victims.len();
        for (_, idx) in victims {
            self.evict_node(idx, EvictionCause::Expired);
        }
        count
    }

    // --- iteration (LRU order, read-only snapshots) -----------------------

    /// The key of a node reached by walking the LRU list (always live).
    fn live_key(n: &Node<K>) -> &K {
        n.key.as_ref().expect("live LRU node has a key")
    }

    /// All keys in LRU order (least-recently-used first). A read-only snapshot:
    /// does NOT refresh recency and never evicts.
    pub fn keys(&self) -> Vec<K> {
        let mut out = Vec::with_capacity(self.len());
        let mut cur = self.head;
        while cur != NIL {
            out.push(Self::live_key(&self.arena[cur]).clone());
            cur = self.arena[cur].next;
        }
        out
    }

    /// All values in LRU order, parallel to [`keys`](Self::keys). Read-only
    /// snapshot.
    pub fn values(&self) -> Vec<i32> {
        let mut out = Vec::with_capacity(self.len());
        let mut cur = self.head;
        while cur != NIL {
            out.push(self.arena[cur].value);
            cur = self.arena[cur].next;
        }
        out
    }

    /// All `(key, value)` entries in LRU order. Read-only snapshot.
    pub fn entries(&self) -> Vec<(K, i32)> {
        let mut out = Vec::with_capacity(self.len());
        let mut cur = self.head;
        while cur != NIL {
            let n = &self.arena[cur];
            out.push((Self::live_key(n).clone(), n.value));
            cur = n.next;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct PanicKey {
        id: i32,
        armed: Rc<std::cell::Cell<i32>>,
        equality: bool,
    }

    impl std::hash::Hash for PanicKey {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            assert!(
                self.equality || self.armed.get() != self.id,
                "armed victim hash"
            );
            0_i32.hash(state); // force Eq probes and backward-shift collisions
        }
    }

    impl PartialEq for PanicKey {
        fn eq(&self, other: &Self) -> bool {
            assert!(
                !self.equality || self.armed.get() != self.id || self.id != other.id,
                "armed victim equality"
            );
            self.id == other.id
        }
    }
    impl Eq for PanicKey {}

    /// `armed_id` 1 arms the victim's own lookup; 2 arms the surviving key
    /// that collides with it, whose Hash the index needs to backward-shift
    /// after deleting the victim (fable72 M1).
    fn exercise_eviction_lookup_panic(equality: bool, expiry: bool, armed_id: i32) {
        let armed = Rc::new(std::cell::Cell::new(-1));
        let key = |id| PanicKey {
            id,
            armed: armed.clone(),
            equality,
        };
        let log = Rc::new(RefCell::new(Vec::new()));
        let recording = log.clone();
        let mut builder = BoundedLruMap::builder()
            .max_size(2)
            .on_evict(move |k: &PanicKey, v, c| recording.borrow_mut().push((k.id, v, c)));
        if expiry {
            builder = builder.ttl(5);
        }
        let mut map = builder.build();
        map.put_at(key(1), 10, 0);
        map.put_at(key(2), 20, 4);
        let links = (map.head, map.tail, map.free_head, map.arena.len());
        armed.set(armed_id);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if expiry {
                map.expire_entries(5);
            } else {
                map.put(key(3), 30);
            }
        }));
        armed.set(-1);
        assert!(
            caught.is_err(),
            "victim lookup must exercise the intended panic"
        );
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.keys().iter().map(|k| k.id).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(map.values(), vec![10, 20]);
        assert_eq!((map.head, map.tail, map.free_head, map.arena.len()), links);
        assert!(map.contains_key(&key(1)) && map.contains_key(&key(2)));
        assert!(!map.contains_key(&key(3)));
        assert!(log.borrow().is_empty());
        if expiry {
            assert_eq!(map.expire_entries(5), 1);
            map.put_at(key(3), 30, 5);
        } else {
            map.put(key(3), 30);
        }
        assert_eq!(
            map.keys().iter().map(|k| k.id).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert_eq!(map.values(), vec![20, 30]);
        assert_eq!(map.arena.len(), 2, "successful retry reuses a freed slot");
        assert_eq!(map.len(), 2);
        assert!(!map.contains_key(&key(1)));
        assert_eq!(map.get(&key(3)), Some(30));
        assert_eq!(map.get(&key(2)), Some(20));
        assert_eq!(
            *log.borrow(),
            vec![(
                1,
                10,
                if expiry {
                    EvictionCause::Expired
                } else {
                    EvictionCause::Size
                }
            )]
        );
    }

    #[test]
    fn size_eviction_hash_panic_preserves_state() {
        exercise_eviction_lookup_panic(false, false, 1);
    }
    #[test]
    fn size_eviction_eq_panic_preserves_state() {
        exercise_eviction_lookup_panic(true, false, 1);
    }
    #[test]
    fn expiry_eviction_hash_panic_preserves_state() {
        exercise_eviction_lookup_panic(false, true, 1);
    }
    #[test]
    fn expiry_eviction_eq_panic_preserves_state() {
        exercise_eviction_lookup_panic(true, true, 1);
    }

    #[test]
    fn size_eviction_survivor_hash_panic_preserves_state() {
        exercise_eviction_lookup_panic(false, false, 2);
    }
    #[test]
    fn expiry_eviction_survivor_hash_panic_preserves_state() {
        exercise_eviction_lookup_panic(false, true, 2);
    }

    /// A resident key's Hash panics while the index grows for a new key. The
    /// new node must not be linked into the LRU list without an index entry,
    /// and the index must keep every resident key.
    #[test]
    fn index_growth_hash_panic_preserves_state() {
        let armed = Rc::new(std::cell::Cell::new(-1));
        let key = |id| PanicKey {
            id,
            armed: armed.clone(),
            equality: false,
        };
        let mut map = BoundedLruMap::with_capacity(20);
        // The 12th insert grows the default 16-slot index.
        for id in 1..=11 {
            map.put(key(id), id * 10);
        }
        let links = (map.head, map.tail, map.free_head, map.arena.len());
        armed.set(5);
        let caught =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| map.put(key(12), 120)));
        armed.set(-1);
        assert!(caught.is_err(), "index growth must hash the armed key");
        let ids: Vec<i32> = (1..=11).collect();
        assert_eq!(map.len(), 11);
        assert_eq!(map.keys().iter().map(|k| k.id).collect::<Vec<_>>(), ids);
        assert_eq!((map.head, map.tail, map.free_head, map.arena.len()), links);
        for &id in &ids {
            assert!(map.contains_key(&key(id)), "key {id} unreachable");
        }
        assert!(!map.contains_key(&key(12)));
        map.put(key(12), 120);
        assert_eq!(map.len(), 12);
        assert_eq!(map.get(&key(12)), Some(120));
        assert_eq!(map.get(&key(5)), Some(50));
    }

    #[test]
    fn expiry_panic_retains_consistency_after_completed_victim() {
        let armed = Rc::new(std::cell::Cell::new(-1));
        let key = |id| PanicKey {
            id,
            armed: armed.clone(),
            equality: true,
        };
        let log = Rc::new(RefCell::new(Vec::new()));
        let recording = log.clone();
        let mut map = BoundedLruMap::builder()
            .max_size(3)
            .ttl(5)
            .on_evict(move |k: &PanicKey, v, c| recording.borrow_mut().push((k.id, v, c)))
            .build();
        for id in 1..=3 {
            map.put_at(key(id), id * 10, (id - 1) as u64);
        }
        armed.set(2);
        let caught =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| map.expire_entries(7)));
        armed.set(-1);
        assert!(caught.is_err());
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.keys().iter().map(|k| k.id).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert_eq!(map.values(), vec![20, 30]);
        assert!(!map.contains_key(&key(1)));
        assert!(map.contains_key(&key(2)) && map.contains_key(&key(3)));
        assert_eq!(*log.borrow(), vec![(1, 10, EvictionCause::Expired)]);
        assert_eq!(map.expire_entries(7), 2);
        assert!(map.is_empty());
        map.put_at(key(4), 40, 8);
        assert_eq!(map.get(&key(4)), Some(40));
        assert_eq!(map.arena.len(), 3);
        assert_eq!(
            *log.borrow(),
            vec![
                (1, 10, EvictionCause::Expired),
                (2, 20, EvictionCause::Expired),
                (3, 30, EvictionCause::Expired)
            ]
        );
    }

    type Log = Rc<RefCell<Vec<(i32, i32, EvictionCause)>>>;
    /// (keys, values, eviction-log) snapshot returned by the determinism replay.
    type ReplaySnapshot = (Vec<i32>, Vec<i32>, Vec<(i32, i32, EvictionCause)>);

    fn map_with_log(n: usize) -> (BoundedLruMap<i32>, Log) {
        let log: Log = Rc::new(RefCell::new(Vec::new()));
        let l2 = log.clone();
        let map = BoundedLruMap::<i32>::builder()
            .max_size(n)
            .on_evict(move |k, v, c| l2.borrow_mut().push((*k, v, c)))
            .build();
        (map, log)
    }

    fn map_with_log_ttl(n: usize, ttl: u64) -> (BoundedLruMap<i32>, Log) {
        let log: Log = Rc::new(RefCell::new(Vec::new()));
        let l2 = log.clone();
        let map = BoundedLruMap::<i32>::builder()
            .max_size(n)
            .ttl(ttl)
            .on_evict(move |k, v, c| l2.borrow_mut().push((*k, v, c)))
            .build();
        (map, log)
    }

    #[test]
    fn evict_basic_victim_is_lru() {
        let (mut m, log) = map_with_log(2);
        m.put(1, 10);
        m.put(2, 20);
        m.put(3, 30); // evicts 1 (LRU)
        assert_eq!(m.keys(), vec![2, 3]);
        assert_eq!(m.values(), vec![20, 30]);
        assert_eq!(*log.borrow(), vec![(1, 10, EvictionCause::Size)]);
    }

    #[test]
    fn get_refreshes_recency() {
        let (mut m, log) = map_with_log(2);
        m.put(1, 10);
        m.put(2, 20);
        assert_eq!(m.get(&1), Some(10)); // 1 now MRU, 2 is LRU
        m.put(3, 30); // evicts 2
        assert_eq!(m.keys(), vec![1, 3]);
        assert_eq!(*log.borrow(), vec![(2, 20, EvictionCause::Size)]);
    }

    #[test]
    fn get_or_default_hit_refreshes_miss_does_not() {
        let (mut m, _log) = map_with_log(2);
        m.put(1, 10);
        m.put(2, 20);
        assert_eq!(m.get_or_default(&1, -1), 10); // hit: 1 MRU
        assert_eq!(m.get_or_default(&99, -1), -1); // miss: no insert, no refresh
        assert_eq!(m.len(), 2);
        assert!(!m.contains_key(&99));
        m.put(3, 30); // evicts 2
        assert_eq!(m.keys(), vec![1, 3]);
    }

    #[test]
    fn contains_key_does_not_refresh() {
        let (mut m, log) = map_with_log(2);
        m.put(1, 10);
        m.put(2, 20);
        assert!(m.contains_key(&1)); // must NOT refresh 1
        m.put(3, 30); // evicts 1 (still LRU)
        assert_eq!(m.keys(), vec![2, 3]);
        assert_eq!(*log.borrow(), vec![(1, 10, EvictionCause::Size)]);
    }

    #[test]
    fn update_at_capacity_does_not_evict() {
        let (mut m, log) = map_with_log(2);
        m.put(1, 10);
        m.put(2, 20);
        assert_eq!(m.put(1, 11), Some(10)); // update: no evict, 1 becomes MRU
        assert!(log.borrow().is_empty());
        assert_eq!(m.keys(), vec![2, 1]);
        m.put(3, 30); // now evicts 2 (LRU)
        assert_eq!(m.keys(), vec![1, 3]);
        assert_eq!(*log.borrow(), vec![(2, 20, EvictionCause::Size)]);
    }

    #[test]
    fn iteration_does_not_refresh_or_evict() {
        let (mut m, log) = map_with_log(2);
        m.put(1, 10);
        m.put(2, 20);
        let snap = m.keys(); // snapshot must not touch recency
        assert_eq!(snap, vec![1, 2]);
        assert!(log.borrow().is_empty());
        m.put(3, 30); // 1 still LRU -> evicted
        assert_eq!(m.keys(), vec![2, 3]);
        assert_eq!(*log.borrow(), vec![(1, 10, EvictionCause::Size)]);
    }

    #[test]
    fn remove_no_callback_no_other_recency_change() {
        let (mut m, log) = map_with_log(3);
        m.put(1, 10);
        m.put(2, 20);
        m.put(3, 30);
        assert_eq!(m.remove(&2), Some(20));
        assert!(log.borrow().is_empty());
        assert_eq!(m.keys(), vec![1, 3]);
        m.put(4, 40);
        m.put(5, 50); // capacity 3: now full {1,3,4} -> 5 evicts 1
        assert_eq!(m.keys(), vec![3, 4, 5]);
        assert_eq!(*log.borrow(), vec![(1, 10, EvictionCause::Size)]);
    }

    #[test]
    fn clear_fires_no_callback() {
        let (mut m, log) = map_with_log(3);
        m.put(1, 10);
        m.put(2, 20);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
        assert!(log.borrow().is_empty());
        // After clear the arena/free-list is sane: reuse works.
        m.put(7, 70);
        assert_eq!(m.keys(), vec![7]);
    }

    #[test]
    fn capacity_zero_drops_everything() {
        let (mut m, log) = map_with_log(0);
        assert_eq!(m.put(1, 10), None);
        assert_eq!(m.put(2, 20), None);
        assert_eq!(m.put(3, 30), None);
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
        assert_eq!(m.get(&1), None);
        assert!(log.borrow().is_empty()); // nothing was ever resident
    }

    #[test]
    fn capacity_one_evicts_then_inserts() {
        let (mut m, log) = map_with_log(1);
        m.put(1, 10);
        m.put(2, 20); // evicts 1
        assert_eq!(m.keys(), vec![2]);
        assert_eq!(*log.borrow(), vec![(1, 10, EvictionCause::Size)]);
        assert_eq!(m.put(2, 22), Some(20)); // update: no new log entry
        assert_eq!(log.borrow().len(), 1);
        assert_eq!(m.keys(), vec![2]);
        assert_eq!(m.values(), vec![22]);
    }

    #[test]
    fn evict_before_insert_new_key_never_self_victim() {
        let (mut m, log) = map_with_log(1);
        m.put(1, 10);
        m.put(2, 20); // 2 inserted, 1 evicted — 2 is never its own victim
        assert!(m.contains_key(&2));
        assert!(!m.contains_key(&1));
        assert_eq!(*log.borrow(), vec![(1, 10, EvictionCause::Size)]);
    }

    #[test]
    fn same_now_recency_uses_useseq_not_now() {
        let (mut m, log) = map_with_log_ttl(2, 100);
        m.put_at(1, 10, 5);
        m.put_at(2, 20, 5); // both written at now=5
        assert_eq!(m.get(&1), Some(10)); // 1 refreshed -> 2 is LRU
        m.put_at(3, 30, 5); // evicts 2, NOT 1 (recency != now)
        assert_eq!(m.keys(), vec![1, 3]);
        assert_eq!(*log.borrow(), vec![(2, 20, EvictionCause::Size)]);
    }

    #[test]
    fn expire_basic_inclusive() {
        let (mut m, log) = map_with_log_ttl(10, 10);
        m.put_at(1, 10, 0); // expire_at 10
        m.put_at(2, 20, 0); // expire_at 10
        m.put_at(3, 30, 5); // expire_at 15
        let n = m.expire_entries(10); // 1,2 expire (<=10), 3 survives
        assert_eq!(n, 2);
        assert_eq!(m.keys(), vec![3]);
        assert_eq!(
            *log.borrow(),
            vec![
                (1, 10, EvictionCause::Expired),
                (2, 20, EvictionCause::Expired),
            ]
        );
    }

    #[test]
    fn expire_tiebreak_by_last_use() {
        // Several entries share one expire_at; their last_use differs by access
        // order. The EXPIRED order is ascending last_use among the tie.
        let (mut m, log) = map_with_log_ttl(10, 10);
        m.put_at(1, 10, 0);
        m.put_at(2, 20, 0);
        m.put_at(3, 30, 0); // all expire_at 10
                            // Touch order so last_use ascending becomes 2, 3, 1.
        m.get(&2); // 2 most-recent so far
        m.get(&3);
        m.get(&1); // now last_use order (asc): 2 < 3 < 1
        let n = m.expire_entries(10);
        assert_eq!(n, 3);
        assert_eq!(
            *log.borrow(),
            vec![
                (2, 20, EvictionCause::Expired),
                (3, 30, EvictionCause::Expired),
                (1, 10, EvictionCause::Expired),
            ]
        );
    }

    #[test]
    fn expire_orders_by_expire_at_then_last_use() {
        let (mut m, log) = map_with_log_ttl(10, 0);
        m.put_at(1, 10, 5); // expire_at 5
        m.put_at(2, 20, 3); // expire_at 3
        m.put_at(3, 30, 5); // expire_at 5
        m.put_at(4, 40, 3); // expire_at 3
                            // Among expire_at=3: last_use order is 2 then 4 (insert order, no touch).
                            // Among expire_at=5: 1 then 3.
        let n = m.expire_entries(5);
        assert_eq!(n, 4);
        assert_eq!(
            *log.borrow(),
            vec![
                (2, 20, EvictionCause::Expired),
                (4, 40, EvictionCause::Expired),
                (1, 10, EvictionCause::Expired),
                (3, 30, EvictionCause::Expired),
            ]
        );
    }

    #[test]
    fn expire_inclusive_and_saturation() {
        let (mut m, log) = map_with_log_ttl(10, u64::MAX);
        // ttl is huge: now + ttl saturates to u64::MAX => never expires.
        m.put_at(1, 10, 5); // expire_at saturates to u64::MAX
                            // A second entry written with a small explicit expire via ttl=0 map is
                            // a different config; here we test saturation only.
        let n = m.expire_entries(u64::MAX - 1);
        assert_eq!(n, 0);
        assert!(m.contains_key(&1));
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn ttl_zero_boundary() {
        let (mut m, _log) = map_with_log_ttl(10, 0);
        m.put_at(1, 10, 5); // expire_at = 5
        assert_eq!(m.expire_entries(4), 0); // 5 > 4: survives
        assert!(m.contains_key(&1));
        assert_eq!(m.expire_entries(5), 1); // 5 <= 5: removed (inclusive)
        assert!(!m.contains_key(&1));
    }

    #[test]
    fn no_ttl_pure_lru() {
        let (mut m, _log) = map_with_log(2); // no ttl
        m.put(1, 10);
        m.put(2, 20);
        assert_eq!(m.expire_entries(u64::MAX), 0); // nothing expires
        assert_eq!(m.keys(), vec![1, 2]);
    }

    #[test]
    fn u64_max_expire_at_is_the_never_sentinel() {
        // INTENTIONAL EDGE (spec §"TTL saturation / unsigned ticks"): a computed
        // `expire_at == u64::MAX` is the "+∞ / never" sentinel — whether it got
        // there by `now + ttl` saturating OR by landing on u64::MAX exactly. So
        // an entry whose write produced u64::MAX never expires, even at the
        // maximum `now`. The spec frames any overflowing/maxed `now + ttl` as
        // "effectively never"; this pins that reading so all ports agree.
        let (mut m, log) = map_with_log_ttl(10, 1);
        m.put_at(1, 10, u64::MAX - 1); // now + ttl = u64::MAX exactly -> sentinel
        assert_eq!(m.expire_entries(u64::MAX), 0); // never expires (sentinel)
        assert!(m.contains_key(&1));
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn expire_then_size_interaction() {
        let (mut m, log) = map_with_log_ttl(2, 10);
        m.put_at(1, 10, 0); // expire_at 10
        m.put_at(2, 20, 0); // expire_at 10
        assert_eq!(m.expire_entries(10), 2); // both expire; map empty
        assert!(m.is_empty());
        m.put_at(3, 30, 20); // below capacity now: no SIZE eviction
        assert_eq!(m.keys(), vec![3]);
        // Only the two EXPIRED entries are logged; no SIZE eviction.
        assert_eq!(
            *log.borrow(),
            vec![
                (1, 10, EvictionCause::Expired),
                (2, 20, EvictionCause::Expired),
            ]
        );
    }

    #[test]
    fn update_before_expire_resets_expiry_and_value() {
        let (mut m, log) = map_with_log_ttl(10, 10);
        m.put_at(1, 10, 0); // expire_at 10
        assert_eq!(m.put_at(1, 11, 5), Some(10)); // update: value 11, expire_at 15
        assert_eq!(m.expire_entries(10), 0); // survives (15 > 10)
        assert!(m.contains_key(&1));
        assert_eq!(m.expire_entries(15), 1); // now expires with UPDATED value 11
        assert_eq!(*log.borrow(), vec![(1, 11, EvictionCause::Expired)]);
    }

    #[test]
    fn miss_does_not_refresh_or_insert() {
        let (mut m, _log) = map_with_log(2);
        m.put(1, 10);
        m.put(2, 20); // {1(LRU), 2}
        assert_eq!(m.get(&99), None); // miss
        assert_eq!(m.get_or_default(&99, -1), -1); // miss, no insert
        assert_eq!(m.len(), 2);
        m.put(4, 40); // 1 still LRU -> evicted
        assert_eq!(m.keys(), vec![2, 4]);
    }

    #[test]
    fn remove_reinsert_gets_fresh_recency() {
        let (mut m, log) = map_with_log(3);
        m.put(1, 10);
        m.put(2, 20);
        m.put(3, 30); // {1,2,3}
        m.remove(&1);
        m.put(1, 11); // fresh insert: 1 is MRU now, order {2,3,1}
        m.put(4, 40); // evicts 2 (LRU)
        assert_eq!(m.keys(), vec![3, 1, 4]);
        assert_eq!(*log.borrow(), vec![(2, 20, EvictionCause::Size)]);
    }

    #[test]
    fn slot_reuse_after_eviction_no_dangling() {
        // Force many evictions so the free-list is exercised heavily; the map
        // must stay correct (no dangling slot index reused while still live).
        let (mut m, _log) = map_with_log(3);
        for k in 0..1000 {
            m.put(k, k * 10);
            // Invariant: size never exceeds capacity; every resident key maps
            // to its own value.
            assert!(m.len() <= 3);
        }
        // The last three keys inserted must be resident in LRU order.
        assert_eq!(m.keys(), vec![997, 998, 999]);
        assert_eq!(m.values(), vec![9970, 9980, 9990]);
        // Arena never grows past capacity + freed slots reused (<= a few slots).
        assert!(m.arena.len() <= 4);
    }

    #[test]
    fn tie_free_determinism_over_random_sequence() {
        // A deterministic pseudo-random op sequence: replaying it twice must
        // give identical contents (the model is tie-free by construction).
        fn replay() -> ReplaySnapshot {
            let log: Log = Rc::new(RefCell::new(Vec::new()));
            let l2 = log.clone();
            let mut m = BoundedLruMap::<i32>::builder()
                .max_size(5)
                .on_evict(move |k, v, c| l2.borrow_mut().push((*k, v, c)))
                .build();
            let mut state: u64 = 0x1234_5678;
            for _ in 0..2000 {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let k = ((state >> 33) % 20) as i32;
                match (state >> 30) & 3 {
                    0 => {
                        m.put(k, k * 100);
                    }
                    1 => {
                        m.get(&k);
                    }
                    2 => {
                        m.contains_key(&k);
                    }
                    _ => {
                        m.remove(&k);
                    }
                }
            }
            let result = (m.keys(), m.values(), log.borrow().clone());
            drop(m);
            result
        }
        let a = replay();
        let b = replay();
        assert_eq!(a, b);
    }

    /// F23: a freed arena slot must not keep the key alive until reuse —
    /// `remove` and eviction drop both map-held copies of the key at once.
    #[test]
    fn freed_slot_drops_key_immediately() {
        let mut m = BoundedLruMap::<Rc<String>>::with_capacity(2);
        let a = Rc::new("a".to_string());
        let b = Rc::new("b".to_string());
        let c = Rc::new("c".to_string());
        m.put(a.clone(), 1);
        m.put(b.clone(), 2);
        assert_eq!(Rc::strong_count(&a), 3); // test + index + arena slot

        // Manual removal: the slot goes on the free-list and is not reused yet.
        assert_eq!(m.remove(&a), Some(1));
        assert_eq!(Rc::strong_count(&a), 1);

        // Eviction path (TTL expiry frees the slot without reusing it).
        let mut t = BoundedLruMap::<Rc<String>>::builder()
            .max_size(4)
            .ttl(5)
            .build();
        t.put_at(b.clone(), 2, 0);
        t.put_at(c.clone(), 3, 10);
        assert_eq!(Rc::strong_count(&b), 5); // test + (index + arena) in m and t
        assert_eq!(t.expire_entries(5), 1);
        assert_eq!(Rc::strong_count(&b), 3); // t's slot for b is free and holds nothing
        assert_eq!(t.keys(), vec![c.clone()]);

        // Size eviction: inside the callback (before the freed slot is reused
        // by the incoming key) the arena and index copies are already gone;
        // only the test's handle and the key passed to the callback remain.
        let seen: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
        let seen2 = seen.clone();
        let mut s = BoundedLruMap::<Rc<String>>::builder()
            .max_size(1)
            .on_evict(move |k, _, cause| {
                assert_eq!(cause, EvictionCause::Size);
                seen2.borrow_mut().push(Rc::strong_count(k));
            })
            .build();
        let x = Rc::new("x".to_string());
        s.put(x.clone(), 1);
        assert_eq!(Rc::strong_count(&x), 3); // test + index + arena slot
        s.put(Rc::new("y".to_string()), 2); // evicts x, then reuses its slot
        assert_eq!(*seen.borrow(), vec![2]); // test + callback's key only
        assert_eq!(Rc::strong_count(&x), 1); // released after the callback
    }
}
