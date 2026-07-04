// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Sorted map backed by a red-black tree with pluggable [`Comparator`].

use super::strategy::{Comparator, Compare, Natural};
use crate::bulk::{BulkError, DuplicatePolicy};
use crate::range::Range;
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Bound as StdBound, RangeBounds};

struct Node<K, V> {
    key: K,
    value: V,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
    red: bool,
    /// Number of nodes in the subtree rooted at this node (this node plus
    /// both children's subtrees). Maintained in O(1) on every structural
    /// change — insert, remove, and all rotations — so that order-statistic
    /// `rank`/`select` run in O(log n). Invariant after any operation:
    /// `size == 1 + size(left) + size(right)`.
    size: usize,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V, red: bool) -> Self {
        Node {
            key,
            value,
            left: None,
            right: None,
            red,
            size: 1,
        }
    }
}

/// Subtree size of an optional node link (`0` for an absent child).
fn node_size<K, V>(node: &Option<Box<Node<K, V>>>) -> usize {
    node.as_ref().map_or(0, |n| n.size)
}

/// Recompute a node's cached subtree size from its children. Called after
/// any rotation or child relinking so the augmentation stays consistent.
fn fix_size<K, V>(node: &mut Node<K, V>) {
    node.size = 1 + node_size(&node.left) + node_size(&node.right);
}

/// Which side of `k` a point-navigation query selects, and whether the
/// match at `k` itself is admissible. Drives the shared [`TreeMap::bound_entry`]
/// walk for `floor`/`ceiling`/`lower`/`higher`.
#[derive(Clone, Copy)]
enum Bound {
    /// Greatest key `<= k`.
    Floor,
    /// Least key `>= k`.
    Ceiling,
    /// Greatest key `< k` (strict).
    Lower,
    /// Least key `> k` (strict).
    Higher,
}

/// A sorted map backed by a left-leaning red-black tree with a pluggable
/// comparator `C` (the [`Compare`] type parameter — `BuildHasher` for order).
///
/// `C` defaults to [`Comparator<K>`] (the runtime `Arc<dyn Fn>` comparator) for
/// full backward compatibility, so `TreeMap<K, V>` behaves exactly as before.
/// Use [`with_comparator`](TreeMap::with_comparator) with a [`Natural`],
/// [`Reverse`](super::strategy::Reverse), or custom zero-sized `C` for a
/// statically-dispatched, inlined comparison. [`DynTreeMap`] names the runtime
/// case explicitly.
pub struct TreeMap<K, V, C = Comparator<K>> {
    root: Option<Box<Node<K, V>>>,
    size: usize,
    cmp: C,
}

/// A [`TreeMap`] whose order is a runtime [`Comparator`] (the pre-v3 default,
/// named explicitly for when the comparator is chosen at runtime).
pub type DynTreeMap<K, V> = TreeMap<K, V, Comparator<K>>;

impl<K: fmt::Debug, V: fmt::Debug, C: Compare<K>> fmt::Debug for TreeMap<K, V, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V> TreeMap<K, V, Comparator<K>> {
    /// Creates an empty `TreeMap` using the given runtime comparator. For a
    /// static/zero-sized comparator use [`with_comparator`](TreeMap::with_comparator);
    /// for the natural `Ord` order use [`natural`](TreeMap::natural).
    pub fn new(cmp: Comparator<K>) -> Self {
        TreeMap {
            root: None,
            size: 0,
            cmp,
        }
    }

    /// Returns a clone of this map's runtime comparator (shares the underlying
    /// closure). Used to preserve ordering semantics when building a
    /// materialized snapshot (`sub_map`).
    pub fn comparator(&self) -> Comparator<K> {
        self.cmp.clone()
    }
}

impl<K: Ord, V> TreeMap<K, V, Natural> {
    /// Creates an empty `TreeMap` ordered by the key's natural [`Ord`]. The
    /// comparator is a zero-sized [`Natural`], so comparisons inline.
    pub fn natural() -> Self {
        TreeMap {
            root: None,
            size: 0,
            cmp: Natural,
        }
    }
}

impl<K, V, C: Compare<K>> TreeMap<K, V, C> {
    /// Creates an empty `TreeMap` using the [`Compare`] value `cmp` (typically
    /// a zero-sized type like [`Natural`] or [`Reverse`](super::strategy::Reverse)).
    pub fn with_comparator(cmp: C) -> Self {
        TreeMap {
            root: None,
            size: 0,
            cmp,
        }
    }

