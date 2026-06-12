// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Thread-safe wrapper for any collection, mirroring Java's
//! `Collections.synchronizedList` / `synchronizedSet` / `synchronizedMap`
//! factory pattern. Single generic type replaces what would be 16+ separate
//! per-primitive synchronized wrapper types.
//!
//! The primary API is the **guard**: [`Synchronized::lock`] returns a
//! [`SyncGuard`] that derefs to the inner collection, so you operate on it
//! directly and the lock releases when the guard drops. This is the standard
//! Rust `Mutex` ergonomic; the old `with` / `with_mut` closure methods were
//! dropped in v2.
//!
//! Usage:
//! ```ignore
//! use mapdb_collections::{synchronized, Synchronized, OpenHashMap};
//!
//! let m: Synchronized<OpenHashMap<i32, String>> = synchronized(OpenHashMap::new());
//! m.lock().insert(1, "one".into());
//! let value = m.lock().get(&1).cloned();
//! ```
//!
//! ## Caveats
//!
//! - The inner `Mutex` is **not reentrant**: locking the same `Synchronized`
//!   instance again while a guard is still held (directly or transitively)
//!   deadlocks. Drop the guard before re-locking.
//! - `lock()` panics on a poisoned mutex (Java's synchronized wrappers have no
//!   poisoning concept). Use [`Synchronized::inner`] to reach the underlying
//!   `Arc<Mutex<C>>` yourself if you need `try_lock` / `PoisonError` recovery.

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard};

/// Java-style synchronized wrapper for any collection `C`. Cheaply cloneable
/// via the inner `Arc` — all clones share the same locked instance.
#[derive(Debug, Default)]
pub struct Synchronized<C> {
    inner: Arc<Mutex<C>>,
}

impl<C> Clone for Synchronized<C> {
    fn clone(&self) -> Self {
        Synchronized {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<C> Synchronized<C> {
    pub fn new(value: C) -> Self {
        Synchronized {
            inner: Arc::new(Mutex::new(value)),
        }
    }

    /// Locks the inner collection and returns a guard that derefs to `&C` /
    /// `&mut C`. The lock is released when the guard is dropped. Panics if the
    /// lock has been poisoned by a previous panic.
    pub fn lock(&self) -> SyncGuard<'_, C> {
        SyncGuard {
            guard: self.inner.lock().expect("Synchronized lock poisoned"),
        }
    }

    /// Borrows the shared `Arc<Mutex<C>>` for callers that need `try_lock`,
    /// `PoisonError` recovery, or to hand the handle to another API.
    pub fn inner(&self) -> &Arc<Mutex<C>> {
        &self.inner
    }
}

/// RAII guard returned by [`Synchronized::lock`]. Derefs to the guarded
/// collection; releases the lock on drop.
pub struct SyncGuard<'a, C> {
    guard: MutexGuard<'a, C>,
}

impl<C> Deref for SyncGuard<'_, C> {
    type Target = C;
    fn deref(&self) -> &C {
        &self.guard
    }
}

impl<C> DerefMut for SyncGuard<'_, C> {
    fn deref_mut(&mut self) -> &mut C {
        &mut self.guard
    }
}

/// Java-style factory: `synchronized(myCollection)` — mirrors
/// `Collections.synchronizedList(list)` etc.
pub fn synchronized<C>(value: C) -> Synchronized<C> {
    Synchronized::new(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash_table::OpenHashMap;
    use std::thread;

    #[test]
    fn synchronized_map_across_threads() {
        let m: Synchronized<OpenHashMap<i32, i32>> = synchronized(OpenHashMap::new());
        let handles: Vec<_> = (0..8)
            .map(|t| {
                let mc = m.clone();
                thread::spawn(move || {
                    for i in (t * 100)..((t + 1) * 100) {
                        mc.lock().insert(i, i * 10);
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let guard = m.lock();
        assert_eq!(guard.len(), 800);
        for i in 0..800 {
            assert_eq!(guard.get(&i), Some(&(i * 10)));
        }
    }

    #[test]
    fn synchronized_clone_shares_state() {
        let a: Synchronized<Vec<i32>> = synchronized(vec![]);
        let b = a.clone();
        a.lock().push(1);
        b.lock().push(2);
        assert_eq!(&*a.lock(), &vec![1, 2]);
    }

    #[test]
    fn inner_exposes_arc_mutex_for_try_lock() {
        let m: Synchronized<Vec<i32>> = synchronized(vec![1, 2, 3]);
        let guard = m.inner().try_lock().expect("uncontended");
        assert_eq!(guard.len(), 3);
    }
}
