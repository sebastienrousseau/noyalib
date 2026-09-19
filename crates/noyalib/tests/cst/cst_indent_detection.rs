// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 2.2 — automatic indent detection.
//!
//! `Document::indent_unit` walks the source and returns the smallest
//! non-zero step between consecutive non-empty/non-comment line
//! indents. `Entry::insert_value` and `Document::insert_entry` both
//! consume that unit when splicing a multi-line block emission, so
//! inserts conform to the surrounding file's 2- vs 4-space convention
//! rather than the serializer's hard-coded default.

#![allow(missing_docs)]

use noyalib::cst::parse_document;
use noyalib::{Mapping, Number, Value};

// ── indent_unit detection — direct ──────────────────────────────────

#[test]
fn detects_two_space_indent() {
    let doc = parse_document("metadata:\n  labels:\n    app: noyalib\n").unwrap();
    assert_eq!(doc.indent_unit(), 2);
}

#[test]
fn detects_four_space_indent() {
    let doc = parse_document("metadata:\n    labels:\n        app: noyalib\n").unwrap();
    assert_eq!(doc.indent_unit(), 4);
}

#[test]
fn detects_three_space_indent() {
    let doc = parse_document("metadata:\n   labels:\n      app: noyalib\n").unwrap();
    assert_eq!(doc.indent_unit(), 3);
}

#[test]
fn flat_document_defaults_to_two() {
    let doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    assert_eq!(doc.indent_unit(), 2);
}

#[test]
fn empty_document_defaults_to_two() {
    let doc = parse_document("").unwrap();
    assert_eq!(doc.indent_unit(), 2);
}

#[test]
fn comments_are_ignored_for_detection() {
    // Inline comment at column 0 must not poison the detection —
    // the only "real" step is the 4 between `outer:` and `inner:`.
    let src = "\
# top
outer:
    inner:
        leaf: 1
";
    let doc = parse_document(src).unwrap();
    assert_eq!(doc.indent_unit(), 4);
}

#[test]
fn mixed_steps_picks_smallest() {
    // First nest is 4 spaces, then 2 — the smallest step wins so
    // that newly-inserted children don't accidentally over-indent.
    let src = "\
outer:
    middle:
      leaf: 1
";
    let doc = parse_document(src).unwrap();
    assert_eq!(doc.indent_unit(), 2);
}

// ── insert_value — scalar inserts (regression baselines) ────────────

#[test]
fn insert_scalar_into_two_space_file_uses_inline_form() {
    let mut doc = parse_document("metadata:\n  labels:\n    app: noyalib\n").unwrap();
    doc.entry("metadata.labels")
        .insert_value("replicas", &Value::Number(Number::Integer(3)))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("    replicas: 3"), "got:\n{out}");
}

#[test]
fn insert_scalar_into_four_space_file_uses_inline_form() {
    let mut doc = parse_document("metadata:\n    labels:\n        app: noyalib\n").unwrap();
    doc.entry("metadata.labels")
        .insert_value("replicas", &Value::Number(Number::Integer(3)))
        .unwrap();
    let out = doc.to_string();
    assert!(
        out.contains("        replicas: 3"),
        "expected 8-space-indented sibling line, got:\n{out}"
    );
}

// ── insert_value — multi-line nested mapping ────────────────────────

#[test]
fn insert_nested_mapping_into_two_space_file_uses_two_space_inner_indent() {
    let mut doc = parse_document("metadata:\n  labels:\n    app: noyalib\n").unwrap();
    let mut nested = Mapping::new();
    let _ = nested.insert("cpu", Value::String("100m".into()));
    let _ = nested.insert("memory", Value::String("128Mi".into()));
    doc.entry("metadata")
        .insert_value("resources", &Value::Mapping(nested))
        .unwrap();
    let out = doc.to_string();
    // Outer key spliced at column 0 (same as `metadata`'s child
    // anchor), children indented one indent_unit (2) further. We
    // assert the indent layout, not the scalar quoting style — the
    // serializer is allowed to quote `100m` if its heuristic
    // demands.
    assert!(
        out.contains("\n  resources:\n    cpu:"),
        "expected 'resources' at column 2, 'cpu' at column 4, got:\n{out}"
    );
    assert!(
        out.contains("\n    memory:"),
        "expected 'memory' indented to column 4, got:\n{out}"
    );
}

#[test]
fn insert_nested_mapping_into_four_space_file_uses_four_space_inner_indent() {
    let mut doc = parse_document("metadata:\n    labels:\n        app: noyalib\n").unwrap();
    let mut nested = Mapping::new();
    let _ = nested.insert("cpu", Value::String("100m".into()));
    let _ = nested.insert("memory", Value::String("128Mi".into()));
    doc.entry("metadata")
        .insert_value("resources", &Value::Mapping(nested))
        .unwrap();
    let out = doc.to_string();
    // Outer key at column 4 (same as `labels`), children indented
    // +4 → column 8.
    assert!(
        out.contains("\n    resources:\n        cpu:"),
        "expected 'resources' at column 4, 'cpu' at column 8, got:\n{out}"
    );
    assert!(
        out.contains("\n        memory:"),
        "expected 'memory' at column 8, got:\n{out}"
    );
}

#[test]
fn insert_keeps_existing_siblings_byte_faithful() {
    // The whole point of routing through replace_span: every byte
    // outside the spliced region (comments, blank lines, sibling
    // entries) survives untouched.
    let src = "\
# project metadata
metadata:
    # label group
    labels:
        app: noyalib  # the project

        team: platform
";
    let mut doc = parse_document(src).unwrap();
    doc.entry("metadata.labels")
        .insert_value("env", &Value::String("prod".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("# project metadata"));
    assert!(out.contains("# label group"));
    assert!(out.contains("app: noyalib  # the project"));
    assert!(out.contains("team: platform"));
    assert!(
        out.contains("        env: prod"),
        "new key must adopt 8-space indent, got:\n{out}"
    );
}

// ── round-trip — re-parse must succeed after every insert ───────────

#[test]
fn re_parsing_after_insert_value_succeeds() {
    let mut doc = parse_document("metadata:\n    labels:\n        app: noyalib\n").unwrap();
    let mut nested = Mapping::new();
    let _ = nested.insert("cpu", Value::String("100m".into()));
    doc.entry("metadata")
        .insert_value("resources", &Value::Mapping(nested))
        .unwrap();
    let out = doc.to_string();
    // Re-parse must succeed and the structure must round-trip.
    let again = parse_document(&out).unwrap();
    assert_eq!(again.to_string(), out);
}
