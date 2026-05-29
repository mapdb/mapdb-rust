// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Parallel and splittable iteration, porting the Eclipse Collections trio:
//!
//! | Eclipse Collections | here | dependency | gating |
//! | --- | --- | --- | --- |
//! | `java.util.Spliterator` | [`spliterator::Spliterator`] | std only | always |
//! | `BatchIterable` (fixed sections) | [`batch::BatchIterable`] | std only | always |
//! | batch parallel executor (no work stealing) | [`batch`] free fns | std threads | `parallel` |
//! | `ParallelIterable` / `asParallel` (work stealing) | `iterable::ParallelSlice` | rayon | `parallel` |
//!
//! The splitting/sectioning *primitives* (`Spliterator`, `BatchIterable`) are
//! pure std and always compiled. Parallel *execution* — both the fixed-chunk
//! batch executor and the rayon work-stealing view — lives behind the
//! `parallel` feature, so with the feature off the crate spawns no threads and
//! pulls no third-party crate. rayon is used only by the work-stealing path and
//! is a Rust-only choice; the sibling language ports use their own runtimes.

pub mod batch;
pub mod spliterator;

#[cfg(feature = "parallel")]
pub mod iterable;

pub use batch::BatchIterable;
pub use spliterator::{SliceSpliterator, Spliterator};

#[cfg(feature = "parallel")]
pub use iterable::{as_parallel, ParallelSlice};

#[cfg(all(test, feature = "parallel"))]
mod stress_tests;
