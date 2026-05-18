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
pub mod hash_table;
pub mod hashable_float;
pub mod immutable;
pub mod interval;
pub mod multimap;
pub mod object;
pub mod pair;
pub mod priority_queue;
pub mod stream;
pub mod synchronized;
pub mod traits;

pub use array_deque::ArrayDeque;
pub use bit_set::BitSet;
pub use hash_table::{OpenHashMap, OpenHashSet};
pub use hashable_float::{HashableF32, HashableF64};
pub use immutable::{ImmutableHashMap, ImmutableHashSet, ImmutableList};
pub use interval::{Interval, SignedPrimInt};
pub use pair::Pair;
pub use priority_queue::PriorityQueue;
pub use synchronized::{synchronized, Synchronized};
