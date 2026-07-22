//! Coverage for public API surface that had **zero** references from
//! the test suite.
//!
//! Found by cross-referencing every `pub fn` in `src/` against the
//! whole `tests/` tree. These are exported, documented entry points
//! that were only ever exercised by their own doctests (which do not
//! contribute to `cargo llvm-cov` line coverage).

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::{Mapping, Value, from_str};

// ── Mapping::get_index_of ────────────────────────────────────────────

#[test]
fn mapping_get_index_of_returns_insertion_order() {
    let v: Value = from_str("first: 1\nsecond: 2\nthird: 3\n").unwrap();
    let m: &Mapping = v.as_mapping().unwrap();
    assert_eq!(m.get_index_of("first"), Some(0));
    assert_eq!(m.get_index_of("second"), Some(1));
    assert_eq!(m.get_index_of("third"), Some(2));
}

#[test]
fn mapping_get_index_of_missing_key_is_none() {
    let v: Value = from_str("a: 1\n").unwrap();
    let m = v.as_mapping().unwrap();
    assert_eq!(m.get_index_of("nope"), None);
}

#[test]
fn mapping_get_index_of_tracks_order_after_insert() {
    let v: Value = from_str("a: 1\nb: 2\n").unwrap();
    let mut m = v.as_mapping().unwrap().clone();
    // `insert` returns the displaced value; the crate denies unused_results.
    let _ = m.insert("c".to_string(), Value::from(3));
    assert_eq!(m.get_index_of("c"), Some(2));
    // Re-inserting an existing key must not move it to the end.
    let _ = m.insert("a".to_string(), Value::from(9));
    assert_eq!(m.get_index_of("a"), Some(0));
}

// ── cst: comments_at ─────────────────────────────────────────────────

#[test]
fn comments_at_returns_leading_and_inline_comments() {
    let src = "# A multi-line\n# leading block\nport: 8080  # inline\n";
    let doc = noyalib::cst::parse_document(src).unwrap();
    let b = doc.comments_at("port");
    assert_eq!(b.before.len(), 2, "expected two leading comment lines");
    assert!(
        b.inline.is_some(),
        "expected the trailing inline comment to be captured"
    );
}

#[test]
fn comments_at_unknown_path_is_empty() {
    let src = "# lead\na: 1\n";
    let doc = noyalib::cst::parse_document(src).unwrap();
    let b = doc.comments_at("does-not-exist");
    assert!(b.before.is_empty());
    assert!(b.inline.is_none());
}

#[test]
fn comments_at_key_without_comments_is_empty() {
    let src = "a: 1\nb: 2\n";
    let doc = noyalib::cst::parse_document(src).unwrap();
    let b = doc.comments_at("b");
    assert!(b.before.is_empty());
    assert!(b.inline.is_none());
}

// ── schema codegen ───────────────────────────────────────────────────

#[cfg(feature = "schema")]
mod schema_codegen {
    use schemars::JsonSchema;

    #[derive(JsonSchema)]
    #[allow(dead_code)]
    struct Config {
        name: String,
        port: u16,
    }

    /// `schema_for` returns the schema as a `Value` tree.
    #[test]
    fn schema_for_emits_object_with_declared_properties() {
        let schema = noyalib::schema_for::<Config>().unwrap();
        let text = format!("{schema:?}");
        assert!(text.contains("name"), "missing `name` property: {text}");
        assert!(text.contains("port"), "missing `port` property: {text}");
    }

    /// `schema_for_yaml` returns the same schema rendered as YAML text.
    #[test]
    fn schema_for_yaml_emits_parsable_yaml() {
        let yaml = noyalib::schema_for_yaml::<Config>().unwrap();
        assert!(yaml.contains("name"), "missing `name` in YAML: {yaml}");
        // The emitted text must itself round-trip back through the parser.
        let reparsed: noyalib::Value = noyalib::from_str(&yaml).unwrap();
        assert!(reparsed.is_mapping(), "schema YAML should be a mapping");
    }
}
