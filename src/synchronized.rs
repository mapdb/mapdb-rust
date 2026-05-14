// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.

//! Thread-safe wrapper for any collection, mirroring Java's
//! `Collections.synchronizedList` / `synchronizedSet` / `synchronizedMap`
//! factory pattern. Single generic type replaces what would be 16+ separate
//! per-primitive synchronized wrapper types.
//!
//! Usage:
//! ```ignore
//! use mapdb_collections::{synchronized, Synchronized, OpenHashMap};
//!
//! let m: Synchronized<OpenHashMap<i32, String>> = synchronized(OpenHashMap::new());
//! m.with_mut(|inner| { inner.insert(1, "one".into()); });
//! let value = m.with(|inner| inner.get(&1).cloned());
//! ```
//!
//! ## Caveats
//!
//! - The inner `Mutex` is **not reentrant**: if the closure passed to `with`
//!   or `with_mut` calls back into the same `Synchronized` instance (directly
//!   or transitively), it deadlocks.
//! - `lock()` / `with` / `with_mut` panic on a poisoned mutex (Java's
//!   synchronized wrappers have no poisoning concept). Use the inner `Arc`
//!   yourself if you need `try_lock` / `PoisonError` recovery.

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

    /// Locks the inner collection for the duration of `f` and returns its result.
    /// Panics if the lock has been poisoned by a previous panic.
    pub fn with<R>(&self, f: impl FnOnce(&C) -> R) -> R {
        let guard = self.lock();
        f(&*guard)
    }

    /// Locks for mutable access.
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut C) -> R) -> R {
        let mut guard = self.lock();
        f(&mut *guard)
    }

    /// Returns the lock guard directly. Prefer `with` / `with_mut` unless you
    /// genuinely need to hold the lock across multiple operations.
    pub fn lock(&self) -> MutexGuard<'_, C> {
        self.inner.lock().expect("Synchronized lock poisoned")
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
                        mc.with_mut(|inner| {
                            inner.insert(i, i * 10);
                        });
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        m.with(|inner| {
            assert_eq!(inner.len(), 800);
            for i in 0..800 {
                assert_eq!(inner.get(&i), Some(&(i * 10)));
            }
        });
    }

    #[test]
    fn synchronized_clone_shares_state() {
        let a: Synchronized<Vec<i32>> = synchronized(vec![]);
        let b = a.clone();
        a.with_mut(|v| v.push(1));
        b.with_mut(|v| v.push(2));
        a.with(|v| assert_eq!(v, &vec![1, 2]));
    }
}
