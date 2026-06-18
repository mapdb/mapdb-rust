// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Space-Saving — a bounded heavy-hitters / top-k summary tracking at most `m`
//! monitored `(item, count, error)` triples with a deterministic eviction rule
//! (see `spec/features/count-min.md`).
//!
//! This is the **reference port**. Unlike the Count-Min Sketch, Space-Saving is
//! **order-DEPENDENT** (eviction depends on which item is the current min when
//! the set is full, which depends on add order). For an identical capacity `m`
//! and an identical add-sequence **in the same order**, the monitored set in
//! canonical order is bit-identical across all five ports. **No floating point**
//! appears in any asserted value.
//!
//! Pinned rulings:
//! - **Eviction tie-break:** the victim is the monitored item minimizing
//!   `(count, signed-i32 item)` — smallest count, then smallest **signed** i32
//!   item (`INT_MIN` < … < `-1` < `0` < `1`). `error` is NOT part of the
//!   tie-break.
//! - **Error accounting:** a displaced new item gets
//!   `count = evicted_count + count`, `error = evicted_count`; an
//!   already-monitored item's `error` NEVER changes; a freshly-admitted (room)
//!   item has `error = 0`.
//! - **Saturating add** at `u64::MAX` (does NOT wrap).
//! - **Canonical order:** `count` DESCENDING, then signed `item` ASCENDING (a
//!   total order; `error` rides along but never decides order). `top_k(k)` is
//!   the first `k` of this order; `top_k(size())` == `monitored_set()`.
//! - **`count = 0` add is a no-op** (no admit, no increment, no eviction).

use std::collections::HashMap;

/// A monitored entry's `(count, error)` pair (the item is the map key).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Entry {
    count: u64,
    error: u64,
}

/// A bounded Space-Saving heavy-hitters summary of capacity `m`.
#[derive(Clone, Debug)]
pub struct SpaceSaving {
    capacity: u32,
    monitored: HashMap<i32, Entry>,
}

impl SpaceSaving {
    /// Construct an empty summary monitoring at most `m` items.
    ///
    /// # Panics
    /// `m == 0` is invalid (a zero-capacity summary can monitor nothing; every
    /// `add` would have to evict from an empty set) and traps.
    pub fn with_capacity(m: u32) -> SpaceSaving {
        assert!(m != 0, "SpaceSaving capacity m must be non-zero");
        SpaceSaving {
            capacity: m,
            monitored: HashMap::new(),
        }
    }

    /// Add `item` with weight `count`.
    ///
    /// - `count = 0` is a no-op (no admit, increment, or eviction).
    /// - If `item` is already monitored: its `count` grows (saturating); its
    ///   `error` is unchanged.
    /// - If there is room (`size < m`): admit with `error = 0`.
    /// - If full: evict the `(count, signed item)`-min victim; the new item
    ///   takes `count = evicted_count + count` (saturating) and
    ///   `error = evicted_count`.
    pub fn add(&mut self, item: i32, count: u64) {
        if count == 0 {
            return; // zero-weight add changes nothing.
        }
        if let Some(e) = self.monitored.get_mut(&item) {
            e.count = e.count.saturating_add(count);
            // error unchanged for an already-monitored item.
            return;
        }
        if (self.monitored.len() as u32) < self.capacity {
            self.monitored.insert(item, Entry { count, error: 0 });
            return;
        }
        // Full + unmonitored item: evict the (count, signed item)-min victim.
        let victim = self.argmin_victim();
        let evicted_count = self
            .monitored
            .remove(&victim)
            .expect("victim present")
            .count;
        self.monitored.insert(
            item,
            Entry {
                count: evicted_count.saturating_add(count),
                error: evicted_count,
            },
        );
    }

    /// Convenience for `add(item, 1)`.
    #[inline]
    pub fn add_one(&mut self, item: i32) {
        self.add(item, 1);
    }

    /// The monitored item minimizing `(count, signed item)`: smallest count,
    /// then smallest signed i32 item on a count tie. Items are distinct, so the
    /// victim is unique. Caller guarantees the set is non-empty.
    fn argmin_victim(&self) -> i32 {
        self.monitored
            .iter()
            .min_by(|(ia, ea), (ib, eb)| ea.count.cmp(&eb.count).then(ia.cmp(ib)))
            .map(|(&item, _)| item)
            .expect("argmin over a non-empty monitored set")
    }