    /// Inserts a key-value pair. Returns `Some(old_value)` if the key was
    /// already present, or `None` if it was new.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let mut old = None;
        self.root = Self::insert_rec(&self.cmp, self.root.take(), key, value, &mut old);
        if old.is_none() {
            self.size += 1;
        }
        if let Some(ref mut root) = self.root {
            root.red = false;
        }
        old
    }

    /// Returns a reference to the value associated with the key, or `None`.
    pub fn get(&self, key: &K) -> Option<&V> {
        let mut current = &self.root;
        while let Some(ref n) = current {
            match self.cmp.compare(key, &n.key) {
                Ordering::Less => current = &n.left,
                Ordering::Greater => current = &n.right,
                Ordering::Equal => return Some(&n.value),
            }
        }
        None
    }

    /// Returns `true` if the map contains the given key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Removes the entry for the given key. Returns `Some(value)` if found.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if !self.contains_key(key) {
            return None;
        }
        // If both children of root are black, set root to red.
        if let Some(ref mut root) = self.root {
            if !is_red(&root.left) && !is_red(&root.right) {
                root.red = true;
            }
        }
        let mut removed = None;
        self.root = Self::remove_rec(&self.cmp, self.root.take(), key, &mut removed);
        if let Some(ref mut root) = self.root {
            root.red = false;
        }
        if removed.is_some() {
            self.size -= 1;
        }
        removed
    }

    /// Returns the number of key-value pairs.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns `true` if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Removes all entries.
    pub fn clear(&mut self) {
        self.root = None;
        self.size = 0;
    }

    /// Returns the minimum key and its value, or `None` if empty.
    pub fn min(&self) -> Option<(&K, &V)> {
        min_ref(&self.root).map(|n| (&n.key, &n.value))
    }

    /// Returns the maximum key and its value, or `None` if empty.
    pub fn max(&self) -> Option<(&K, &V)> {
        max_ref(&self.root).map(|n| (&n.key, &n.value))
    }

    // ── Point navigation (NavigableMap surface) ─────────────────────
    //
    // floor `<= k`, ceiling `>= k`, lower `< k` (strict), higher `> k`
    // (strict). All comparisons go through the tree comparator, so the
    // float total order carries through for `HashableF32`/`HashableF64`
    // keys exactly as in-order iteration does.

    /// Greatest key `<= k` and its value, or `None`.
    pub fn floor_entry(&self, k: &K) -> Option<(&K, &V)> {
        self.bound_entry(k, Bound::Floor)
    }

    /// Greatest key `<= k`, or `None`.
    pub fn floor_key(&self, k: &K) -> Option<&K> {
        self.floor_entry(k).map(|(key, _)| key)
    }

    /// Least key `>= k` and its value, or `None`.
    pub fn ceiling_entry(&self, k: &K) -> Option<(&K, &V)> {
        self.bound_entry(k, Bound::Ceiling)
    }

    /// Least key `>= k`, or `None`.
    pub fn ceiling_key(&self, k: &K) -> Option<&K> {
        self.ceiling_entry(k).map(|(key, _)| key)
    }

    /// Greatest key `< k` (strict) and its value, or `None`.
    pub fn lower_entry(&self, k: &K) -> Option<(&K, &V)> {
        self.bound_entry(k, Bound::Lower)
    }

    /// Greatest key `< k` (strict), or `None`.
    pub fn lower_key(&self, k: &K) -> Option<&K> {
        self.lower_entry(k).map(|(key, _)| key)
    }

    /// Least key `> k` (strict) and its value, or `None`.
    pub fn higher_entry(&self, k: &K) -> Option<(&K, &V)> {
        self.bound_entry(k, Bound::Higher)
    }

    /// Least key `> k` (strict), or `None`.
    pub fn higher_key(&self, k: &K) -> Option<&K> {
        self.higher_entry(k).map(|(key, _)| key)
    }

    /// Minimum key and its value, or `None`. Alias for [`min`](Self::min)
    /// completing the navigable surface.
    pub fn first_entry(&self) -> Option<(&K, &V)> {
        self.min()
    }

    /// Minimum key, or `None`.
    pub fn first_key(&self) -> Option<&K> {
        self.min().map(|(k, _)| k)
    }

    /// Maximum key and its value, or `None`. Alias for [`max`](Self::max).
    pub fn last_entry(&self) -> Option<(&K, &V)> {
        self.max()
    }

    /// Maximum key, or `None`.
    pub fn last_key(&self) -> Option<&K> {
        self.max().map(|(k, _)| k)
    }

    /// Shared walk for the four point-navigation queries: descend the
    /// tree tracking the best candidate seen on the relevant side.
    fn bound_entry(&self, k: &K, bound: Bound) -> Option<(&K, &V)> {
        let mut current = &self.root;
        let mut best: Option<&Node<K, V>> = None;
        while let Some(ref n) = current {
            let ord = self.cmp.compare(k, &n.key);
            let take = match bound {
                // floor/lower track the greatest key on the `<` side;
                // floor also accepts equality.
                Bound::Floor => ord != Ordering::Less, // n.key <= k
                Bound::Lower => ord == Ordering::Greater, // n.key < k
                // ceiling/higher track the least key on the `>` side;
                // ceiling also accepts equality.
                Bound::Ceiling => ord != Ordering::Greater, // n.key >= k
                Bound::Higher => ord == Ordering::Less,     // n.key > k
            };
            if take {
                best = Some(n);
                // candidate qualifies; move toward k for a tighter one.
                current = match bound {
                    Bound::Floor | Bound::Lower => &n.right,
                    Bound::Ceiling | Bound::Higher => &n.left,
                };
            } else {
                // n.key on the wrong side; move toward the accepted side.
                current = match bound {
                    Bound::Floor | Bound::Lower => &n.left,
                    Bound::Ceiling | Bound::Higher => &n.right,
                };
            }
        }
        best.map(|n| (&n.key, &n.value))
    }

    // ── Order statistics (rank / select) ────────────────────────────
    //
    // Backed by the per-node subtree-size augmentation; both run in
    // O(log n) on the balanced tree. Comparisons go through the tree
    // comparator, so the order is exactly the in-order traversal order
    // (the float total order carries through for `HashableF32`/`F64` keys).

    /// Returns the number of keys strictly less than `key` under the tree's
    /// comparator — the **0-based lower-bound index** the key occupies (if
    /// present) or would occupy (if absent). Defined for present and absent
    /// keys alike; the result is in `0..=len()` (`len()` for any key greater
    /// than the maximum). Pure query; never mutates.
    pub fn rank(&self, key: &K) -> usize {
        let mut rank = 0;
        let mut current = &self.root;
        while let Some(ref n) = current {
            match self.cmp.compare(key, &n.key) {
                // key < n.key: everything in this node's right subtree (and
                // n itself) is >= key; descend left without counting.
                Ordering::Less => current = &n.left,
                // key > n.key: n and its whole left subtree are strictly
                // less than key; count them, then descend right.
                Ordering::Greater => {
                    rank += 1 + node_size(&n.left);
                    current = &n.right;
                }
                // key == n.key: exactly the left subtree is strictly less.
                Ordering::Equal => return rank + node_size(&n.left),
            }
        }
        rank
    }

    /// Number of keys `<= key` under the tree's comparator (the upper-bound
    /// count; `rank` is the strict lower-bound count `<`). O(log n) via the
    /// subtree-size augmentation.
    fn count_le(&self, key: &K) -> usize {
        let mut count = 0;
        let mut current = &self.root;
        while let Some(ref n) = current {
            match self.cmp.compare(key, &n.key) {
                Ordering::Less => current = &n.left,
                Ordering::Greater => {
                    count += 1 + node_size(&n.left);
                    current = &n.right;
                }
                Ordering::Equal => return count + node_size(&n.left) + 1,
            }
        }
        count
    }

    /// A lazy, **double-ended** iterator over the `(&K, &V)` entries whose key
    /// falls in `range`, in ascending comparator order (`.rev()` for
    /// descending).
    ///
    /// Bounds are any [`RangeBounds<K>`] — `map.range(a..=b)`, `map.range(..)`,
    /// `map.range(a..)`, `map.range((Excluded(a), Unbounded))` — and are
    /// compared through the map's **own comparator** `C`, so range selection can
    /// never disagree with the tree's order (this is what designs away the
    /// natural-order-only divergence of the legacy `Range<K>` methods).
    ///
    /// The iterator is [`ExactSizeIterator`]: the element count is precomputed
    /// from the subtree-size augmentation at seek time, so iteration performs no
    /// per-item bound comparison and the two ends can never cross.
    ///
    /// # Inverted / empty bounds
    /// An inverted or empty range (`b..a` with `a < b`, or `a..a` exclusive)
    /// yields **nothing** — it is treated as empty rather than a panic
    /// (deliberately diverging from `std::collections::BTreeMap::range`, and
    /// consistent with this crate's `Range<T>` "cut-empty is valid" model).
    pub fn range<R: RangeBounds<K>>(&self, range: R) -> RangeIter<'_, K, V> {
        let lo = match range.start_bound() {
            StdBound::Unbounded => 0,
            StdBound::Included(q) => self.rank(q), // # keys strictly < q
            StdBound::Excluded(q) => self.count_le(q), // # keys <= q
        };
        let hi = match range.end_bound() {
            StdBound::Unbounded => self.size,
            StdBound::Included(q) => self.count_le(q), // # keys <= q
            StdBound::Excluded(q) => self.rank(q),     // # keys strictly < q
        };
        let remaining = hi.saturating_sub(lo); // inverted/empty -> 0
        RangeIter {
            front: seed_forward(&self.root, lo),
            back: seed_backward(&self.root, hi.saturating_sub(1)),
            remaining,
        }
    }

    /// Returns the `i`-th smallest key (0-based), or `None` if `i >= len()`.
    /// `i == len()` (and any larger index, including on an empty map) is
    /// absence, not a trap. Round-trips with [`rank`](Self::rank):
    /// `select_key(rank(k)) == Some(k)` for any present `k`, and
    /// `rank(select_key(i)) == i` for every `0 <= i < len()`.
    pub fn select_key(&self, i: usize) -> Option<&K> {
        self.select_node(i).map(|n| &n.key)
    }

    /// Returns the `i`-th smallest `(key, value)` entry (0-based), or `None`
    /// if `i >= len()`. Same index domain as [`select_key`](Self::select_key).
    pub fn select_entry(&self, i: usize) -> Option<(&K, &V)> {
        self.select_node(i).map(|n| (&n.key, &n.value))
    }

    /// Test-only: verify the subtree-size invariant holds at every node,
    /// returning the recomputed total. Asserts `size == 1 + left + right`
    /// throughout and that the root total equals [`len`](Self::len).
    #[cfg(test)]
    fn assert_size_invariant(&self) {
        fn check<K, V>(node: &Option<Box<Node<K, V>>>) -> usize {
            match node {
                None => 0,
                Some(n) => {
                    let l = check(&n.left);
                    let r = check(&n.right);
                    assert_eq!(n.size, 1 + l + r, "subtree-size invariant violated");
                    n.size
                }
            }
        }
        assert_eq!(
            check(&self.root),
            self.size,
            "root size mismatch with len()"
        );
    }

    /// Walks to the node at 0-based sorted index `i`, or `None` if out of
    /// range. The subtree-size augmentation makes this O(log n).
    fn select_node(&self, mut i: usize) -> Option<&Node<K, V>> {
        let mut current = self.root.as_deref();
        while let Some(n) = current {
            let left = node_size(&n.left);
            match i.cmp(&left) {
                Ordering::Less => current = n.left.as_deref(),
                Ordering::Equal => return Some(n),
                Ordering::Greater => {
                    // Skip the left subtree and this node.
                    i -= left + 1;
                    current = n.right.as_deref();
                }
            }
        }
        None
    }

    /// Returns an iterator over `(&K, &V)` pairs in sorted order.
    pub fn iter(&self) -> TreeMapIter<'_, K, V> {
        let mut stack = Vec::new();
        push_left_spine(&self.root, &mut stack);
        TreeMapIter { stack }
    }

    /// Returns an iterator over keys in sorted order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(k, _)| k)
    }

    /// Returns an iterator over values in key-sorted order.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }

    /// Calls `f` for each key-value pair in sorted order.
    pub fn for_each(&self, mut f: impl FnMut(&K, &V)) {
        in_order(&self.root, &mut f);
    }

    // ── internal: insert ────────────────────────────────────────────

    fn insert_rec(
        cmp: &C,
        node: Option<Box<Node<K, V>>>,
        key: K,
        value: V,
        old: &mut Option<V>,
    ) -> Option<Box<Node<K, V>>> {
        let mut node = match node {
            None => return Some(Box::new(Node::new(key, value, true))),
            Some(n) => n,
        };

        match cmp.compare(&key, &node.key) {
            Ordering::Less => {
                node.left = Self::insert_rec(cmp, node.left.take(), key, value, old);
            }
            Ordering::Greater => {
                node.right = Self::insert_rec(cmp, node.right.take(), key, value, old);
            }
            Ordering::Equal => {
                *old = Some(std::mem::replace(&mut node.value, value));
            }
        }

        Some(fix_up(node))
    }

    // ── internal: remove ────────────────────────────────────────────

    fn remove_rec(
        cmp: &C,
        node: Option<Box<Node<K, V>>>,
        key: &K,
        removed: &mut Option<V>,
    ) -> Option<Box<Node<K, V>>> {
        let mut node = node?;

        if cmp.compare(key, &node.key) == Ordering::Less {
            if !is_red(&node.left) && !node.left.as_ref().is_some_and(|l| is_red(&l.left)) {
                node = move_red_left(node);
            }
            node.left = Self::remove_rec(cmp, node.left.take(), key, removed);
        } else {
            if is_red(&node.left) {
                node = rotate_right(node);
            }
            if cmp.compare(key, &node.key) == Ordering::Equal && node.right.is_none() {
                *removed = Some(node.value);
                return None;
            }
            if !is_red(&node.right) && !node.right.as_ref().is_some_and(|r| is_red(&r.left)) {
                node = move_red_right(node);
            }
            if cmp.compare(key, &node.key) == Ordering::Equal {
                // Replace with min of right subtree.
                let (new_right, min_key, min_value) = delete_min_node(node.right.take());
                node.right = new_right;
                node.key = min_key;
                let old_value = std::mem::replace(&mut node.value, min_value);
                *removed = Some(old_value);
            } else {
                node.right = Self::remove_rec(cmp, node.right.take(), key, removed);
            }
        }
        Some(fix_up(node))
    }
}

