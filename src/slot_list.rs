// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Intrusive doubly-linked list over a slot arena (`SlotList<T>`).
//!
//! An internal, crate-private primitive: a contiguous `Vec<Slot<T>>` in which
//! each live slot carries `{prev, next}` **slot indices** (never raw pointers)
//! forming an insertion-ordered doubly-linked list, and freed slots are recycled
//! through an intrusive free-list. It is the shared successor of the arena that
//! `bounded_lru.rs` grew organically (the "Phase-0 arena/slot-index intrusive
//! list"): the list keeps element order stable across removals in O(1) with no
//! index fix-up sweep, which is exactly what an insertion-ordered map/set needs.
//!
//! ## Invariants (upheld by every public method)
//!
//! - `len` equals the number of *linked* (live) slots, i.e. slots reachable from
//!   `head` by following `next` until [`NIL`].
//! - A live slot holds `item == Some(_)`; a free slot holds `item == None`. The
//!   two sets are disjoint and partition the arena.
//! - `head`/`tail` are `NIL` iff `len == 0`. Otherwise `arena[head].prev == NIL`
//!   and `arena[tail].next == NIL`.
//! - The free-list is singly linked through `next` from `free_head` to `NIL`;
//!   `prev` of a free slot is meaningless.
//!
//! ## Drop discipline (hardening item (c) from blueprint doc 14 §5)
//!
//! Freeing a slot **takes** its `Option<T>` and returns the owned `T` to the
//! caller, so the value's `Drop` runs deterministically at removal time rather
//! than lingering in a dead slot until the arena itself is dropped or the slot
//! is overwritten. `bounded_lru`'s original `free_node` only relinked and left
//! the old value in place — harmless for its `i32` payload, wrong for an owning
//! `V`. Storing `Option<T>` (no `unsafe`, no `MaybeUninit`) is the price of that
//! guarantee: one discriminant per slot.

/// Sentinel index meaning "no node" (list end / free-list end).
pub(crate) const NIL: usize = usize::MAX;

/// One arena slot. When live it links into the order list (`prev`/`next` are
/// slot indices, `item` is `Some`). When free it sits on the free-list (`next`
/// chains the free-list, `prev` is dead, `item` is `None`).
#[derive(Clone)]
struct Slot<T> {
    prev: usize,
    next: usize,
    item: Option<T>,
}

/// Insertion-ordered intrusive list over a recycling slot arena.
pub(crate) struct SlotList<T> {
    arena: Vec<Slot<T>>,
    /// Free-list head (a slot index), or [`NIL`] when no free slot is available.
    free_head: usize,
    /// Order-list head = oldest live entry, or [`NIL`] when empty.
    head: usize,
    /// Order-list tail = newest live entry, or [`NIL`] when empty.
    tail: usize,
    /// Number of live (linked) slots.
    len: usize,
}

impl<T> SlotList<T> {
    /// A new empty list.
    pub(crate) fn new() -> Self {
        SlotList {
            arena: Vec::new(),
            free_head: NIL,
            head: NIL,
            tail: NIL,
            len: 0,
        }
    }

    /// A new empty list with arena space reserved for `cap` slots.
    pub(crate) fn with_capacity(cap: usize) -> Self {
        SlotList {
            arena: Vec::with_capacity(cap),
            free_head: NIL,
            head: NIL,
            tail: NIL,
            len: 0,
        }
    }

    /// The number of live entries.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no live entries.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Append `item` at the tail (newest end) of the order list and return its
    /// slot index. Reuses a free slot when one is available, else grows the
    /// arena. The returned index is stable until the slot is freed.
    pub(crate) fn push_back(&mut self, item: T) -> usize {
        let idx = if self.free_head != NIL {
            // Reuse a free slot. Its `item` is `None` by the free invariant.
            let idx = self.free_head;
            self.free_head = self.arena[idx].next;
            let slot = &mut self.arena[idx];
            debug_assert!(slot.item.is_none());
            slot.prev = self.tail;
            slot.next = NIL;
            slot.item = Some(item);
            idx
        } else {
            let idx = self.arena.len();
            self.arena.push(Slot {
                prev: self.tail,
                next: NIL,
                item: Some(item),
            });
            idx
        };
        // Link at the tail.
        let old_tail = self.tail;
        if old_tail != NIL {
            self.arena[old_tail].next = idx;
        } else {
            self.head = idx;
        }
        self.tail = idx;
        self.len += 1;
        idx
    }

