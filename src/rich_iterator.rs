// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! [`RichIterator`]: the Eclipse Collections vocabulary as a single blanket
//! extension trait over [`Iterator`].
//!
//! This is the v3 replacement for the boxed-iterator "trait towers"
//! (`traits.rs`, `object/traits.rs`) and the free functions in
//! [`crate::stream`]. Because it is blanket-implemented for every `I: Iterator`,
//! the vocabulary (`select`, `reject`, `detect`, `inject_into`, `group_by`, …)
//! is available on **every** iterator — crate types, `std` types, and downstream
//! types alike — lazily and unboxed.
//!
//! ```
//! use mapdb_collections::rich_iterator::RichIterator;
//!
//! let evens: Vec<i32> = (1..=6).select(|n| n % 2 == 0).collect();
//! assert_eq!(evens, vec![2, 4, 6]);
//!
//! let first_big = (1..=10).detect(|n| *n > 7);
//! assert_eq!(first_big, Some(8));
//! ```
//!
//! # Relationship to `std`
//!
//! Most methods that duplicate an [`Iterator`] method are `#[inline]` aliases
//! kept for cross-language parity with the sibling ports (`detect`≈`find`,
//! `inject_into`≈`fold`, `any_satisfy`≈`any`). The methods that earn the trait's
//! existence beyond aliasing are the eager collectors that return **crate**
//! types ([`group_by`](RichIterator::group_by) → [`OpenHashMap`],
//! [`to_bag`](RichIterator::to_bag) → [`HashBag`]) and the additive
//! [`top_n`](RichIterator::top_n)/[`bottom_n`](RichIterator::bottom_n)/
//! [`partition_into`](RichIterator::partition_into) family that `std` has no
//! equivalent for.

use crate::hash_table::OpenHashMap;
use crate::object::{HashBag, MutableBag};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::Display;
use std::hash::Hash;

/// Eclipse Collections vocabulary over any [`Iterator`]. See the
/// [module docs](self) for the design rationale.
///
/// Blanket-implemented for all `I: Iterator`, so importing the trait brings the
/// whole vocabulary into scope for every iterator.
pub trait RichIterator: Iterator + Sized {
    // ── lazy adapters (return iterators; zero allocation) ────────────────