    /// The monitored `count` for `item`, or `0` if not monitored.
    pub fn count(&self, item: i32) -> u64 {
        self.monitored.get(&item).map(|e| e.count).unwrap_or(0)
    }

    /// The monitored `error` for `item`, or `0` if not monitored.
    pub fn error(&self, item: i32) -> u64 {
        self.monitored.get(&item).map(|e| e.error).unwrap_or(0)
    }

    /// Whether `item` is currently monitored.
    pub fn is_monitored(&self, item: i32) -> bool {
        self.monitored.contains_key(&item)
    }

    /// The number of currently monitored items (`<= m`).
    #[inline]
    pub fn size(&self) -> u32 {
        self.monitored.len() as u32
    }

    /// The capacity `m`.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// The entire monitored set as `(item, count, error)` triples in canonical
    /// order: `count` DESCENDING, then signed `item` ASCENDING.
    pub fn monitored_set(&self) -> Vec<(i32, u64, u64)> {
        let mut out: Vec<(i32, u64, u64)> = self
            .monitored
            .iter()
            .map(|(&item, e)| (item, e.count, e.error))
            .collect();
        // count DESC, then signed item ASC.
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }

    /// The `k` highest-`count` monitored items in canonical order (the first
    /// `k` of [`SpaceSaving::monitored_set`]). `k > size()` returns all
    /// monitored items (no padding); `k = 0` returns the empty list.
    pub fn top_k(&self, k: u32) -> Vec<(i32, u64, u64)> {
        let mut all = self.monitored_set();
        all.truncate(k as usize);
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admit_under_capacity_no_eviction() {
        let mut s = SpaceSaving::with_capacity(3);
        s.add_one(7);
        s.add_one(7);
        s.add_one(-1);
        assert_eq!(s.size(), 2);
        assert_eq!(s.count(7), 2);
        assert_eq!(s.error(7), 0);
        assert_eq!(s.count(-1), 1);
        // 7 (count 2) before -1 (count 1); both error 0.
        assert_eq!(s.monitored_set(), vec![(7, 2, 0), (-1, 1, 0)]);
        assert_eq!(s.top_k(1), vec![(7, 2, 0)]);
    }

    #[test]
    fn evict_min_tiebreak_smaller_signed_item() {
        // capacity 2: add(1), add(2) -> both count 1 (full); add(3) evicts the
        // min-count item; tie -> smallest signed item = 1 evicted. 3 admitted
        // with count = 1 + 1 = 2, error = 1.
        let mut s = SpaceSaving::with_capacity(2);
        s.add_one(1);
        s.add_one(2);
        s.add_one(3);
        assert_eq!(s.monitored_set(), vec![(3, 2, 1), (2, 1, 0)]);
        assert_eq!(s.count(1), 0); // evicted -> 0
        assert!(!s.is_monitored(1));
        assert_eq!(s.count(3), 2);
        assert_eq!(s.error(3), 1);
    }

    #[test]
    fn evict_tiebreak_negative_beats_positive() {
        // Monitored -5 (count 1) and 2 (count 1); add a new item -> -5 < 2
        // (SIGNED), so -5 is evicted (an unsigned comparison would evict 2).
        let mut s = SpaceSaving::with_capacity(2);
        s.add_one(-5);
        s.add_one(2);
        s.add_one(9);
        assert!(!s.is_monitored(-5)); // smaller signed item evicted
        assert!(s.is_monitored(2));
        assert!(s.is_monitored(9));
        assert_eq!(s.count(9), 2);
        assert_eq!(s.error(9), 1);
    }

    #[test]
    fn already_monitored_error_never_changes() {
        let mut s = SpaceSaving::with_capacity(2);
        s.add_one(1);
        s.add_one(2);
        s.add_one(3); // evicts 1; 3 -> count 2, error 1
        assert_eq!(s.error(3), 1);
        s.add(3, 100); // re-add of a monitored item: error unchanged.
        assert_eq!(s.count(3), 102);
        assert_eq!(s.error(3), 1);
    }

    #[test]
    fn admitted_with_room_has_zero_error() {
        let mut s = SpaceSaving::with_capacity(5);
        s.add(7, 9);
        assert_eq!(s.error(7), 0);
        assert_eq!(s.count(7), 9);
    }

    #[test]
    fn count_zero_is_noop() {
        let mut s = SpaceSaving::with_capacity(1);
        s.add_one(1);
        s.add(2, 0); // must NOT evict 1.
        assert!(s.is_monitored(1));
        assert!(!s.is_monitored(2));
        assert_eq!(s.size(), 1);
    }

    #[test]
    fn empty_summary() {
        let s = SpaceSaving::with_capacity(3);
        assert_eq!(s.size(), 0);
        assert_eq!(s.capacity(), 3);
        assert_eq!(s.monitored_set(), Vec::<(i32, u64, u64)>::new());
        assert_eq!(s.count(7), 0);
        assert_eq!(s.error(7), 0);
        assert_eq!(s.top_k(3), Vec::<(i32, u64, u64)>::new());
    }

    #[test]
    fn top_k_canonical_order_and_bounds() {
        let mut s = SpaceSaving::with_capacity(10);
        s.add(1, 5);
        s.add(2, 3);
        s.add(3, 3); // tie at count 3 -> 2 before 3 (signed asc)
        s.add(4, 1);
        let full = s.monitored_set();
        assert_eq!(full, vec![(1, 5, 0), (2, 3, 0), (3, 3, 0), (4, 1, 0)]);
        assert_eq!(s.top_k(1), vec![(1, 5, 0)]);
        assert_eq!(s.top_k(2), vec![(1, 5, 0), (2, 3, 0)]);
        assert_eq!(s.top_k(4), full);
        assert_eq!(s.top_k(99), full); // k > size -> all, no padding
        assert_eq!(s.top_k(0), Vec::<(i32, u64, u64)>::new());
    }

    #[test]
    fn count_unmonitored_is_zero() {
        let mut s = SpaceSaving::with_capacity(1);
        s.add_one(1);
        s.add_one(2); // evicts 1
        assert_eq!(s.count(1), 0);
        assert!(!s.is_monitored(1));
    }

    #[test]
    fn overflow_saturates() {
        let mut s = SpaceSaving::with_capacity(1);
        s.add(7, u64::MAX);
        s.add(7, u64::MAX);
        assert_eq!(s.count(7), u64::MAX);
    }

    #[test]
    fn order_dependence() {
        // Same multiset, different order -> potentially different monitored set.
        // capacity 2. Sequence A: 1,1,2,3 ; Sequence B: 3,2,1,1.
        let mut a = SpaceSaving::with_capacity(2);
        for it in [1, 1, 2, 3] {
            a.add_one(it);
        }
        let mut b = SpaceSaving::with_capacity(2);
        for it in [3, 2, 1, 1] {
            b.add_one(it);
        }
        // They need not be equal; this just exercises that order matters and the
        // result is deterministic per order.
        let _ = (a.monitored_set(), b.monitored_set());
        // A: 1->1, 1->2, 2 admitted->1, 3 evicts min(count): 2 has count1 vs 1
        // has count2 -> victim 2 (count1); 3-> count2 error1. Set {1:2, 3:2e1}.
        assert_eq!(a.monitored_set(), vec![(1, 2, 0), (3, 2, 1)]);
    }

    #[test]
    fn error_floor_unchanged_across_evictions() {
        let mut s = SpaceSaving::with_capacity(2);
        s.add_one(1); // {1:1}
        s.add_one(2); // {1:1, 2:1} full
        s.add_one(3); // evict 1 (tie, smallest signed): 3-> count2 error1
        assert_eq!(s.error(3), 1);
        s.add_one(3); // monitored re-add: count3 error1 (UNCHANGED)
        assert_eq!(s.count(3), 3);
        assert_eq!(s.error(3), 1);
        s.add_one(4); // full {2:1, 3:3}; evict 2 (count1): 4-> count2 error1
        assert_eq!(s.error(4), 1);
        assert_eq!(s.count(4), 2);
        assert_eq!(s.error(3), 1); // 3 still unchanged
    }

    #[test]
    #[should_panic(expected = "capacity m must be non-zero")]
    fn m_zero_traps() {
        let _ = SpaceSaving::with_capacity(0);
    }
}
