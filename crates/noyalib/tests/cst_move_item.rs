// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::move_item` — move a block-sequence item to a new index.
//!
//! Built on `swap_items`, so only item value bytes move and the whole
//! move is atomic: a refused step leaves the document byte-identical.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

// ── Happy paths ─────────────────────────────────────────────────────

#[test]
fn move_forward() {
    let mut doc = parse_document("- a\n- b\n- c\n- d\n").unwrap();
    doc.move_item("", 0, 2).unwrap();
    assert_eq!(doc.source(), "- b\n- c\n- a\n- d\n");
}

#[test]
fn move_backward() {
    let mut doc = parse_document("- a\n- b\n- c\n- d\n").unwrap();
    doc.move_item("", 3, 1).unwrap();
    assert_eq!(doc.source(), "- a\n- d\n- b\n- c\n");
}

#[test]
fn move_to_first() {
    let mut doc = parse_document("- a\n- b\n- c\n").unwrap();
    doc.move_item("", 2, 0).unwrap();
    assert_eq!(doc.source(), "- c\n- a\n- b\n");
}

#[test]
fn move_to_last() {
    let mut doc = parse_document("- a\n- b\n- c\n").unwrap();
    doc.move_item("", 0, 2).unwrap();
    assert_eq!(doc.source(), "- b\n- c\n- a\n");
}

#[test]
fn move_adjacent_equals_swap() {
    let mut doc = parse_document("- a\n- b\n- c\n").unwrap();
    doc.move_item("", 1, 2).unwrap();
    assert_eq!(doc.source(), "- a\n- c\n- b\n");
}

#[test]
fn move_in_nested_sequence() {
    let mut doc = parse_document("items:\n  - a\n  - b\n  - c\n").unwrap();
    doc.move_item("items", 2, 0).unwrap();
    assert_eq!(doc.source(), "items:\n  - c\n  - a\n  - b\n");
}

#[test]
fn move_preserves_other_items_verbatim() {
    let mut doc = parse_document("- keep0\n- move\n- keep2\n- keep3\n").unwrap();
    doc.move_item("", 1, 3).unwrap();
    assert_eq!(doc.source(), "- keep0\n- keep2\n- keep3\n- move\n");
}

// ── No-ops ──────────────────────────────────────────────────────────

#[test]
fn move_to_same_index_is_noop() {
    let src = "- a\n- b\n- c\n";
    let mut doc = parse_document(src).unwrap();
    doc.move_item("", 1, 1).unwrap();
    assert_eq!(doc.source(), src);
}

// ── Refusals (document left untouched) ──────────────────────────────

#[test]
fn out_of_bounds_is_refused() {
    let src = "- a\n- b\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.move_item("", 0, 9).is_err());
    assert!(doc.move_item("", 5, 0).is_err());
    assert_eq!(doc.source(), src);
}

#[test]
fn non_sequence_path_is_refused() {
    let src = "a: 1\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.move_item("", 0, 0).is_err());
    assert_eq!(doc.source(), src);
}

// ── Typed value after move ──────────────────────────────────────────

#[test]
fn typed_value_reflects_the_move() {
    use noyalib::Value;
    use noyalib::from_str;

    let mut doc = parse_document("- 1\n- 2\n- 3\n- 4\n").unwrap();
    doc.move_item("", 0, 3).unwrap();
    let v: Value = from_str(doc.source()).unwrap();
    let expected: Value = from_str("- 2\n- 3\n- 4\n- 1\n").unwrap();
    assert_eq!(v, expected);
}
