// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::bool_i8_hash_map::BoolI8HashMap;

/// Thread-safe wrapper around [`BoolI8HashMap`]. Reads are concurrent;
/// mutations are exclusive. Lock poisoning panics (parity with Go).
#[derive(Debug, Default)]
pub struct SynchronizedBoolI8HashMap {
    inner: RwLock<BoolI8HashMap>,
}

impl SynchronizedBoolI8HashMap {
    pub fn new() -> Self {
        SynchronizedBoolI8HashMap {
            inner: RwLock::new(BoolI8HashMap::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        SynchronizedBoolI8HashMap {
            inner: RwLock::new(BoolI8HashMap::with_capacity(capacity)),
        }
    }

    pub fn insert(&self, key: bool, value: i8) -> Option<i8> {
        self.inner.write().unwrap().insert(key, value)
    }

    pub fn get(&self, key: bool) -> Option<i8> {
        self.inner.read().unwrap().get(key)
    }

    pub fn get_or_default(&self, key: bool, default: i8) -> i8 {
        self.inner.read().unwrap().get_or_default(key, default)
    }

    pub fn remove(&self, key: bool) -> Option<i8> {
        self.inner.write().unwrap().remove(key)
    }

    pub fn contains_key(&self, key: bool) -> bool {
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

    pub fn keys_to_vec(&self) -> Vec<bool> {
        self.inner.read().unwrap().keys_to_vec()
    }

    pub fn values_to_vec(&self) -> Vec<i8> {
        self.inner.read().unwrap().values_to_vec()
    }

    /// Returns an owned snapshot of entries for iteration under read lock.
    pub fn to_vec(&self) -> Vec<(bool, i8)> {
        self.inner.read().unwrap().iter().collect()
    }

    pub fn for_each(&self, mut f: impl FnMut(bool, i8)) {
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
        let m = SynchronizedBoolI8HashMap::new();
        assert!(m.is_empty());
        assert_eq!(m.insert(true, 1), None);
        assert_eq!(m.insert(false, 2), None);
        assert_eq!(m.get(true), Some(1));
        assert!(m.contains_key(true));
        assert!(m.contains_value(2));
        let old = m.insert(true, 2);
        assert_eq!(old, Some(1));
        assert_eq!(m.remove(true), Some(2));
        assert!(!m.contains_key(true));
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_get_or_default() {
        let m = SynchronizedBoolI8HashMap::new();
        m.insert(true, 1);
        assert_eq!(m.get_or_default(true, 3), 1);
        assert_eq!(m.get_or_default(false, 3), 3);
    }

    #[test]
    fn test_snapshots_and_for_each() {
        let m = SynchronizedBoolI8HashMap::new();
        m.insert(true, 1);
        m.insert(false, 2);
        assert_eq!(m.keys_to_vec().len(), 2);
        assert_eq!(m.values_to_vec().len(), 2);
        assert_eq!(m.to_vec().len(), 2);
        let mut count = 0usize;
        m.for_each(|_, _| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_concurrent() {
        let m = Arc::new(SynchronizedBoolI8HashMap::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&m);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.insert(true, 1);
                    c.get(true);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(true), Some(1));
    }
}
