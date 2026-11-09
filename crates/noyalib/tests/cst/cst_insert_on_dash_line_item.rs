// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Inserting a sibling entry into a sequence-item mapping whose only
//! key sits on the `- ` line (issue #499).
//!
//! `column_of_key_at` took the key's column from the line's leading
//! spaces, which on `  - name: web` is the dash's column, not the
//! key's. The new entry was spliced where the parser expects another
//! `- ` item (or, indentless, at the parent mapping's level, caught by
//! the integrity oracle). The key's column now counts past `- `
//! indicators, so the sibling lands under the key.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

#[test]
fn a_sole_key_on_the_dash_line_takes_a_sibling() {
    // The issue's lead reproduction, through both insertion mutators.
    let mut doc = parse_document("servers:\n  - name: web\n").unwrap();
    doc.insert_entry_value("servers[0]", "port", &Value::from(80i64))
        .unwrap();
    assert_eq!(doc.to_string(), "servers:\n  - name: web\n    port: 80\n");

    let mut doc = parse_document("servers:\n  - name: web\n").unwrap();
    doc.insert_entry("servers[0]", "port", "80").unwrap();
    assert_eq!(doc.to_string(), "servers:\n  - name: web\n    port: 80\n");
}

#[test]
fn an_indentless_item_takes_a_sibling() {
    let mut doc = parse_document("servers:\n- name: web\n").unwrap();
    doc.insert_entry_value("servers[0]", "port", &Value::from(80i64))
        .unwrap();
    assert_eq!(doc.to_string(), "servers:\n- name: web\n  port: 80\n");
}

#[test]
fn a_root_sequence_item_takes_a_sibling() {
    let mut doc = parse_document("- name: web\n").unwrap();
    doc.insert_entry_value("[0]", "port", &Value::from(80i64))
        .unwrap();
    assert_eq!(doc.to_string(), "- name: web\n  port: 80\n");
}

#[test]
fn nested_dash_indicators_all_count() {
    // `- - key: v`: both indicators own their columns; the sibling
    // lands under the key, two indicators in.
    let mut doc = parse_document("- - name: web\n").unwrap();
    doc.insert_entry_value("[0][0]", "port", &Value::from(80i64))
        .unwrap();
    assert_eq!(doc.to_string(), "- - name: web\n    port: 80\n");
}

#[test]
fn a_nested_block_anchor_on_a_dash_line_item_still_places_the_sibling() {
    // The anchor entry's value occupies its own lines, so the key
    // column comes from the nested-block walk; the key's line is the
    // `- ` line.
    let mut doc = parse_document("servers:\n  - cfg:\n      a: 1\n").unwrap();
    doc.insert_entry_value("servers[0]", "port", &Value::from(80i64))
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "servers:\n  - cfg:\n      a: 1\n    port: 80\n"
    );
}

#[test]
fn an_item_with_a_second_key_is_unchanged_by_the_fix() {
    // This shape always worked: the anchor is the key on its own line,
    // whose leading-space count is the key's column. Pinned so the fix
    // cannot move it.
    let mut doc = parse_document("servers:\n  - name: web\n    host: h\n").unwrap();
    doc.insert_entry_value("servers[0]", "port", &Value::from(80i64))
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "servers:\n  - name: web\n    host: h\n    port: 80\n"
    );
}

#[test]
fn a_plain_block_mapping_is_unchanged_by_the_fix() {
    let mut doc = parse_document("a:\n  name: web\n").unwrap();
    doc.insert_entry_value("a", "port", &Value::from(80i64))
        .unwrap();
    assert_eq!(doc.to_string(), "a:\n  name: web\n  port: 80\n");
}

#[test]
fn the_inserted_entry_loads_back() {
    // The load-back oracle's promise, asserted from the caller's side
    // for the dash-line shape.
    let mut doc = parse_document("servers:\n  - name: web\n").unwrap();
    doc.insert_entry_value("servers[0]", "port", &Value::from(80i64))
        .unwrap();
    let v = doc.as_value();
    assert_eq!(v["servers"][0]["port"], Value::from(80i64));
    assert_eq!(v["servers"][0]["name"], Value::from("web"));
}

#[test]
fn rewriting_the_dash_line_key_with_a_multi_line_value_indents_for_the_key() {
    // The existing-key arm of the same anchor: a block literal written
    // over `name` must indent its body past the key's column, not the
    // dash's.
    let mut doc = parse_document("servers:\n  - name: web\n").unwrap();
    doc.insert_entry_value("servers[0]", "name", &Value::from("a\nb"))
        .unwrap();
    let v = doc.as_value();
    assert_eq!(v["servers"][0]["name"], Value::from("a\nb"), "{doc}");
}

#[test]
fn a_dash_line_insert_preserves_crlf() {
    let src = "servers:\r\n  - name: web\r\n";
    let expected = "servers:\r\n  - name: web\r\n    port: 80\r\n";
    let mut typed = parse_document(src).unwrap();
    typed
        .insert_entry_value("servers[0]", "port", &Value::from(80i64))
        .unwrap();
    assert_eq!(typed.source(), expected);

    let mut fragment = parse_document(src).unwrap();
    fragment.insert_entry("servers[0]", "port", "80").unwrap();
    assert_eq!(fragment.source(), expected);
}
