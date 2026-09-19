// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Entry::or_insert / or_insert_with / or_insert_value /
//! and_modify — std-collections-style ergonomics over the
//! lossless CST splice path.

#![allow(missing_docs)]

use noyalib::cst::parse_document;
use noyalib::{Number, Value};

const TWO_SPACE_DOC: &str = "\
metadata:
  labels:
    app: noyalib
";

#[test]
fn or_insert_runs_when_path_is_vacant() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let inserted = doc.entry("metadata.labels.env").or_insert("prod").unwrap();
    assert!(inserted, "vacant path must report inserted=true");
    assert!(doc.to_string().contains("env: prod"));
}

#[test]
fn or_insert_no_op_when_path_is_occupied() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let inserted = doc
        .entry("metadata.labels.app")
        .or_insert("staging")
        .unwrap();
    assert!(!inserted, "occupied path must report inserted=false");
    // Original value untouched.
    assert!(doc.to_string().contains("app: noyalib"));
    assert!(!doc.to_string().contains("app: staging"));
}

#[test]
fn or_insert_with_lazy_default() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let mut called = 0;
    let _ = doc
        .entry("metadata.labels.env")
        .or_insert_with(|| {
            called += 1;
            "prod".to_owned()
        })
        .unwrap();
    assert_eq!(called, 1);

    // On the occupied branch, the closure must NOT run.
    let mut called2 = 0;
    let _ = doc
        .entry("metadata.labels.app")
        .or_insert_with(|| {
            called2 += 1;
            "should-not-run".to_owned()
        })
        .unwrap();
    assert_eq!(called2, 0);
}

#[test]
fn or_insert_value_typed_default() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let inserted = doc
        .entry("metadata.labels.replicas")
        .or_insert_value(&Value::Number(Number::Integer(3)))
        .unwrap();
    assert!(inserted);
    assert!(doc.to_string().contains("replicas: 3"));
}

#[test]
fn or_insert_value_no_op_when_occupied() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let inserted = doc
        .entry("metadata.labels.app")
        .or_insert_value(&Value::String("ignored".into()))
        .unwrap();
    assert!(!inserted);
    assert!(doc.to_string().contains("app: noyalib"));
}

#[test]
fn or_insert_top_level_key_errors_actionably() {
    // Top-level paths can't be added by or_insert (no parent
    // mapping); the error must point at the workaround.
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let err = doc.entry("brand_new_top_key").or_insert("x").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("top-level"),
        "error must mention top-level: {msg}"
    );
}

#[test]
fn or_insert_at_sequence_index_errors_actionably() {
    let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    let err = doc.entry("items[5]").or_insert("x").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("push_back") || msg.contains("insert_after"),
        "error must redirect to push_back / insert_after: {msg}"
    );
}

#[test]
fn and_modify_runs_when_occupied() {
    let mut doc = parse_document("service:\n  port: 8080\n").unwrap();
    let _ = doc
        .entry("service.port")
        .and_modify(|d| {
            let _ = d.set("service.port", "9090");
        })
        .or_insert("8080") // no-op: path now still occupied
        .unwrap();
    assert!(doc.to_string().contains("port: 9090"));
}

#[test]
fn and_modify_skipped_when_vacant() {
    let mut doc = parse_document("service:\n  port: 8080\n").unwrap();
    let mut ran = 0;
    let _ = doc
        .entry("service.replicas")
        .and_modify(|_| ran += 1)
        .or_insert("3")
        .unwrap();
    assert_eq!(ran, 0);
    assert!(doc.to_string().contains("replicas: 3"));
}

#[test]
fn chained_modify_then_or_insert_idempotent() {
    // Standard "increment-or-default" pattern: the closure
    // always sees the current value if present, otherwise the
    // default supplies a fresh entry.
    let mut doc = parse_document("counters:\n  hits: 1\n").unwrap();
    let _ = doc
        .entry("counters.hits")
        .and_modify(|d| {
            let _ = d.set("counters.hits", "2");
        })
        .or_insert("1")
        .unwrap();
    assert!(doc.to_string().contains("hits: 2"));

    // Same pipeline on a missing key — and_modify is skipped,
    // or_insert provides the default.
    let _ = doc
        .entry("counters.misses")
        .and_modify(|d| {
            let _ = d.set("counters.misses", "999");
        })
        .or_insert("0")
        .unwrap();
    assert!(doc.to_string().contains("misses: 0"));
}

