// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::bool_hash_bag::BoolHashBag;

/// Thread-safe wrapper around [`BoolHashBag`].
#[derive(Debug, Default)]
pub struct SynchronizedBoolHashBag {
    inner: RwLock<BoolHashBag>,
}

impl SynchronizedBoolHashBag {
    pub fn new() -> Self {
        SynchronizedBoolHashBag {
            inner: RwLock::new(BoolHashBag::new()),
        }
    }

    pub fn of(values: &[bool]) -> Self {
        SynchronizedBoolHashBag {
            inner: RwLock::new(BoolHashBag::of(values)),
        }
    }

    pub fn add(&self, value: bool) {
        self.inner.write().unwrap().add(value);
    }

    pub fn remove(&self, value: bool) -> bool {
        self.inner.write().unwrap().remove(value)
    }

    pub fn remove_all(&self, value: bool) -> bool {
        self.inner.write().unwrap().remove_all(value)
    }

    pub fn occurrences_of(&self, value: bool) -> usize {
        self.inner.read().unwrap().occurrences_of(value)
    }

    pub fn contains(&self, value: bool) -> bool {
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

    pub fn to_vec(&self) -> Vec<bool> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, mut f: impl FnMut(bool)) {
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
        let b = SynchronizedBoolHashBag::new();
        assert!(b.is_empty());
        b.add(true);
        b.add(true);
        b.add(false);
        assert_eq!(b.size(), 3);
        assert_eq!(b.occurrences_of(true), 2);
        assert!(b.contains(true));
        assert!(b.remove(true));
        assert_eq!(b.occurrences_of(true), 1);
        b.clear();
        assert!(b.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let b = SynchronizedBoolHashBag::new();
        b.add(true);
        b.add(false);
        assert_eq!(b.to_vec().len(), 2);
        let mut count = 0usize;
        b.for_each(|_| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_concurrent() {
        let b = Arc::new(SynchronizedBoolHashBag::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&b);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.add(true);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(b.size(), 400);
        assert_eq!(b.occurrences_of(true), 400);
    }
}
