// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::i8_array_stack::I8ArrayStack;

/// Thread-safe wrapper around [`I8ArrayStack`].
#[derive(Debug, Default)]
pub struct SynchronizedI8ArrayStack {
    inner: RwLock<I8ArrayStack>,
}

impl SynchronizedI8ArrayStack {
    pub fn new() -> Self {
        SynchronizedI8ArrayStack {
            inner: RwLock::new(I8ArrayStack::new()),
        }
    }

    pub fn of(values: &[i8]) -> Self {
        SynchronizedI8ArrayStack {
            inner: RwLock::new(I8ArrayStack::of(values)),
        }
    }

    pub fn push(&self, value: i8) {
        self.inner.write().unwrap().push(value);
    }

    pub fn pop(&self) -> Option<i8> {
        self.inner.write().unwrap().pop()
    }

    pub fn peek(&self) -> Option<i8> {
        self.inner.read().unwrap().peek()
    }

    pub fn contains(&self, value: i8) -> bool {
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

    pub fn to_vec(&self) -> Vec<i8> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, f: impl FnMut(i8)) {
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
        let s = SynchronizedI8ArrayStack::new();
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
        let s = SynchronizedI8ArrayStack::of(&[1, 2, 3]);
        assert_eq!(s.to_vec().len(), 3);
        let mut count = 0usize;
        s.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let s = Arc::new(SynchronizedI8ArrayStack::new());
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
