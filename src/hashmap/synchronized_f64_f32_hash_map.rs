// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::f64_f32_hash_map::F64F32HashMap;

/// Thread-safe wrapper around [`F64F32HashMap`]. Reads are concurrent;
/// mutations are exclusive. Lock poisoning panics (parity with Go).
#[derive(Debug, Default)]
pub struct SynchronizedF64F32HashMap {
    inner: RwLock<F64F32HashMap>,
}

impl SynchronizedF64F32HashMap {
    pub fn new() -> Self {
        SynchronizedF64F32HashMap {
            inner: RwLock::new(F64F32HashMap::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        SynchronizedF64F32HashMap {
            inner: RwLock::new(F64F32HashMap::with_capacity(capacity)),
        }
    }

    pub fn insert(&self, key: f64, value: f32) -> Option<f32> {
        self.inner.write().unwrap().insert(key, value)
    }

    pub fn get(&self, key: f64) -> Option<f32> {
        self.inner.read().unwrap().get(key)
    }

    pub fn get_or_default(&self, key: f64, default: f32) -> f32 {
        self.inner.read().unwrap().get_or_default(key, default)
    }

    pub fn remove(&self, key: f64) -> Option<f32> {
        self.inner.write().unwrap().remove(key)
    }

    pub fn contains_key(&self, key: f64) -> bool {
        self.inner.read().unwrap().contains_key(key)
    }

    pub fn contains_value(&self, value: f32) -> bool {
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

    pub fn keys_to_vec(&self) -> Vec<f64> {
        self.inner.read().unwrap().keys_to_vec()
    }

    pub fn values_to_vec(&self) -> Vec<f32> {
        self.inner.read().unwrap().values_to_vec()
    }

    /// Returns an owned snapshot of entries for iteration under read lock.
    pub fn to_vec(&self) -> Vec<(f64, f32)> {
        self.inner.read().unwrap().iter().collect()
    }

    pub fn for_each(&self, mut f: impl FnMut(f64, f32)) {
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
        let m = SynchronizedF64F32HashMap::new();
        assert!(m.is_empty());
        assert_eq!(m.insert(1.0f64, 1.0f32), None);
        assert_eq!(m.insert(2.0f64, 2.0f32), None);
        assert_eq!(m.insert(3.0f64, 3.0f32), None);
        assert_eq!(m.get(1.0f64), Some(1.0f32));
        assert!(m.contains_key(1.0f64));
        assert!(m.contains_value(2.0f32));
        let old = m.insert(1.0f64, 2.0f32);
        assert_eq!(old, Some(1.0f32));
        assert_eq!(m.remove(1.0f64), Some(2.0f32));
        assert!(!m.contains_key(1.0f64));
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_get_or_default() {
        let m = SynchronizedF64F32HashMap::new();
        m.insert(1.0f64, 1.0f32);
        assert_eq!(m.get_or_default(1.0f64, 3.0f32), 1.0f32);
        assert_eq!(m.get_or_default(99.0f64, 3.0f32), 3.0f32);
    }

    #[test]
    fn test_snapshots_and_for_each() {
        let m = SynchronizedF64F32HashMap::new();
        m.insert(1.0f64, 1.0f32);
        m.insert(2.0f64, 2.0f32);
        assert_eq!(m.keys_to_vec().len(), 2);
        assert_eq!(m.values_to_vec().len(), 2);
        assert_eq!(m.to_vec().len(), 2);
        let mut count = 0usize;
        m.for_each(|_, _| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_concurrent() {
        let m = Arc::new(SynchronizedF64F32HashMap::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&m);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.insert(1.0f64, 1.0f32);
                    c.get(1.0f64);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(1.0f64), Some(1.0f32));
    }
}
