// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! [`Frozen<C>`] — a generic, cheaply-shareable **read-only** view over any
//! collection `C`, the one abstraction that replaces a per-type immutable
//! wrapper for every collection at once (blueprint T6 / M7).
//!
//! ## How it enforces read-only
//!
//! `Frozen<C>` holds an `Arc<C>` and derefs to `&C`. Because a `Frozen` only
//! ever hands out a **shared** `&C`, exactly `C`'s `&self` methods are reachable
//! (`get`, `contains`, `len`, `iter`, `keys`, `values`, …) while every `&mut
//! self` mutator (`insert`, `remove`, `clear`, …) is *unreachable by
//! construction* — there is no `unsafe`, no wrapper method list to keep in sync,
//! and no `ReadOnly` associated type. Freeze any map/set/list/bag and you get its
//! full read surface for free:
//!
//! ```
//! use mapdb_collections::{Frozen, OpenHashMap};
//!
//! let mut m = OpenHashMap::new();
//! m.insert("a", 1);
//! m.insert("b", 2);
//! let frozen = Frozen::new(m);         // ownership moves in; now read-only
//! assert_eq!(frozen.get(&"a"), Some(&1)); // C's &self methods via Deref
//! assert_eq!(frozen.len(), 2);
//! // frozen.insert(...) does not compile — no &mut is ever handed out.
//! let shared = frozen.clone();          // O(1): bumps the Arc refcount
//! assert_eq!(shared.get(&"b"), Some(&2));
//! ```
//!
//! ## Sharing and reclaiming
//!
//! [`clone`](Frozen::clone) is O(1) — it bumps the `Arc` refcount, so many
//! handles share one collection (the deterministic, eager "GC for one object
//! graph" that `Arc` provides). When you hold the **only** handle you can reclaim
//! mutability with [`get_mut`](Frozen::get_mut) or ownership with
//! [`try_unwrap`](Frozen::try_unwrap); while any clone is alive, both refuse —
//! immutability of the shared view is never violated.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

/// A cheaply-shareable read-only view over a collection `C`. See the
/// [module docs](crate::frozen) for the `Deref`-enforced read-only model.
pub struct Frozen<C> {
    inner: Arc<C>,
}

impl<C> Frozen<C> {
    /// Freeze `collection`, moving ownership into a fresh `Arc`.
    pub fn new(collection: C) -> Self {
        Frozen {
            inner: Arc::new(collection),
        }
    }

    /// Wrap an existing `Arc<C>` (shares whatever it already points at).
    pub fn from_arc(inner: Arc<C>) -> Self {
        Frozen { inner }
    }

    /// Borrow the backing `Arc` (e.g. to `Arc::clone` it into non-`Frozen` code).
    pub fn as_arc(&self) -> &Arc<C> {
        &self.inner
    }

    /// Consume this handle, returning the backing `Arc<C>`.
    pub fn into_arc(self) -> Arc<C> {
        self.inner
    }

    /// The number of live handles sharing this collection (`Arc` strong count).
    pub fn handle_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// Whether this is the **sole** handle — no other strong (`Frozen`) clone
    /// *and* no outstanding [`Weak`](std::sync::Weak) (which a caller can derive
    /// from [`as_arc`](Frozen::as_arc)). Exactly the condition under which
    /// [`get_mut`](Frozen::get_mut) returns `Some`. (Note
    /// [`try_unwrap`](Frozen::try_unwrap) is looser — a `Weak` does not block it.)
    pub fn is_unique(&self) -> bool {
        Arc::strong_count(&self.inner) == 1 && Arc::weak_count(&self.inner) == 0
    }

    /// Mutable access to the collection **iff** no other handle exists — no
    /// strong `Frozen` clone and no outstanding [`Weak`](std::sync::Weak)
    /// (matching [`Arc::get_mut`], and equal to [`is_unique`](Frozen::is_unique)).
    /// `None` otherwise. Lets a freeze be temporarily thawed to mutate, without
    /// ever exposing mutation through a shared handle.
    pub fn get_mut(&mut self) -> Option<&mut C> {
        Arc::get_mut(&mut self.inner)
    }

    /// Recover the owned collection **iff** no other **strong** handle exists
    /// (matching [`Arc::try_unwrap`] — an outstanding `Weak` does *not* block it);
    /// otherwise returns `Err(self)` so the caller keeps its shared handle.
    pub fn try_unwrap(self) -> Result<C, Self> {
        Arc::try_unwrap(self.inner).map_err(|inner| Frozen { inner })
    }
}

impl<C> Deref for Frozen<C> {
    type Target = C;
    fn deref(&self) -> &C {
        &self.inner
    }
}

impl<C> AsRef<C> for Frozen<C> {
    fn as_ref(&self) -> &C {
        &self.inner
    }
}

/// O(1): bumps the shared `Arc` refcount (does **not** deep-copy `C`).
impl<C> Clone for Frozen<C> {
    fn clone(&self) -> Self {
        Frozen {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<C> From<C> for Frozen<C> {
    fn from(collection: C) -> Self {
        Frozen::new(collection)
    }
}

impl<C: fmt::Debug> fmt::Debug for Frozen<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Transparent: show the underlying collection, not the Arc wrapper.
        fmt::Debug::fmt(&*self.inner, f)
    }
}

impl<C: Default> Default for Frozen<C> {
    fn default() -> Self {
        Frozen::new(C::default())
    }
}

/// Equal iff the underlying collections are equal (with an `Arc`-pointer
/// fast-path for handles that share one collection).
impl<C: PartialEq> PartialEq for Frozen<C> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner) || *self.inner == *other.inner
    }
}