    /// Lazily yields the elements for which `pred` returns `true`
    /// (Eclipse `select`; alias of [`Iterator::filter`] with a nameable type).
    #[doc(alias = "filter")]
    fn select<P>(self, pred: P) -> Select<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        Select { it: self, pred }
    }

    /// Lazily yields the elements for which `pred` returns `false` — the
    /// complement of [`select`](RichIterator::select) (Eclipse `reject`).
    fn reject<P>(self, pred: P) -> Reject<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        Reject { it: self, pred }
    }

    // ── eager, crate-returning (the trait's reason to exist) ─────────────

    /// Groups elements into an [`OpenHashMap`] keyed by `key_fn(&item)`, each
    /// value the [`Vec`] of elements (in iteration order) that produced that key.
    fn group_by<K, F>(self, mut key_fn: F) -> OpenHashMap<K, Vec<Self::Item>>
    where
        K: Hash + Eq,
        F: FnMut(&Self::Item) -> K,
    {
        let mut map: OpenHashMap<K, Vec<Self::Item>> = OpenHashMap::new();
        for item in self {
            let k = key_fn(&item);
            map.entry(k).or_default().push(item);
        }
        map
    }

    /// Like [`group_by`](RichIterator::group_by) but `key_fn` yields *several*
    /// keys per element; the element (cloned) is filed under each.
    fn group_by_each<K, I, F>(self, mut key_fn: F) -> OpenHashMap<K, Vec<Self::Item>>
    where
        K: Hash + Eq,
        Self::Item: Clone,
        I: IntoIterator<Item = K>,
        F: FnMut(&Self::Item) -> I,
    {
        let mut map: OpenHashMap<K, Vec<Self::Item>> = OpenHashMap::new();
        for item in self {
            for k in key_fn(&item) {
                map.entry(k).or_default().push(item.clone());
            }
        }
        map
    }

    /// Collects into a [`HashBag`], counting occurrences of equal elements.
    fn to_bag(self) -> HashBag<Self::Item>
    where
        Self::Item: Hash + Eq,
    {
        let mut bag = HashBag::new();
        for item in self {
            bag.insert(item);
        }
        bag
    }

    /// Splits elements into `(matching, rest)`, each collected into any
    /// `Default + Extend` sink `B` — generalizing [`Iterator::partition`], which
    /// fixes both sinks to the same type, to two arbitrary collections.
    fn partition_into<B>(self, mut pred: impl FnMut(&Self::Item) -> bool) -> (B, B)
    where
        B: Default + Extend<Self::Item>,
    {
        let mut yes = B::default();
        let mut no = B::default();
        for item in self {
            if pred(&item) {
                yes.extend(std::iter::once(item));
            } else {
                no.extend(std::iter::once(item));
            }
        }
        (yes, no)
    }

    // ── genuinely additive (no std equivalent) ───────────────────────────

    /// Up to `n` **largest** elements, **descending** (largest first).
    /// `O(len · log n)` via a bounded min-heap — cheaper than sorting when
    /// `n ≪ len`. Ties are kept in unspecified order.
    fn top_n(self, n: usize) -> Vec<Self::Item>
    where
        Self::Item: Ord,
    {
        if n == 0 {
            return Vec::new();
        }
        // Min-heap of the running top-n: the smallest kept element is at the top
        // and is evicted when a larger one arrives.
        let mut heap: BinaryHeap<Reverse<Self::Item>> = BinaryHeap::with_capacity(n + 1);
        for item in self {
            heap.push(Reverse(item));
            if heap.len() > n {
                heap.pop();
            }
        }
        let mut out: Vec<Self::Item> = heap.into_iter().map(|Reverse(x)| x).collect();
        out.sort_unstable_by(|a, b| b.cmp(a));
        out
    }

    /// Up to `n` **smallest** elements, **ascending** (smallest first).
    /// `O(len · log n)` via a bounded max-heap. Ties in unspecified order.
    fn bottom_n(self, n: usize) -> Vec<Self::Item>
    where
        Self::Item: Ord,
    {
        if n == 0 {
            return Vec::new();
        }
        let mut heap: BinaryHeap<Self::Item> = BinaryHeap::with_capacity(n + 1);
        for item in self {
            heap.push(item);
            if heap.len() > n {
                heap.pop();
            }
        }
        let mut out: Vec<Self::Item> = heap.into_vec();
        out.sort_unstable();
        out
    }

    /// Renders the elements as their [`Display`] joined by `sep`
    /// (Eclipse `makeString`). Named `join_display` to avoid colliding with
    /// `itertools::Itertools::join`.
    fn join_display(mut self, sep: &str) -> String
    where
        Self::Item: Display,
    {
        let mut s = String::new();
        if let Some(first) = self.next() {
            use std::fmt::Write;
            let _ = write!(s, "{first}");
            for item in self {
                s.push_str(sep);
                let _ = write!(s, "{item}");
            }
        }
        s
    }

    // ── thin Eclipse-name delegations (aliases, `#[inline]`) ──────────────

    /// First element satisfying `pred` (Eclipse `detect`; alias of
    /// [`Iterator::find`]).
    #[doc(alias = "find")]
    #[inline]
    fn detect<P>(&mut self, mut pred: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.find(|x| pred(x))
    }

    /// `true` if any element satisfies `pred` (Eclipse `anySatisfy`; alias of
    /// [`Iterator::any`]).
    #[doc(alias = "any")]
    #[inline]
    fn any_satisfy<P>(&mut self, pred: P) -> bool
    where
        P: FnMut(Self::Item) -> bool,
    {
        self.any(pred)
    }

    /// `true` if all elements satisfy `pred` (Eclipse `allSatisfy`; alias of
    /// [`Iterator::all`]).
    #[doc(alias = "all")]
    #[inline]
    fn all_satisfy<P>(&mut self, pred: P) -> bool
    where
        P: FnMut(Self::Item) -> bool,
    {
        self.all(pred)
    }

    /// `true` if no element satisfies `pred` (Eclipse `noneSatisfy`).
    #[inline]
    fn none_satisfy<P>(&mut self, pred: P) -> bool
    where
        P: FnMut(Self::Item) -> bool,
    {
        !self.any(pred)
    }

    /// Folds `init` over the elements with `f` (Eclipse `injectInto`; alias of
    /// [`Iterator::fold`]).
    #[doc(alias = "fold")]
    #[inline]
    fn inject_into<R, F>(self, init: R, f: F) -> R
    where
        F: FnMut(R, Self::Item) -> R,
    {
        self.fold(init, f)
    }

    /// Counts the elements satisfying `pred` (Eclipse `count`).
    #[inline]
    fn count_where<P>(self, mut pred: P) -> usize
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.filter(|x| pred(x)).count()
    }
}

impl<I: Iterator> RichIterator for I {}

/// Iterator returned by [`RichIterator::select`]. A nameable, unboxed filter.
#[derive(Clone, Debug)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Select<I, P> {
    it: I,
    pred: P,
}

impl<I, P> Iterator for Select<I, P>
where
    I: Iterator,
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let pred = &mut self.pred;
        self.it.by_ref().find(|item| pred(item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.it.size_hint().1)
    }
}

