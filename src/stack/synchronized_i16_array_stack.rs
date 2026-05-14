// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::i16_array_stack::I16ArrayStack;

/// Thread-safe wrapper around [`I16ArrayStack`].
#[derive(Debug, Default)]
pub struct SynchronizedI16ArrayStack {
    inner: RwLock<I16ArrayStack>,
}

impl SynchronizedI16ArrayStack {
    pub fn new() -> Self {
        SynchronizedI16ArrayStack {
            inner: RwLock::new(I16ArrayStack::new()),
        }
    }

    pub fn of(values: &[i16]) -> Self {
        SynchronizedI16ArrayStack {
            inner: RwLock::new(I16ArrayStack::of(values)),
        }
    }

    pub fn push(&self, value: i16) {
        self.inner.write().unwrap().push(value);
    }

    pub fn pop(&self) -> Option<i16> {
        self.inner.write().unwrap().pop()
    }

    pub fn peek(&self) -> Option<i16> {
        self.inner.read().unwrap().peek()
    }

    pub fn contains(&self, value: i16) -> bool {
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

    pub fn to_vec(&self) -> Vec<i16> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, f: impl FnMut(i16)) {
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
        let s = SynchronizedI16ArrayStack::new();
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
        let s = SynchronizedI16ArrayStack::of(&[1, 2, 3]);
        assert_eq!(s.to_vec().len(), 3);
        let mut count = 0usize;
        s.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let s = Arc::new(SynchronizedI16ArrayStack::new());
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