    /// Unlink the live slot `idx` from the order list, return it to the
    /// free-list, and hand back its owned value. `idx` must be a live slot.
    pub(crate) fn unlink_free(&mut self, idx: usize) -> T {
        // Unlink from the order list.
        let (prev, next) = {
            let s = &self.arena[idx];
            (s.prev, s.next)
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
        // Take the value (deterministic drop timing) and push onto the free-list.
        let slot = &mut self.arena[idx];
        let item = slot.item.take().expect("unlink_free on a free slot");
        slot.prev = NIL;
        slot.next = self.free_head;
        self.free_head = idx;
        self.len -= 1;
        item
    }

    /// Shared reference to the value in live slot `idx`.
    #[inline]
    pub(crate) fn get(&self, idx: usize) -> &T {
        self.arena[idx].item.as_ref().expect("get on a free slot")
    }

    /// Mutable reference to the value in live slot `idx`.
    #[inline]
    pub(crate) fn get_mut(&mut self, idx: usize) -> &mut T {
        self.arena[idx]
            .item
            .as_mut()
            .expect("get_mut on a free slot")
    }

    /// Remove every entry and reset the arena. Values are dropped as the arena
    /// is cleared.
    pub(crate) fn clear(&mut self) {
        self.arena.clear();
        self.free_head = NIL;
        self.head = NIL;
        self.tail = NIL;
        self.len = 0;
    }

    /// Iterate live values in insertion order (oldest → newest).
    pub(crate) fn iter(&self) -> Iter<'_, T> {
        Iter {
            arena: &self.arena,
            cur: self.head,
            remaining: self.len,
        }
    }

    /// Iterate mutable references to live values in insertion order.
    ///
    /// Materialized eagerly: because the order list threads the arena in an
    /// arbitrary pattern, a lazy borrowing cursor cannot hand out `&'a mut T`
    /// without `unsafe` (which this crate forbids). Instead we read the link
    /// order in one immutable pass, take a disjoint `&mut` to every slot in one
    /// `slice::iter_mut` pass, then reorder those references into insertion
    /// order. All borrows are provably disjoint, so it is entirely safe.
    pub(crate) fn iter_mut(&mut self) -> IterMut<'_, T> {
        // 1. Insertion-order sequence of live slot indices (immutable walk).
        let mut order = Vec::with_capacity(self.len);
        let mut cur = self.head;
        while cur != NIL {
            order.push(cur);
            cur = self.arena[cur].next;
        }
        // 2. One mutable pass yields a disjoint `&mut` per slot, addressable by
        //    index; free slots contribute `None`.
        let mut cells: Vec<Option<&mut T>> =
            self.arena.iter_mut().map(|s| s.item.as_mut()).collect();
        // 3. Pull the references out in insertion order.
        let mut ordered = Vec::with_capacity(order.len());
        for idx in order {
            ordered.push(cells[idx].take().expect("live slot in order list"));
        }
        IterMut {
            inner: ordered.into_iter(),
        }
    }
}

impl<T> Default for SlotList<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Owned iterator over live values in insertion order. Exact-size.
pub(crate) struct IntoIter<T> {
    inner: std::vec::IntoIter<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}
impl<T> std::iter::FusedIterator for IntoIter<T> {}

impl<T> IntoIterator for SlotList<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(mut self) -> IntoIter<T> {
        // Take owned values out in insertion order. Free slots are skipped
        // because the walk follows the live order list, never the arena order.
        let mut items = Vec::with_capacity(self.len);
        let mut cur = self.head;
        while cur != NIL {
            let slot = &mut self.arena[cur];
            let next = slot.next;
            items.push(slot.item.take().expect("live slot in order list"));
            cur = next;
        }
        IntoIter {
            inner: items.into_iter(),
        }
    }
}

impl<T: Clone> Clone for SlotList<T> {
    fn clone(&self) -> Self {
        // A *structural* clone: copy the arena (including free slots) and all
        // links verbatim, so every slot index is preserved. This matters when a
        // separate structure (an `IndexTable`) stores these indices — a
        // compacting clone would silently invalidate them.
        SlotList {
            arena: self.arena.clone(),
            free_head: self.free_head,
            head: self.head,
            tail: self.tail,
            len: self.len,
        }
    }
}

/// Shared-reference iterator over live values, insertion order. Exact-size.
pub(crate) struct Iter<'a, T> {
    arena: &'a [Slot<T>],
    cur: usize,
    remaining: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        if self.cur == NIL {
            return None;
        }
        let slot = &self.arena[self.cur];
        self.cur = slot.next;
        self.remaining -= 1;
        Some(slot.item.as_ref().expect("live slot in order list"))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}
impl<T> std::iter::FusedIterator for Iter<'_, T> {}

/// Mutable-reference iterator over live values, insertion order. Exact-size.
/// References are pre-collected in insertion order (see [`SlotList::iter_mut`]).
pub(crate) struct IterMut<'a, T> {
    inner: std::vec::IntoIter<&'a mut T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for IterMut<'_, T> {}