impl<C: Eq> Eq for Frozen<C> {}

impl<C: Hash> Hash for Frozen<C> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self.inner).hash(state);
    }
}

/// Freeze the result of a `collect()` — e.g.
/// `iter.collect::<Frozen<OpenHashMap<_, _>>>()`.
impl<T, C: FromIterator<T>> FromIterator<T> for Frozen<C> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Frozen::new(iter.into_iter().collect())
    }
}

/// Borrow-iterate whenever the underlying `&C` can: `for x in &frozen`.
impl<'a, C> IntoIterator for &'a Frozen<C>
where
    &'a C: IntoIterator,
{
    type Item = <&'a C as IntoIterator>::Item;
    type IntoIter = <&'a C as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&*self.inner).into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OpenHashMap, OpenHashSet};

    #[test]
    fn deref_exposes_read_only_surface() {
        let mut m = OpenHashMap::new();
        m.insert(1, 10);
        m.insert(2, 20);
        let f = Frozen::new(m);
        // C's &self methods reach through Deref.
        assert_eq!(f.get(&1), Some(&10));
        assert!(f.contains_key(&2));
        assert_eq!(f.len(), 2);
        let mut keys: Vec<i32> = f.keys().copied().collect();
        keys.sort_unstable();
        assert_eq!(keys, vec![1, 2]);
    }

    #[test]
    fn clone_is_a_shared_handle_not_a_deep_copy() {
        let f = Frozen::new(OpenHashSet::<i32>::from_iter([1, 2, 3]));
        assert!(f.is_unique());
        let g = f.clone();
        assert_eq!(f.handle_count(), 2);
        assert!(!f.is_unique());
        // Both see the same data.
        assert!(f.contains(&2) && g.contains(&2));
    }

    #[test]
    fn get_mut_only_when_unique() {
        let mut f = Frozen::new(OpenHashMap::<i32, i32>::from_iter([(1, 1)]));
        // Unique -> can thaw and mutate.
        f.get_mut().unwrap().insert(2, 2);
        assert_eq!(f.len(), 2);
        let _g = f.clone();
        // Shared -> refused.
        assert!(f.get_mut().is_none());
    }

    #[test]
    fn is_unique_agrees_with_get_mut_even_with_a_weak() {
        // A Weak derived from as_arc() blocks get_mut; is_unique must reflect that
        // (strong count alone would wrongly report unique), but try_unwrap — which
        // a Weak does NOT block — still succeeds.
        let mut f = Frozen::new(OpenHashMap::<i32, i32>::from_iter([(1, 1)]));
        let weak = Arc::downgrade(f.as_arc());
        assert_eq!(f.handle_count(), 1); // one strong handle
        assert!(!f.is_unique()); // but a Weak is outstanding
        assert!(f.get_mut().is_none()); // consistent with is_unique
        drop(weak);
        assert!(f.is_unique());
        assert!(f.get_mut().is_some());

        // try_unwrap ignores Weak: succeeds with a live Weak present.
        let f2 = Frozen::new(OpenHashMap::<i32, i32>::from_iter([(2, 2)]));
        let _w2 = Arc::downgrade(f2.as_arc());
        assert!(f2.try_unwrap().is_ok());
    }

    #[test]
    fn try_unwrap_recovers_sole_owner() {
        let f = Frozen::new(OpenHashMap::<i32, i32>::from_iter([(1, 1)]));
        let m = f.try_unwrap().expect("unique");
        assert_eq!(m.get(&1), Some(&1));

        let f = Frozen::new(OpenHashSet::<i32>::from_iter([9]));
        let g = f.clone();
        // Shared -> Err returns a handle back.
        let back = f.try_unwrap().expect_err("shared, should fail");
        assert!(back.contains(&9) && g.contains(&9));
    }

    #[test]
    fn from_iter_and_iter() {
        let f: Frozen<OpenHashSet<i32>> = [1, 2, 3, 2, 1].into_iter().collect();
        assert_eq!(f.len(), 3);
        let mut seen: Vec<i32> = (&f).into_iter().copied().collect();
        seen.sort_unstable();
        assert_eq!(seen, vec![1, 2, 3]);
    }

    #[test]
    fn equality_and_pointer_fast_path() {
        let a = Frozen::new(OpenHashMap::<i32, i32>::from_iter([(1, 1), (2, 2)]));
        let b = a.clone(); // shares the Arc -> ptr fast path
        assert_eq!(a, b);
        // Distinct allocations, equal contents -> value comparison.
        let c = Frozen::new(OpenHashMap::<i32, i32>::from_iter([(2, 2), (1, 1)]));
        assert_eq!(a, c);
        let d = Frozen::new(OpenHashMap::<i32, i32>::from_iter([(1, 99)]));
        assert_ne!(a, d);
    }

    #[test]
    fn debug_is_transparent() {
        let f = Frozen::new(OpenHashSet::<i32>::from_iter([7]));
        // Debug shows the collection, not an Arc/Frozen wrapper.
        assert!(format!("{f:?}").contains('7'));
    }
}
