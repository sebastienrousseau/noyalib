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
//!
//! The other direction lives here too, since it is the same boundary: a
//! scalar written *over* a collection. It goes where the collection's
//! content sat, one indent step past the key when the collection sat at
//! the key's own column.

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

// ── A scalar over a collection (the other direction) ────────────────
//
// The resolver widens a block collection's span to its first line so a
// read slice is uniformly indented. A scalar spliced over that span
// used to land at the line start, which is the key's own column: this
// parser reads `k:` / `5` back, and PyYAML and libyaml reject it.

#[test]
fn a_scalar_over_a_block_mapping_stays_indented_past_its_key() {
    let out = set("k:\n  a: 1\nafter: 1\n", "k", "5").unwrap();
    assert_eq!(out, "k:\n  5\nafter: 1\n");
    let doc = parse_document(&out).unwrap();
    assert_eq!(doc.as_value().get_path("k"), Some(&Value::from(5i64)));
}

#[test]
fn a_scalar_over_a_block_sequence() {
    let out = set("k:\n  - 1\n  - 2\nafter: 1\n", "k", "5").unwrap();
    assert_eq!(out, "k:\n  5\nafter: 1\n");
}

#[test]
fn an_indentless_block_sequence_gets_a_column_that_clears_its_key() {
    // `on:` / `- push` is allowed to sit at its key's column; what
    // replaces it is not. This is the rule `sole_entry_range` already
    // applies when `remove` empties a sole entry, which is why that path
    // yields `on:` / `  []` (see `cst_remove_flow_and_sole.rs`).
    let out = set("on:\n- push\njobs: {}\n", "on", "5").unwrap();
    assert_eq!(out, "on:\n  5\njobs: {}\n");
}

#[test]
fn a_nested_block_collection_no_longer_blames_the_input() {
    // This was an error: "inconsistent indentation: token at a column
    // that does not match any open block scope", over a document that
    // has none. The edit created it; the input was fine.
    let out = set("outer:\n  k:\n    a: 1\n  after: 1\n", "outer.k", "5").unwrap();
    assert_eq!(out, "outer:\n  k:\n    5\n  after: 1\n");
}

#[test]
fn an_inline_comment_on_the_key_line_survives() {
    // The splice stays inside the value's span, so the key's line is not
    // rewritten. Collapsing the entry onto that line would have to
    // swallow the comment and put it back.
    let out = set("k:  # why\n  a: 1\n", "k", "5").unwrap();
    assert_eq!(out, "k:  # why\n  5\n");
}

#[test]
fn a_deeply_indented_collection_keeps_its_column() {
    let out = set("a:\n      x: 1\nb: 2\n", "a", "5").unwrap();
    assert_eq!(out, "a:\n      5\nb: 2\n");
}

#[test]
fn a_string_over_a_block_collection_now_writes() {
    // The leaf at a widened span is the indentation, which the formatter
    // reported as "target site is not a scalar leaf" — so a string
    // errored while a number corrupted, the arm disagreeing with itself.
    let out = set("k:\n  a: 1\n", "k", "hello").unwrap();
    assert_eq!(out, "k:\n  hello\n");
}

#[test]
fn a_multi_line_string_over_a_block_collection_is_a_block_literal() {
    let out = set("k:\n  a: 1\n", "k", "\"one\\ntwo\"").unwrap();
    assert_eq!(out, "k:\n  |-\n    one\n    two\n");
    let doc = parse_document(&out).unwrap();
    assert_eq!(
        doc.as_value().get_path("k").and_then(Value::as_str),
        Some("one\ntwo")
    );
}

#[test]
fn writing_the_same_shape_twice_is_byte_stable() {
    let once = set("k:\n  a: 1\nafter: 1\n", "k", "5").unwrap();
    let twice = set(&once, "k", "5").unwrap();
    assert_eq!(once, twice);
}

// Controls. Each says what it stops the rule from becoming.

#[test]
fn a_scalar_over_a_flow_collection_is_unchanged() {
    // A flow collection shares its key's line, so its span never starts
    // at a line start and the rule does not apply.
    let out = set("k: {a: 1}\nafter: 1\n", "k", "5").unwrap();
    assert_eq!(out, "k: 5\nafter: 1\n");
}

#[test]
fn a_scalar_over_a_sequence_item_is_unchanged() {
    // The item's mapping shares the `- ` line, so the resolver never
    // widens it.
    let out = set("xs:\n  - a: 1\n  - b: 2\n", "xs[0]", "5").unwrap();
    assert_eq!(out, "xs:\n  - 5\n  - b: 2\n");
}

#[test]
fn a_scalar_over_a_scalar_on_its_own_line_is_unchanged() {
    // This is the behaviour the rule was derived from: the new value
    // goes where the old value's content began. The block-collection
    // case was the only place that was not true.
    let out = set("k:\n  hello\n", "k", "5").unwrap();
    assert_eq!(out, "k:\n  5\n");
}

#[test]
fn the_document_root_is_not_indented() {
    // The root has no key above it to clear.
    let out = set("a: 1\nb: 2\n", "", "5").unwrap();
    assert_eq!(out, "5\n");
}
