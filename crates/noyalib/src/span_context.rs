//! Thread-local span context for wiring source locations into `Spanned<T>`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::error::Location;
use crate::prelude::*;
#[cfg(not(feature = "std"))]
use alloc::rc::Rc;
use core::cell::OnceCell;
#[cfg(feature = "std")]
use core::cell::RefCell;
#[cfg(feature = "std")]
use std::rc::Rc;

use crate::prelude::FxHashMap;

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
        items: Vec<Self>,
    },
    /// A mapping with its own span and per-entry (key-span, value-tree) pairs.
    Mapping {
        start: usize,
        end: usize,
        entries: Vec<((usize, usize), Self)>,
    },
    /// An alias reference (`*name`) resolved *through* to its anchor's
    /// definition tree (issue #149): the wrapped tree is the anchor's, so a
    /// read here yields the anchor's value span. The wrapper records that the
    /// resolution passed through an alias — a *write* would splice the
    /// anchor's bytes (a different key), which `Document::set` must refuse.
    Alias(Box<Self>),
}

/// Holds the span map and source string for the current deserialization.
#[derive(Debug)]
pub struct SpanContext {
    /// Maps `&Value` pointer address → `(start_byte, end_byte)`.
    pub spans: FxHashMap<usize, (usize, usize)>,
    /// The original source string (for `Location::from_index`).
    pub source: Arc<str>,
    /// Line and character index over `source`, built on first use and
    /// shared by every context over the same source.
    lines: SharedLineIndex,
}

/// A lazily built [`LineIndex`], shareable between the span contexts of
/// one multi-document source so the index is built once per source.
pub(crate) type SharedLineIndex = Rc<OnceCell<LineIndex>>;

impl SpanContext {
    /// A context over `source` with its own line index. Contexts are
    /// only built by the span-tracking `std` loaders.
    #[cfg(feature = "std")]
    pub(crate) fn new(spans: FxHashMap<usize, (usize, usize)>, source: Arc<str>) -> Self {
        Self::with_lines(spans, source, SharedLineIndex::default())
    }

    /// A context whose line index is shared with other contexts over
    /// the same `source`.
    #[cfg(feature = "std")]
    pub(crate) fn with_lines(
        spans: FxHashMap<usize, (usize, usize)>,
        source: Arc<str>,
        lines: SharedLineIndex,
    ) -> Self {
        Self {
            spans,
            source,
            lines,
        }
    }

    /// The [`Location`] of byte `index` in the source; equal to
    /// [`Location::from_index`] but answered from the index, so each
    /// call costs a binary search instead of a scan from byte 0.
    pub(crate) fn location(&self, index: usize) -> Location {
        self.lines
            .get_or_init(|| LineIndex::new(&self.source))
            .location(self.source.as_bytes(), index)
    }
}

/// Byte positions of every line start, plus a running character count
/// sampled every [`LineIndex::STRIDE`] bytes. Together they answer a
/// line/column query in `O(log lines + STRIDE)`.
#[derive(Debug)]
pub(crate) struct LineIndex {
    line_starts: Vec<usize>,
    /// `chars_at[k]` is the number of characters in the first
    /// `k * STRIDE` bytes of the source.
    chars_at: Vec<usize>,
}

impl LineIndex {
    /// Sampling interval of the character count. Small, so a column on
    /// a long line costs a short scan; the samples take one `usize` per
    /// `STRIDE` bytes of source.
    const STRIDE: usize = 128;

    pub(crate) fn new(source: &str) -> Self {
        let bytes = source.as_bytes();
        let mut line_starts = vec![0];
        line_starts.extend(
            bytes
                .iter()
                .enumerate()
                .filter(|&(_, &b)| b == b'\n')
                .map(|(i, _)| i + 1),
        );
        let mut chars_at = Vec::with_capacity(bytes.len() / Self::STRIDE + 2);
        let mut chars = 0;
        chars_at.push(0);
        for chunk in bytes.chunks(Self::STRIDE) {
            chars += count_chars(chunk);
            chars_at.push(chars);
        }
        Self {
            line_starts,
            chars_at,
        }
    }

