// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Green-tree path resolution without typed-cache materialization.

use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::path::QuerySegment;
use crate::prelude::*;

// ── Green-tree path resolution (Phase A.3) ──────────────────────────

/// Resolve `segments` against the green tree of `root`, returning
/// the byte range of the value at that path. Walks the structural
/// CST directly — does not consult the typed `Value` / `SpanTree`,
/// so callers that drive many edits via `set` / `set_value` can
/// resolve paths without warming the typed cache between
/// iterations.
///
/// Returns `None` for paths the walker does not yet handle
/// (quoted-key escapes that aren't a simple single-quote-doubling,
/// aliases, merge keys, anchors); the caller is expected to fall
/// back to the typed cache for those cases.
pub(super) fn resolve_path_in_green(
    root: &GreenNode,
    segments: &[QuerySegment],
    source: &str,
) -> Option<(usize, usize)> {
    // The Document root holds collection composites among its
    // children. Find the first one and treat it as the entry
    // point.
    let (collection, base) = first_collection_child(root, 0)?;
    walk_path(collection, segments, base, source)
}

fn first_collection_child(node: &GreenNode, base: usize) -> Option<(&GreenNode, usize)> {
    let mut pos = base;
    for child in node.children() {
        let len = child.text_len();
        if let GreenChild::Node(inner) = child {
            if matches!(
                inner.kind(),
                SyntaxKind::BlockMapping
                    | SyntaxKind::BlockSequence
                    | SyntaxKind::FlowMapping
                    | SyntaxKind::FlowSequence
            ) {
                return Some((inner, pos));
            }
        }
        pos += len;
    }
    None
}

fn walk_path(
    node: &GreenNode,
    segments: &[QuerySegment],
    base: usize,
    source: &str,
) -> Option<(usize, usize)> {
    if segments.is_empty() {
        return Some((base, base + node.text_len()));
    }
    let (head, tail) = segments.split_first()?;
    match (head, node.kind()) {
        (QuerySegment::Key(k), SyntaxKind::BlockMapping | SyntaxKind::FlowMapping) => {
            walk_mapping(node, k, tail, base, source)
        }
        (QuerySegment::Index(i), SyntaxKind::BlockSequence | SyntaxKind::FlowSequence) => {
            walk_sequence(node, *i, tail, base, source)
        }
        // Wildcard / recursive descent / kind mismatch — bail out;
        // the caller falls back to the typed cache.
        _ => None,
    }
}

fn walk_mapping(
    node: &GreenNode,
    key: &str,
    tail: &[QuerySegment],
    base: usize,
    source: &str,
) -> Option<(usize, usize)> {
    // Duplicate keys resolve to the *last* occurrence, matching the
    // typed view: under the default `DuplicateKeyPolicy::Last` (the
    // YAML 1.2 behaviour, and the config `as_value` loads with),
    // `k: one\nk: two` yields `k = "two"`, so the span for `k` must
    // denote `two` — never the bytes of a node the typed view did
    // not select. The whole mapping is scanned before committing.
    //
    // An entry whose key text cannot be decoded here (double-quoted
    // escapes, complex keys) could be a hidden duplicate of `key`,
    // making the green walk inconclusive — bail out and let the
    // caller resolve via the typed cache, which sees every key in
    // decoded form.
    let mut found: Option<(&GreenNode, usize)> = None;
    let mut undecodable_key = false;
    let mut pos = base;
    for child in node.children() {
        let len = child.text_len();
        if let GreenChild::Node(entry) = child {
            if entry.kind() == SyntaxKind::MappingEntry {
                match entry_key_text(entry, source, pos) {
                    Some(entry_key) => {
                        if entry_key == key {
                            found = Some((entry, pos));
                        }
                    }
                    None => undecodable_key = true,
                }
            }
        }
        pos += len;
    }
    if undecodable_key {
        return None;
    }
    let (entry, entry_pos) = found?;
    resolve_value_in_entry(entry, entry_pos, tail, source)
}

fn walk_sequence(
    node: &GreenNode,
    target_index: usize,
    tail: &[QuerySegment],
    base: usize,
    source: &str,
) -> Option<(usize, usize)> {
    let mut pos = base;
    let mut idx = 0usize;
    for child in node.children() {
        let len = child.text_len();
        if let GreenChild::Node(item) = child {
            if item.kind() == SyntaxKind::SequenceItem {
                if idx == target_index {
                    return resolve_value_in_item(item, pos, tail, source);
                }
                idx += 1;
            }
        }
        pos += len;
    }
    None
}

