// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for the skip-span fast path in
//! `parser::loader::NoSpanLoader`. This loader was previously
//! `#[cfg(not(feature = "std"))]`-only, so its lines were not
//! counted under `cargo +nightly llvm-cov --all-features`. Commit
//! 1e7dace ungated it and routed `from_str::<Value>(...)` through
//! it for the zero-rewalk fast path, which dropped overall function
//! coverage on `parser/loader.rs` because every NoSpanLoader arm
//! was suddenly counted but only the common-case ones exercised.
//!
//! This file walks every event variant the NoSpanLoader handles —
//! StreamStart/End, DocumentStart/End, Scalar (plain, tagged, with
//! anchor, with alias), SequenceStart/End (anchored, tagged,
//! nested), MappingStart/End (with merge key, with anchor) — so
//! the function-coverage gate stays ≥ 95%.

use noyalib::Value;

#[test]
fn no_span_path_handles_plain_scalar() {
    let v: Value = noyalib::from_str("hello").expect("plain");
    assert_eq!(v.as_str(), Some("hello"));
}

#[test]
fn no_span_path_handles_typed_scalar() {
    let v: Value = noyalib::from_str("42").expect("int");
    assert!(v.as_i64().is_some());
}

#[test]
fn no_span_path_handles_quoted_scalar() {
    let v: Value = noyalib::from_str("\"hello world\"").expect("quoted");
    assert_eq!(v.as_str(), Some("hello world"));
}

#[test]
fn no_span_path_handles_tagged_scalar_core() {
    let v: Value = noyalib::from_str("!!int 42").expect("tagged int");
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn no_span_path_handles_tagged_scalar_custom() {
    let v: Value = noyalib::from_str("!Custom hello").expect("tagged custom");
    // Custom tags surface as Value::Tagged on the from_str::<Value>
    // fast path.
    assert!(v.as_tagged().is_some());
}

#[test]
fn no_span_path_handles_block_sequence() {
    let v: Value = noyalib::from_str("- a\n- b\n- c\n").expect("seq");
    assert_eq!(v.as_sequence().map(Vec::len), Some(3));
}

#[test]
fn no_span_path_handles_flow_sequence() {
    let v: Value = noyalib::from_str("[1, 2, 3]").expect("flow seq");
    assert_eq!(v.as_sequence().map(Vec::len), Some(3));
}

#[test]
fn no_span_path_handles_block_mapping() {
    let v: Value = noyalib::from_str("a: 1\nb: 2\n").expect("map");
    assert_eq!(v.as_mapping().map(|m| m.len()), Some(2));
}

#[test]
fn no_span_path_handles_flow_mapping() {
    let v: Value = noyalib::from_str("{a: 1, b: 2}").expect("flow map");
    assert_eq!(v.as_mapping().map(|m| m.len()), Some(2));
}

#[test]
fn no_span_path_handles_anchor_then_alias() {
    let yaml = "\
- &first one
- *first
- *first
";
    let v: Value = noyalib::from_str(yaml).expect("anchor + alias");
    let seq = v.as_sequence().expect("seq");
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_str(), Some("one"));
    assert_eq!(seq[1].as_str(), Some("one"));
    assert_eq!(seq[2].as_str(), Some("one"));
}

#[test]
fn no_span_path_handles_anchored_collection() {
    let yaml = "\
defaults: &defaults
  retry: 3
  timeout: 30
prod:
  <<: *defaults
  region: us-east-1
";
    let v: Value = noyalib::from_str(yaml).expect("anchored map + merge");
    let m = v.as_mapping().expect("map");
    let prod = m.get("prod").and_then(|v| v.as_mapping()).expect("prod");
    assert_eq!(prod.get("retry").and_then(|v| v.as_i64()), Some(3));
    assert_eq!(prod.get("timeout").and_then(|v| v.as_i64()), Some(30));
    assert_eq!(
        prod.get("region").and_then(|v| v.as_str()),
        Some("us-east-1")
    );
}

#[test]
fn no_span_path_handles_nested_sequence_in_mapping() {
    let yaml = "\
items:
  - id: 1
    tags:
      - a
      - b
  - id: 2
    tags: []
";
    let v: Value = noyalib::from_str(yaml).expect("nested");
    let m = v.as_mapping().expect("root map");
    let items = m.get("items").and_then(|v| v.as_sequence()).expect("items");
    assert_eq!(items.len(), 2);
}

#[test]
fn no_span_path_handles_tagged_collection() {
    // !!seq is a core schema tag on a sequence — wrap_with_tag's
    // is_core_collection branch fires here.
    let yaml = "!!seq\n- 1\n- 2\n";
    let v: Value = noyalib::from_str(yaml).expect("tagged seq");
    assert_eq!(v.as_sequence().map(Vec::len), Some(2));
}

#[test]
fn no_span_path_handles_empty_document() {
    let v: Value = noyalib::from_str("").expect("empty doc");
    assert!(v.is_null());
}

#[test]
fn no_span_path_handles_explicit_doc_marker() {
    let yaml = "---\nfoo: bar\n";
    let v: Value = noyalib::from_str(yaml).expect("explicit doc start");
    assert_eq!(
        v.as_mapping()
            .and_then(|m| m.get("foo"))
            .and_then(|v| v.as_str()),
        Some("bar")
    );
}

#[test]
fn no_span_path_handles_explicit_doc_end() {
    let yaml = "foo: bar\n...\n";
    let v: Value = noyalib::from_str(yaml).expect("explicit doc end");
    assert_eq!(
        v.as_mapping()
            .and_then(|m| m.get("foo"))
            .and_then(|v| v.as_str()),
        Some("bar")
    );
}

#[test]
fn no_span_path_handles_mapping_with_complex_value() {
    let yaml = "\
key1: value1
key2:
  nested: deep
  list:
    - item1
    - item2
key3: 42
key4: true
key5: null
key6: ~
key7: 3.14
";
    let v: Value = noyalib::from_str(yaml).expect("complex map");
    let m = v.as_mapping().expect("map");
    assert_eq!(m.len(), 7);
}

#[test]
fn no_span_path_handles_block_scalar_literal() {
    let yaml = "\
text: |
  line one
  line two
";
    let v: Value = noyalib::from_str(yaml).expect("literal block");
    let m = v.as_mapping().expect("map");
    assert_eq!(
        m.get("text").and_then(|v| v.as_str()),
        Some("line one\nline two\n")
    );
}

#[test]
fn no_span_path_handles_block_scalar_folded() {
    let yaml = "\
text: >
  line one
  line two
";
    let v: Value = noyalib::from_str(yaml).expect("folded block");
    let m = v.as_mapping().expect("map");
    assert_eq!(
        m.get("text").and_then(|v| v.as_str()),
        Some("line one line two\n")
    );
}
