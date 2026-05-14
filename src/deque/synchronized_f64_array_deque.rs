// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::f64_array_deque::F64ArrayDeque;

/// Thread-safe wrapper around [`F64ArrayDeque`].
#[derive(Debug, Default)]
pub struct SynchronizedF64ArrayDeque {
    inner: RwLock<F64ArrayDeque>,
}

impl SynchronizedF64ArrayDeque {
    pub fn new() -> Self {
        SynchronizedF64ArrayDeque {
            inner: RwLock::new(F64ArrayDeque::new()),
        }
    }

    pub fn of(values: &[f64]) -> Self {
        SynchronizedF64ArrayDeque {
            inner: RwLock::new(F64ArrayDeque::of(values)),
        }
    }

    pub fn add_first(&self, value: f64) {
        self.inner.write().unwrap().add_first(value);
    }

    pub fn add_last(&self, value: f64) {
        self.inner.write().unwrap().add_last(value);
    }

    pub fn remove_first(&self) -> Option<f64> {
        self.inner.write().unwrap().remove_first()
    }

    pub fn remove_last(&self) -> Option<f64> {
        self.inner.write().unwrap().remove_last()
    }

    pub fn peek_first(&self) -> Option<f64> {
        self.inner.read().unwrap().peek_first()
    }

    pub fn peek_last(&self) -> Option<f64> {
        self.inner.read().unwrap().peek_last()
    }

    pub fn contains(&self, value: f64) -> bool {
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

    pub fn to_vec(&self) -> Vec<f64> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, f: impl FnMut(f64)) {
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
        let d = SynchronizedF64ArrayDeque::new();
        assert!(d.is_empty());
        d.add_last(1.0f64);
        d.add_last(2.0f64);
        d.add_first(3.0f64);
        assert_eq!(d.len(), 3);
        assert_eq!(d.peek_first(), Some(3.0f64));
        assert_eq!(d.peek_last(), Some(2.0f64));
        assert_eq!(d.remove_first(), Some(3.0f64));
        assert_eq!(d.remove_last(), Some(2.0f64));
        assert!(d.contains(1.0f64));
        d.clear();
        assert!(d.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let d = SynchronizedF64ArrayDeque::of(&[1.0f64, 2.0f64, 3.0f64]);
        assert_eq!(d.to_vec().len(), 3);
        let mut count = 0usize;
        d.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let d = Arc::new(SynchronizedF64ArrayDeque::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&d);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.add_last(1.0f64);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(d.len(), 400);
    }
}