/// Extract the key text of a `MappingEntry`. Supports plain scalar
/// keys verbatim and single-quoted keys with the YAML
/// `''`-doubling escape. Returns `None` for keys whose textual
/// representation differs from the segment string the user would
/// pass — the caller falls back to the typed cache.
fn entry_key_text<'s>(entry: &GreenNode, source: &'s str, base: usize) -> Option<Cow<'s, str>> {
    let mut pos = base;
    for child in entry.children() {
        let child_len = child.text_len();
        match child {
            GreenChild::Token { kind, len } => {
                let start = pos;
                let end = pos + *len as usize;
                match kind {
                    SyntaxKind::QuestionIndicator
                    | SyntaxKind::Whitespace
                    | SyntaxKind::Newline
                    | SyntaxKind::Comment
                    | SyntaxKind::AnchorMark
                    | SyntaxKind::TagMark => {}
                    SyntaxKind::PlainScalar => {
                        return Some(Cow::Borrowed(&source[start..end]));
                    }
                    SyntaxKind::SingleQuotedScalar => {
                        return decode_single_quoted(&source[start..end]);
                    }
                    _ => return None,
                }
            }
            GreenChild::Node(_) => {
                return None;
            }
        }
        pos += child_len;
    }
    None
}

pub(super) fn decode_single_quoted(raw: &str) -> Option<Cow<'_, str>> {
    // Strip surrounding quotes.
    let inner = raw.strip_prefix('\'')?.strip_suffix('\'')?;
    if !inner.contains('\'') {
        return Some(Cow::Borrowed(inner));
    }
    // Replace `''` with `'`. Anything else inside single quotes is
    // taken verbatim.
    Some(Cow::Owned(inner.replace("''", "'")))
}

/// Find the value position inside a `MappingEntry` and either
/// return its byte range (if `tail` is empty) or recurse into it
/// with `tail`.
/// Whether a resolved value node is a block (indentation-structured)
/// collection, whose span begins on its own source line.
fn is_block_collection(k: SyntaxKind) -> bool {
    matches!(k, SyntaxKind::BlockMapping | SyntaxKind::BlockSequence)
}

/// Back `start` up over the inline whitespace that indents a value's first
/// line, but only when that value begins its own line (the whitespace run is
/// preceded by a line break or the start of input). A value that shares its
/// line with a `-` / `:` / `{` (e.g. the inner sequence of `- - a`) is left
/// untouched. This makes a block collection's slice uniformly indented — its
/// first line keeps the indentation the following lines already carry — so it
/// re-parses to the selected value instead of silently re-nesting.
fn extend_to_line_start(source: &str, start: usize) -> usize {
    let b = source.as_bytes();
    let mut i = start;
    while i > 0 && matches!(b[i - 1], b' ' | b'\t') {
        i -= 1;
    }
    if i == 0 || matches!(b[i - 1], b'\n' | b'\r') {
        i
    } else {
        start
    }
}

fn resolve_value_in_entry(
    entry: &GreenNode,
    base: usize,
    tail: &[QuerySegment],
    source: &str,
) -> Option<(usize, usize)> {
    let (value_kind, value_range, value_node) = entry_value(entry, base)?;
    if tail.is_empty() {
        // A block collection's node starts at its first key/item token,
        // leaving its first line's indentation just outside the span; widen
        // to the line start so the slice is uniformly indented.
        let start = if is_block_collection(value_kind) {
            extend_to_line_start(source, value_range.0)
        } else {
            value_range.0
        };
        return Some((start, value_range.1));
    }
    // Recursing further requires the value to be a composite.
    let node = value_node?;
    walk_path(node, tail, value_range.0, source)
}

fn resolve_value_in_item(
    item: &GreenNode,
    base: usize,
    tail: &[QuerySegment],
    source: &str,
) -> Option<(usize, usize)> {
    let (value_kind, value_range, value_node) = item_value(item, base)?;
    if tail.is_empty() {
        let start = if is_block_collection(value_kind) {
            extend_to_line_start(source, value_range.0)
        } else {
            value_range.0
        };
        return Some((start, value_range.1));
    }
    let node = value_node?;
    walk_path(node, tail, value_range.0, source)
}

