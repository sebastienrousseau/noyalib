// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::grow_value`: the scalar-to-collection re-shape that
//! `set_value` refuses, as its own opt-in route.
//!
//! Every successful case asserts the exact bytes, so placement — block
//! at one indent step past the key, in place at the scalar's own
//! column, flow inside flow — is pinned, not just the loaded value.
//! Every refusal leaves the source byte-identical.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

fn seq(items: &[&str]) -> Value {
    Value::Sequence(items.iter().map(|i| Value::from(*i)).collect())
}

fn map(entries: &[(&str, i64)]) -> Value {
    Value::Mapping(
        entries
            .iter()
            .map(|(k, v)| ((*k).to_string(), Value::from(*v)))
            .collect(),
    )
}

#[test]
fn a_scalar_on_its_keys_line_grows_a_block_sequence() {
    let mut doc = parse_document("tags: solo\nname: x\n").unwrap();
    doc.grow_value("tags", &seq(&["a", "b"])).unwrap();
    assert_eq!(doc.to_string(), "tags:\n  - a\n  - b\nname: x\n");
}

#[test]
fn a_scalar_on_its_keys_line_grows_a_block_mapping() {
    let mut doc = parse_document("m: x\nafter: 1\n").unwrap();
    doc.grow_value("m", &map(&[("a", 1), ("b", 2)])).unwrap();
    assert_eq!(doc.to_string(), "m:\n  a: 1\n  b: 2\nafter: 1\n");
}

#[test]
fn a_nested_key_grows_at_its_own_indent_and_keeps_siblings() {
    let mut doc = parse_document("top:\n  k: v\n  z: 2\n").unwrap();
    doc.grow_value("top.k", &seq(&["a"])).unwrap();
    assert_eq!(doc.to_string(), "top:\n  k:\n    - a\n  z: 2\n");
}

#[test]
fn a_trailing_comment_stays_on_the_keys_line() {
    let mut doc = parse_document("tags: solo # keep\nname: x\n").unwrap();
    doc.grow_value("tags", &seq(&["a", "b"])).unwrap();
    assert_eq!(doc.to_string(), "tags: # keep\n  - a\n  - b\nname: x\n");
}

#[test]
fn an_implicit_null_grows_in_place() {
    let mut doc = parse_document("k:\nx: 1\n").unwrap();
    doc.grow_value("k", &seq(&["a"])).unwrap();
    assert_eq!(doc.to_string(), "k:\n  - a\nx: 1\n");
}

#[test]
fn a_scalar_on_its_own_line_is_replaced_at_its_column() {
    let mut doc = parse_document("k:\n  solo\nx: 1\n").unwrap();
    doc.grow_value("k", &seq(&["a", "b"])).unwrap();
    assert_eq!(doc.to_string(), "k:\n  - a\n  - b\nx: 1\n");
}

#[test]
fn a_sequence_items_scalar_grows_on_the_dash_line() {
    let mut doc = parse_document("xs:\n  - solo\n  - 2\n").unwrap();
    doc.grow_value("xs[0]", &map(&[("a", 1), ("b", 2)]))
        .unwrap();
    assert_eq!(doc.to_string(), "xs:\n  - a: 1\n    b: 2\n  - 2\n");
}

#[test]
fn a_key_on_a_dash_line_grows_past_the_key_not_the_dash() {
    let mut doc = parse_document("xs:\n  - k: solo\n    z: 2\n").unwrap();
    doc.grow_value("xs[0].k", &seq(&["a"])).unwrap();
    assert_eq!(doc.to_string(), "xs:\n  - k:\n      - a\n    z: 2\n");
}

#[test]
fn a_scalar_inside_a_flow_collection_grows_flow() {
    let mut doc = parse_document("m: {a: x, b: 2}\n").unwrap();
    let grown = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    doc.grow_value("m.a", &grown).unwrap();
    assert_eq!(doc.to_string(), "m: {a: [1, 2], b: 2}\n");
}

