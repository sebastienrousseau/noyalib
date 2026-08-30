//! `from_str` / `from_str_with_config` reject a multi-document stream
//! instead of silently returning its first document.
//!
//! Refs #351: `from_str::<Value>("a: 1\n---\nb: 2\n")` used to give
//! `Ok({a: 1})`, discarding the second document without any signal.
//! `serde_yaml` errors in this situation; match its wording exactly
//! (`"deserializing from YAML containing more than one document is not
//! supported"`) so downstream error messages line up. `from_str_multi`
//! (and `document::load_all`/`load_all_as`) are unaffected — they are
//! the multi-document entry points and keep accepting streams. A
//! single document with a leading `---` or a trailing `...` marker is
//! not "more than one document" and must keep parsing.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Value, from_str, load_all_as};
use serde::Deserialize;

const MULTI: &str = "a: 1\n---\nb: 2\n";
const EXPECTED_MESSAGE: &str =
    "deserializing from YAML containing more than one document is not supported";

#[derive(Debug, Deserialize)]
struct A {
    #[allow(dead_code)]
    a: i32,
}

#[test]
fn typed_target_rejects_multi_document_stream() {
    let err = from_str::<A>(MULTI).expect_err("a second document must be rejected");
    assert_eq!(err.to_string(), EXPECTED_MESSAGE);
}

#[test]
fn value_target_rejects_multi_document_stream() {
    let err = from_str::<Value>(MULTI).expect_err("a second document must be rejected");
    assert_eq!(err.to_string(), EXPECTED_MESSAGE);
}

#[test]
fn single_document_with_leading_marker_still_parses() {
    let v: Value = from_str("---\na: 1\n").expect("a single leading `---` is not multi-document");
    assert_eq!(v.get("a").and_then(Value::as_i64), Some(1));
}

#[test]
fn single_document_with_trailing_end_marker_still_parses() {
    let v: Value = from_str("a: 1\n...\n").expect("a trailing `...` is not multi-document");
    assert_eq!(v.get("a").and_then(Value::as_i64), Some(1));
}

#[test]
fn single_document_with_both_markers_still_parses() {
    let v: Value =
        from_str("---\na: 1\n...\n").expect("both markers around one document still parse");
    assert_eq!(v.get("a").and_then(Value::as_i64), Some(1));
}

#[test]
fn from_str_multi_still_returns_every_document() {
    // `compat::serde_yaml::from_str_multi` is a thin wrapper over
    // `load_all_as` (feature-gated behind `compat-serde-yaml`); exercise
    // the underlying multi-document entry point directly so this test
    // runs under the same feature set as the rest of the suite.
    let docs: Vec<i32> =
        load_all_as("1\n---\n2\n---\n3\n").expect("multi-doc entry point still works");
    assert_eq!(docs, vec![1, 2, 3]);
}