/// Inside a `MappingEntry`, walk past the key + ColonIndicator and
/// return the first non-trivia "value" child. `value_node` is
/// `Some` if the value is a composite (a nested collection), `None`
/// if it is a leaf scalar.
fn entry_value(
    entry: &GreenNode,
    base: usize,
) -> Option<(SyntaxKind, (usize, usize), Option<&GreenNode>)> {
    let mut pos = base;
    let mut after_colon = false;
    // First-property-token start: when a value is preceded by an
    // [`SyntaxKind::AnchorMark`] / [`SyntaxKind::TagMark`] (or a
    // combination), the conceptual value span covers the entire
    // property prefix plus the scalar / node that follows.
    // Capture that earliest property start here so the returned
    // `(start, end)` stretches across the whole prefixed value.
    let mut prefix_start: Option<usize> = None;
    for child in entry.children() {
        let len = child.text_len();
        let child_start = pos;
        let child_end = pos + len;
        match child {
            GreenChild::Token { kind, .. } => {
                if !after_colon {
                    if *kind == SyntaxKind::ColonIndicator {
                        after_colon = true;
                    }
                } else if *kind == SyntaxKind::AliasMark {
                    // An alias reference (`*name`) is a single token with
                    // no value node of its own; its bytes are a dangling
                    // alias that does not re-parse standalone. Bail so
                    // span_at falls back to the typed cache, whose SpanTree
                    // resolves the alias through to its anchor definition's
                    // self-contained value span.
                    return None;
                } else if is_value_property_kind(*kind) {
                    // `!Tag` / `&anchor` prefix — remember the earliest
                    // start and keep scanning for the scalar that follows.
                    let _ = prefix_start.get_or_insert(child_start);
                } else if *kind == SyntaxKind::DashIndicator {
                    // An indentless block sequence: the green tree keeps
                    // its `-` items as entry-level tokens rather than a
                    // nested BlockSequence node, so this walker cannot
                    // see the sequence's true extent (#375 reported the
                    // resulting indicator-only span). Bail to the typed
                    // cache, whose SpanTree seals block sequences at
                    // their last item.
                    return None;
                } else if !is_trivia_kind(*kind) {
                    let start = prefix_start.unwrap_or(child_start);
                    return Some((*kind, (start, child_end), None));
                }
            }
            GreenChild::Node(inner) => {
                if after_colon {
                    let start = prefix_start.unwrap_or(child_start);
                    return Some((inner.kind(), (start, child_end), Some(inner)));
                }
            }
        }
        pos += len;
    }
    // Fall-through: the entry has a tag/anchor prefix but nothing
    // followed it before EOF — surface the prefix span so callers
    // see a meaningful range rather than `None`.
    prefix_start.map(|start| (SyntaxKind::PlainScalar, (start, pos), None))
}

/// Inside a `SequenceItem`, walk past the DashIndicator and return
/// the first non-trivia "value" child. Mirrors [`entry_value`]'s
/// tag/anchor-prefix handling: the returned span covers any
/// `!Tag` / `&anchor` / `*alias` property tokens **plus** the
/// scalar / node that follows.
fn item_value(
    item: &GreenNode,
    base: usize,
) -> Option<(SyntaxKind, (usize, usize), Option<&GreenNode>)> {
    let mut pos = base;
    let mut after_dash = false;
    let mut prefix_start: Option<usize> = None;
    for child in item.children() {
        let len = child.text_len();
        let child_start = pos;
        let child_end = pos + len;
        match child {
            GreenChild::Token { kind, .. } => {
                if !after_dash {
                    if *kind == SyntaxKind::DashIndicator {
                        after_dash = true;
                    }
                } else if *kind == SyntaxKind::AliasMark {
                    // Alias reference as a sequence item: bail to the typed
                    // cache, which resolves it to the anchor's value span.
                    return None;
                } else if is_value_property_kind(*kind) {
                    let _ = prefix_start.get_or_insert(child_start);
                } else if !is_trivia_kind(*kind) {
                    let start = prefix_start.unwrap_or(child_start);
                    return Some((*kind, (start, child_end), None));
                }
            }
            GreenChild::Node(inner) => {
                if after_dash {
                    let start = prefix_start.unwrap_or(child_start);
                    return Some((inner.kind(), (start, child_end), Some(inner)));
                }
            }
        }
        pos += len;
    }
    prefix_start.map(|start| (SyntaxKind::PlainScalar, (start, pos), None))
}

fn is_trivia_kind(k: SyntaxKind) -> bool {
    matches!(
        k,
        SyntaxKind::Whitespace
            | SyntaxKind::Newline
            | SyntaxKind::Comment
            | SyntaxKind::Bom
            | SyntaxKind::Directive
    )
}

/// Tokens that are part of a YAML *value* by attaching properties
/// (anchor, alias, tag) but are not themselves the value content.
/// The CST span resolver treats these as a *prefix* of the value
/// span — `entry_value` / `item_value` stretch their returned
/// `(start, end)` to cover the prefix plus the scalar / node that
/// follows, so `Document::span_at("name")` on
/// `name: !Custom 'app-1'` returns `6..21` (covering both the
/// tag and the quoted scalar) rather than `6..13` (the tag
/// alone, which was the pre-fix behaviour).
fn is_value_property_kind(k: SyntaxKind) -> bool {
    // Alias marks are handled separately (they bail the green walk to the
    // typed cache); only anchor/tag definition prefixes stretch the value
    // span to cover the property plus the scalar / node that follows.
    matches!(k, SyntaxKind::AnchorMark | SyntaxKind::TagMark)
}
