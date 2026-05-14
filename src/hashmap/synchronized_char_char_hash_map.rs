// AUTO-GENERATED. DO NOT EDIT.

use std::sync::RwLock;

use super::char_char_hash_map::CharCharHashMap;

/// Thread-safe wrapper around [`CharCharHashMap`]. Reads are concurrent;
/// mutations are exclusive. Lock poisoning panics (parity with Go).
#[derive(Debug, Default)]
pub struct SynchronizedCharCharHashMap {
    inner: RwLock<CharCharHashMap>,
}

impl SynchronizedCharCharHashMap {
    pub fn new() -> Self {
        SynchronizedCharCharHashMap {
            inner: RwLock::new(CharCharHashMap::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        SynchronizedCharCharHashMap {
            inner: RwLock::new(CharCharHashMap::with_capacity(capacity)),
        }
    }

    pub fn insert(&self, key: char, value: char) -> Option<char> {
        self.inner.write().unwrap().insert(key, value)
    }

    pub fn get(&self, key: char) -> Option<char> {
        self.inner.read().unwrap().get(key)
    }

    pub fn get_or_default(&self, key: char, default: char) -> char {
        self.inner.read().unwrap().get_or_default(key, default)
    }

    pub fn remove(&self, key: char) -> Option<char> {
        self.inner.write().unwrap().remove(key)
    }

    pub fn contains_key(&self, key: char) -> bool {
        self.inner.read().unwrap().contains_key(key)
    }

    pub fn contains_value(&self, value: char) -> bool {
        self.inner.read().unwrap().contains_value(value)
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

    pub fn keys_to_vec(&self) -> Vec<char> {
        self.inner.read().unwrap().keys_to_vec()
    }

    pub fn values_to_vec(&self) -> Vec<char> {
        self.inner.read().unwrap().values_to_vec()
    }

    /// Returns an owned snapshot of entries for iteration under read lock.
    pub fn to_vec(&self) -> Vec<(char, char)> {
        self.inner.read().unwrap().iter().collect()
    }

    pub fn for_each(&self, mut f: impl FnMut(char, char)) {
        let snap = self.to_vec();
        for (k, v) in snap {
            f(k, v);
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
        let m = SynchronizedCharCharHashMap::new();
        assert!(m.is_empty());
        assert_eq!(m.insert('a', 'a'), None);
        assert_eq!(m.insert('b', 'b'), None);
        assert_eq!(m.insert('c', 'c'), None);
        assert_eq!(m.get('a'), Some('a'));
        assert!(m.contains_key('a'));
        assert!(m.contains_value('b'));
        let old = m.insert('a', 'b');
        assert_eq!(old, Some('a'));
        assert_eq!(m.remove('a'), Some('b'));
        assert!(!m.contains_key('a'));
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_get_or_default() {
        let m = SynchronizedCharCharHashMap::new();
        m.insert('a', 'a');
        assert_eq!(m.get_or_default('a', 'c'), 'a');
        assert_eq!(m.get_or_default('z', 'c'), 'c');
    }

    #[test]
    fn test_snapshots_and_for_each() {
        let m = SynchronizedCharCharHashMap::new();
        m.insert('a', 'a');
        m.insert('b', 'b');
        assert_eq!(m.keys_to_vec().len(), 2);
        assert_eq!(m.values_to_vec().len(), 2);
        assert_eq!(m.to_vec().len(), 2);
        let mut count = 0usize;
        m.for_each(|_, _| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_concurrent() {
        let m = Arc::new(SynchronizedCharCharHashMap::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&m);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.insert('a', 'a');
                    c.get('a');
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(m.len(), 1);
        assert_eq!(m.get('a'), Some('a'));
    }
}