impl<I, P> DoubleEndedIterator for Select<I, P>
where
    I: DoubleEndedIterator,
    P: FnMut(&I::Item) -> bool,
{
    fn next_back(&mut self) -> Option<I::Item> {
        let pred = &mut self.pred;
        self.it.by_ref().rfind(|item| pred(item))
    }
}

impl<I, P> std::iter::FusedIterator for Select<I, P>
where
    I: std::iter::FusedIterator,
    P: FnMut(&I::Item) -> bool,
{
}

/// Iterator returned by [`RichIterator::reject`]. A nameable, unboxed
/// anti-filter.
#[derive(Clone, Debug)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Reject<I, P> {
    it: I,
    pred: P,
}

impl<I, P> Iterator for Reject<I, P>
where
    I: Iterator,
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let pred = &mut self.pred;
        self.it.by_ref().find(|item| !pred(item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.it.size_hint().1)
    }
}

impl<I, P> DoubleEndedIterator for Reject<I, P>
where
    I: DoubleEndedIterator,
    P: FnMut(&I::Item) -> bool,
{
    fn next_back(&mut self) -> Option<I::Item> {
        let pred = &mut self.pred;
        self.it.by_ref().rfind(|item| !pred(item))
    }
}

impl<I, P> std::iter::FusedIterator for Reject<I, P>
where
    I: std::iter::FusedIterator,
    P: FnMut(&I::Item) -> bool,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::Bag;

    #[test]
    fn select_and_reject_are_complements() {
        let evens: Vec<i32> = (1..=6).select(|n| n % 2 == 0).collect();
        let odds: Vec<i32> = (1..=6).reject(|n| n % 2 == 0).collect();
        assert_eq!(evens, vec![2, 4, 6]);
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[test]
    fn select_is_double_ended() {
        let v: Vec<i32> = (1..=6).select(|n| n % 2 == 0).rev().collect();
        assert_eq!(v, vec![6, 4, 2]);
    }

    #[test]
    fn detect_any_all_none() {
        assert_eq!((1..=10).detect(|n| *n > 7), Some(8));
        assert!((1..=10).any_satisfy(|n| n == 5));
        assert!((1..=10).all_satisfy(|n| n > 0));
        assert!((1..=10).none_satisfy(|n| n > 100));
    }

    #[test]
    fn inject_into_and_count_where() {
        assert_eq!((1..=5).inject_into(0, |acc, n| acc + n), 15);
        assert_eq!((1..=10).count_where(|n| n % 3 == 0), 3);
    }

    #[test]
    fn group_by_buckets_in_order() {
        let m = (1..=6).group_by(|n| n % 2);
        assert_eq!(m.get(&0), Some(&vec![2, 4, 6]));
        assert_eq!(m.get(&1), Some(&vec![1, 3, 5]));
    }

    #[test]
    fn group_by_each_files_under_every_key() {
        // each number filed under each of its divisors in {2,3}
        let m = (1..=6).group_by_each(|n| {
            let mut ks = vec![];
            if n % 2 == 0 {
                ks.push(2);
            }
            if n % 3 == 0 {
                ks.push(3);
            }
            ks
        });
        assert_eq!(m.get(&2), Some(&vec![2, 4, 6]));
        assert_eq!(m.get(&3), Some(&vec![3, 6]));
    }

    #[test]
    fn to_bag_counts_occurrences() {
        let bag = ["a", "b", "a", "a", "b"].into_iter().to_bag();
        assert_eq!(bag.occurrences_of(&"a"), 3);
        assert_eq!(bag.occurrences_of(&"b"), 2);
    }

    #[test]
    fn partition_into_two_vecs() {
        let (evens, odds): (Vec<i32>, Vec<i32>) = (1..=6).partition_into(|n| n % 2 == 0);
        assert_eq!(evens, vec![2, 4, 6]);
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[test]
    fn top_and_bottom_n() {
        let data = [5, 1, 9, 3, 7, 2, 8];
        assert_eq!(data.into_iter().top_n(3), vec![9, 8, 7]);
        assert_eq!(data.into_iter().bottom_n(3), vec![1, 2, 3]);
        assert_eq!(data.into_iter().top_n(0), Vec::<i32>::new());
        // n larger than the stream returns everything, sorted.
        assert_eq!(data.into_iter().top_n(100), vec![9, 8, 7, 5, 3, 2, 1]);
    }

    #[test]
    fn join_display_matches_std() {
        assert_eq!((1..=3).join_display(", "), "1, 2, 3");
        assert_eq!(std::iter::empty::<i32>().join_display(", "), "");
    }
}
