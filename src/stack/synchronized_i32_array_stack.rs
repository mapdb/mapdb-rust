// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::i32_array_stack::I32ArrayStack;

/// Thread-safe wrapper around [`I32ArrayStack`].
#[derive(Debug, Default)]
pub struct SynchronizedI32ArrayStack {
    inner: RwLock<I32ArrayStack>,
}

impl SynchronizedI32ArrayStack {
    pub fn new() -> Self {
        SynchronizedI32ArrayStack {
            inner: RwLock::new(I32ArrayStack::new()),
        }
    }

    pub fn of(values: &[i32]) -> Self {
        SynchronizedI32ArrayStack {
            inner: RwLock::new(I32ArrayStack::of(values)),
        }
    }

    pub fn push(&self, value: i32) {
        self.inner.write().unwrap().push(value);
    }

    pub fn pop(&self) -> Option<i32> {
        self.inner.write().unwrap().pop()
    }

    pub fn peek(&self) -> Option<i32> {
        self.inner.read().unwrap().peek()
    }

    pub fn contains(&self, value: i32) -> bool {
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

    pub fn to_vec(&self) -> Vec<i32> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, f: impl FnMut(i32)) {
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
        let s = SynchronizedI32ArrayStack::new();
        assert!(s.is_empty());
        s.push(1);
        s.push(2);
        assert_eq!(s.len(), 2);
        assert_eq!(s.peek(), Some(2));
        assert_eq!(s.pop(), Some(2));
        assert_eq!(s.len(), 1);
        assert!(s.contains(1));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let s = SynchronizedI32ArrayStack::of(&[1, 2, 3]);
        assert_eq!(s.to_vec().len(), 3);
        let mut count = 0usize;
        s.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let s = Arc::new(SynchronizedI32ArrayStack::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&s);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.push(1);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(s.len(), 400);
    }
}
