// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Sorted map backed by a red-black tree with pluggable [`Comparator`].

use super::strategy::Comparator;
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
        f.debug_map()
            .entries(self.iter().map(|(k, v)| (k, v)))
            .finish()
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
        let mut node = match node {
            None => return None,
            Some(n) => n,
        };

        if cmp.compare(key, &node.key) == Ordering::Less {
            if !is_red(&node.left) && !node.left.as_ref().map_or(false, |l| is_red(&l.left)) {
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
            if !is_red(&node.right) && !node.right.as_ref().map_or(false, |r| is_red(&r.left)) {
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

// ── Free functions for tree manipulation ────────────────────────────

fn is_red<K, V>(node: &Option<Box<Node<K, V>>>) -> bool {
    node.as_ref().map_or(false, |n| n.red)
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
    if is_red(&node.left) && node.left.as_ref().map_or(false, |l| is_red(&l.left)) {
        node = rotate_right(node);
    }
    if is_red(&node.left) && is_red(&node.right) {
        flip_colors(&mut node);
    }
    node
}

fn move_red_left<K, V>(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
    flip_colors(&mut node);
    if node.right.as_ref().map_or(false, |r| is_red(&r.left)) {
        let r = rotate_right(node.right.take().unwrap());
        node.right = Some(r);
        node = rotate_left(node);
        flip_colors(&mut node);
    }
    node
}

fn move_red_right<K, V>(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
    flip_colors(&mut node);
    if node.left.as_ref().map_or(false, |l| is_red(&l.left)) {
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

    if !is_red(&node.left) && !node.left.as_ref().map_or(false, |l| is_red(&l.left)) {
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
        age: i32,
    }

    #[test]
    fn test_by_field_comparator() {
        let mut m = TreeMap::new(comparator_by_field(|p: &Person| p.name.clone()));
        m.insert(
            Person {
                name: "Charlie".into(),
                age: 30,
            },
            "c",
        );
        m.insert(
            Person {
                name: "Alice".into(),
                age: 25,
            },
            "a",
        );
        m.insert(
            Person {
                name: "Bob".into(),
                age: 35,
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
}
