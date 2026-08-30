// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::remove` on entries that share their line with a sequence
//! `-` indicator, and on implicit-null items (issue #336).
//!
//! The first key of a `- name: x` item mapping lives on the indicator's
//! line; deleting the whole line took the indicator -- and the item --
//! with it, so the removal was refused. Now only the entry's own bytes
//! go and the following key moves up beside the indicator. An
//! implicit-null item (`-` alone) owns no bytes at all; its `-` line is
//! now found and removed.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

#[test]
fn removing_the_first_key_of_a_sequence_item_mapping() {
    let src = "items:\n  - name: x\n    v: 1\n  - name: y\n    v: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("items[0].name").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - v: 1\n  - name: y\n    v: 2\n");

    let mut doc = parse_document(src).unwrap();
    doc.remove("items[1].name").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - name: x\n    v: 1\n  - v: 2\n");
}

#[test]
fn a_multi_line_first_value_goes_with_its_key() {
    let src = "items:\n  - note: |\n      a\n      b\n    v: 1\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("items[0].note").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - v: 1\n");
}

#[test]
fn later_keys_on_their_own_lines_are_unaffected() {
    let src = "items:\n  - name: x\n    v: 1\n    w: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("items[0].v").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - name: x\n    w: 2\n");
}

#[test]
fn an_implicit_null_item_is_removed_with_its_dash_line() {
    let mut doc = parse_document("tags:\n  -\n  - x\n").unwrap();
    doc.remove("tags[0]").unwrap();
    assert_eq!(doc.to_string(), "tags:\n  - x\n");

    let mut doc = parse_document("tags:\n  - x\n  -\n").unwrap();
    doc.remove("tags[1]").unwrap();
    assert_eq!(doc.to_string(), "tags:\n  - x\n");
}

#[test]
fn a_sequence_nested_in_a_sequence_keeps_the_outer_dash() {
    let src = "nested:\n  - - a\n    - b\n  - - c\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("nested[0][0]").unwrap();
    assert_eq!(doc.to_string(), "nested:\n  - - b\n  - - c\n");
    assert_eq!(doc.as_value()["nested"][0][0].as_str(), Some("b"));
}

#[test]
fn the_sole_key_of_a_sequence_item_mapping_empties_the_mapping() {
    let mut doc = parse_document("items:\n  - name: x\n  - name: y\n").unwrap();
    doc.remove("items[0].name").unwrap();
    let v = doc.as_value().clone();
    assert_eq!(v["items"].as_sequence().map(Vec::len), Some(2));
    assert!(
        v["items"][0]
            .as_mapping()
            .is_some_and(noyalib::Mapping::is_empty)
            || v["items"][0].is_null(),
        "first item must be an emptied mapping, got {:?}",
        v["items"][0]
    );
    assert_eq!(v["items"][1]["name"].as_str(), Some("y"));
}

#[test]
fn refusals_still_leave_the_document_unchanged() {
    let src = "items:\n  - name: x\n    v: 1\n";
    let mut doc = parse_document(src).unwrap();
    assert!(doc.remove("items[0].missing").is_err());
    assert!(doc.remove("items[9]").is_err());
    assert_eq!(doc.to_string(), src);
}
