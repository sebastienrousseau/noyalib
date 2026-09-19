// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::push_back` and `Document::insert_after` for block
//! sequences — entry manipulation that gets indentation and the
//! `-` indicator right.

use noyalib::cst::parse_document;

#[test]
fn push_back_appends_with_existing_indent() {
    let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    doc.push_back("items", "three").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n  - three\n");
    assert_eq!(doc.as_value()["items"][2].as_str(), Some("three"));
}

#[test]
fn push_back_at_top_level_zero_indent() {
    let mut doc = parse_document("- one\n- two\n").unwrap();
    doc.push_back("", "three").unwrap();
    assert_eq!(doc.to_string(), "- one\n- two\n- three\n");
}

#[test]
fn push_back_into_nested_sequence() {
    let mut doc = parse_document("matrix:\n  inner:\n    - a\n    - b\n").unwrap();
    doc.push_back("matrix.inner", "c").unwrap();
    assert_eq!(
        doc.to_string(),
        "matrix:\n  inner:\n    - a\n    - b\n    - c\n"
    );
}

#[test]
fn push_back_preserves_following_siblings() {
    let mut doc = parse_document("items:\n  - one\n  - two\nnext: 1\n").unwrap();
    doc.push_back("items", "three").unwrap();
    assert_eq!(
        doc.to_string(),
        "items:\n  - one\n  - two\n  - three\nnext: 1\n"
    );
    assert_eq!(doc.as_value()["next"].as_i64(), Some(1));
}

#[test]
fn push_back_quoted_value_stays_quoted() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    doc.push_back("items", "\"two\"").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - \"two\"\n");
    assert_eq!(doc.as_value()["items"][1].as_str(), Some("two"));
}

#[test]
fn push_back_rejects_non_sequence_path() {
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    let err = doc.push_back("a", "x").unwrap_err();
    assert!(format!("{err}").contains("not a sequence"));
}

#[test]
fn push_back_rejects_empty_sequence() {
    let mut doc = parse_document("items: []\n").unwrap();
    let err = doc.push_back("items", "x").unwrap_err();
    assert!(format!("{err}").contains("empty sequence"));
}

#[test]
fn insert_after_inserts_in_the_middle() {
    let mut doc = parse_document("items:\n  - one\n  - three\n").unwrap();
    doc.insert_after("items[0]", "two").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n  - three\n");
}

#[test]
fn insert_after_at_top_level() {
    let mut doc = parse_document("- a\n- c\n").unwrap();
    doc.insert_after("[0]", "b").unwrap();
    assert_eq!(doc.to_string(), "- a\n- b\n- c\n");
}

#[test]
fn insert_after_at_last_index_is_equivalent_to_push_back() {
    let mut a = parse_document("items:\n  - one\n  - two\n").unwrap();
    let mut b = parse_document("items:\n  - one\n  - two\n").unwrap();
    a.push_back("items", "three").unwrap();
    b.insert_after("items[1]", "three").unwrap();
    assert_eq!(a.to_string(), b.to_string());
}

#[test]
fn insert_after_rejects_non_index_path() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.insert_after("a", "x").unwrap_err();
    assert!(format!("{err}").contains("must end with a sequence index"));
}

#[test]
fn push_back_supplies_the_line_break_a_terminator_less_file_lacks() {
    // `end_of_line` returns the end of the source when the last line
    // has no `\n`; splicing there without a break of our own would
    // produce `- one  - two` on one line.
    let mut doc = parse_document("items:\n  - one").unwrap();
    doc.push_back("items", "two").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n");
}

#[test]
fn insert_after_supplies_the_line_break_too() {
    let mut doc = parse_document("- a\n- c").unwrap();
    doc.insert_after("[1]", "d").unwrap();
    assert_eq!(doc.to_string(), "- a\n- c\n- d\n");
}

#[test]
fn insert_entry_supplies_the_line_break_too() {
    let mut doc = parse_document("m:\n  a: 1").unwrap();
    doc.insert_entry("m", "b", "2").unwrap();
    assert_eq!(doc.to_string(), "m:\n  a: 1\n  b: 2\n");
}