#[test]
fn a_multi_line_block_scalar_is_replaced_whole() {
    let mut doc = parse_document("k: |-\n  one\n  two\nx: 1\n").unwrap();
    doc.grow_value("k", &seq(&["a"])).unwrap();
    assert_eq!(doc.to_string(), "k:\n  - a\nx: 1\n");
}

#[test]
fn a_crlf_document_stays_crlf() {
    let mut doc = parse_document("tags: solo\r\nname: x\r\n").unwrap();
    doc.grow_value("tags", &seq(&["a", "b"])).unwrap();
    assert_eq!(doc.to_string(), "tags:\r\n  - a\r\n  - b\r\nname: x\r\n");
}

#[test]
fn a_collection_already_there_delegates_to_the_328_arm() {
    // Flow stays flow, block stays block: the existing collection
    // replacement, reachable through grow_value so one call serves
    // whatever is at the path.
    let mut flow = parse_document("tags: [a, b]\n").unwrap();
    flow.grow_value("tags", &seq(&["a", "c"])).unwrap();
    assert_eq!(flow.to_string(), "tags: [a, c]\n");

    let mut block = parse_document("tags:\n  - a\n  - b\n").unwrap();
    block.grow_value("tags", &seq(&["a", "c"])).unwrap();
    assert_eq!(block.to_string(), "tags:\n  - a\n  - c\n");
}

#[test]
fn an_equal_value_is_a_byte_level_no_op() {
    let src = "tags: [a, 'b']\n";
    let mut doc = parse_document(src).unwrap();
    doc.grow_value("tags", &seq(&["a", "b"])).unwrap();
    assert_eq!(doc.to_string(), src);
}

#[test]
fn the_loaded_value_matches_what_was_written() {
    // The oracle's promise, asserted from the caller's side: strings
    // that need quoting and nested collections load back exactly.
    let value = Value::Mapping(
        [
            ("plain".to_string(), Value::from("word")),
            ("tricky".to_string(), Value::from("yes")),
            ("nested".to_string(), seq(&["x", "y: z"])),
        ]
        .into_iter()
        .collect(),
    );
    let mut doc = parse_document("k: solo\n").unwrap();
    doc.grow_value("k", &value).unwrap();
    let loaded = doc.as_value();
    assert_eq!(loaded["k"], value, "{doc}");
}

#[test]
fn a_scalar_value_is_refused_toward_set_value() {
    let src = "k: solo\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.grow_value("k", &Value::from(5)).unwrap_err();
    assert!(err.to_string().contains("set_value"), "{err}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn an_alias_token_value_is_refused() {
    let src = "a: &x 1\nk: *x\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.grow_value("k", &seq(&["a"])).unwrap_err();
    assert!(err.to_string().contains("alias"), "{err}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn a_property_led_scalar_is_refused() {
    for src in ["k: &an solo\nj: *an\n", "k: !t solo\n"] {
        let mut doc = parse_document(src).unwrap();
        let err = doc.grow_value("k", &seq(&["a"])).unwrap_err();
        assert!(
            err.to_string().contains("anchor or tag property"),
            "{src:?}: {err}"
        );
        assert_eq!(doc.to_string(), src);
    }
}

#[test]
fn a_scalar_inside_a_shared_anchor_is_refused() {
    let src = "a: &x\n  k: 1\nb: *x\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.grow_value("a.k", &seq(&["a"])).unwrap_err();
    assert!(err.to_string().contains("grow_value"), "{err}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn a_missing_path_is_refused() {
    let src = "k: 1\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.grow_value("absent", &seq(&["a"])).unwrap_err();
    assert!(err.to_string().contains("path not found"), "{err}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn an_implicit_sequence_item_is_refused_as_unsupported_layout() {
    // A bare `-` has no value byte for a block collection to start on;
    // nobody renders a usable result there (see #425 for the comment
    // API's version of the same judgement).
    let src = "xs:\n  -\n  - 2\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.grow_value("xs[0]", &seq(&["a"])).unwrap_err();
    assert!(err.to_string().contains("layout"), "{err}");
    assert_eq!(doc.to_string(), src);
}
