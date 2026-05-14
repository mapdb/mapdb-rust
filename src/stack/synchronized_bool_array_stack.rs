// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::bool_array_stack::BoolArrayStack;

/// Thread-safe wrapper around [`BoolArrayStack`].
#[derive(Debug, Default)]
pub struct SynchronizedBoolArrayStack {
    inner: RwLock<BoolArrayStack>,
}

impl SynchronizedBoolArrayStack {
    pub fn new() -> Self {
        SynchronizedBoolArrayStack {
            inner: RwLock::new(BoolArrayStack::new()),
        }
    }

    pub fn of(values: &[bool]) -> Self {
        SynchronizedBoolArrayStack {
            inner: RwLock::new(BoolArrayStack::of(values)),
        }
    }

    pub fn push(&self, value: bool) {
        self.inner.write().unwrap().push(value);
    }

    pub fn pop(&self) -> Option<bool> {
        self.inner.write().unwrap().pop()
    }

    pub fn peek(&self) -> Option<bool> {
        self.inner.read().unwrap().peek()
    }

    pub fn contains(&self, value: bool) -> bool {
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

    pub fn to_vec(&self) -> Vec<bool> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, f: impl FnMut(bool)) {
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
        let s = SynchronizedBoolArrayStack::new();
        assert!(s.is_empty());
        s.push(true);
        s.push(false);
        assert_eq!(s.len(), 2);
        assert_eq!(s.peek(), Some(false));
        assert_eq!(s.pop(), Some(false));
        assert_eq!(s.len(), 1);
        assert!(s.contains(true));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let s = SynchronizedBoolArrayStack::of(&[true, false, true]);
        assert_eq!(s.to_vec().len(), 3);
        let mut count = 0usize;
        s.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let s = Arc::new(SynchronizedBoolArrayStack::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&s);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.push(true);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(s.len(), 400);
    }
}
