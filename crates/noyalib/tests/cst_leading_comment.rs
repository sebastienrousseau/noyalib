// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::set_leading_comment` / `remove_leading_comment` — mutation
//! of the leading comment block above a single-line mapping key.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

// ── set ─────────────────────────────────────────────────────────────

#[test]
fn set_adds_leading_comment_when_none() {
    let mut doc = parse_document("port: 8080\n").unwrap();
    doc.set_leading_comment("port", "the listen port").unwrap();
    assert_eq!(doc.source(), "# the listen port\nport: 8080\n");
}

#[test]
fn set_multi_line_block() {
    let mut doc = parse_document("port: 8080\n").unwrap();
    doc.set_leading_comment("port", "line one\nline two")
        .unwrap();
    assert_eq!(doc.source(), "# line one\n# line two\nport: 8080\n");
}

#[test]
fn set_replaces_existing_block() {
    let mut doc = parse_document("# old\nport: 8080\n").unwrap();
    doc.set_leading_comment("port", "new").unwrap();
    assert_eq!(doc.source(), "# new\nport: 8080\n");
}

#[test]
fn set_replaces_multi_line_existing_block() {
    let mut doc = parse_document("# a\n# b\nport: 8080\n").unwrap();
    doc.set_leading_comment("port", "single").unwrap();
    assert_eq!(doc.source(), "# single\nport: 8080\n");
}

#[test]
fn set_empty_text_writes_bare_hash() {
    let mut doc = parse_document("port: 8080\n").unwrap();
    doc.set_leading_comment("port", "").unwrap();
    assert_eq!(doc.source(), "#\nport: 8080\n");
}

#[test]
fn set_on_nested_single_line_key_matches_indent() {
    let mut doc = parse_document("server:\n  host: localhost\n").unwrap();
    doc.set_leading_comment("server.host", "loopback").unwrap();
    assert_eq!(doc.source(), "server:\n  # loopback\n  host: localhost\n");
}

#[test]
fn set_preserves_siblings() {
    let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    doc.set_leading_comment("b", "middle").unwrap();
    assert_eq!(doc.source(), "a: 1\n# middle\nb: 2\nc: 3\n");
}

// ── set: refusals ───────────────────────────────────────────────────

#[test]
fn set_rejects_multi_line_entry() {
    let src = "server:\n  host: x\n  port: 8080\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.set_leading_comment("server", "nope").is_err());
    assert_eq!(doc.source(), src);
}

#[test]
fn set_rejects_missing_path() {
    let src = "port: 8080\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.set_leading_comment("nope", "x").is_err());
    assert_eq!(doc.source(), src);
}

// ── remove ──────────────────────────────────────────────────────────

#[test]
fn remove_leading_block() {
    let mut doc = parse_document("# noise\n# more\nport: 8080\n").unwrap();
    doc.remove_leading_comment("port").unwrap();
    assert_eq!(doc.source(), "port: 8080\n");
}

#[test]
fn remove_keeps_inline_comment() {
    let mut doc = parse_document("# lead\nport: 8080  # inline\n").unwrap();
    doc.remove_leading_comment("port").unwrap();
    assert_eq!(doc.source(), "port: 8080  # inline\n");
}

#[test]
fn remove_when_none_is_noop() {
    let src = "port: 8080\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove_leading_comment("port").unwrap();
    assert_eq!(doc.source(), src);
}

#[test]
fn remove_on_unsupported_path_is_noop() {
    let src = "server:\n  host: x\n  port: 8080\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove_leading_comment("server").unwrap(); // multi-line entry
    assert_eq!(doc.source(), src);
}

// ── round-trip + typed value ────────────────────────────────────────

#[test]
fn set_then_read_back_via_comments_at() {
    let mut doc = parse_document("port: 8080\n").unwrap();
    doc.set_leading_comment("port", "hello").unwrap();
    let before = doc.comments_at("port").before;
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].text, " hello");
}

#[test]
fn value_unchanged_by_leading_comment_edits() {
    use noyalib::Value;
    use noyalib::from_str;

    let before: Value = from_str("a: 1\nb: 2\n").unwrap();
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    doc.set_leading_comment("a", "x").unwrap();
    doc.remove_leading_comment("a").unwrap();
    let after: Value = from_str(doc.source()).unwrap();
    assert_eq!(before, after);
}

#[test]
fn set_on_a_sequence_item_is_refused() {
    // A sequence item is not a mapping key, so it owns no leading block
    // this method addresses — `set` refuses, `remove` is a no-op.
    let src = "- a\n- b\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.set_leading_comment("[0]", "x").is_err());
    assert_eq!(doc.source(), src);
    doc.remove_leading_comment("[0]").unwrap();
    assert_eq!(doc.source(), src);
}
