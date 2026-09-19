// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::dominant_quote_style` and
//! `Document::dominant_flow_style` — sibling-aware style detection
//! that lets new inserts match the file's existing convention.

#![allow(missing_docs)]

use noyalib::cst::parse_document;
use noyalib::{FlowStyle, Mapping, ScalarStyle, Value};

// ── dominant_quote_style ────────────────────────────────────────────

#[test]
fn plain_dominates_in_plain_only_doc() {
    let doc = parse_document("a: one\nb: two\nc: three\n").unwrap();
    assert_eq!(doc.dominant_quote_style(), ScalarStyle::Plain);
}

#[test]
fn single_quoted_dominates_when_majority() {
    let doc = parse_document("a: 'one'\nb: 'two'\nc: \"three\"\n").unwrap();
    assert_eq!(doc.dominant_quote_style(), ScalarStyle::SingleQuoted);
}

#[test]
fn double_quoted_dominates_when_majority() {
    let doc = parse_document("a: \"one\"\nb: \"two\"\nc: 'three'\n").unwrap();
    assert_eq!(doc.dominant_quote_style(), ScalarStyle::DoubleQuoted);
}

#[test]
fn empty_doc_defaults_to_plain() {
    let doc = parse_document("").unwrap();
    assert_eq!(doc.dominant_quote_style(), ScalarStyle::Plain);
}

#[test]
fn no_quoted_scalars_means_plain() {
    // No quoted values anywhere — the detection ignores plain
    // mapping keys (they're the YAML default and would otherwise
    // drown out any real signal) and falls back to Plain.
    let doc = parse_document("a: one\nb: two\nc: three\n").unwrap();
    assert_eq!(doc.dominant_quote_style(), ScalarStyle::Plain);
}

// ── dominant_flow_style ─────────────────────────────────────────────

#[test]
fn block_dominates_in_block_only_doc() {
    let doc = parse_document("a:\n  - 1\n  - 2\nb:\n  k: v\n").unwrap();
    assert_eq!(doc.dominant_flow_style(), FlowStyle::Block);
}

#[test]
fn flow_dominates_when_majority() {
    let doc = parse_document("a: [1, 2, 3]\nb: [4, 5]\nc: {k: v}\n").unwrap();
    assert_eq!(doc.dominant_flow_style(), FlowStyle::Auto);
}

#[test]
fn empty_or_scalar_only_defaults_to_block() {
    let doc1 = parse_document("").unwrap();
    assert_eq!(doc1.dominant_flow_style(), FlowStyle::Block);
    let doc2 = parse_document("a: 1\nb: 2\n").unwrap();
    assert_eq!(doc2.dominant_flow_style(), FlowStyle::Block);
}

#[test]
fn block_wins_on_tie() {
    // 1 block sequence, 1 flow sequence — block wins because the
    // detection is a strict-majority test.
    let doc = parse_document("a:\n  - 1\n  - 2\nb: [3, 4]\n").unwrap();
    assert_eq!(doc.dominant_flow_style(), FlowStyle::Block);
}

// ── Style adoption when inserting ───────────────────────────────────

#[test]
fn insert_value_adopts_single_quote_style() {
    let mut doc = parse_document("config:\n  app: 'noyalib'\n  env: 'prod'\n").unwrap();
    doc.entry("config")
        .insert_value("region", &Value::String("us-west".into()))
        .unwrap();
    let out = doc.to_string();
    // The dominant quote style is single-quoted; the new value
    // should match that convention.
    assert!(
        out.contains("region: 'us-west'"),
        "inserted scalar must adopt single-quoted style, got:\n{out}"
    );
}

#[test]
fn insert_value_adopts_double_quote_style() {
    let mut doc = parse_document("config:\n  app: \"noyalib\"\n  env: \"prod\"\n").unwrap();
    doc.entry("config")
        .insert_value("region", &Value::String("us-west".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(
        out.contains("region: \"us-west\""),
        "inserted scalar must adopt double-quoted style, got:\n{out}"
    );
}

#[test]
fn insert_value_adopts_plain_style_in_plain_doc() {
    let mut doc = parse_document("config:\n  app: noyalib\n  env: prod\n").unwrap();
    doc.entry("config")
        .insert_value("region", &Value::String("us-west".into()))
        .unwrap();
    let out = doc.to_string();
    // The dominant style is plain — the inserted scalar should
    // not be wrapped in quotes (the YAML grammar permits plain
    // for `us-west`).
    assert!(out.contains("region: us-west"), "got:\n{out}");
}

#[test]
fn dominant_flow_style_drives_caller_branching() {
    // The block-vs-flow accessor is exposed for callers to
    // inspect; the serializer itself emits top-level mappings
    // and sequences in block form regardless. This test
    // documents the contract: detection works, and the caller
    // can use it to decide whether to wrap with `FlowMap` /
    // `FlowSeq` from the `fmt` module before serializing.
    let flow = parse_document("widgets: [a, b]\nbutton: {color: red, size: m}\n").unwrap();
    assert_eq!(flow.dominant_flow_style(), FlowStyle::Auto);
    let block = parse_document("items:\n  - one\nservices:\n  api:\n    port: 8080\n").unwrap();
    assert_eq!(block.dominant_flow_style(), FlowStyle::Block);
}

#[test]
fn insert_value_uses_block_form_in_block_dominant_doc() {
    let mut doc = parse_document("items:\n  - one\nservices:\n  api:\n    port: 8080\n").unwrap();
    let mut nested = Mapping::new();
    let _ = nested.insert("port", Value::String("9090".into()));
    doc.entry("services")
        .insert_value("admin", &Value::Mapping(nested))
        .unwrap();
    let out = doc.to_string();
    // Block-dominated file: the new sub-mapping uses block form
    // and the inner indent matches the file's 2-space convention.
    assert!(out.contains("admin:\n"), "got:\n{out}");
    // The serializer auto-quotes `9090` because it would
    // otherwise round-trip as an integer; the indent / position
    // is what the heuristic controls.
    assert!(
        out.contains("    port: \"9090\"") || out.contains("    port: 9090"),
        "got:\n{out}"
    );
}
