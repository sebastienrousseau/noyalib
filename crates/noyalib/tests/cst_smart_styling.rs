// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Neighbour-aware scalar styling for `Document::set_value`.
//!
//! When the target site is currently emitted plain (no quoting
//! intent to preserve) and a sibling style dominates the
//! surrounding `BlockMapping`, the new value should adopt the
//! neighbours' style.

use noyalib::Value;
use noyalib::cst::parse_document;

#[test]
fn single_quoted_neighbours_drive_single_quoted_emit() {
    let src = "a: 'one'\nb: 'two'\nc: 0.0.1\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("c", &Value::String("three".into())).unwrap();
    assert!(
        doc.to_string().contains("c: 'three'"),
        "expected 'three' single-quoted, got: {}",
        doc
    );
}

#[test]
fn double_quoted_neighbours_drive_double_quoted_emit() {
    let src = "a: \"one\"\nb: \"two\"\nc: foo\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("c", &Value::String("three".into())).unwrap();
    assert!(
        doc.to_string().contains("c: \"three\""),
        "expected double-quoted, got: {}",
        doc
    );
}

#[test]
fn plain_dominant_neighbourhood_keeps_plain() {
    let src = "a: one\nb: two\nc: three\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("c", &Value::String("four".into())).unwrap();
    // Plain dominant → plain emit (no quoting added).
    assert!(
        doc.to_string().contains("c: four\n"),
        "expected plain, got: {}",
        doc
    );
}

#[test]
fn mixed_neighbourhood_falls_back_to_plain_when_safe() {
    // 1 single-quoted vs 1 double-quoted vs 1 plain — no single
    // style hits the (>=2 AND strict plurality) threshold.
    let src = "a: 'one'\nb: \"two\"\nc: three\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("c", &Value::String("four".into())).unwrap();
    assert!(
        doc.to_string().contains("c: four\n"),
        "ambiguous neighbourhood should keep plain, got: {}",
        doc
    );
}

#[test]
fn explicit_quoted_site_is_preserved_regardless_of_neighbours() {
    // Site is single-quoted but every other sibling is double-quoted.
    // The site's existing intent wins — we don't reformat what the
    // user already chose.
    let src = "a: \"one\"\nb: \"two\"\nc: 'three'\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("c", &Value::String("four".into())).unwrap();
    assert!(
        doc.to_string().contains("c: 'four'"),
        "expected single-quoted preserved, got: {}",
        doc
    );
}

#[test]
fn unsafe_string_in_single_quoted_neighbourhood_uses_single_quoted() {
    // The replacement string contains characters that are unsafe
    // plain (`:` followed by space). With single-quoted neighbours
    // the result should be single-quoted, not double-quoted.
    let src = "a: 'one'\nb: 'two'\nc: foo\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("c", &Value::String("hello: world".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(
        out.contains("c: 'hello: world'"),
        "expected single-quoted with unsafe chars, got: {out}",
    );
}

#[test]
fn neighbour_only_applies_when_target_is_plain() {
    // The site here is *single-quoted* and neighbours are
    // double-quoted. Site wins.
    let src = "a: \"one\"\nb: \"two\"\nc: 'three'\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("c", &Value::String("four".into())).unwrap();
    assert!(
        doc.to_string().contains("c: 'four'"),
        "site style wins over neighbours, got: {}",
        doc
    );
}

#[test]
fn neighbour_lookup_ignores_nested_collection_values() {
    // Nested collection siblings are ignored — only scalar siblings
    // count toward the dominant style.
    let src = "\
inner:
  - x
a: 'one'
b: 'two'
c: foo
";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("c", &Value::String("three".into())).unwrap();
    assert!(
        doc.to_string().contains("c: 'three'"),
        "expected single-quoted via plurality of scalar siblings, got: {}",
        doc
    );
}
