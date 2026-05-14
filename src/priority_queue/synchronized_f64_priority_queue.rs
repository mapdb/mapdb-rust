// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::f64_priority_queue::F64PriorityQueue;

/// Thread-safe wrapper around [`F64PriorityQueue`].
#[derive(Debug, Default)]
pub struct SynchronizedF64PriorityQueue {
    inner: RwLock<F64PriorityQueue>,
}

impl SynchronizedF64PriorityQueue {
    pub fn new() -> Self {
        SynchronizedF64PriorityQueue {
            inner: RwLock::new(F64PriorityQueue::new()),
        }
    }

    pub fn of(values: &[f64]) -> Self {
        SynchronizedF64PriorityQueue {
            inner: RwLock::new(F64PriorityQueue::of(values)),
        }
    }

    pub fn push(&self, value: f64) {
        self.inner.write().unwrap().push(value);
    }

    pub fn pop(&self) -> Option<f64> {
        self.inner.write().unwrap().pop()
    }

    pub fn peek(&self) -> Option<f64> {
        self.inner.read().unwrap().peek()
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

    pub fn for_each(&self, mut f: impl FnMut(f64)) {
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
        let q = SynchronizedF64PriorityQueue::new();
        assert!(q.is_empty());
        q.push(3.0f64);
        q.push(1.0f64);
        q.push(2.0f64);
        assert_eq!(q.len(), 3);
        assert!(q.peek().is_some());
        assert!(q.pop().is_some());
        assert_eq!(q.len(), 2);
        q.clear();
        assert!(q.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let q = SynchronizedF64PriorityQueue::of(&[1.0f64, 2.0f64, 3.0f64]);
        assert_eq!(q.to_vec().len(), 3);
        let mut count = 0usize;
        q.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let q = Arc::new(SynchronizedF64PriorityQueue::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&q);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.push(1.0f64);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(q.len(), 400);
    }
}
