// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Regression: children of a block collection whose entry carries an
//! `&anchor` or `!tag` property must report their own spans. The green
//! walker stretches the entry's value span to cover the property
//! prefix, which is right for the entry itself, but it also recursed
//! into the nested collection with that prefix start as the base, so
//! every child span came back shifted by the width of the property and
//! the trivia after it. A shifted sequence-item span was returned with
//! confidence (the sequence walk is arithmetic); a shifted mapping
//! walk misread its key bytes and fell back to the typed cache, hiding
//! the same defect behind a slower path.

use noyalib::cst::parse_document;

#[test]
fn items_of_an_anchored_indented_sequence_report_their_own_spans() {
    let src = "a: &x\n  - 1\n  - 2\nb: *x\n";
    let doc = parse_document(src).unwrap();
    let (s, e) = doc.span_at("a[0]").unwrap();
    assert_eq!(&src[s..e], "1");
    let (s, e) = doc.span_at("a[1]").unwrap();
    assert_eq!(&src[s..e], "2");
}

#[test]
fn entries_of_an_anchored_block_mapping_report_their_own_spans() {
    let src = "a: &x\n  k: 1\n  z: 2\nb: *x\n";
    let doc = parse_document(src).unwrap();
    let (s, e) = doc.span_at("a.k").unwrap();
    assert_eq!(&src[s..e], "1");
    let (s, e) = doc.span_at("a.z").unwrap();
    assert_eq!(&src[s..e], "2");
}

#[test]
fn the_anchored_entry_itself_still_covers_the_property_prefix() {
    // The entry's own span deliberately includes the anchor property;
    // only the recursion into the nested value was off.
    let src = "a: &x\n  - 1\n  - 2\nb: *x\n";
    let doc = parse_document(src).unwrap();
    let (s, e) = doc.span_at("a").unwrap();
    assert!(src[s..e].starts_with("&x"), "span: {:?}", &src[s..e]);
}

#[test]
fn items_under_a_tagged_key_report_their_own_spans() {
    let src = "a: !Set\n  - 1\n  - 2\n";
    let doc = parse_document(src).unwrap();
    let (s, e) = doc.span_at("a[0]").unwrap();
    assert_eq!(&src[s..e], "1");
}

#[test]
fn an_anchored_sequence_item_with_a_nested_mapping_reports_entry_spans() {
    let src = "xs:\n  - &x\n    k: 1\nb: *x\n";
    let doc = parse_document(src).unwrap();
    let (s, e) = doc.span_at("xs[0].k").unwrap();
    assert_eq!(&src[s..e], "1");
}

#[test]
fn an_item_span_drives_a_correct_edit_under_an_anchored_key() {
    // The span feeds every mutator; the edit is what a wrong span
    // silently corrupts. The anchor here has no alias, so the
    // shared-value policy does not apply and the write must land on
    // the item's own bytes.
    use noyalib::Value;
    let src = "a: &x\n  - 1\n  - 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("a[0]", &Value::from(9)).unwrap();
    assert_eq!(doc.source(), "a: &x\n  - 9\n  - 2\n");
}
