// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::remove` — multi-line / nested block values (#221, gap 4).
//!
//! Removing an entry whose value spans multiple lines takes the whole
//! entry (key/`-` through its last owned line). The splice is guarded by
//! a re-parse and a typed-value oracle; siblings stay byte-identical.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

// ── Nested block mapping values ─────────────────────────────────────

#[test]
fn remove_nested_mapping_in_the_middle() {
    let mut doc = parse_document("a: 1\nserver:\n  host: x\n  port: 8080\nb: 2\n").unwrap();
    doc.remove("server").unwrap();
    assert_eq!(doc.source(), "a: 1\nb: 2\n");
}

#[test]
fn remove_nested_mapping_at_the_end() {
    let mut doc = parse_document("a: 1\nserver:\n  host: x\n  port: 8080\n").unwrap();
    doc.remove("server").unwrap();
    assert_eq!(doc.source(), "a: 1\n");
}

#[test]
fn remove_first_entry_that_is_nested() {
    let mut doc = parse_document("server:\n  host: x\n  port: 8080\nb: 2\n").unwrap();
    doc.remove("server").unwrap();
    assert_eq!(doc.source(), "b: 2\n");
}

#[test]
fn remove_deeply_nested_entry() {
    let src = "root:\n  mid:\n    leaf: 1\n    other: 2\n  keep: 3\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("root.mid").unwrap();
    assert_eq!(doc.source(), "root:\n  keep: 3\n");
}

// ── Block sequence values under a key ───────────────────────────────

#[test]
fn remove_block_sequence_value() {
    let mut doc = parse_document("a: 1\nlist:\n  - x\n  - y\nb: 2\n").unwrap();
    doc.remove("list").unwrap();
    assert_eq!(doc.source(), "a: 1\nb: 2\n");
}

// ── Block scalar values ─────────────────────────────────────────────

#[test]
fn remove_literal_block_scalar() {
    let mut doc = parse_document("a: 1\ntext: |\n  hello\n  world\nb: 2\n").unwrap();
    doc.remove("text").unwrap();
    assert_eq!(doc.source(), "a: 1\nb: 2\n");
}

// ── Multi-line item in a sequence ───────────────────────────────────

#[test]
fn remove_multiline_sequence_item() {
    let src = "- name: a\n  role: x\n- name: b\n  role: y\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("[0]").unwrap();
    assert_eq!(doc.source(), "- name: b\n  role: y\n");
}

// ── Regressions: single-line still works, refusals still hold ───────

#[test]
fn single_line_removal_unchanged() {
    let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    doc.remove("b").unwrap();
    assert_eq!(doc.source(), "a: 1\nc: 3\n");
}

#[test]
fn sole_nested_entry_still_refused() {
    // Removing the only entry of a mapping is still refused, even when
    // that entry is multi-line.
    let src = "only:\n  a: 1\n  b: 2\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.remove("only").is_err());
    assert_eq!(doc.source(), src);
}

// ── Typed value after a nested removal ──────────────────────────────

#[test]
fn typed_value_after_nested_removal() {
    use noyalib::Value;
    use noyalib::from_str;

    let mut doc = parse_document("a: 1\nserver:\n  host: x\nb: 2\n").unwrap();
    doc.remove("server").unwrap();
    let v: Value = from_str(doc.source()).unwrap();
    let expected: Value = from_str("a: 1\nb: 2\n").unwrap();
    assert_eq!(v, expected);
}