    /// Characters wholly or partly in `bytes[..pos]`, `pos` clamped to
    /// the source length.
    fn chars_before(&self, bytes: &[u8], pos: usize) -> usize {
        let pos = pos.min(bytes.len());
        let block = pos / Self::STRIDE;
        self.chars_at[block] + count_chars(&bytes[block * Self::STRIDE..pos])
    }

    /// Same result as [`Location::from_index`] over the indexed source.
    pub(crate) fn location(&self, bytes: &[u8], index: usize) -> Location {
        // Line starts at or before `index`: one per newline before it,
        // plus the start of the source.
        let line = self.line_starts.partition_point(|&start| start <= index);
        let line_start = self.line_starts[line - 1];
        let end = index.min(bytes.len());
        let column = if end - line_start <= Self::STRIDE {
            1 + count_chars(&bytes[line_start..end])
        } else {
            1 + self.chars_before(bytes, end) - self.chars_before(bytes, line_start)
        };
        Location::new(line, column, index)
    }
}

/// Characters beginning in `bytes`: every byte that is not a UTF-8
/// continuation byte.
fn count_chars(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| b & 0xC0 != 0x80).count()
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
    use core::cell::Cell;

    thread_local! {
        pub(super) static SPAN_CONTEXT: RefCell<Option<SpanContext>> = const { RefCell::new(None) };
        /// Address of the `Value` node whose typed rejection
        /// `Deserializer::wrap_err` most recently located (refs #353).
        /// `from_str` reads it after a failed deserialize to derive the
        /// field path (`server.port`) by walking the root value; a
        /// success consumes and discards it, so a rejection swallowed
        /// mid-parse (untagged-enum probing) cannot leak into the next
        /// document's error.
        pub(super) static ERROR_NODE: Cell<Option<usize>> = const { Cell::new(None) };
    }
}

/// Record the address of the node whose error `wrap_err` just wrapped.
#[cfg(feature = "std")]
pub(crate) fn record_error_node(addr: usize) {
    tls::ERROR_NODE.with(|cell| cell.set(Some(addr)));
}

/// Take (and clear) the most recently recorded failing-node address.
#[cfg(feature = "std")]
pub(crate) fn take_error_node() -> Option<usize> {
    tls::ERROR_NODE.with(core::cell::Cell::take)
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
        // A rejection created and then swallowed during this parse
        // (untagged-enum probing that ultimately succeeded) must not
        // survive into the next parse's error handling.
        tls::ERROR_NODE.with(|cell| cell.set(None));
    }
}

/// Install a thread-local span context. Returns an RAII guard that owns
/// the context and clears the thread-local on drop. The thread-local
/// stores only the source `Arc<str>` (the hot lookup path consults the
/// guard's `SpanContext` directly, avoiding a map clone per parse).
#[cfg(feature = "std")]
pub(crate) fn set_span_context(ctx: SpanContext) -> SpanContextGuard {
    let thread_local_ctx = SpanContext::with_lines(
        FxHashMap::default(),
        Arc::clone(&ctx.source),
        Rc::clone(&ctx.lines),
    );
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

#[cfg(all(test, feature = "std"))]
mod line_index_tests {
    use super::*;

    #[test]
    fn line_index_matches_from_index_everywhere() {
        let filler = "x".repeat(LineIndex::STRIDE + 7);
        let sources = [
            String::new(),
            "a".to_string(),
            "a: 1\nb: 2\n".to_string(),
            "\n\n\nx".to_string(),
            "é: ü\n  - 日本\r\nend 🦀\n".to_string(),
            format!("k: {filler}é\n{filler}\n🦀{filler}"),
        ];
        for source in &sources {
            let index = LineIndex::new(source);
            for i in 0..=source.len() + 2 {
                assert_eq!(
                    index.location(source.as_bytes(), i),
                    Location::from_index(source, i),
                    "source {source:?} index {i}"
                );
            }
        }
    }

    #[test]
    fn contexts_share_one_index() {
        let source: Arc<str> = "a: 1\n".into();
        let lines = SharedLineIndex::default();
        let first = SpanContext::with_lines(FxHashMap::default(), source.clone(), lines.clone());
        let second = SpanContext::with_lines(FxHashMap::default(), source, lines.clone());
        assert_eq!(first.location(3).line(), 1);
        assert!(lines.get().is_some());
        assert_eq!(second.location(5).line(), 2);
    }
}
