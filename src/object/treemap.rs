// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Sorted map backed by a red-black tree with pluggable [`Comparator`].

use super::strategy::Comparator;
use crate::range::Range;
use std::cmp::Ordering;
use std::fmt;

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
/// [`Comparator`]. Keys are maintained in the order defined by the comparator.
pub struct TreeMap<K, V> {
    root: Option<Box<Node<K, V>>>,
    size: usize,
    cmp: Comparator<K>,
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for TreeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V> TreeMap<K, V> {
    /// Creates an empty `TreeMap` using the given comparator.
    pub fn new(cmp: Comparator<K>) -> Self {
        TreeMap {
            root: None,
            size: 0,
            cmp,
        }
    }

    /// Returns a clone of this map's comparator (shares the underlying
    /// closure). Used to preserve ordering semantics when building a
    /// materialized snapshot (`sub_map`).
    pub fn comparator(&self) -> Comparator<K> {
        self.cmp.clone()
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
        cmp: &Comparator<K>,
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
        cmp: &Comparator<K>,
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

impl<K: Clone, V> TreeMap<K, V> {
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

    /// Keys in `range`, ascending. Snapshot taken at call time; read-only.
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

    /// A **new independent** map of the entries whose key ∈ `range`.
    /// Mutating the snapshot never affects the original and vice versa
    /// (it is a materialized copy, not a live view). The snapshot preserves the
    /// **source map's comparator**, so reverse/custom/float-total-order keyed
    /// maps keep their ordering semantics in the slice.
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

/// Borrowing iteration in sorted order: `for (k, v) in &map`.
///
/// Owned iteration / `FromIterator` are intentionally not provided: a
/// `TreeMap` needs a [`Comparator`] that an iterator alone cannot supply.
impl<'a, K, V> IntoIterator for &'a TreeMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = TreeMapIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
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
}
