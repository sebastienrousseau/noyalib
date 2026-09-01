// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `set_value` with a `Sequence` / `Mapping`, in the target node's
//! style (#328, ADR-0010).
//!
//! The issue's own test list, pinned: `[a, b]` → `[a, c]` keeps flow;
//! a block sequence stays block; a nested mapping value inside a
//! block mapping; a mapping value inside a flow mapping; byte-faithful
//! outside the touched span. Plus the refusals: a scalar target, and a
//! target inside an anchored value (#338's policy).

#![cfg(feature = "std")]

use noyalib::cst::parse_document;
use noyalib::{Value, from_str};

fn set(src: &str, path: &str, v: &str) -> Result<String, (noyalib::Error, String)> {
    let val: Value = from_str(v).unwrap();
    let mut doc = parse_document(src).unwrap();
    match doc.set_value(path, &val) {
        Ok(()) => Ok(doc.to_string()),
        Err(e) => Err((e, doc.to_string())),
    }
}

#[test]
fn flow_sequence_stays_flow() {
    let out = set("tags: [a, b]\nname: x\n", "tags", "[a, c]").unwrap();
    assert_eq!(out, "tags: [a, c]\nname: x\n");
}

#[test]
fn block_sequence_stays_block_at_its_column() {
    let out = set("tags:\n  - a\n  - b\nname: x\n", "tags", "[a, c]").unwrap();
    assert_eq!(out, "tags:\n  - a\n  - c\nname: x\n");
}

#[test]
fn indentless_block_sequence_keeps_its_shape() {
    let out = set("tags:\n- a\n- b\n", "tags", "[a, c, d]").unwrap();
    assert_eq!(out, "tags:\n- a\n- c\n- d\n");
}

#[test]
fn flow_mapping_stays_flow() {
    let out = set("menu: {x: 1}\n", "menu", "{x: 2, y: 3}").unwrap();
    assert_eq!(out, "menu: {x: 2, y: 3}\n");
}

#[test]
fn block_mapping_stays_block() {
    let out = set("menu:\n  x: 1\nname: z\n", "menu", "{x: 2, y: 3}").unwrap();
    assert_eq!(out, "menu:\n  x: 2\n  y: 3\nname: z\n");
}

#[test]
fn nested_mapping_value_inside_a_block_mapping() {
    let out = set("a:\n  b:\n    c: 1\n", "a.b", "{c: 2, d: 3}").unwrap();
    assert_eq!(out, "a:\n  b:\n    c: 2\n    d: 3\n");
}

#[test]
fn mapping_value_inside_a_flow_mapping() {
    let out = set("a: {b: {c: 1}}\n", "a.b", "{c: 2}").unwrap();
    assert_eq!(out, "a: {b: {c: 2}}\n");
}

#[test]
fn sequence_item_holding_a_collection() {
    let out = set("items:\n  - [1, 2]\n", "items[0]", "[3]").unwrap();
    assert_eq!(out, "items:\n  - [3]\n");
}

#[test]
fn deeper_nesting_with_mixed_shapes() {
    let out = set(
        "cfg:\n  servers:\n    - h: a\n",
        "cfg.servers",
        "[{h: b, p: 1}, {h: c}]",
    )
    .unwrap();
    assert_eq!(
        out,
        "cfg:\n  servers:\n    - h: b\n      p: 1\n    - h: c\n"
    );
    let loaded: Value = from_str(&out).unwrap();
    assert_eq!(loaded["cfg"]["servers"][1]["h"].as_str(), Some("c"));
}

#[test]
fn shape_change_mapping_to_sequence_in_block() {
    let out = set("m:\n  x: 1\n", "m", "[1, 2]").unwrap();
    assert_eq!(out, "m:\n  - 1\n  - 2\n");
}

#[test]
fn multiline_flow_target_collapses_to_one_line() {
    let out = set("tags: [a,\n  b]\n", "tags", "[c]").unwrap();
    assert_eq!(out, "tags: [c]\n");
}

#[test]
fn bytes_outside_the_touched_span_are_untouched() {
    let out = set(
        "# header\nbefore: 1\ntags:\n  - a\nname: x  # keep\n",
        "tags",
        "[b, c]",
    )
    .unwrap();
    assert_eq!(
        out,
        "# header\nbefore: 1\ntags:\n  - b\n  - c\nname: x  # keep\n"
    );
}

#[test]
fn equal_collection_is_a_byte_noop() {
    let src = "tags: [a, b]\n";
    assert_eq!(set(src, "tags", "[a, b]").unwrap(), src);
}

#[test]
fn scalar_target_still_refuses() {
    let src = "tags: plain\n";
    let (err, out) = set(src, "tags", "[a]").unwrap_err();
    assert_eq!(out, src);
    assert!(err.to_string().contains("collection"), "got: {err}");
}

#[test]
fn alias_valued_target_refuses() {
    let src = "base: &b [1]\nother: *b\n";
    let (err, out) = set(src, "other", "[2]").unwrap_err();
    assert_eq!(out, src);
    assert!(err.to_string().contains("alias"), "got: {err}");
}

#[test]
fn anchored_target_refuses_with_materialise_guidance() {
    let src = "base: &b [1]\nother: *b\n";
    let (err, out) = set(src, "base", "[2]").unwrap_err();
    assert_eq!(out, src);
    assert!(
        err.to_string().contains("materialise_aliases_of"),
        "got: {err}"
    );
}