impl<T> std::iter::FusedIterator for IterMut<'_, T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn push_back_preserves_order() {
        let mut sl = SlotList::new();
        let a = sl.push_back("a");
        let b = sl.push_back("b");
        let c = sl.push_back("c");
        assert_eq!(sl.len(), 3);
        assert_eq!(sl.iter().copied().collect::<Vec<_>>(), vec!["a", "b", "c"]);
        assert_eq!(*sl.get(a), "a");
        assert_eq!(*sl.get(b), "b");
        assert_eq!(*sl.get(c), "c");
    }

    #[test]
    fn unlink_free_keeps_order_and_recycles() {
        let mut sl = SlotList::new();
        let _a = sl.push_back(1);
        let b = sl.push_back(2);
        let _c = sl.push_back(3);
        assert_eq!(sl.unlink_free(b), 2);
        assert_eq!(sl.len(), 2);
        assert_eq!(sl.iter().copied().collect::<Vec<_>>(), vec![1, 3]);
        // The freed slot `b` is reused by the next push_back (no arena growth).
        let arena_before = sl.arena.len();
        let d = sl.push_back(4);
        assert_eq!(d, b, "freed slot index should be recycled");
        assert_eq!(sl.arena.len(), arena_before, "arena must not grow");
        assert_eq!(sl.iter().copied().collect::<Vec<_>>(), vec![1, 3, 4]);
    }

    #[test]
    fn unlink_head_and_tail() {
        let mut sl = SlotList::new();
        let a = sl.push_back(1);
        let _b = sl.push_back(2);
        let c = sl.push_back(3);
        assert_eq!(sl.unlink_free(a), 1); // remove head
        assert_eq!(sl.unlink_free(c), 3); // remove tail
        assert_eq!(sl.iter().copied().collect::<Vec<_>>(), vec![2]);
        assert_eq!(sl.len(), 1);
    }

    #[test]
    fn iter_mut_mutates_in_order() {
        let mut sl = SlotList::new();
        sl.push_back(1);
        sl.push_back(2);
        sl.push_back(3);
        for v in sl.iter_mut() {
            *v *= 10;
        }
        assert_eq!(sl.iter().copied().collect::<Vec<_>>(), vec![10, 20, 30]);
    }

    #[test]
    fn get_mut_updates_in_place() {
        let mut sl = SlotList::new();
        let a = sl.push_back(1);
        *sl.get_mut(a) = 99;
        assert_eq!(*sl.get(a), 99);
    }

    /// A type that records its drops so we can pin deterministic drop timing.
    struct DropSpy(Rc<RefCell<Vec<i32>>>, i32);
    impl Drop for DropSpy {
        fn drop(&mut self) {
            self.0.borrow_mut().push(self.1);
        }
    }

    #[test]
    fn unlink_free_drops_value_at_removal_time() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut sl = SlotList::new();
        let a = sl.push_back(DropSpy(log.clone(), 1));
        let _b = sl.push_back(DropSpy(log.clone(), 2));
        // Removing `a` returns the value; dropping it here logs `1` NOW, not
        // later — proving the freed slot did not retain the value.
        let removed = sl.unlink_free(a);
        assert!(
            log.borrow().is_empty(),
            "not dropped until we drop the return"
        );
        drop(removed);
        assert_eq!(*log.borrow(), vec![1]);
        // Reusing the freed slot must not resurrect or double-drop the old value.
        let _d = sl.push_back(DropSpy(log.clone(), 3));
        assert_eq!(*log.borrow(), vec![1], "reuse must not re-drop");
        drop(sl);
        // Remaining live values (2, 3) drop when the list drops.
        let mut got = log.borrow().clone();
        got.sort_unstable();
        assert_eq!(got, vec![1, 2, 3], "every value dropped exactly once");
    }

    #[test]
    fn clone_preserves_order_and_indices() {
        let mut sl = SlotList::new();
        let a = sl.push_back(1);
        let b = sl.push_back(2);
        let c = sl.push_back(3);
        sl.unlink_free(b); // leaves a hole in the source arena
        let cl = sl.clone();
        assert_eq!(cl.iter().copied().collect::<Vec<_>>(), vec![1, 3]);
        assert_eq!(cl.len(), 2);
        // Structural clone: live slot indices are identical to the source, so an
        // external index keyed on them stays valid.
        assert_eq!(*cl.get(a), 1);
        assert_eq!(*cl.get(c), 3);
    }

    #[test]
    fn clear_resets_and_reuses() {
        let mut sl = SlotList::new();
        sl.push_back(1);
        sl.push_back(2);
        sl.clear();
        assert!(sl.is_empty());
        assert_eq!(sl.iter().count(), 0);
        sl.push_back(7);
        assert_eq!(sl.iter().copied().collect::<Vec<_>>(), vec![7]);
    }
}
