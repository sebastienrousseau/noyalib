// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::set_inline_comment` / `remove_inline_comment` — first-class
//! mutation of the trailing `#` comment on a single-line node.
//!
//! Each test parses a document, edits the inline comment, and checks the
//! byte-exact result. The typed value must never change; refusals leave
//! the document untouched.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

// ── set: happy paths ────────────────────────────────────────────────

#[test]
fn set_adds_comment_when_none() {
    let mut doc = parse_document("port: 8080\n").unwrap();
    doc.set_inline_comment("port", "the listen port").unwrap();
    assert_eq!(doc.source(), "port: 8080  # the listen port\n");
}

#[test]
fn set_replaces_existing_comment_keeping_spacing() {
    let mut doc = parse_document("port: 8080  # old\n").unwrap();
    doc.set_inline_comment("port", "new").unwrap();
    assert_eq!(doc.source(), "port: 8080  # new\n");
}

#[test]
fn set_replaces_single_space_separated_comment() {
    let mut doc = parse_document("port: 8080 # old\n").unwrap();
    doc.set_inline_comment("port", "new").unwrap();
    // The existing single-space separation is preserved.
    assert_eq!(doc.source(), "port: 8080 # new\n");
}

#[test]
fn set_empty_text_writes_bare_hash() {
    let mut doc = parse_document("port: 8080\n").unwrap();
    doc.set_inline_comment("port", "").unwrap();
    assert_eq!(doc.source(), "port: 8080  #\n");
}

#[test]
fn set_on_nested_key() {
    let mut doc = parse_document("server:\n  host: localhost\n").unwrap();
    doc.set_inline_comment("server.host", "loopback").unwrap();
    assert_eq!(doc.source(), "server:\n  host: localhost  # loopback\n");
}

#[test]
fn set_on_a_sequence_item() {
    let mut doc = parse_document("- a\n- b\n").unwrap();
    doc.set_inline_comment("[1]", "second").unwrap();
    assert_eq!(doc.source(), "- a\n- b  # second\n");
}

#[test]
fn set_leaves_siblings_and_value_untouched() {
    let mut doc = parse_document("a: 1\nb: 2  # keep\nc: 3\n").unwrap();
    doc.set_inline_comment("a", "added").unwrap();
    assert_eq!(doc.source(), "a: 1  # added\nb: 2  # keep\nc: 3\n");
}

// ── set: refusals ───────────────────────────────────────────────────

#[test]
fn set_rejects_multiline_node() {
    let src = "server:\n  host: localhost\n  port: 8080\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.set_inline_comment("server", "nope").is_err());
    assert_eq!(doc.source(), src);
}

#[test]
fn set_rejects_newline_in_text() {
    let src = "port: 8080\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.set_inline_comment("port", "line1\nline2").is_err());
    assert_eq!(doc.source(), src);
}

#[test]
fn set_rejects_missing_path() {
    let src = "port: 8080\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.set_inline_comment("nope", "x").is_err());
    assert_eq!(doc.source(), src);
}

// ── remove ──────────────────────────────────────────────────────────

#[test]
fn remove_existing_comment() {
    let mut doc = parse_document("port: 8080  # noise\n").unwrap();
    doc.remove_inline_comment("port").unwrap();
    assert_eq!(doc.source(), "port: 8080\n");
}

#[test]
fn remove_when_none_is_noop() {
    let src = "port: 8080\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove_inline_comment("port").unwrap();
    assert_eq!(doc.source(), src);
}

#[test]
fn remove_missing_path_is_noop() {
    let src = "port: 8080\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove_inline_comment("nope").unwrap();
    assert_eq!(doc.source(), src);
}

#[test]
fn remove_keeps_leading_comment() {
    let mut doc = parse_document("# above\nport: 8080  # inline\n").unwrap();
    doc.remove_inline_comment("port").unwrap();
    assert_eq!(doc.source(), "# above\nport: 8080\n");
}

// ── round-trip via comments_at + typed value ────────────────────────

#[test]
fn set_then_read_back_via_comments_at() {
    let mut doc = parse_document("port: 8080\n").unwrap();
    doc.set_inline_comment("port", "hello").unwrap();
    assert_eq!(doc.comments_at("port").inline.unwrap().text, " hello");
}

#[test]
fn value_is_unchanged_by_comment_edits() {
    use noyalib::Value;
    use noyalib::from_str;

    let before: Value = from_str("a: 1\nb: 2\n").unwrap();
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    doc.set_inline_comment("a", "x").unwrap();
    doc.remove_inline_comment("a").unwrap();
    let after: Value = from_str(doc.source()).unwrap();
    assert_eq!(before, after);
}
