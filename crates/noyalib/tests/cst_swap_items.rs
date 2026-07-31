// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::swap_items` — exchange two items of a block sequence.
//!
//! Each test parses a document, swaps two items, and checks the result
//! is byte-identical to the expected output — only the two items' value
//! bytes move; the `- ` indicators, indentation, and every other item
//! stay verbatim. Refusal tests assert the document is left untouched.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

// ── Happy paths ─────────────────────────────────────────────────────

#[test]
fn swap_root_sequence_ends() {
    let mut doc = parse_document("- a\n- b\n- c\n").unwrap();
    doc.swap_items("", 0, 2).unwrap();
    assert_eq!(doc.source(), "- c\n- b\n- a\n");
}

#[test]
fn swap_is_order_independent() {
    let mut doc = parse_document("- a\n- b\n- c\n").unwrap();
    doc.swap_items("", 2, 0).unwrap();
    assert_eq!(doc.source(), "- c\n- b\n- a\n");
}

#[test]
fn swap_adjacent_items() {
    let mut doc = parse_document("- a\n- b\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- b\n- a\n");
}

#[test]
fn swap_nested_sequence_under_a_key() {
    let mut doc = parse_document("items:\n  - a\n  - b\n  - c\n").unwrap();
    doc.swap_items("items", 0, 2).unwrap();
    assert_eq!(doc.source(), "items:\n  - c\n  - b\n  - a\n");
}

#[test]
fn swap_preserves_inline_comment_position() {
    // Only the value bytes move; the comment annotates the slot.
    let mut doc = parse_document("- a  # first\n- b  # second\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- b  # first\n- a  # second\n");
}

#[test]
fn swap_items_of_different_widths() {
    let mut doc = parse_document("- short\n- a_much_longer_value\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- a_much_longer_value\n- short\n");
}

#[test]
fn swap_quoted_and_plain() {
    let mut doc = parse_document("- \"q: v\"\n- plain\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- plain\n- \"q: v\"\n");
}

// ── No-ops ──────────────────────────────────────────────────────────

#[test]
fn swap_index_with_itself_is_noop() {
    let src = "- a\n- b\n";
    let mut doc = parse_document(src).unwrap();
    doc.swap_items("", 1, 1).unwrap();
    assert_eq!(doc.source(), src);
}

#[test]
fn swap_equal_values_is_byte_preserving() {
    let src = "- x\n- x\n";
    let mut doc = parse_document(src).unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), src);
}

// ── Refusals (document left untouched) ──────────────────────────────

#[test]
fn out_of_bounds_is_refused() {
    let src = "- a\n- b\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.swap_items("", 0, 5).is_err());
    assert_eq!(doc.source(), src);
    assert!(doc.swap_items("", 9, 0).is_err());
    assert_eq!(doc.source(), src);
}

#[test]
fn path_not_a_sequence_is_refused() {
    let src = "a: 1\nb: 2\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.swap_items("", 0, 1).is_err()); // root is a mapping
    assert!(doc.swap_items("a", 0, 1).is_err()); // scalar
    assert_eq!(doc.source(), src);
}

#[test]
fn missing_path_is_refused() {
    let src = "items:\n  - a\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.swap_items("nope", 0, 0).is_err());
    assert_eq!(doc.source(), src);
}

// ── Typed value after swap ──────────────────────────────────────────

#[test]
fn typed_value_reflects_the_swap() {
    use noyalib::Value;
    use noyalib::from_str;

    let mut doc = parse_document("- 1\n- 2\n- 3\n").unwrap();
    doc.swap_items("", 0, 2).unwrap();
    let v: Value = from_str(doc.source()).unwrap();
    let expected: Value = from_str("- 3\n- 2\n- 1\n").unwrap();
    assert_eq!(v, expected);
}
