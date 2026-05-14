// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::char_array_stack::CharArrayStack;

/// Thread-safe wrapper around [`CharArrayStack`].
#[derive(Debug, Default)]
pub struct SynchronizedCharArrayStack {
    inner: RwLock<CharArrayStack>,
}

impl SynchronizedCharArrayStack {
    pub fn new() -> Self {
        SynchronizedCharArrayStack {
            inner: RwLock::new(CharArrayStack::new()),
        }
    }

    pub fn of(values: &[char]) -> Self {
        SynchronizedCharArrayStack {
            inner: RwLock::new(CharArrayStack::of(values)),
        }
    }

    pub fn push(&self, value: char) {
        self.inner.write().unwrap().push(value);
    }

    pub fn pop(&self) -> Option<char> {
        self.inner.write().unwrap().pop()
    }

    pub fn peek(&self) -> Option<char> {
        self.inner.read().unwrap().peek()
    }

    pub fn contains(&self, value: char) -> bool {
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

    pub fn to_vec(&self) -> Vec<char> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, f: impl FnMut(char)) {
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
        let s = SynchronizedCharArrayStack::new();
        assert!(s.is_empty());
        s.push('a');
        s.push('b');
        assert_eq!(s.len(), 2);
        assert_eq!(s.peek(), Some('b'));
        assert_eq!(s.pop(), Some('b'));
        assert_eq!(s.len(), 1);
        assert!(s.contains('a'));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let s = SynchronizedCharArrayStack::of(&['a', 'b', 'c']);
        assert_eq!(s.to_vec().len(), 3);
        let mut count = 0usize;
        s.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let s = Arc::new(SynchronizedCharArrayStack::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&s);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.push('a');
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(s.len(), 400);
    }
}
