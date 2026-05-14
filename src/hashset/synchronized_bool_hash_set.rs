// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::bool_hash_set::BoolHashSet;

/// Thread-safe wrapper around [`BoolHashSet`]. Uses an internal `RwLock`
/// so reads may run concurrently while mutations are exclusive.
#[derive(Debug, Default)]
pub struct SynchronizedBoolHashSet {
    inner: RwLock<BoolHashSet>,
}

impl SynchronizedBoolHashSet {
    pub fn new() -> Self {
        SynchronizedBoolHashSet {
            inner: RwLock::new(BoolHashSet::new()),
        }
    }

    pub fn of(values: &[bool]) -> Self {
        SynchronizedBoolHashSet {
            inner: RwLock::new(BoolHashSet::of(values)),
        }
    }

    pub fn add(&self, value: bool) -> bool {
        self.inner.write().unwrap().add(value)
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

    pub fn to_vec(&self) -> Vec<bool> {
        self.inner.read().unwrap().to_vec()
    }

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
        let s = SynchronizedBoolHashSet::new();
        assert!(s.is_empty());
        assert!(s.add(true));
        assert!(s.add(false));
        assert!(!s.add(true));
        assert!(s.contains(true));
        assert_eq!(s.len(), 2);
        assert!(s.remove(true));
        assert!(!s.contains(true));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let s = SynchronizedBoolHashSet::of(&[true, false]);
        assert!(s.to_vec().len() >= 1);
        let mut count = 0usize;
        s.for_each(|_| count += 1);
        assert!(count >= 1);
    }

    #[test]
    fn test_concurrent() {
        let s = Arc::new(SynchronizedBoolHashSet::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&s);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.add(true);
                    c.contains(true);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert!(s.contains(true));
        assert_eq!(s.len(), 1);
    }
}
