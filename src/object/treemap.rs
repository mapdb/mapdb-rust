// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Sorted map backed by a red-black tree with pluggable [`Comparator`].

use super::strategy::Comparator;
use crate::bulk::{BulkError, DuplicatePolicy};
use std::cmp::Ordering;
use std::fmt;

struct Node<K, V> {
    key: K,
    value: V,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
    red: bool,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V, red: bool) -> Self {
        Node {
            key,
            value,
            left: None,
            right: None,
            red,
        }
    }
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
    }));
    let (root_k, root_v) = it.next().expect("buffer underrun in build_black");
    let right = build_black(it, sizes[2], child_bh);
    Some(Box::new(Node {
        key: root_k,
        value: root_v,
        left: red_left,
        right,
        red: false,
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
    r.left = Some(node);
    r
}

fn rotate_right<K, V>(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
    let mut l = node.left.take().unwrap();
    node.left = l.right.take();
    l.red = node.red;
    node.red = true;
    l.right = Some(node);
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
