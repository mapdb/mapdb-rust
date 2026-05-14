// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::i32_array_list::I32ArrayList;

/// Thread-safe wrapper around [`I32ArrayList`]. Uses an internal `RwLock`
/// so reads may run concurrently while mutations are exclusive. Wrap in
/// `Arc` for sharing between threads. Lock poisoning panics (parity with
/// Go's panic-on-misuse).
#[derive(Debug, Default)]
pub struct SynchronizedI32ArrayList {
    inner: RwLock<I32ArrayList>,
}

impl SynchronizedI32ArrayList {
    pub fn new() -> Self {
        SynchronizedI32ArrayList {
            inner: RwLock::new(I32ArrayList::new()),
        }
    }

    pub fn of(values: &[i32]) -> Self {
        SynchronizedI32ArrayList {
            inner: RwLock::new(I32ArrayList::of(values)),
        }
    }

    pub fn push(&self, value: i32) {
        self.inner.write().unwrap().push(value);
    }

    pub fn push_all(&self, values: &[i32]) {
        self.inner.write().unwrap().push_all(values);
    }

    pub fn get(&self, index: usize) -> Option<i32> {
        self.inner.read().unwrap().get(index)
    }

    pub fn set(&self, index: usize, value: i32) -> i32 {
        self.inner.write().unwrap().set(index, value)
    }

    pub fn remove_at_index(&self, index: usize) -> i32 {
        self.inner.write().unwrap().remove_at_index(index)
    }

    pub fn remove(&self, value: i32) -> bool {
        self.inner.write().unwrap().remove(value)
    }

    pub fn contains(&self, value: i32) -> bool {
        self.inner.read().unwrap().contains(value)
    }

    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }

    pub fn clear(&self) {
        self.inner.write().unwrap().clear();
    }

    /// Returns an owned snapshot for iteration.
    pub fn to_vec(&self) -> Vec<i32> {
        self.inner.read().unwrap().to_vec()
    }

    /// Iterates under the read lock. The closure must not re-enter the
    /// wrapper or a deadlock will result.
    pub fn for_each(&self, f: impl FnMut(i32)) {
        self.inner.read().unwrap().for_each(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_basic_ops() {
        let l = SynchronizedI32ArrayList::new();
        assert!(l.is_empty());
        l.push(1);
        l.push(2);
        l.push(3);
        assert_eq!(l.len(), 3);
        assert_eq!(l.get(0), Some(1));
        assert!(l.contains(2));
        let old = l.set(0, 3);
        assert_eq!(old, 1);
        let removed = l.remove_at_index(0);
        assert_eq!(removed, 3);
        assert_eq!(l.len(), 2);
        l.clear();
        assert!(l.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let l = SynchronizedI32ArrayList::of(&[1, 2, 3]);
        let snap = l.to_vec();
        assert_eq!(snap.len(), 3);
        let mut count = 0usize;
        l.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let l = Arc::new(SynchronizedI32ArrayList::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&l);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.push(1);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(l.len(), 400);
    }
}