impl<K: Clone, V, C: Compare<K>> TreeMap<K, V, C> {
    // ── Poll (positional removal) ───────────────────────────────────

    /// Removes and returns the minimum entry, or `None` if empty. Does not
    /// trap on an empty map.
    pub fn poll_first_entry(&mut self) -> Option<(K, V)> {
        let key = self.first_key()?.clone();
        let value = self.remove(&key)?;
        Some((key, value))
    }

    /// Removes and returns the maximum entry, or `None` if empty. Does not
    /// trap on an empty map.
    pub fn poll_last_entry(&mut self) -> Option<(K, V)> {
        let key = self.last_key()?.clone();
        let value = self.remove(&key)?;
        Some((key, value))
    }
}

impl<K: Ord + Copy, V> TreeMap<K, V> {
    // ── Range slice & descending iteration (consume `Range<K>`) ──────
    //
    // Range membership is EXACTLY `range.contains(key)`: e.g. `open(1, 2)`
    // over `i32` matches no key yet is a valid, non-cut-empty range. We
    // never infer discrete-domain emptiness from the cuts.
    //
    // ⚠️ NATURAL-ORDER-ONLY. These `Range<K>`-argument methods select
    // membership by the key's natural `Ord` (via `Range::contains`), NOT by
    // the map's `Comparator`. When the map is built with a non-natural
    // comparator (e.g. `reverse_comparator`, a by-field comparator, or a
    // float total-order), selection can disagree with the tree's own ordering:
    // the ascending/descending labels follow natural order, and two keys that
    // are comparator-equal but `Ord`-distinct select inconsistently. For
    // comparator-correct range queries use [`TreeMap::range`] (the
    // `RangeBounds` API), which compares bounds through the map's comparator
    // and thus makes this divergence unrepresentable.

    /// Keys in `range`, ascending under the key's **natural `Ord`** (see the
    /// natural-order-only caveat on this impl block). Snapshot; read-only.
    pub fn range_keys(&self, range: Range<K>) -> Vec<K> {
        self.keys()
            .copied()
            .filter(|k| range.contains(*k))
            .collect()
    }

    /// `(key, value)` pairs whose key ∈ `range`, ascending. Values are
    /// copied so the result is an independent snapshot.
    pub fn range_entries(&self, range: Range<K>) -> Vec<(K, V)>
    where
        V: Copy,
    {
        self.iter()
            .filter(|(k, _)| range.contains(**k))
            .map(|(k, v)| (*k, *v))
            .collect()
    }

    /// Keys in `range`, descending.
    pub fn descending_range_keys(&self, range: Range<K>) -> Vec<K> {
        let mut v = self.range_keys(range);
        v.reverse();
        v
    }

    /// `(key, value)` pairs whose key ∈ `range`, descending.
    pub fn descending_range_entries(&self, range: Range<K>) -> Vec<(K, V)>
    where
        V: Copy,
    {
        let mut v = self.range_entries(range);
        v.reverse();
        v
    }

    /// All keys, descending.
    pub fn descending_keys(&self) -> Vec<K> {
        let mut v: Vec<K> = self.keys().copied().collect();
        v.reverse();
        v
    }

    /// All `(key, value)` pairs, descending.
    pub fn descending_entries(&self) -> Vec<(K, V)>
    where
        V: Copy,
    {
        let mut v: Vec<(K, V)> = self.iter().map(|(k, v)| (*k, *v)).collect();
        v.reverse();
        v
    }

    /// A **new independent SNAPSHOT** map of the entries whose key ∈ `range`.
    ///
    /// This is a **materialized copy, not a live write-through view** (unlike
    /// Guava/`java.util` `subMap`): mutating the snapshot never affects the
    /// original and vice versa. The snapshot preserves the **source map's
    /// comparator**, so reverse/custom/float-total-order keyed maps keep their
    /// ordering semantics in the slice — but membership `∈ range` is selected by
    /// natural `Ord` (see the natural-order-only caveat on this impl block).
    pub fn sub_map(&self, range: Range<K>) -> TreeMap<K, V>
    where
        K: 'static,
        V: Copy,
    {
        let mut out = TreeMap::new(self.cmp.clone());
        for (k, v) in self.iter() {
            if range.contains(*k) {
                out.insert(*k, *v);
            }
        }
        out
    }

    /// Removes every entry whose key ∈ `range`; returns the count removed.
    /// `remove_range` over a range that matches nothing is a no-op
    /// returning `0`.
    pub fn remove_range(&mut self, range: Range<K>) -> usize {
        let victims: Vec<K> = self
            .keys()
            .copied()
            .filter(|k| range.contains(*k))
            .collect();
        let count = victims.len();
        for k in victims {
            self.remove(&k);
        }
        count
    }
}

// ── Data pump: bottom-up bulk construction from sorted input ─────────

impl<K, V> TreeMap<K, V> {
    /// Builds a fresh `TreeMap` from already-sorted `(K, V)` input in a single
    /// O(n) pass, skipping per-element rebalancing.
    ///
    /// Input must be **strictly ascending** under `cmp`; an equal or
    /// out-of-order step is a [`BulkError::OutOfOrder`] (or, under
    /// [`DuplicatePolicy::Error`], a [`BulkError::Duplicate`]). With
    /// [`DuplicatePolicy::IgnoreDuplicates`] a run of equal keys keeps the
    /// first value and skips the rest.
    ///
    /// The resulting tree is a valid left-leaning red-black tree (built via the
    /// 2-3-tree-sized bottom-up builder), so later `insert`/`remove` keep the
    /// LLRB invariants. This is a thin wrapper over [`TreeMapSink`].
    pub fn from_sorted<I: IntoIterator<Item = (K, V)>>(
        cmp: Comparator<K>,
        iter: I,
        dup: DuplicatePolicy,
    ) -> Result<Self, BulkError> {
        let mut sink = TreeMapSink::new(cmp, dup);
        sink.put_all(iter)?;
        Ok(sink.create())
    }

    /// Assembles a `TreeMap` directly from a pre-validated, strictly-ascending
    /// buffer (the shared finish step for the sink and `from_sorted`).
    fn from_sorted_buffer(buf: Vec<(K, V)>, cmp: Comparator<K>) -> Self {
        let size = buf.len();
        let root = build_balanced(buf);
        let map = TreeMap { root, size, cmp };
        #[cfg(debug_assertions)]
        debug_assert!(
            map.is_valid_llrb(),
            "bulk builder produced an invalid LLRB tree"
        );
        map
    }

    /// White-box LLRB structural validator (BST order under the comparator,
    /// no right-leaning red, no consecutive reds, uniform black-height, black
    /// root). Used by `debug_assert` after a bulk build and by tests.
    #[cfg(any(test, debug_assertions))]
    fn is_valid_llrb(&self) -> bool {
        fn check<K, V>(
            cmp: &Comparator<K>,
            node: &Option<Box<Node<K, V>>>,
            lo: Option<&K>,
            hi: Option<&K>,
        ) -> Option<usize> {
            let n = match node {
                None => return Some(0),
                Some(n) => n,
            };
            if let Some(lo) = lo {
                if cmp.compare(lo, &n.key) != Ordering::Less {
                    return None;
                }
            }
            if let Some(hi) = hi {
                if cmp.compare(&n.key, hi) != Ordering::Less {
                    return None;
                }
            }
            if is_red(&n.right) {
                return None; // right-leaning red link
            }
            if n.red && is_red(&n.left) {
                return None; // two consecutive reds
            }
            let lh = check(cmp, &n.left, lo, Some(&n.key))?;
            let rh = check(cmp, &n.right, Some(&n.key), hi)?;
            if lh != rh {
                return None; // black-height mismatch
            }
            Some(lh + usize::from(!n.red))
        }
        if let Some(ref root) = self.root {
            if root.red {
                return false; // root must be black
            }
        }
        check(&self.cmp, &self.root, None, None).is_some()
    }
}

/// Smallest size representable by a black-rooted LLRB subtree of black height
/// `bh`: a perfect all-black binary tree, `2^bh - 1`.
#[inline]
fn min_size(bh: usize) -> usize {
    (1usize << bh) - 1
}

/// Largest size representable by a black-rooted LLRB subtree of black height
/// `bh`: a full 2-3 tree, `3^bh - 1`.
#[inline]
fn max_size(bh: usize) -> usize {
    let mut p = 1usize;
    for _ in 0..bh {
        p = p.saturating_mul(3);
    }
    p - 1
}

/// Smallest black height whose `max_size` can hold `n` nodes.
fn choose_black_height(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut bh = 0;
    while max_size(bh) < n {
        bh += 1;
    }
    bh
}

/// Distributes `total` across `parts` buckets, each within `[lo, hi]`, as
/// evenly as possible (any in-range split is structurally valid; balanced just
/// gives a nicer shape). Algorithm from the codex (gpt-5.5) LLRB design review.
fn split_sizes(total: usize, parts: usize, lo: usize, hi: usize) -> Vec<usize> {
    debug_assert!(parts * lo <= total && total <= parts * hi);
    let mut out = vec![lo; parts];
    let mut rem = total - parts * lo;
    let max_here = hi - lo;
    for (i, slot) in out.iter_mut().enumerate() {
        let remaining_parts = parts - i - 1;
        let give = rem
            .saturating_sub(remaining_parts * max_here)
            .max((rem + remaining_parts) / (remaining_parts + 1))
            .min(max_here);
        *slot += give;
        rem -= give;
    }
    debug_assert_eq!(out.iter().sum::<usize>(), total);
    out
}

