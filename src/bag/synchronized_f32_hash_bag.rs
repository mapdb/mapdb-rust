// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::f32_hash_bag::F32HashBag;

/// Thread-safe wrapper around [`F32HashBag`].
#[derive(Debug, Default)]
pub struct SynchronizedF32HashBag {
    inner: RwLock<F32HashBag>,
}

impl SynchronizedF32HashBag {
    pub fn new() -> Self {
        SynchronizedF32HashBag {
            inner: RwLock::new(F32HashBag::new()),
        }
    }

    pub fn of(values: &[f32]) -> Self {
        SynchronizedF32HashBag {
            inner: RwLock::new(F32HashBag::of(values)),
        }
    }

    pub fn add(&self, value: f32) {
        self.inner.write().unwrap().add(value);
    }

    pub fn remove(&self, value: f32) -> bool {
        self.inner.write().unwrap().remove(value)
    }

    pub fn remove_all(&self, value: f32) -> bool {
        self.inner.write().unwrap().remove_all(value)
    }

    pub fn occurrences_of(&self, value: f32) -> usize {
        self.inner.read().unwrap().occurrences_of(value)
    }

    pub fn contains(&self, value: f32) -> bool {
        self.inner.read().unwrap().contains(value)
    }

    pub fn size(&self) -> usize {
        self.inner.read().unwrap().size()
    }
    pub fn size_distinct(&self) -> usize {
        self.inner.read().unwrap().size_distinct()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }
    pub fn clear(&self) {
        self.inner.write().unwrap().clear();
    }

    pub fn to_vec(&self) -> Vec<f32> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, mut f: impl FnMut(f32)) {
        let snap = self.inner.read().unwrap().to_vec();
        for v in snap {
            f(v);
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
        let b = SynchronizedF32HashBag::new();
        assert!(b.is_empty());
        b.add(1.0f32);
        b.add(1.0f32);
        b.add(2.0f32);
        assert_eq!(b.size(), 3);
        assert_eq!(b.occurrences_of(1.0f32), 2);
        assert!(b.contains(1.0f32));
        assert!(b.remove(1.0f32));
        assert_eq!(b.occurrences_of(1.0f32), 1);
        b.clear();
        assert!(b.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let b = SynchronizedF32HashBag::new();
        b.add(1.0f32);
        b.add(2.0f32);
        assert_eq!(b.to_vec().len(), 2);
        let mut count = 0usize;
        b.for_each(|_| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_concurrent() {
        let b = Arc::new(SynchronizedF32HashBag::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&b);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.add(1.0f32);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(b.size(), 400);
        assert_eq!(b.occurrences_of(1.0f32), 400);
    }
}
