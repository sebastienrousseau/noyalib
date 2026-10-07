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

// ── Every single-document entry point reads to the end ─────────────
//
// `from_str_borrowing*` and a directly driven `StreamingDeserializer`
// stopped as soon as the target was complete, so they returned the
// first document of `role: user\n---\nrole: admin` where `from_str`
// refused the stream: two components reading the same bytes saw
// different data. Trailing syntax errors and undefined aliases in the
// second document went unread too.

#[derive(Debug, Deserialize)]
struct Role<'a> {
    #[allow(dead_code)]
    role: &'a str,
}

const TRAILING: [&str; 4] = [
    "role: user\n---\nrole: admin\n",
    "role: user\n...\n%YAML 1.2\n---\nrole: admin\n",
    "role: user\n---\n[unterminated\n",
    "role: user\n---\n{a: *undefined_alias}\n",
];

#[test]
fn from_str_borrowing_refuses_what_from_str_refuses() {
    for yaml in TRAILING {
        assert!(
            from_str::<Value>(yaml).is_err(),
            "from_str refuses {yaml:?}"
        );
        let err = noyalib::from_str_borrowing::<Role<'_>>(yaml)
            .expect_err(&format!("from_str_borrowing must refuse {yaml:?}"));
        if yaml.ends_with("admin\n") {
            assert_eq!(err.to_string(), EXPECTED_MESSAGE, "{yaml:?}");
        }
    }
}

#[test]
fn streaming_deserializer_end_refuses_trailing_input() {
    for yaml in TRAILING {
        let mut de = noyalib::StreamingDeserializer::new(yaml);
        let first = Role::deserialize(&mut de).expect("the first document reads");
        assert_eq!(first.role, "user");
        assert!(de.end().is_err(), "end() must refuse {yaml:?}");
    }
}

#[test]
fn borrowing_still_accepts_one_document_with_markers() {
    for yaml in [
        "---\nrole: user\n",
        "role: user\n...\n",
        "---\nrole: user\n...\n",
    ] {
        let r: Role<'_> = noyalib::from_str_borrowing(yaml).expect("one document");
        assert_eq!(r.role, "user");
        let mut de = noyalib::StreamingDeserializer::new(yaml);
        let _ = Role::deserialize(&mut de).expect("reads");
        de.end().expect("nothing trails one document");
    }
}

#[test]
fn a_directly_built_streaming_deserializer_enforces_max_document_length() {
    let yaml = format!("k: {}\n", "x".repeat(100));
    let cfg = noyalib::ParserConfig::default().max_document_length(64);
    let mut de = noyalib::StreamingDeserializer::with_config(&yaml, &cfg);
    let err = Value::deserialize(&mut de).expect_err("over the length limit");
    assert!(err.to_string().contains("maximum length of 64"), "{err}");
}