/// Builds a valid LLRB from a strictly-ascending buffer in O(n). Consumes the
/// buffer left-to-right (in-order), so the produced tree's in-order traversal
/// equals the input.
fn build_balanced<K, V>(buf: Vec<(K, V)>) -> Option<Box<Node<K, V>>> {
    let n = buf.len();
    let bh = choose_black_height(n);
    let mut it = buf.into_iter();
    let root = build_black(&mut it, n, bh);
    debug_assert!(
        it.next().is_none(),
        "builder did not consume the whole buffer"
    );
    root
}

/// Builds a black-rooted LLRB subtree of exactly `n` nodes and black height
/// `bh`, pulling nodes from `it` in ascending (in-order) sequence. A black node
/// alone is a 2-node; a black node with a red left child is a 3-node. The 2-3
/// sizing keeps the tree LLRB-valid for arbitrary `n`.
fn build_black<K, V, I: Iterator<Item = (K, V)>>(
    it: &mut I,
    n: usize,
    bh: usize,
) -> Option<Box<Node<K, V>>> {
    if bh == 0 {
        debug_assert_eq!(n, 0);
        return None;
    }
    let child_bh = bh - 1;
    let lo = min_size(child_bh);
    let hi = max_size(child_bh);

    // 2-node: black root with two black-rooted children. Representable when
    // `n - 1` (the two child subtrees) splits into [lo, hi] each.
    let two_node = n >= 1 && (n - 1) >= 2 * lo && (n - 1) <= 2 * hi;
    if two_node {
        let sizes = split_sizes(n - 1, 2, lo, hi);
        let left = build_black(it, sizes[0], child_bh);
        let (k, v) = it.next().expect("buffer underrun in build_black");
        let right = build_black(it, sizes[1], child_bh);
        return Some(Box::new(Node {
            key: k,
            value: v,
            left,
            right,
            red: false,
            size: n,
        }));
    }

    // 3-node: black root whose left child is red, three black-rooted grandchildren.
    debug_assert!(n >= 2 && (n - 2) >= 3 * lo && (n - 2) <= 3 * hi);
    let sizes = split_sizes(n - 2, 3, lo, hi);
    let a = build_black(it, sizes[0], child_bh);
    let (red_k, red_v) = it.next().expect("buffer underrun in build_black");
    let c = build_black(it, sizes[1], child_bh);
    let red_left = Some(Box::new(Node {
        key: red_k,
        value: red_v,
        left: a,
        right: c,
        red: true,
        size: sizes[0] + sizes[1] + 1,
    }));
    let (root_k, root_v) = it.next().expect("buffer underrun in build_black");
    let right = build_black(it, sizes[2], child_bh);
    Some(Box::new(Node {
        key: root_k,
        value: root_v,
        left: red_left,
        right,
        red: false,
        size: n,
    }))
}

/// Streaming bulk builder for [`TreeMap`]. Accepts strictly-ascending `(K, V)`
/// pairs via [`put`](TreeMapSink::put) / [`put_all`](TreeMapSink::put_all),
/// validates order with the comparator as it goes, and assembles the tree in
/// O(n) on [`create`](TreeMapSink::create).
///
/// The sink is **poisoned** after any error: subsequent `put`/`create` calls
/// fail with the same error. `create` is once-only (it consumes `self`).
pub struct TreeMapSink<K, V> {
    cmp: Comparator<K>,
    dup: DuplicatePolicy,
    buf: Vec<(K, V)>,
    index: usize,
    poisoned: Option<BulkError>,
}

impl<K, V> TreeMapSink<K, V> {
    /// Starts a fresh sorted bulk build under `cmp` with duplicate policy `dup`.
    pub fn new(cmp: Comparator<K>, dup: DuplicatePolicy) -> Self {
        TreeMapSink {
            cmp,
            dup,
            buf: Vec::new(),
            index: 0,
            poisoned: None,
        }
    }

    fn poison(&mut self, err: BulkError) -> BulkError {
        let cloned = clone_err(&err);
        self.poisoned = Some(err);
        cloned
    }

    /// Appends one prepared `(key, value)`. The key must be strictly greater
    /// than the previous key under the comparator.
    pub fn put(&mut self, key: K, value: V) -> Result<(), BulkError> {
        if let Some(ref e) = self.poisoned {
            return Err(clone_err(e));
        }
        let index = self.index;
        self.index += 1;
        if let Some((prev_k, _)) = self.buf.last() {
            match self.cmp.compare(prev_k, &key) {
                Ordering::Less => {}
                Ordering::Equal => match self.dup {
                    DuplicatePolicy::Error => {
                        return Err(self.poison(BulkError::Duplicate { index }));
                    }
                    DuplicatePolicy::IgnoreDuplicates => return Ok(()), // first wins
                },
                Ordering::Greater => {
                    return Err(self.poison(BulkError::OutOfOrder { index }));
                }
            }
        }
        self.buf.push((key, value));
        Ok(())
    }

    /// Convenience: `put` every element of `iter`, short-circuiting on the
    /// first error (which also poisons the sink).
    pub fn put_all<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) -> Result<(), BulkError> {
        for (k, v) in iter {
            self.put(k, v)?;
        }
        Ok(())
    }

    /// Finishes the build, returning the constructed `TreeMap`. Consuming
    /// `self` makes `create` once-only.
    ///
    /// A poisoned sink (any prior `put` error) **panics in all build modes** —
    /// never returns a half-built collection. The data-pump contract requires
    /// that a failed pump never yields a partial result; use
    /// [`try_create`](TreeMapSink::try_create) for the fallible form.
    pub fn create(self) -> TreeMap<K, V> {
        match self.try_create() {
            Ok(map) => map,
            Err(e) => panic!("create() called on a poisoned sink: {e:?}"),
        }
    }

    /// Like [`create`](TreeMapSink::create) but returns the poison error
    /// instead of panicking, so a poisoned sink is observable to callers that
    /// prefer a `Result`.
    pub fn try_create(self) -> Result<TreeMap<K, V>, BulkError> {
        if let Some(e) = self.poisoned {
            return Err(e);
        }
        Ok(TreeMap::from_sorted_buffer(self.buf, self.cmp))
    }
}

/// `BulkError` is not `Clone` (it wraps a non-`Clone` `TryReserveError`), but a
/// poisoned sink needs to hand the same logical error to every later call. The
/// data-shape variants are the only ones a sink ever poisons with, so this is a
/// faithful, allocation-free re-creation.
fn clone_err(e: &BulkError) -> BulkError {
    match e {
        BulkError::Duplicate { index } => BulkError::Duplicate { index: *index },
        BulkError::OutOfOrder { index } => BulkError::OutOfOrder { index: *index },
        BulkError::CountOverflow { index } => BulkError::CountOverflow { index: *index },
        BulkError::ExactSizeExceeded { expected } => BulkError::ExactSizeExceeded {
            expected: *expected,
        },
        BulkError::IndexOverflow { index } => BulkError::IndexOverflow { index: *index },
        // A sink only ever poisons via `put`, which produces Duplicate or
        // OutOfOrder — never Alloc (the allocation happens in `create`). This
        // arm is unreachable.
        BulkError::Alloc(_) => unreachable!("sink never poisons with an Alloc error"),
    }
}

// ── Free functions for tree manipulation ────────────────────────────

fn is_red<K, V>(node: &Option<Box<Node<K, V>>>) -> bool {
    node.as_ref().is_some_and(|n| n.red)
}

fn rotate_left<K, V>(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
    let mut r = node.right.take().unwrap();
    node.right = r.left.take();
    r.red = node.red;
    node.red = true;
    // `node` keeps `r`'s old subtree size (it now occupies `r`'s former
    // position); recompute both bottom-up: the demoted `node` first, then
    // the promoted `r`.
    let old_size = node.size;
    fix_size(&mut node);
    r.left = Some(node);
    r.size = old_size;
    r
}

fn rotate_right<K, V>(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
    let mut l = node.left.take().unwrap();
    node.left = l.right.take();
    l.red = node.red;
    node.red = true;
    let old_size = node.size;
    fix_size(&mut node);
    l.right = Some(node);
    l.size = old_size;
    l
}

fn flip_colors<K, V>(node: &mut Box<Node<K, V>>) {
    node.red = !node.red;
    if let Some(ref mut left) = node.left {
        left.red = !left.red;
    }
    if let Some(ref mut right) = node.right {
        right.red = !right.red;
    }
}

fn fix_up<K, V>(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
    // A child subtree may have changed below us (insert/remove descended
    // through one side); refresh this node's cached size before the
    // rotations read it, then each rotation maintains its own sizes.
    fix_size(&mut node);
    if is_red(&node.right) && !is_red(&node.left) {
        node = rotate_left(node);
    }
    if is_red(&node.left) && node.left.as_ref().is_some_and(|l| is_red(&l.left)) {
        node = rotate_right(node);
    }
    if is_red(&node.left) && is_red(&node.right) {
        flip_colors(&mut node);
    }
    node
}

fn move_red_left<K, V>(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
    flip_colors(&mut node);
    if node.right.as_ref().is_some_and(|r| is_red(&r.left)) {
        let r = rotate_right(node.right.take().unwrap());
        node.right = Some(r);
        node = rotate_left(node);
        flip_colors(&mut node);
    }
    node
}

