// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! # Deprecated (v0.3)
//!
//! The `stream` free functions predate the [`RichIterator`] extension trait and
//! are superseded by it (and by `std`'s own iterator constructors). They are
//! kept as deprecated shims for one release.
//!
//! | `stream` fn | Replacement |
//! |---|---|
//! | `collectors::group_by` | [`RichIterator::group_by`] (returns a crate `OpenHashMap`) |
//! | `collectors::group_by_each` | [`RichIterator::group_by_each`] |
//! | `collectors::partition` | [`RichIterator::partition_into`] |
//! | `collectors::joining` | [`RichIterator::join_display`] / `slice.join` |
//! | `collectors::to_map_by` | `iter.map(\|v\| (k(v), val(v))).collect()` |
//! | `collectors::sum_by` | [`Iterator::map`] + [`Iterator::sum`] |
//! | `collectors::min_by`/`max_by` | [`Iterator::min_by`]/[`Iterator::max_by`] |
//! | `collectors::chunked` | `itertools::chunks`, or collect + `slice::chunks` |
//! | `generators::range`/`range_closed` | `a..b` / `a..=b` |
//! | `generators::iterate` | [`std::iter::successors`] |
//! | `generators::generate` | [`std::iter::from_fn`] |
//! | `generators::repeat` | [`std::iter::repeat_n`] |
//! | `generators::of` | `vec.into_iter()` |
//! | `generators::empty` | [`std::iter::empty`] |
//!
//! [`RichIterator`]: crate::rich_iterator::RichIterator
//! [`RichIterator::group_by`]: crate::rich_iterator::RichIterator::group_by
//! [`RichIterator::group_by_each`]: crate::rich_iterator::RichIterator::group_by_each
//! [`RichIterator::partition_into`]: crate::rich_iterator::RichIterator::partition_into
//! [`RichIterator::join_display`]: crate::rich_iterator::RichIterator::join_display

pub mod collectors;
pub mod generators;
