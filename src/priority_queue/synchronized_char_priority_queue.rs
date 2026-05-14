// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::char_priority_queue::CharPriorityQueue;

/// Thread-safe wrapper around [`CharPriorityQueue`].
#[derive(Debug, Default)]
pub struct SynchronizedCharPriorityQueue {
    inner: RwLock<CharPriorityQueue>,
}

impl SynchronizedCharPriorityQueue {
    pub fn new() -> Self {
        SynchronizedCharPriorityQueue {
            inner: RwLock::new(CharPriorityQueue::new()),
        }
    }

    pub fn of(values: &[char]) -> Self {
        SynchronizedCharPriorityQueue {
            inner: RwLock::new(CharPriorityQueue::of(values)),
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

    pub fn for_each(&self, mut f: impl FnMut(char)) {
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
        let q = SynchronizedCharPriorityQueue::new();
        assert!(q.is_empty());
        q.push('c');
        q.push('a');
        q.push('b');
        assert_eq!(q.len(), 3);
        assert!(q.peek().is_some());
        assert!(q.pop().is_some());
        assert_eq!(q.len(), 2);
        q.clear();
        assert!(q.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let q = SynchronizedCharPriorityQueue::of(&['a', 'b', 'c']);
        assert_eq!(q.to_vec().len(), 3);
        let mut count = 0usize;
        q.for_each(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_concurrent() {
        let q = Arc::new(SynchronizedCharPriorityQueue::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&q);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.push('a');
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(q.len(), 400);
    }
}
