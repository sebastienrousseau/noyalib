// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::swap_items` — exchange two items of a block sequence.
//!
//! Each test parses a document, swaps two items, and checks the result
//! is byte-identical to the expected output. Each item's **whole entry**
//! moves — its own lines and the head-comment run above them, the range
//! `remove` deletes — while every other item and the surrounding
//! structure stay verbatim. Refusal tests assert the document is left
//! untouched.

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
fn inline_comment_travels_with_its_item() {
    // This test previously asserted the opposite — that only the value
    // bytes move, so a comment annotates the *slot* rather than the
    // item. It was changed deliberately, not incidentally, so the
    // reasoning is recorded here rather than in a commit message.
    //
    // `remove` already decides this question the other way for the same
    // bytes: an entry owns the comment run directly above it
    // (`owned_entry_range`), because leaving it behind "silently becomes
    // documentation for the *next* entry". Two mutators in one crate
    // cannot hold opposite views of who a comment belongs to. Reorder
    // now matches remove.
    let mut doc = parse_document("- a  # first\n- b  # second\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- b  # second\n- a  # first\n");
}

#[test]
fn head_comment_travels_with_its_item() {
    // The unambiguous half: `# about one` documents `one`, not slot 0.
    let mut doc = parse_document("# about one\n- one\n# about two\n- two\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "# about two\n- two\n# about one\n- one\n");
}

#[test]
fn a_multi_line_head_comment_run_travels_whole() {
    let mut doc = parse_document("# one, line 1\n# one, line 2\n- one\n- two\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- two\n# one, line 1\n# one, line 2\n- one\n");
}

#[test]
fn a_comment_between_items_belongs_to_the_one_below_it() {
    // Same rule `owned_value_end` applies for `remove`: trailing
    // comment lines are not the previous entry's, so this run travels
    // with `two`.
    let mut doc = parse_document("- one\n# doc two\n- two\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "# doc two\n- two\n- one\n");
}

#[test]
fn a_blank_detached_comment_stays_put() {
    // A blank line detaches the run, so a document header is not swept
    // into the first item — the property `absorb_head_comments` exists
    // to hold.
    let mut doc = parse_document("# header\n\n- one\n- two\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "# header\n\n- two\n- one\n");
}

#[test]
fn only_one_item_carrying_a_comment_still_swaps_cleanly() {
    let mut doc = parse_document("- one  # first\n- two\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- two\n- one  # first\n");
}

#[test]
fn multi_line_items_exchange_whole_entries() {
    let mut doc = parse_document("- a: 1\n  b: 2\n- c: 3\n  d: 4\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- c: 3\n  d: 4\n- a: 1\n  b: 2\n");
}

#[test]
fn a_crlf_document_keeps_crlf() {
    let mut doc = parse_document("- one  # first\r\n- two  # second\r\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- two  # second\r\n- one  # first\r\n");
}

#[test]
fn a_missing_final_newline_is_not_invented_or_lost() {
    // Each position keeps its own terminator while the bodies move. Get
    // this wrong and the two lines splice into one (`- b- a`).
    let mut doc = parse_document("- a\n- b").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "- b\n- a");
}

#[test]
fn swap_in_a_nested_sequence_carries_the_comment_and_indent() {
    let mut doc = parse_document("items:\n  # doc a\n  - a\n  - b\n").unwrap();
    doc.swap_items("items", 0, 1).unwrap();
    assert_eq!(doc.source(), "items:\n  - b\n  # doc a\n  - a\n");
}

#[test]
fn a_flow_sequence_keeps_the_value_span_exchange() {
    // Flow members share a line with each other and with the brackets,
    // so there is no per-item line to move. Unchanged behaviour.
    let mut doc = parse_document("[one, two, three]\n").unwrap();
    doc.swap_items("", 0, 2).unwrap();
    assert_eq!(doc.source(), "[three, two, one]\n");
}

#[test]
fn item_indentation_travels_with_the_item() {
    // The doc comment used to claim this case was refused. It was not —
    // it silently swapped values and left each item's own spacing
    // behind. Now the entry moves whole.
    let mut doc = parse_document("- a\n-   b\n").unwrap();
    doc.swap_items("", 0, 1).unwrap();
    assert_eq!(doc.source(), "-   b\n- a\n");
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
