// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::f32_i16_hash_map::F32I16HashMap;

/// Thread-safe wrapper around [`F32I16HashMap`]. Reads are concurrent;
/// mutations are exclusive. Lock poisoning panics (parity with Go).
#[derive(Debug, Default)]
pub struct SynchronizedF32I16HashMap {
    inner: RwLock<F32I16HashMap>,
}

impl SynchronizedF32I16HashMap {
    pub fn new() -> Self {
        SynchronizedF32I16HashMap {
            inner: RwLock::new(F32I16HashMap::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        SynchronizedF32I16HashMap {
            inner: RwLock::new(F32I16HashMap::with_capacity(capacity)),
        }
    }

    pub fn insert(&self, key: f32, value: i16) -> Option<i16> {
        self.inner.write().unwrap().insert(key, value)
    }

    pub fn get(&self, key: f32) -> Option<i16> {
        self.inner.read().unwrap().get(key)
    }

    pub fn get_or_default(&self, key: f32, default: i16) -> i16 {
        self.inner.read().unwrap().get_or_default(key, default)
    }

    pub fn remove(&self, key: f32) -> Option<i16> {
        self.inner.write().unwrap().remove(key)
    }

    pub fn contains_key(&self, key: f32) -> bool {
        self.inner.read().unwrap().contains_key(key)
    }

    pub fn contains_value(&self, value: i16) -> bool {
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

    pub fn keys_to_vec(&self) -> Vec<f32> {
        self.inner.read().unwrap().keys_to_vec()
    }

    pub fn values_to_vec(&self) -> Vec<i16> {
        self.inner.read().unwrap().values_to_vec()
    }

    /// Returns an owned snapshot of entries for iteration under read lock.
    pub fn to_vec(&self) -> Vec<(f32, i16)> {
        self.inner.read().unwrap().iter().collect()
    }

    pub fn for_each(&self, mut f: impl FnMut(f32, i16)) {
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
        let m = SynchronizedF32I16HashMap::new();
        assert!(m.is_empty());
        assert_eq!(m.insert(1.0f32, 1), None);
        assert_eq!(m.insert(2.0f32, 2), None);
        assert_eq!(m.insert(3.0f32, 3), None);
        assert_eq!(m.get(1.0f32), Some(1));
        assert!(m.contains_key(1.0f32));
        assert!(m.contains_value(2));
        let old = m.insert(1.0f32, 2);
        assert_eq!(old, Some(1));
        assert_eq!(m.remove(1.0f32), Some(2));
        assert!(!m.contains_key(1.0f32));
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_get_or_default() {
        let m = SynchronizedF32I16HashMap::new();
        m.insert(1.0f32, 1);
        assert_eq!(m.get_or_default(1.0f32, 3), 1);
        assert_eq!(m.get_or_default(99.0f32, 3), 3);
    }

    #[test]
    fn test_snapshots_and_for_each() {
        let m = SynchronizedF32I16HashMap::new();
        m.insert(1.0f32, 1);
        m.insert(2.0f32, 2);
        assert_eq!(m.keys_to_vec().len(), 2);
        assert_eq!(m.values_to_vec().len(), 2);
        assert_eq!(m.to_vec().len(), 2);
        let mut count = 0usize;
        m.for_each(|_, _| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_concurrent() {
        let m = Arc::new(SynchronizedF32I16HashMap::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&m);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.insert(1.0f32, 1);
                    c.get(1.0f32);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(1.0f32), Some(1));
    }
}
