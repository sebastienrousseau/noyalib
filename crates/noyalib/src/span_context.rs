//! Thread-local span context for wiring source locations into `Spanned<T>`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
#[cfg(feature = "std")]
use core::cell::RefCell;

use rustc_hash::FxHashMap;

#[cfg(feature = "std")]
use crate::value::Value;

/// Parallel tree of source spans, built alongside `Value` during loading.
///
/// Only built on the `std` path; `no_std` builds use the span-free
/// loader (`load_one_no_spans` / `load_all_no_spans`) and so never
/// instantiate this enum.
#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub(crate) enum SpanTree {
    /// A leaf node (scalar, alias, null).
    Leaf(usize, usize),
    /// A sequence with its own span and per-element span trees.
    Sequence {
        start: usize,
        end: usize,
        items: Vec<SpanTree>,
    },
    /// A mapping with its own span and per-entry (key-span, value-tree) pairs.
    Mapping {
        start: usize,
        end: usize,
        entries: Vec<((usize, usize), SpanTree)>,
    },
    /// An alias reference (`*name`) resolved *through* to its anchor's
    /// definition tree (issue #149): the wrapped tree is the anchor's, so a
    /// read here yields the anchor's value span. The wrapper records that the
    /// resolution passed through an alias — a *write* would splice the
    /// anchor's bytes (a different key), which `Document::set` must refuse.
    Alias(Box<SpanTree>),
}

/// Holds the span map and source string for the current deserialization.
#[derive(Debug)]
pub struct SpanContext {
    /// Maps `&Value` pointer address → `(start_byte, end_byte)`.
    pub spans: FxHashMap<usize, (usize, usize)>,
    /// The original source string (for `Location::from_index`).
    pub source: Arc<str>,
}

// Thread-local storage requires `std::thread` and is unavailable under
// `#![no_std]`. The TLS-backed `SpanContextGuard` and `set_span_context`
// helpers wire `Spanned<T>` deserialization via a shared context, so
// they are only compiled with the `std` feature. The `SpanTree` data
// structure and `build_span_map` walker above are alloc-only and stay
// available everywhere.
#[cfg(feature = "std")]
mod tls {
    use super::{RefCell, SpanContext};

    thread_local! {
        pub(super) static SPAN_CONTEXT: RefCell<Option<SpanContext>> = const { RefCell::new(None) };
    }
}

/// RAII guard that owns the span context and clears the thread-local on
/// drop. Holding the guard keeps the context alive so callers can borrow
/// it via [`SpanContextGuard::as_ref`] without cloning the span map.
#[cfg(feature = "std")]
pub(crate) struct SpanContextGuard {
    ctx: SpanContext,
}

#[cfg(feature = "std")]
impl SpanContextGuard {
    pub(crate) fn as_ref(&self) -> &SpanContext {
        &self.ctx
    }
}

#[cfg(feature = "std")]
impl Drop for SpanContextGuard {
    fn drop(&mut self) {
        tls::SPAN_CONTEXT.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }
}

/// Install a thread-local span context. Returns an RAII guard that owns
/// the context and clears the thread-local on drop. The thread-local
/// stores only the source `Arc<str>` (the hot lookup path consults the
/// guard's `SpanContext` directly, avoiding a map clone per parse).
#[cfg(feature = "std")]
pub(crate) fn set_span_context(ctx: SpanContext) -> SpanContextGuard {
    let thread_local_ctx = SpanContext {
        spans: FxHashMap::default(),
        source: Arc::clone(&ctx.source),
    };
    tls::SPAN_CONTEXT.with(|cell| {
        *cell.borrow_mut() = Some(thread_local_ctx);
    });
    SpanContextGuard { ctx }
}

/// Walk a `Value` tree and a `SpanTree` in lockstep, collecting pointer → span
/// mappings.
#[cfg(feature = "std")]
pub(crate) fn build_span_map(value: &Value, tree: &SpanTree) -> FxHashMap<usize, (usize, usize)> {
    let mut map = FxHashMap::default();
    walk(value, tree, &mut map);
    map
}

#[cfg(feature = "std")]
fn walk(value: &Value, tree: &SpanTree, map: &mut FxHashMap<usize, (usize, usize)>) {
    // Walk the tree in DFS order to build the pointer → span map.
    let p: *const Value = value;
    let ptr = p as usize;
    match tree {
        SpanTree::Leaf(start, end) => {
            let _ = map.insert(ptr, (*start, *end));
        }
        SpanTree::Sequence { start, end, items } => {
            let _ = map.insert(ptr, (*start, *end));
            if let Value::Sequence(seq) = value {
                for (v, t) in seq.iter().zip(items.iter()) {
                    walk(v, t, map);
                }
            }
        }
        SpanTree::Mapping {
            start,
            end,
            entries,
        } => {
            let _ = map.insert(ptr, (*start, *end));
            if let Value::Mapping(mapping) = value {
                // Mapping entries are in insertion order (IndexMap), matching SpanTree order.
                for ((_, v), (_, vt)) in mapping.iter().zip(entries.iter()) {
                    walk(v, vt, map);
                }
            }
        }
        // An alias site maps to the anchor's spans; walk through it.
        SpanTree::Alias(inner) => walk(value, inner, map),
    }
}
