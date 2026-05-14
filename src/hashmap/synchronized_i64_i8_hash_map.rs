// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::i64_i8_hash_map::I64I8HashMap;

/// Thread-safe wrapper around [`I64I8HashMap`]. Reads are concurrent;
/// mutations are exclusive. Lock poisoning panics (parity with Go).
#[derive(Debug, Default)]
pub struct SynchronizedI64I8HashMap {
    inner: RwLock<I64I8HashMap>,
}

impl SynchronizedI64I8HashMap {
    pub fn new() -> Self {
        SynchronizedI64I8HashMap {
            inner: RwLock::new(I64I8HashMap::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        SynchronizedI64I8HashMap {
            inner: RwLock::new(I64I8HashMap::with_capacity(capacity)),
        }
    }

    pub fn insert(&self, key: i64, value: i8) -> Option<i8> {
        self.inner.write().unwrap().insert(key, value)
    }

    pub fn get(&self, key: i64) -> Option<i8> {
        self.inner.read().unwrap().get(key)
    }

    pub fn get_or_default(&self, key: i64, default: i8) -> i8 {
        self.inner.read().unwrap().get_or_default(key, default)
    }

    pub fn remove(&self, key: i64) -> Option<i8> {
        self.inner.write().unwrap().remove(key)
    }

    pub fn contains_key(&self, key: i64) -> bool {
        self.inner.read().unwrap().contains_key(key)
    }

    pub fn contains_value(&self, value: i8) -> bool {
        self.inner.read().unwrap().contains_value(value)
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

    pub fn keys_to_vec(&self) -> Vec<i64> {
        self.inner.read().unwrap().keys_to_vec()
    }

    pub fn values_to_vec(&self) -> Vec<i8> {
        self.inner.read().unwrap().values_to_vec()
    }

    /// Returns an owned snapshot of entries for iteration under read lock.
    pub fn to_vec(&self) -> Vec<(i64, i8)> {
        self.inner.read().unwrap().iter().collect()
    }

    pub fn for_each(&self, mut f: impl FnMut(i64, i8)) {
        let snap = self.to_vec();
        for (k, v) in snap {
            f(k, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_basic_ops() {
        let m = SynchronizedI64I8HashMap::new();
        assert!(m.is_empty());
        assert_eq!(m.insert(1, 1), None);
        assert_eq!(m.insert(2, 2), None);
        assert_eq!(m.insert(3, 3), None);
        assert_eq!(m.get(1), Some(1));
        assert!(m.contains_key(1));
        assert!(m.contains_value(2));
        let old = m.insert(1, 2);
        assert_eq!(old, Some(1));
        assert_eq!(m.remove(1), Some(2));
        assert!(!m.contains_key(1));
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_get_or_default() {
        let m = SynchronizedI64I8HashMap::new();
        m.insert(1, 1);
        assert_eq!(m.get_or_default(1, 3), 1);
        assert_eq!(m.get_or_default(99, 3), 3);
    }

    #[test]
    fn test_snapshots_and_for_each() {
        let m = SynchronizedI64I8HashMap::new();
        m.insert(1, 1);
        m.insert(2, 2);
        assert_eq!(m.keys_to_vec().len(), 2);
        assert_eq!(m.values_to_vec().len(), 2);
        assert_eq!(m.to_vec().len(), 2);
        let mut count = 0usize;
        m.for_each(|_, _| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_concurrent() {
        let m = Arc::new(SynchronizedI64I8HashMap::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&m);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.insert(1, 1);
                    c.get(1);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(1), Some(1));
    }
}
