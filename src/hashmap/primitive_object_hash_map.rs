// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

// Hand-written generic maps — not code-generated (Rust generics handle the object side).

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

/// Hash map from a primitive key `K` to any value `V`.
/// Use this when the key is a primitive type (i32, i64, etc.) and the value is any type.
///
/// For primitive-to-primitive maps, use the specialized generated types instead
/// (e.g., `I32I64HashMap`) which avoid boxing.
#[derive(Debug, Clone)]
pub struct PrimitiveObjectHashMap<K: Copy + Eq + Hash, V> {
    inner: HashMap<K, V>,
}

impl<K: Copy + Eq + Hash, V> PrimitiveObjectHashMap<K, V> {
    pub fn new() -> Self {
        PrimitiveObjectHashMap {
            inner: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        PrimitiveObjectHashMap {
            inner: HashMap::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.inner.get(&key)
    }

    pub fn get_or_default<'a>(&'a self, key: K, default: &'a V) -> &'a V {
        self.inner.get(&key).unwrap_or(default)
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        self.inner.remove(&key)
    }

    pub fn contains_key(&self, key: K) -> bool {
        self.inner.contains_key(&key)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> + '_ {
        self.inner.iter().map(|(&k, v)| (k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = K> + '_ {
        self.inner.keys().copied()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.inner.values()
    }

    pub fn for_each(&self, mut f: impl FnMut(K, &V)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }

    pub fn select(&self, predicate: impl Fn(K, &V) -> bool) -> Self
    where
        V: Clone,
    {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if predicate(k, v) {
                result.insert(k, v.clone());
            }
        }
        result
    }

    pub fn reject(&self, predicate: impl Fn(K, &V) -> bool) -> Self
    where
        V: Clone,
    {
        let mut result = Self::new();
        for (k, v) in self.iter() {
            if !predicate(k, v) {
                result.insert(k, v.clone());
            }
        }
        result
    }
}

impl<K: Copy + Eq + Hash, V> Default for PrimitiveObjectHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Copy + Eq + Hash + fmt::Display, V: fmt::Display> fmt::Display
    for PrimitiveObjectHashMap<K, V>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (k, v) in self.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}={}", k, v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

/// Hash map from any key `K` to a primitive value `V`.
#[derive(Debug, Clone)]
pub struct ObjectPrimitiveHashMap<K: Eq + Hash, V: Copy> {
    inner: HashMap<K, V>,
}

impl<K: Eq + Hash, V: Copy> ObjectPrimitiveHashMap<K, V> {
    pub fn new() -> Self {
        ObjectPrimitiveHashMap {
            inner: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.inner.get(key).copied()
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.remove(key)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, V)> + '_ {
        self.inner.iter().map(|(k, &v)| (k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.inner.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = V> + '_ {
        self.inner.values().copied()
    }

    pub fn for_each(&self, mut f: impl FnMut(&K, V)) {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
}

impl<K: Eq + Hash, V: Copy> Default for ObjectPrimitiveHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + fmt::Display, V: Copy + fmt::Display> fmt::Display
    for ObjectPrimitiveHashMap<K, V>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (k, v) in self.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}={}", k, v)?;
            first = false;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_object_insert_get() {
        let mut m = PrimitiveObjectHashMap::<i32, String>::new();
        m.insert(1, "hello".to_string());
        m.insert(2, "world".to_string());
        assert_eq!(m.get(1), Some(&"hello".to_string()));
        assert_eq!(m.get(99), None);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_primitive_object_remove() {
        let mut m = PrimitiveObjectHashMap::<i32, String>::new();
        m.insert(1, "a".to_string());
        assert_eq!(m.remove(1), Some("a".to_string()));
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn test_primitive_object_select() {
        let mut m = PrimitiveObjectHashMap::<i32, String>::new();
        m.insert(1, "a".to_string());
        m.insert(2, "b".to_string());
        m.insert(3, "c".to_string());
        let sel = m.select(|k, _v| k > 1);
        assert_eq!(sel.len(), 2);
    }

    #[test]
    fn test_primitive_object_clear() {
        let mut m = PrimitiveObjectHashMap::<i32, String>::new();
        m.insert(1, "a".to_string());
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn test_primitive_object_display() {
        let mut m = PrimitiveObjectHashMap::<i32, String>::new();
        m.insert(1, "a".to_string());
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_object_primitive_insert_get() {
        let mut m = ObjectPrimitiveHashMap::<String, i32>::new();
        m.insert("hello".to_string(), 100);
        m.insert("world".to_string(), 200);
        assert_eq!(m.get(&"hello".to_string()), Some(100));
        assert_eq!(m.get(&"missing".to_string()), None);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_object_primitive_remove() {
        let mut m = ObjectPrimitiveHashMap::<String, i32>::new();
        m.insert("a".to_string(), 10);
        assert_eq!(m.remove(&"a".to_string()), Some(10));
        assert!(m.is_empty());
    }

    #[test]
    fn test_object_primitive_keys_values() {
        let mut m = ObjectPrimitiveHashMap::<String, i32>::new();
        m.insert("a".to_string(), 10);
        m.insert("b".to_string(), 20);
        assert_eq!(m.keys().count(), 2);
        assert_eq!(m.values().count(), 2);
    }

    #[test]
    fn test_object_primitive_display() {
        let mut m = ObjectPrimitiveHashMap::<String, i32>::new();
        m.insert("a".to_string(), 10);
        assert!(!m.to_string().is_empty());
    }

    #[test]
    fn test_resize() {
        let mut m = PrimitiveObjectHashMap::<i32, i32>::new();
        for i in 0..100 {
            m.insert(i, i * 10);
        }
        assert_eq!(m.len(), 100);
        for i in 0..100 {
            assert_eq!(m.get(i), Some(&(i * 10)));
        }
    }
}