#[test]
fn or_insert_preserves_byte_faithful_outside_target() {
    // The whole point of routing through replace_span:
    // comments, blank lines, and sibling formatting outside
    // the touched span survive verbatim.
    let src = "\
# project metadata
metadata:
  labels:
    app: noyalib  # the project name

    team: platform
";
    let mut doc = parse_document(src).unwrap();
    let _ = doc.entry("metadata.labels.env").or_insert("prod").unwrap();
    let out = doc.to_string();
    assert!(out.contains("# project metadata"));
    assert!(out.contains("# the project name"));
    assert!(out.contains("team: platform"));
    assert!(out.contains("env: prod"));
}

// ── Keys the path grammar cannot express (#288) ─────────────────────
//
// The anchor for a new entry used to be found by taking the last key
// from the typed view, composing it back into a path string, and
// re-parsing that. `parse_query_path` splits on `.`, `[` and `*`
// unconditionally, so no key containing one survived the round trip —
// and a mapping whose keys all contain one looked as if none of its
// entries had source bytes at all. That is the Kubernetes
// `app.kubernetes.io/name` convention, so it was most of the manifests
// in the world. The anchor now comes from the span tree, which needs no
// path at all.

#[test]
fn a_new_key_lands_in_a_mapping_whose_keys_hold_a_dot() {
    let mut doc = parse_document(
        "metadata:\n  labels:\n    app.kubernetes.io/name: web\n    app.kubernetes.io/component: frontend\n",
    )
    .unwrap();
    doc.insert_entry("metadata.labels", "tier", "frontend")
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "metadata:\n  labels:\n    app.kubernetes.io/name: web\n    app.kubernetes.io/component: frontend\n    tier: frontend\n"
    );
}

#[test]
fn the_same_holds_for_bracket_star_and_quoted_keys() {
    for (src, expected) in [
        ("m:\n  a[0]: 1\n", "m:\n  a[0]: 1\n  b: 2\n"),
        ("m:\n  a*b: 1\n", "m:\n  a*b: 1\n  b: 2\n"),
        ("m:\n  \"a.b\": 1\n", "m:\n  \"a.b\": 1\n  b: 2\n"),
    ] {
        let mut doc = parse_document(src).unwrap();
        doc.insert_entry("m", "b", "2").unwrap();
        assert_eq!(doc.to_string(), expected, "for {src:?}");
    }
}

#[test]
fn a_dotted_key_mapping_keeps_its_line_endings() {
    let mut doc = parse_document("labels:\r\n  app.kubernetes.io/name: web\r\n").unwrap();
    doc.insert_entry("labels", "tier", "web").unwrap();
    assert_eq!(
        doc.to_string(),
        "labels:\r\n  app.kubernetes.io/name: web\r\n  tier: web\r\n"
    );
}

#[test]
fn a_trailing_implicit_null_is_the_anchor_rather_than_a_gap() {
    // `b:` owns no value bytes, but its key is a real line at the right
    // column — and it is the line a new sibling belongs after. Anchoring
    // on the last entry that *has* a value would insert above it.
    let mut doc = parse_document("m:\n  a: 1\n  b:\n").unwrap();
    doc.insert_entry("m", "c", "3").unwrap();
    assert_eq!(doc.to_string(), "m:\n  a: 1\n  b:\n  c: 3\n");

    let mut doc = parse_document("m:\n  a:\n  b:\n").unwrap();
    doc.insert_entry("m", "c", "3").unwrap();
    assert_eq!(doc.to_string(), "m:\n  a:\n  b:\n  c: 3\n");
}

#[test]
fn a_mapping_with_a_merge_key_and_an_entry_of_its_own_anchors_on_the_entry() {
    let mut doc = parse_document("d: &d\n  x: 1\ns:\n  <<: *d\n  own: 1\n").unwrap();
    doc.insert_entry("s", "y", "2").unwrap();
    assert_eq!(
        doc.to_string(),
        "d: &d\n  x: 1\ns:\n  <<: *d\n  own: 1\n  y: 2\n"
    );
}

#[test]
fn the_typed_insert_path_gains_the_same_reach() {
    // `insert_entry_value` already went through `mapping_insert_anchor`;
    // this pins that the shared fix reaches it too.
    let mut doc = parse_document("labels:\n  app.kubernetes.io/name: web\n").unwrap();
    doc.insert_entry_value("labels", "tier", &Value::String("frontend".into()))
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "labels:\n  app.kubernetes.io/name: web\n  tier: frontend\n"
    );
}
