// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

#![allow(
    clippy::needless_borrow,
    clippy::unnecessary_cast,
    clippy::explicit_auto_deref,
    clippy::new_without_default
)]

pub mod array_deque;
pub mod bit_set;
pub mod bloom;
pub mod bounded_lru;
pub mod bounded_map;
pub mod bulk;
pub mod count_min;
pub mod fenwick;
pub mod frozen;
pub mod hash;
pub mod hash_table;
pub mod hashable_float;
pub mod hyperloglog;
pub mod immutable;
pub mod immutable_sorted;
mod index_table;
pub mod interval;
pub mod multimap;
pub mod object;
pub mod pair;
pub mod parallel;
pub mod priority_queue;
pub mod range;
pub mod range_map;
pub mod range_set;
pub mod rich_iterator;
pub mod roaring;
mod slot_list;
pub mod space_saving;
pub mod synchronized;

pub use array_deque::ArrayDeque;
pub use bit_set::BitSet;
pub use bloom::Bloom;
pub use bounded_lru::{BoundedLruMap, BoundedLruMapBuilder, EvictionCause};
pub use bounded_map::{
    BoundedMap, BoundedMapIntoIter, BoundedMapIter, BoundedMapIterMut, EvictionPolicy, Fifo, Lru,
};
pub use bulk::{BulkError, DuplicatePolicy};
pub use count_min::CountMin;
pub use fenwick::FenwickTree;
pub use frozen::Frozen;
pub use hash_table::{
    Entry, OccupiedEntry, OpenHashMap, OpenHashMapDrain, OpenHashMapIterMut, OpenHashSet,
    OpenHashSetDrain, VacantEntry,
};
pub use hashable_float::{HashableF32, HashableF64};
pub use hyperloglog::{HllError, HyperLogLog};
pub use immutable::{ImmutableHashMap, ImmutableHashSet, ImmutableList};
pub use immutable_sorted::{
    ImmutableSortedMap, ImmutableSortedSet, SortedRangeElemIter, SortedRangeIter,
};
pub use interval::{Interval, SignedPrimInt};
pub use multimap::{Multimap, SetMultimap};
pub use pair::Pair;
pub use parallel::spliterator::{SliceSpliterator, Spliterator};
pub use parallel::BatchIterable;
#[cfg(feature = "parallel")]
pub use parallel::{as_parallel, ParallelSlice};
pub use priority_queue::PriorityQueue;
pub use range::{BoundType, Cut, Range};
pub use range_map::RangeMap;
pub use range_set::RangeSet;
pub use rich_iterator::RichIterator;
pub use roaring::{RoaringError, RoaringU32};
pub use space_saving::SpaceSaving;
pub use synchronized::{synchronized, SyncGuard, Synchronized};
