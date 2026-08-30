// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::set_value` writing a multi-line string beside a trailing
//! comment (issue #333).
//!
//! A multi-line string becomes a literal block scalar, which owns every
//! byte through the end of its last content line. The comment that
//! trailed the old one-line value used to land there and become part of
//! the value. It now moves onto the block scalar's header line.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

fn s(v: &str) -> Value {
    Value::String(v.into())
}

#[test]
fn the_trailing_comment_moves_onto_the_header_line() {
    let mut doc = parse_document("title: Hello # trailing\nnext: 1\n").unwrap();
    doc.set_value("title", &s("multi\nline")).unwrap();
    assert_eq!(
        doc.to_string(),
        "title: |- # trailing\n  multi\n  line\nnext: 1\n"
    );
    assert_eq!(doc.as_value()["title"].as_str(), Some("multi\nline"));
    assert_eq!(doc.as_value()["next"].as_i64(), Some(1));
}

#[test]
fn a_keep_newline_literal_hoists_the_comment_too() {
    let mut doc = parse_document("title: Hello # trailing\n").unwrap();
    doc.set_value("title", &s("trailing\n")).unwrap();
    assert_eq!(doc.to_string(), "title: | # trailing\n  trailing\n");
    assert_eq!(doc.as_value()["title"].as_str(), Some("trailing\n"));
}

#[test]
fn sequence_items_and_nested_entries_behave_the_same() {
    let mut doc = parse_document("tags:\n  - a # first\n  - b\n").unwrap();
    doc.set_value("tags[0]", &s("multi\nline")).unwrap();
    assert_eq!(
        doc.to_string(),
        "tags:\n  - |- # first\n    multi\n    line\n  - b\n"
    );
    assert_eq!(doc.as_value()["tags"][0].as_str(), Some("multi\nline"));
    assert_eq!(doc.as_value()["tags"][1].as_str(), Some("b"));

    let mut doc = parse_document("menu:\n  order: 3 # why\n  visible: true\n").unwrap();
    doc.set_value("menu.order", &s("a\n\nb")).unwrap();
    assert_eq!(
        doc.to_string(),
        "menu:\n  order: |- # why\n    a\n    \n    b\n  visible: true\n"
    );
    assert_eq!(doc.as_value()["menu"]["order"].as_str(), Some("a\n\nb"));
}

#[test]
fn an_implicit_null_with_a_comment_is_filled_in_under_the_comment() {
    let mut doc = parse_document("a: # note\nb: 2\n").unwrap();
    doc.set_value("a", &s("x\ny")).unwrap();
    assert_eq!(doc.to_string(), "a: |- # note\n  x\n  y\nb: 2\n");
    assert_eq!(doc.as_value()["a"].as_str(), Some("x\ny"));
}

#[test]
fn without_a_comment_nothing_changes() {
    let mut doc = parse_document("title: Hello\n").unwrap();
    doc.set_value("title", &s("multi\nline")).unwrap();
    assert_eq!(doc.to_string(), "title: |-\n  multi\n  line\n");
}

#[test]
fn a_one_line_value_keeps_the_comment_after_the_value() {
    let mut doc = parse_document("title: Hello # trailing\n").unwrap();
    doc.set_value("title", &s("x")).unwrap();
    assert_eq!(doc.to_string(), "title: x # trailing\n");
    doc.set_value("title", &s("NO")).unwrap();
    assert_eq!(doc.to_string(), "title: \"NO\" # trailing\n");
}

#[test]
fn a_hash_inside_the_new_value_is_not_a_comment() {
    let mut doc = parse_document("title: Hello # trailing\n").unwrap();
    doc.set_value("title", &s("a #b\nc")).unwrap();
    assert_eq!(doc.to_string(), "title: |- # trailing\n  a #b\n  c\n");
    assert_eq!(doc.as_value()["title"].as_str(), Some("a #b\nc"));
}