fn move_red_right<K, V>(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
    flip_colors(&mut node);
    if node.left.as_ref().is_some_and(|l| is_red(&l.left)) {
        node = rotate_right(node);
        flip_colors(&mut node);
    }
    node
}

fn delete_min_node<K, V>(node: Option<Box<Node<K, V>>>) -> (Option<Box<Node<K, V>>>, K, V) {
    let mut node = node.unwrap();

    if node.left.is_none() {
        return (None, node.key, node.value);
    }

    if !is_red(&node.left) && !node.left.as_ref().is_some_and(|l| is_red(&l.left)) {
        node = move_red_left(node);
    }

    let (new_left, min_k, min_v) = delete_min_node(node.left.take());
    node.left = new_left;
    (Some(fix_up(node)), min_k, min_v)
}

fn min_ref<K, V>(node: &Option<Box<Node<K, V>>>) -> Option<&Node<K, V>> {
    let mut current = node.as_ref()?;
    while let Some(ref left) = current.left {
        current = left;
    }
    Some(current)
}

fn max_ref<K, V>(node: &Option<Box<Node<K, V>>>) -> Option<&Node<K, V>> {
    let mut current = node.as_ref()?;
    while let Some(ref right) = current.right {
        current = right;
    }
    Some(current)
}

fn in_order<K, V>(node: &Option<Box<Node<K, V>>>, f: &mut impl FnMut(&K, &V)) {
    if let Some(ref n) = node {
        in_order(&n.left, f);
        f(&n.key, &n.value);
        in_order(&n.right, f);
    }
}

fn push_left_spine<'a, K, V>(node: &'a Option<Box<Node<K, V>>>, stack: &mut Vec<&'a Node<K, V>>) {
    let mut current = node;
    while let Some(ref n) = current {
        stack.push(n);
        current = &n.left;
    }
}

/// Iterator over `(&K, &V)` pairs of a `TreeMap` in sorted order.
pub struct TreeMapIter<'a, K, V> {
    stack: Vec<&'a Node<K, V>>,
}

impl<'a, K, V> Iterator for TreeMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        let result = (&node.key, &node.value);
        push_left_spine(&node.right, &mut self.stack);
        Some(result)
    }
}

// ── Range iterator (double-ended, exact-size via the size augmentation) ──

/// Seeds an ascending in-order stack whose top is the node at 0-based sorted
/// index `i` (empty if `i >= len`). Ancestors visited later sit below it.
fn seed_forward<K, V>(root: &Option<Box<Node<K, V>>>, mut i: usize) -> Vec<&Node<K, V>> {
    let mut stack = Vec::new();
    let mut current = root.as_deref();
    while let Some(n) = current {
        let l = node_size(&n.left);
        match i.cmp(&l) {
            Ordering::Less => {
                stack.push(n);
                current = n.left.as_deref();
            }
            Ordering::Equal => {
                stack.push(n);
                break;
            }
            Ordering::Greater => {
                i -= l + 1;
                current = n.right.as_deref();
            }
        }
    }
    stack
}

/// Seeds a descending (reverse in-order) stack whose top is the node at 0-based
/// sorted index `j` (empty if the tree is empty; a too-large `j` clamps to the
/// max element's path — harmless because `remaining` gates emission).
fn seed_backward<K, V>(root: &Option<Box<Node<K, V>>>, mut j: usize) -> Vec<&Node<K, V>> {
    let mut stack = Vec::new();
    let mut current = root.as_deref();
    while let Some(n) = current {
        let l = node_size(&n.left);
        match j.cmp(&l) {
            Ordering::Greater => {
                stack.push(n);
                j -= l + 1;
                current = n.right.as_deref();
            }
            Ordering::Equal => {
                stack.push(n);
                break;
            }
            Ordering::Less => {
                current = n.left.as_deref();
            }
        }
    }
    stack
}

/// A lazy, double-ended, exact-size iterator over a key range of a [`TreeMap`],
/// returned by [`TreeMap::range`]. Front and back share a `remaining` count
/// (from the subtree-size augmentation), so the ends cannot cross and no
/// per-item bound comparison is needed.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct RangeIter<'a, K, V> {
    front: Vec<&'a Node<K, V>>,
    back: Vec<&'a Node<K, V>>,
    remaining: usize,
}

impl<'a, K, V> Iterator for RangeIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let n = self.front.pop()?;
        // Push the left spine of the right child (in-order successor path).
        let mut current = n.right.as_deref();
        while let Some(x) = current {
            self.front.push(x);
            current = x.left.as_deref();
        }
        self.remaining -= 1;
        Some((&n.key, &n.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K, V> DoubleEndedIterator for RangeIter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let n = self.back.pop()?;
        // Push the right spine of the left child (reverse in-order path).
        let mut current = n.left.as_deref();
        while let Some(x) = current {
            self.back.push(x);
            current = x.right.as_deref();
        }
        self.remaining -= 1;
        Some((&n.key, &n.value))
    }
}

impl<K, V> ExactSizeIterator for RangeIter<'_, K, V> {}
impl<K, V> std::iter::FusedIterator for RangeIter<'_, K, V> {}

/// Borrowing iteration in sorted order: `for (k, v) in &map`.
impl<'a, K, V, C: Compare<K>> IntoIterator for &'a TreeMap<K, V, C> {
    type Item = (&'a K, &'a V);
    type IntoIter = TreeMapIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Consumes the tree in ascending order into `out`. Recursion depth is
/// O(log n) on the balanced LLRB.
fn consume_in_order<K, V>(node: Option<Box<Node<K, V>>>, out: &mut Vec<(K, V)>) {
    if let Some(n) = node {
        let Node {
            key,
            value,
            left,
            right,
            ..
        } = *n;
        consume_in_order(left, out);
        out.push((key, value));
        consume_in_order(right, out);
    }
}

/// Owning iterator over `(K, V)` pairs in ascending comparator order, from
/// `TreeMap::into_iter`.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct TreeMapIntoIter<K, V> {
    inner: std::vec::IntoIter<(K, V)>,
}

impl<K, V> Iterator for TreeMapIntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<(K, V)> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<K, V> DoubleEndedIterator for TreeMapIntoIter<K, V> {
    fn next_back(&mut self) -> Option<(K, V)> {
        self.inner.next_back()
    }
}
impl<K, V> ExactSizeIterator for TreeMapIntoIter<K, V> {}
impl<K, V> std::iter::FusedIterator for TreeMapIntoIter<K, V> {}

/// Owned iteration in sorted order: `for (k, v) in map`, yielding `(K, V)` by
/// value — the bulk ownership-transfer exit.
impl<K, V, C> IntoIterator for TreeMap<K, V, C> {
    type Item = (K, V);
    type IntoIter = TreeMapIntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        let mut out = Vec::with_capacity(self.size);
        consume_in_order(self.root, &mut out);
        TreeMapIntoIter {
            inner: out.into_iter(),
        }
    }
}

impl<K, V, C> TreeMap<K, V, C> {
    /// Consumes the map, yielding keys in ascending order.
    pub fn into_keys(self) -> impl DoubleEndedIterator<Item = K> + ExactSizeIterator {
        self.into_iter().map(|(k, _)| k)
    }

    /// Consumes the map, yielding values in ascending key order.
    pub fn into_values(self) -> impl DoubleEndedIterator<Item = V> + ExactSizeIterator {
        self.into_iter().map(|(_, v)| v)
    }
}

