// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Generic collection traits mirroring the primitive trait hierarchy.

/// Read-only collection of `T` values.
pub trait Collection<T: PartialEq> {
    fn len(&self) -> usize;
    fn contains(&self, value: &T) -> bool;
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
        Self: Sized,
    {
        self.iter().cloned().collect()
    }
    fn for_each(&self, mut f: impl FnMut(&T))
    where
        Self: Sized,
    {
        for v in self.iter() {
            f(v);
        }
    }
    fn any_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool
    where
        Self: Sized,
    {
        self.iter().any(predicate)
    }
    fn all_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool
    where
        Self: Sized,
    {
        self.iter().all(predicate)
    }
    fn none_satisfy(&self, predicate: impl Fn(&T) -> bool) -> bool
    where
        Self: Sized,
    {
        !self.iter().any(predicate)
    }
    fn count_where(&self, predicate: impl Fn(&T) -> bool) -> usize
    where
        Self: Sized,
    {
        self.iter().filter(|v| predicate(v)).count()
    }
    fn detect(&self, predicate: impl Fn(&T) -> bool) -> Option<&T>
    where
        Self: Sized,
    {
        self.iter().find(|v| predicate(v))
    }
    fn select(&self, predicate: impl Fn(&T) -> bool) -> Vec<T>
    where
        T: Clone,
        Self: Sized,
    {
        self.iter().filter(|v| predicate(v)).cloned().collect()
    }
    fn reject(&self, predicate: impl Fn(&T) -> bool) -> Vec<T>
    where
        T: Clone,
        Self: Sized,
    {
        self.iter().filter(|v| !predicate(v)).cloned().collect()
    }
    fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, &T) -> R) -> R
    where
        Self: Sized,
    {
        let mut acc = initial;
        for v in self.iter() {
            acc = f(acc, v);
        }
        acc
    }
}

/// Mutable collection — adds `clear`.
pub trait MutableCollection<T: PartialEq>: Collection<T> {
    fn clear(&mut self);
}

/// Ordered list with positional access.
pub trait List<T: PartialEq>: Collection<T> {
    fn get(&self, index: usize) -> Option<&T>;
    fn index_of(&self, value: &T) -> Option<usize>;
}

/// Mutable list.
pub trait MutableList<T: PartialEq>: List<T> + MutableCollection<T> {
    fn push(&mut self, value: T);
    fn set(&mut self, index: usize, value: T) -> T;
}

/// Set (unique elements).
pub trait Set<T: PartialEq>: Collection<T> {}

/// Mutable set.
pub trait MutableSet<T: PartialEq>: Set<T> + MutableCollection<T> {
    fn add(&mut self, value: T) -> bool;
    fn remove(&mut self, value: &T) -> bool;
}

/// Bag (multiset with occurrence counts).
pub trait Bag<T: PartialEq>: Collection<T> {
    fn occurrences_of(&self, value: &T) -> usize;
    fn size_distinct(&self) -> usize;
}

/// Mutable bag.
pub trait MutableBag<T: PartialEq>: Bag<T> + MutableCollection<T> {
    fn add(&mut self, value: T);
}

/// LIFO stack.
pub trait Stack<T: PartialEq>: Collection<T> {
    fn peek(&self) -> Option<&T>;
}

/// Mutable stack.
pub trait MutableStack<T: PartialEq>: Stack<T> + MutableCollection<T> {
    fn push(&mut self, value: T);
    fn pop(&mut self) -> Option<T>;
}

/// Read-only map.
pub trait MapIterable<K, V> {
    fn len(&self) -> usize;
    fn contains_key(&self, key: &K) -> bool;
    fn get(&self, key: &K) -> Option<&V>;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn iter(&self) -> Box<dyn Iterator<Item = (&K, &V)> + '_>;
    fn for_each(&self, mut f: impl FnMut(&K, &V))
    where
        Self: Sized,
    {
        for (k, v) in self.iter() {
            f(k, v);
        }
    }
    fn any_satisfy(&self, predicate: impl Fn(&K, &V) -> bool) -> bool
    where
        Self: Sized,
    {
        self.iter().any(|(k, v)| predicate(k, v))
    }
    fn all_satisfy(&self, predicate: impl Fn(&K, &V) -> bool) -> bool
    where
        Self: Sized,
    {
        self.iter().all(|(k, v)| predicate(k, v))
    }
    fn none_satisfy(&self, predicate: impl Fn(&K, &V) -> bool) -> bool
    where
        Self: Sized,
    {
        !self.iter().any(|(k, v)| predicate(k, v))
    }
}

/// Mutable map.
pub trait MutableMap<K, V>: MapIterable<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V>;
    fn remove(&mut self, key: &K) -> Option<V>;
    fn clear(&mut self);
}
