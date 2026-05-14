// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::i16_hash_bag::I16HashBag;

/// Thread-safe wrapper around [`I16HashBag`].
#[derive(Debug, Default)]
pub struct SynchronizedI16HashBag {
    inner: RwLock<I16HashBag>,
}

impl SynchronizedI16HashBag {
    pub fn new() -> Self {
        SynchronizedI16HashBag {
            inner: RwLock::new(I16HashBag::new()),
        }
    }

    pub fn of(values: &[i16]) -> Self {
        SynchronizedI16HashBag {
            inner: RwLock::new(I16HashBag::of(values)),
        }
    }

    pub fn add(&self, value: i16) {
        self.inner.write().unwrap().add(value);
    }

    pub fn remove(&self, value: i16) -> bool {
        self.inner.write().unwrap().remove(value)
    }

    pub fn remove_all(&self, value: i16) -> bool {
        self.inner.write().unwrap().remove_all(value)
    }

    pub fn occurrences_of(&self, value: i16) -> usize {
        self.inner.read().unwrap().occurrences_of(value)
    }

    pub fn contains(&self, value: i16) -> bool {
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

    pub fn to_vec(&self) -> Vec<i16> {
        self.inner.read().unwrap().to_vec()
    }

    pub fn for_each(&self, mut f: impl FnMut(i16)) {
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
        let b = SynchronizedI16HashBag::new();
        assert!(b.is_empty());
        b.add(1);
        b.add(1);
        b.add(2);
        assert_eq!(b.size(), 3);
        assert_eq!(b.occurrences_of(1), 2);
        assert!(b.contains(1));
        assert!(b.remove(1));
        assert_eq!(b.occurrences_of(1), 1);
        b.clear();
        assert!(b.is_empty());
    }

    #[test]
    fn test_to_vec_and_for_each() {
        let b = SynchronizedI16HashBag::new();
        b.add(1);
        b.add(2);
        assert_eq!(b.to_vec().len(), 2);
        let mut count = 0usize;
        b.for_each(|_| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_concurrent() {
        let b = Arc::new(SynchronizedI16HashBag::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&b);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.add(1);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(b.size(), 400);
        assert_eq!(b.occurrences_of(1), 400);
    }
}