impl<K: Ord, V> Default for TreeMap<K, V, Natural> {
    /// An empty map ordered by natural [`Ord`].
    fn default() -> Self {
        Self::natural()
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for TreeMap<K, V, Natural> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = TreeMap::natural();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl<K: Ord, V> Extend<(K, V)> for TreeMap<K, V, Natural> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::strategy::*;

    #[test]
    fn test_basic_insert_get() {
        let mut m = TreeMap::new(natural_comparator::<String>());
        m.insert("banana".to_string(), 2);
        m.insert("apple".to_string(), 1);
        m.insert("cherry".to_string(), 3);

        assert_eq!(m.len(), 3);
        assert_eq!(m.get(&"apple".to_string()), Some(&1));
        assert_eq!(m.get(&"banana".to_string()), Some(&2));
        assert_eq!(m.get(&"cherry".to_string()), Some(&3));
        assert_eq!(m.get(&"date".to_string()), None);
    }

    #[test]
    fn test_sorted_iteration() {
        let mut m = TreeMap::new(natural_comparator::<String>());
        m.insert("banana".to_string(), 2);
        m.insert("apple".to_string(), 1);
        m.insert("cherry".to_string(), 3);

        let keys: Vec<&String> = m.keys().collect();
        assert_eq!(
            keys,
            vec![
                &"apple".to_string(),
                &"banana".to_string(),
                &"cherry".to_string()
            ]
        );
    }

    #[test]
    fn test_overwrite() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        m.insert(1, "one".to_string());
        let old = m.insert(1, "ONE".to_string());
        assert_eq!(old, Some("one".to_string()));
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(&1), Some(&"ONE".to_string()));
    }

    #[test]
    fn test_remove() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        for i in 0..100 {
            m.insert(i, i * 10);
        }
        for i in (0..100).step_by(2) {
            m.remove(&i);
        }
        assert_eq!(m.len(), 50);
        for (k, _) in m.iter() {
            assert!(k % 2 != 0, "even key {} should have been removed", k);
        }
    }

    #[test]
    fn test_min_max() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        assert!(m.min().is_none());
        m.insert(5, "five".to_string());
        m.insert(1, "one".to_string());
        m.insert(9, "nine".to_string());

        let (k, _) = m.min().unwrap();
        assert_eq!(*k, 1);
        let (k, _) = m.max().unwrap();
        assert_eq!(*k, 9);
    }

    #[test]
    fn test_reverse_comparator() {
        let mut m = TreeMap::new(reverse_comparator::<i32>());
        m.insert(1, 10);
        m.insert(3, 30);
        m.insert(2, 20);

        let keys: Vec<&i32> = m.keys().collect();
        assert_eq!(keys, vec![&3, &2, &1]);
    }

    #[test]
    fn owned_into_iter_and_into_keys_values() {
        let m: TreeMap<i32, i32, Natural> = (0..5).map(|i| (i, i * 10)).collect();
        let pairs: Vec<(i32, i32)> = m.into_iter().collect();
        assert_eq!(pairs, vec![(0, 0), (1, 10), (2, 20), (3, 30), (4, 40)]);

        let m2: TreeMap<i32, i32, Natural> = (0..5).map(|i| (i, i * 10)).collect();
        assert_eq!(m2.into_keys().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
        let m3: TreeMap<i32, i32, Natural> = (0..5).map(|i| (i, i * 10)).collect();
        assert_eq!(
            m3.into_values().rev().collect::<Vec<_>>(),
            vec![40, 30, 20, 10, 0]
        );
    }

    #[test]
    fn range_bounds_all_shapes() {
        let m: TreeMap<i32, i32, Natural> = (0..10).map(|i| (i, i * 10)).collect();
        let keys = |it: RangeIter<'_, i32, i32>| it.map(|(k, _)| *k).collect::<Vec<_>>();
        assert_eq!(keys(m.range(3..7)), vec![3, 4, 5, 6]);
        assert_eq!(keys(m.range(3..=7)), vec![3, 4, 5, 6, 7]);
        assert_eq!(keys(m.range(..3)), vec![0, 1, 2]);
        assert_eq!(keys(m.range(7..)), vec![7, 8, 9]);
        assert_eq!(keys(m.range(..)), (0..10).collect::<Vec<_>>());
        use std::ops::Bound::{Excluded, Unbounded};
        assert_eq!(
            keys(m.range((Excluded(3), Unbounded))),
            vec![4, 5, 6, 7, 8, 9]
        );
        // exact-size + double-ended.
        let mut it = m.range(2..8);
        assert_eq!(it.len(), 6);
        assert_eq!(it.next().map(|(k, _)| *k), Some(2));
        assert_eq!(it.next_back().map(|(k, _)| *k), Some(7));
        assert_eq!(it.len(), 4);
        // descending via .rev()
        let desc: Vec<i32> = m.range(2..8).rev().map(|(k, _)| *k).collect();
        assert_eq!(desc, vec![7, 6, 5, 4, 3, 2]);
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)] // deliberately testing empty/inverted ranges
    fn range_empty_and_inverted_are_empty_not_panic() {
        let m: TreeMap<i32, i32, Natural> = (0..10).map(|i| (i, i)).collect();
        assert_eq!(m.range(5..5).count(), 0); // empty
        assert_eq!(m.range(8..2).count(), 0); // inverted -> empty, no panic
        use std::ops::Bound::Excluded;
        assert_eq!(m.range((Excluded(3), Excluded(4))).count(), 0); // (3,4) over ints
        let empty: TreeMap<i32, i32, Natural> = TreeMap::natural();
        assert_eq!(empty.range(..).count(), 0);
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)] // `7..=3` is meaningful under the reverse comparator
    fn range_is_comparator_correct_under_reverse() {
        // The headline T4 win: range() compares bounds through the map's OWN
        // comparator, so a reverse-ordered map ranges in reverse order — the
        // exact case the legacy natural-order-only range_keys got wrong.
        let mut m = TreeMap::new(reverse_comparator::<i32>());
        for k in 0..10 {
            m.insert(k, k);
        }
        // Under reverse order the keys descend 9,8,..,0. range(7..=3) means
        // "from 7 down to 3" in the map's order.
        let got: Vec<i32> = m.range(7..=3).map(|(k, _)| *k).collect();
        assert_eq!(got, vec![7, 6, 5, 4, 3]);
        // Cross-check every element is exactly those the comparator places in
        // [7,3] descending — and count matches ExactSize.
        assert_eq!(m.range(7..=3).len(), 5);
    }

    #[test]
    fn range_matches_naive_filter_randomized() {
        // Differential check against a brute-force scan across many bounds.
        let m: TreeMap<i32, i32, Natural> = (0..50).map(|i| (i * 2, i)).collect();
        for lo in [-1i32, 0, 1, 10, 49, 98, 99] {
            for hi in [-1i32, 0, 11, 50, 98, 100] {
                let via_range: Vec<i32> = m.range(lo..hi).map(|(k, _)| *k).collect();
                let naive: Vec<i32> = (0..50)
                    .map(|i| i * 2)
                    .filter(|k| *k >= lo && *k < hi)
                    .collect();
                assert_eq!(via_range, naive, "half-open [{lo},{hi})");
            }
        }
    }

    #[test]
    fn natural_comparator_type_param() {
        // Zero-sized Natural comparator: new(), Default, FromIterator, Extend.
        let mut m: TreeMap<i32, &str, Natural> = TreeMap::natural();
        m.insert(3, "c");
        m.insert(1, "a");
        m.insert(2, "b");
        assert_eq!(m.keys().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(m.get(&2), Some(&"b"));

        // FromIterator / collect.
        let c: TreeMap<i32, i32, Natural> = (0..5).map(|i| (i, i * i)).collect();
        assert_eq!(c.len(), 5);
        assert_eq!(c.get(&4), Some(&16));

        // Default is an empty Natural map.
        let d: TreeMap<i32, i32, Natural> = TreeMap::default();
        assert!(d.is_empty());
    }

    #[test]
    fn reverse_and_fncmp_type_params() {
        use crate::object::strategy::{FnCmp, Reverse};
        let mut r: TreeMap<i32, (), Reverse> = TreeMap::with_comparator(Reverse(Natural));
        for k in [1, 3, 2] {
            r.insert(k, ());
        }
        assert_eq!(r.keys().copied().collect::<Vec<_>>(), vec![3, 2, 1]);

        // Ad-hoc closure comparator by absolute value.
        let cmp = FnCmp(|a: &i32, b: &i32| a.abs().cmp(&b.abs()));
        let mut m = TreeMap::with_comparator(cmp);
        for k in [-5, 2, -1, 3] {
            m.insert(k, ());
        }
        assert_eq!(m.keys().copied().collect::<Vec<_>>(), vec![-1, 2, 3, -5]);
    }

    #[test]
    fn range_keys_is_natural_order_only_under_custom_comparator() {
        // Pins the documented NATURAL-ORDER-ONLY divergence of the legacy
        // `Range<K>` methods: membership `∈ range` is selected by natural `Ord`
        // (so the correct SET {1,2} is chosen), but the result order follows the
        // tree's comparator (here reverse), NOT the "ascending" the method name
        // implies. Comparator-correct queries are the job of `range()` (T4).
        let mut m = TreeMap::new(reverse_comparator::<i32>());
        m.insert(1, 10);
        m.insert(2, 20);
        m.insert(3, 30);
        // natural membership picks {1,2}; iteration is tree (reverse) order.
        assert_eq!(m.range_keys(Range::closed(1, 2)), vec![2, 1]);
    }

    #[derive(Debug, Clone)]
    struct Person {
        name: String,
        _age: i32,
    }

    #[test]
    fn test_by_field_comparator() {
        let mut m = TreeMap::new(comparator_by_field(|p: &Person| p.name.clone()));
        m.insert(
            Person {
                name: "Charlie".into(),
                _age: 30,
            },
            "c",
        );
        m.insert(
            Person {
                name: "Alice".into(),
                _age: 25,
            },
            "a",
        );
        m.insert(
            Person {
                name: "Bob".into(),
                _age: 35,
            },
            "b",
        );

        let names: Vec<&str> = m.keys().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_clear() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        m.insert(1, 1);
        m.insert(2, 2);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn test_stress_insert_sorted_order() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        for i in (0..500).rev() {
            m.insert(i, i);
        }
        assert_eq!(m.len(), 500);
        let keys: Vec<i32> = m.keys().copied().collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn test_for_each() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        m.insert(3, 30);
        m.insert(1, 10);
        m.insert(2, 20);
        let mut pairs = Vec::new();
        m.for_each(|k, v| pairs.push((*k, *v)));
        assert_eq!(pairs, vec![(1, 10), (2, 20), (3, 30)]);
    }

    #[test]
    fn test_contains_key() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        m.insert(1, 10);
        assert!(m.contains_key(&1));
        assert!(!m.contains_key(&2));
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        m.insert(1, 10);
        assert_eq!(m.remove(&2), None);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn test_remove_all() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        for i in 0..50 {
            m.insert(i, i);
        }
        for i in 0..50 {
            assert!(m.remove(&i).is_some());
        }
        assert!(m.is_empty());
    }

    #[test]
    fn test_into_iter_borrowing_sorted() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        m.insert(3, 30);
        m.insert(1, 10);
        m.insert(2, 20);
        let pairs: Vec<(i32, i32)> = (&m).into_iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(pairs, vec![(1, 10), (2, 20), (3, 30)]);
    }

    // ── NavigableMap surface ────────────────────────────────────────

    use crate::range::Range;

    fn map_of(keys: &[i32]) -> TreeMap<i32, i32> {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        for &k in keys {
            m.insert(k, k.wrapping_mul(10));
        }
        m
    }

    #[test]
    fn test_floor_ceiling_lower_higher() {
        let m = map_of(&[10, 20, 30]);
        assert_eq!(m.floor_key(&25), Some(&20));
        assert_eq!(m.ceiling_key(&25), Some(&30));
        assert_eq!(m.floor_key(&10), Some(&10)); // inclusive
        assert_eq!(m.lower_key(&10), None); // strict, nothing below
        assert_eq!(m.higher_key(&30), None); // strict, nothing above
        assert_eq!(m.ceiling_key(&5), Some(&10));
        assert_eq!(m.lower_key(&25), Some(&20));
        assert_eq!(m.higher_key(&25), Some(&30));
        // entry forms carry value = key*10.
        assert_eq!(m.floor_entry(&25), Some((&20, &200)));
        assert_eq!(m.ceiling_entry(&25), Some((&30, &300)));
        assert_eq!(m.first_key(), Some(&10));
        assert_eq!(m.last_key(), Some(&30));
    }

    #[test]
    fn test_nav_empty() {
        let m: TreeMap<i32, i32> = map_of(&[]);
        assert_eq!(m.floor_key(&5), None);
        assert_eq!(m.ceiling_key(&5), None);
        assert_eq!(m.lower_key(&5), None);
        assert_eq!(m.higher_key(&5), None);
        assert_eq!(m.first_key(), None);
        assert_eq!(m.last_key(), None);
    }

    #[test]
    fn test_nav_signed_extremes() {
        let m = map_of(&[i32::MIN, -1, 0, 1, i32::MAX]);
        assert_eq!(m.floor_key(&i32::MIN), Some(&i32::MIN));
        assert_eq!(m.lower_key(&i32::MIN), None);
        assert_eq!(m.higher_key(&-1), Some(&0));
        assert_eq!(m.ceiling_key(&i32::MAX), Some(&i32::MAX));
        assert_eq!(m.higher_key(&i32::MAX), None);
        assert_eq!(m.descending_keys(), vec![i32::MAX, 1, 0, -1, i32::MIN]);
    }

    #[test]
    fn test_poll_first_last() {
        let mut m = map_of(&[10, 20, 30]);
        assert_eq!(m.poll_first_entry(), Some((10, 100)));
        assert_eq!(m.poll_last_entry(), Some((30, 300)));
        assert_eq!(m.len(), 1);
        assert_eq!(m.poll_first_entry(), Some((20, 200)));
        // now empty: returns None, does not trap.
        assert_eq!(m.poll_first_entry(), None);
        assert_eq!(m.poll_last_entry(), None);
    }

    #[test]
    fn test_poll_single_then_empty() {
        let mut m = map_of(&[]);
        m.insert(7, 700);
        assert_eq!(m.poll_first_entry(), Some((7, 700)));
        assert_eq!(m.poll_first_entry(), None);
        assert!(m.is_empty());
    }

    #[test]
    fn test_range_closed_open() {
        let m = map_of(&[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        assert_eq!(
            m.range_keys(Range::closed_open(30, 70)),
            vec![30, 40, 50, 60]
        );
        assert_eq!(
            m.descending_range_keys(Range::closed_open(30, 70)),
            vec![60, 50, 40, 30]
        );
        assert_eq!(
            m.range_entries(Range::closed_open(30, 50)),
            vec![(30, 300), (40, 400)]
        );
    }

    #[test]
    fn test_range_open_no_integer_is_empty() {
        // open(1, 2) over i32 matches NOTHING (membership = contains), but
        // is not cut-empty.
        let mut m = map_of(&[1, 2]);
        assert_eq!(m.range_keys(Range::open(1, 2)), Vec::<i32>::new());
        assert_eq!(m.remove_range(Range::open(1, 2)), 0);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_remove_range_count_and_noop() {
        let mut m = map_of(&[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        assert_eq!(m.remove_range(Range::closed_open(30, 70)), 4);
        assert_eq!(m.remove_range(Range::closed_open(30, 70)), 0); // no-op
        let keys: Vec<i32> = m.keys().copied().collect();
        assert_eq!(keys, vec![10, 20, 70, 80, 90, 100]);
    }

    // ── Order statistics (rank / select) ────────────────────────────

    #[test]
    fn test_rank_present_and_absent() {
        let m = map_of(&[10, 20, 30, 40, 50]);
        // present keys → their 0-based index
        assert_eq!(m.rank(&10), 0);
        assert_eq!(m.rank(&30), 2);
        assert_eq!(m.rank(&50), 4);
        // absent keys → lower-bound index
        assert_eq!(m.rank(&5), 0); // before min
        assert_eq!(m.rank(&25), 2); // between 20 and 30
        assert_eq!(m.rank(&55), 5); // past max → size
    }

    #[test]
    fn test_select_key_and_entry() {
        let m = map_of(&[10, 20, 30, 40, 50]);
        assert_eq!(m.select_key(0), Some(&10));
        assert_eq!(m.select_key(2), Some(&30));
        assert_eq!(m.select_key(4), Some(&50));
        assert_eq!(m.select_key(5), None); // == size, out of range
        assert_eq!(m.select_key(999), None);
        // entry form carries value = key*10
        assert_eq!(m.select_entry(0), Some((&10, &100)));
        assert_eq!(m.select_entry(2), Some((&30, &300)));
        assert_eq!(m.select_entry(5), None);
    }

    #[test]
    fn test_rank_select_empty_single() {
        let empty: TreeMap<i32, i32> = map_of(&[]);
        assert_eq!(empty.rank(&5), 0);
        assert_eq!(empty.select_key(0), None);

        let mut single = map_of(&[]);
        single.insert(7, 70);
        assert_eq!(single.rank(&6), 0);
        assert_eq!(single.rank(&7), 0);
        assert_eq!(single.rank(&8), 1);
        assert_eq!(single.select_key(0), Some(&7));
        assert_eq!(single.select_entry(0), Some((&7, &70)));
        assert_eq!(single.select_key(1), None);
    }

    #[test]
    fn test_rank_select_signed_extremes() {
        let m = map_of(&[i32::MIN, -1, 0, 1, i32::MAX]);
        assert_eq!(m.rank(&i32::MIN), 0);
        assert_eq!(m.rank(&0), 2);
        assert_eq!(m.rank(&i32::MAX), 4);
        assert_eq!(m.select_key(0), Some(&i32::MIN));
        assert_eq!(m.select_key(4), Some(&i32::MAX));
        assert_eq!(m.select_key(5), None);
    }

    #[test]
    fn test_rank_select_after_remove() {
        let mut m = map_of(&[10, 20, 30, 40, 50]);
        assert_eq!(m.remove(&30), Some(300));
        let keys: Vec<i32> = m.keys().copied().collect();
        assert_eq!(keys, vec![10, 20, 40, 50]);
        // stale subtree sizes after a remove/transplant would corrupt these
        assert_eq!(m.rank(&40), 2);
        assert_eq!(m.rank(&35), 2);
        assert_eq!(m.select_key(2), Some(&40));
        assert_eq!(m.select_key(4), None);
        m.assert_size_invariant();
    }

    #[test]
    fn test_round_trip_select_rank() {
        let m = map_of(&[10, 20, 30, 40, 50, -7, 0, 99]);
        // select(rank(k)) == k for every present key
        for k in m.keys().copied().collect::<Vec<_>>() {
            assert_eq!(m.select_key(m.rank(&k)), Some(&k));
        }
        // rank(select(i)) == i for every 0 <= i < size
        for i in 0..m.len() {
            let k = m.select_key(i).copied().unwrap();
            assert_eq!(m.rank(&k), i);
        }
        // select(size) is absence
        assert_eq!(m.select_key(m.len()), None);
    }

    /// Deterministic xorshift so the randomized invariant test never relies
    /// on external randomness (and stays reproducible across ports).
    fn next_rand(state: &mut u64) -> u64 {
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *state = x;
        x
    }

    #[test]
    fn test_size_invariant_randomized_insert_remove() {
        let mut m = TreeMap::new(natural_comparator::<i32>());
        let mut present = std::collections::BTreeSet::new();
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..4000 {
            let key = (next_rand(&mut state) % 200) as i32;
            if next_rand(&mut state) & 1 == 0 {
                m.insert(key, key.wrapping_mul(10));
                present.insert(key);
            } else {
                m.remove(&key);
                present.remove(&key);
            }
            m.assert_size_invariant();
            assert_eq!(m.len(), present.len());
        }
        // After the churn, rank/select must agree with the oracle ordering.
        let sorted: Vec<i32> = present.iter().copied().collect();
        for (i, &k) in sorted.iter().enumerate() {
            assert_eq!(m.rank(&k), i);
            assert_eq!(m.select_key(i), Some(&k));
        }
        assert_eq!(m.select_key(sorted.len()), None);
    }

    #[test]
    fn test_rank_select_reverse_comparator() {
        // Order statistics follow the comparator: under reverse order the
        // 0-th element is the largest natural key.
        let mut m = TreeMap::new(reverse_comparator::<i32>());
        for k in [10, 20, 30, 40, 50] {
            m.insert(k, k * 10);
        }
        assert_eq!(m.select_key(0), Some(&50));
        assert_eq!(m.select_key(4), Some(&10));
        assert_eq!(m.rank(&50), 0);
        assert_eq!(m.rank(&10), 4);
        m.assert_size_invariant();
    }

    #[test]
    fn test_sub_map_independence() {
        let mut m = map_of(&[10, 20, 30, 40, 50]);
        let mut snap = m.sub_map(Range::closed(20, 40));
        let snap_keys: Vec<i32> = snap.keys().copied().collect();
        assert_eq!(snap_keys, vec![20, 30, 40]);
        // Mutate snapshot — original unchanged.
        snap.insert(99, 990);
        snap.remove(&20);
        assert!(m.contains_key(&20));
        assert!(!m.contains_key(&99));
        // Mutate original — snapshot unchanged.
        m.remove(&30);
        assert!(snap.contains_key(&30));
    }

    // ---- Data pump (from_sorted / TreeMapSink) ----

    use crate::bulk::{BulkError, DuplicatePolicy};

    fn pumped(n: i32) -> TreeMap<i32, i32> {
        let data: Vec<(i32, i32)> = (0..n).map(|i| (i, i * 10)).collect();
        TreeMap::from_sorted(natural_comparator::<i32>(), data, DuplicatePolicy::Error).unwrap()
    }

    #[test]
    fn pump_equals_incremental_iteration_order() {
        for &n in &[0, 1, 2, 3, 7, 8, 16, 100, 500] {
            let m = pumped(n);
            let pumped_pairs: Vec<(i32, i32)> = m.iter().map(|(k, v)| (*k, *v)).collect();
            let mut inc = TreeMap::new(natural_comparator::<i32>());
            // insert shuffled-ish (reverse) to prove order-independence.
            for i in (0..n).rev() {
                inc.insert(i, i * 10);
            }
            let inc_pairs: Vec<(i32, i32)> = inc.iter().map(|(k, v)| (*k, *v)).collect();
            assert_eq!(pumped_pairs, inc_pairs, "mismatch at n={n}");
            assert_eq!(m.len(), n as usize);
        }
    }

    // The critical gate: the bulk builder must produce a valid LLRB for every n
    // (BST order, no right red, no double red, uniform black-height, black root).
    #[test]
    fn pump_produces_valid_llrb_all_sizes() {
        for n in 0..2000i32 {
            let m = pumped(n);
            assert!(m.is_valid_llrb(), "invalid LLRB at n={n}");
        }
    }

    // Boundary sizes around the 2^h-1 / 3^h-1 transitions (codex's list).
    #[test]
    fn pump_valid_llrb_at_boundaries() {
        for &n in &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 26, 27, 28, 63, 64, 65, 80, 81, 82, 242, 243,
            244,
        ] {
            let m = pumped(n);
            assert!(m.is_valid_llrb(), "invalid LLRB at boundary n={n}");
            let keys: Vec<i32> = m.keys().copied().collect();
            assert_eq!(keys, (0..n).collect::<Vec<_>>());
        }
    }

    // Post-build random insert/remove must keep the tree a valid LLRB and stay
    // observably equal to a std BTreeMap oracle. This is where a right-leaning
    // coloring bug would surface.
    #[test]
    fn pump_then_mutations_match_oracle() {
        struct Rng(u64);
        impl Rng {
            fn next(&mut self) -> u64 {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                self.0 >> 33
            }
        }
        let mut rng = Rng(12345);
        for trial in 0..50 {
            let n = (rng.next() % 400) as i32;
            // step keeps keys spaced so inserts/removes hit both hit and miss.
            let data: Vec<(i32, i32)> = (0..n).map(|i| (i * 2, i)).collect();
            let mut tree = TreeMap::from_sorted(
                natural_comparator::<i32>(),
                data.clone(),
                DuplicatePolicy::Error,
            )
            .unwrap();
            let mut oracle: std::collections::BTreeMap<i32, i32> = data.into_iter().collect();
            assert!(tree.is_valid_llrb(), "invalid after build, trial {trial}");

            for _ in 0..300 {
                let k = (rng.next() % 800) as i32;
                if rng.next() % 2 == 0 {
                    let v = (rng.next() % 1000) as i32;
                    tree.insert(k, v);
                    oracle.insert(k, v);
                } else {
                    tree.remove(&k);
                    oracle.remove(&k);
                }
                assert!(
                    tree.is_valid_llrb(),
                    "invalid LLRB mid-mutation, trial {trial}"
                );
                let tk: Vec<(i32, i32)> = tree.iter().map(|(k, v)| (*k, *v)).collect();
                let ok: Vec<(i32, i32)> = oracle.iter().map(|(k, v)| (*k, *v)).collect();
                assert_eq!(tk, ok, "diverged from oracle, trial {trial}");
            }
        }
    }

    #[test]
    fn pump_out_of_order_errors_at_index() {
        // first / middle / last out-of-order positions.
        let mut sink = TreeMapSink::new(natural_comparator::<i32>(), DuplicatePolicy::Error);
        sink.put(5, 0).unwrap();
        let err = sink.put(3, 0).unwrap_err(); // index 1
        assert!(matches!(err, BulkError::OutOfOrder { index: 1 }));

        // via from_sorted, middle.
        let err = TreeMap::from_sorted(
            natural_comparator::<i32>(),
            vec![(1, 0), (2, 0), (2, 0)],
            DuplicatePolicy::Error,
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::Duplicate { index: 2 }));

        let err = TreeMap::from_sorted(
            natural_comparator::<i32>(),
            vec![(1, 0), (5, 0), (4, 0)],
            DuplicatePolicy::Error,
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 2 }));
    }

    #[test]
    fn pump_ignore_duplicates_first_wins() {
        let m = TreeMap::from_sorted(
            natural_comparator::<i32>(),
            vec![(1, 10), (1, 20), (1, 30), (2, 99)],
            DuplicatePolicy::IgnoreDuplicates,
        )
        .unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&1), Some(&10));
        assert_eq!(m.get(&2), Some(&99));
        assert!(m.is_valid_llrb());
    }

    #[test]
    fn pump_sink_poison_and_double_create() {
        // Poison: after an error, put/create fail.
        let mut sink = TreeMapSink::new(natural_comparator::<i32>(), DuplicatePolicy::Error);
        sink.put(1, 0).unwrap();
        assert!(sink.put(1, 0).is_err()); // duplicate -> poison
        assert!(sink.put(2, 0).is_err()); // still poisoned
        assert!(matches!(
            sink.try_create().unwrap_err(),
            BulkError::Duplicate { index: 1 }
        ));
    }

    #[test]
    #[should_panic(expected = "poisoned sink")]
    fn pump_sink_create_after_error_panics_in_all_modes() {
        // The public `create()` must never return a half-built collection after
        // an error — it must abort in release as well as debug. `#[should_panic]`
        // is enforced in both profiles, unlike `debug_assert!`.
        let mut sink = TreeMapSink::new(natural_comparator::<i32>(), DuplicatePolicy::Error);
        sink.put(1, 0).unwrap();
        assert!(sink.put(1, 0).is_err()); // duplicate -> poison
        let _ = sink.create(); // must panic, not return the prefix
    }

    #[test]
    fn pump_sink_empty_input() {
        let sink =
            TreeMapSink::<i32, i32>::new(natural_comparator::<i32>(), DuplicatePolicy::Error);
        let m = sink.create();
        assert!(m.is_empty());
        assert!(m.is_valid_llrb());
    }

    #[test]
    fn pump_float_keys_total_order_via_comparator() {
        use crate::HashableF64;
        // Strictly-ascending under HashableF64's total_cmp (Ord):
        // -NaN(payload) < -Inf < -0 < +0 < +Inf < +NaN(canonical).
        let neg_nan = HashableF64(f64::from_bits(0xFFF8_0000_0000_0000));
        let data = vec![
            (neg_nan, 1),
            (HashableF64(f64::NEG_INFINITY), 2),
            (HashableF64(-0.0), 3),
            (HashableF64(0.0), 4),
            (HashableF64(f64::INFINITY), 5),
            (HashableF64(f64::NAN), 6),
        ];
        let m = TreeMap::from_sorted(
            natural_comparator::<HashableF64>(),
            data,
            DuplicatePolicy::Error,
        )
        .unwrap();
        assert_eq!(m.len(), 6);
        assert!(m.is_valid_llrb());
        let vals: Vec<i32> = m.values().copied().collect();
        assert_eq!(vals, vec![1, 2, 3, 4, 5, 6]);

        // Using raw `<` would wrongly reject this ordering; the comparator
        // (total_cmp) accepts it. Confirm a genuinely out-of-order float fails.
        let bad = vec![(HashableF64(1.0), 0), (HashableF64(f64::NEG_INFINITY), 0)];
        let err = TreeMap::from_sorted(
            natural_comparator::<HashableF64>(),
            bad,
            DuplicatePolicy::Error,
        )
        .unwrap_err();
        assert!(matches!(err, BulkError::OutOfOrder { index: 1 }));
    }

    #[test]
    fn pump_reverse_comparator() {
        let data = vec![(3, 30), (2, 20), (1, 10)]; // ascending under reverse cmp
        let m = TreeMap::from_sorted(reverse_comparator::<i32>(), data, DuplicatePolicy::Error)
            .unwrap();
        let keys: Vec<i32> = m.keys().copied().collect();
        assert_eq!(keys, vec![3, 2, 1]);
        assert!(m.is_valid_llrb());
    }
}
