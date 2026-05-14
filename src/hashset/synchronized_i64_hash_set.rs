// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::i64_hash_set::I64HashSet;

/// Thread-safe wrapper around [`I64HashSet`]. Uses an internal `RwLock`
/// so reads may run concurrently while mutations are exclusive.
#[derive(Debug, Default)]
pub struct SynchronizedI64HashSet {
    inner: RwLock<I64HashSet>,
}

impl SynchronizedI64HashSet {
    pub fn new() -> Self {
        SynchronizedI64HashSet {
            inner: RwLock::new(I64HashSet::new()),
        }
    }

    pub fn of(values: &[i64]) -> Self {
        SynchronizedI64HashSet {
            inner: RwLock::new(I64HashSet::of(values)),
        }
    }

    pub fn add(&self, value: i64) -> bool {
        self.inner.write().unwrap().add(value)
    }

    pub fn remove(&self, value: i64) -> bool {
        self.inner.write().unwrap().remove(value)
    }

    pub fn contains(&self, value: i64) -> bool {
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

    pub fn to_vec(&self) -> Vec<i64> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, f: impl FnMut(i64)) {
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
        let s = SynchronizedI64HashSet::new();
        assert!(s.is_empty());
        assert!(s.add(1));
        assert!(s.add(2));
        assert!(!s.add(1));
        assert!(s.contains(1));
        assert_eq!(s.len(), 2);
        assert!(s.remove(1));
        assert!(!s.contains(1));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let s = SynchronizedI64HashSet::of(&[1, 2]);
        assert!(s.to_vec().len() >= 1);
        let mut count = 0usize;
        s.for_each(|_| count += 1);
        assert!(count >= 1);
    }

    #[test]
    fn test_concurrent() {
        let s = Arc::new(SynchronizedI64HashSet::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&s);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.add(1);
                    c.contains(1);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert!(s.contains(1));
        assert_eq!(s.len(), 1);
    }
}
