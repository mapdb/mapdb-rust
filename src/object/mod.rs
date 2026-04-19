// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Generic (object-typed) collection types and traits.
//!
//! These complement the primitive-specialized collections (e.g., `I32ArrayList`)
//! with generic versions that work with any type (`ArrayList<T>`, `HashSet<T>`, etc.).

mod arraylist;
mod arraystack;
mod hashbag;
mod hashbimap;
mod hashmap;
mod hashset;
mod linkedhashmap;
mod linkedhashset;
pub mod strategy;
mod strategy_hashmap;
mod strategy_hashset;
mod traits;
pub mod treemap;
mod treeset;

pub use arraylist::ArrayList;
pub use arraystack::ArrayStack;
pub use hashbag::HashBag;
pub use hashbimap::HashBiMap;
pub use hashmap::HashMap;
pub use hashset::HashSet;
pub use linkedhashmap::LinkedHashMap;
pub use linkedhashset::LinkedHashSet;
pub use strategy::{
    by_field, case_insensitive_hashing_strategy, comparator_by_field, comparator_by_field_with,
    natural_comparator, reverse_comparator, reversed, string_hashing_strategy, then_comparing,
    Comparator, HashingStrategy,
};
pub use strategy_hashmap::HashMapWithStrategy;
pub use strategy_hashset::HashSetWithStrategy;
pub use traits::*;
pub use treemap::TreeMap;
pub use treeset::TreeSet;

#[cfg(test)]
mod property_tests;

#[cfg(test)]
mod strategy_examples;

#[cfg(all(test, feature = "kata"))]
mod pet_kata;
