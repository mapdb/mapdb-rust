// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::bool_array_list::BoolArrayList;

/// Thread-safe wrapper around [`BoolArrayList`]. Uses an internal `RwLock`
/// so reads may run concurrently while mutations are exclusive. Wrap in
/// `Arc` for sharing between threads. Lock poisoning panics (parity with
/// Go's panic-on-misuse).
#[derive(Debug, Default)]
pub struct SynchronizedBoolArrayList {
    inner: RwLock<BoolArrayList>,
}

impl SynchronizedBoolArrayList {
    pub fn new() -> Self {
        SynchronizedBoolArrayList {
            inner: RwLock::new(BoolArrayList::new()),
        }
    }

    pub fn of(values: &[bool]) -> Self {
        SynchronizedBoolArrayList {
            inner: RwLock::new(BoolArrayList::of(values)),
        }
    }

    pub fn push(&self, value: bool) {
        self.inner.write().unwrap().push(value);
    }

    pub fn push_all(&self, values: &[bool]) {
        self.inner.write().unwrap().push_all(values);
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        self.inner.read().unwrap().get(index)
    }

    pub fn set(&self, index: usize, value: bool) -> bool {
        self.inner.write().unwrap().set(index, value)
    }

    pub fn remove_at_index(&self, index: usize) -> bool {
        self.inner.write().unwrap().remove_at_index(index)
    }

    pub fn remove(&self, value: bool) -> bool {
        self.inner.write().unwrap().remove(value)
    }

    pub fn contains(&self, value: bool) -> bool {
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
    pub fn to_vec(&self) -> Vec<bool> {
        self.inner.read().unwrap().to_vec()
    }

    /// Iterates under the read lock. The closure must not re-enter the
    /// wrapper or a deadlock will result.
    pub fn for_each(&self, f: impl FnMut(bool)) {
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
        let l = SynchronizedBoolArrayList::new();
        assert!(l.is_empty());
        l.push(true);
        l.push(false);
        l.push(true);
        assert_eq!(l.len(), 3);
        assert_eq!(l.get(0), Some(true));
        assert!(l.contains(false));
        let old = l.set(0, true);
        assert_eq!(old, true);
        let removed = l.remove_at_index(0);
        assert_eq!(removed, true);
        assert_eq!(l.len(), 2);
        l.clear();
        assert!(l.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let l = SynchronizedBoolArrayList::of(&[true, false, true]);
        let snap = l.to_vec();
        assert_eq!(snap.len(), 3);
        let mut count = 0usize;
        l.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let l = Arc::new(SynchronizedBoolArrayList::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&l);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.push(true);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(l.len(), 400);
    }
}
