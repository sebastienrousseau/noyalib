//! Duplicate-key span resolution.
//!
//! Under the default `DuplicateKeyPolicy::Last`, the typed view
//! (`as_value`, `from_str`) keeps the value of the *last* occurrence of
//! a duplicated mapping key. Every span-shaped accessor of the same
//! document — `span_at`, `get`, `Spanned<T>` deserialization — and the
//! path-shaped edits built on them (`set`, `remove`) must select that
//! same occurrence: a consumer must never be handed the bytes of a node
//! the typed view did not select.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::cst::parse_document;
use noyalib::{Spanned, from_str};
use serde::Deserialize;

/// Parse `src`, resolve `path`, and return the raw source slice the
/// span denotes.
fn slice_at(src: &str, path: &str) -> String {
    let doc = parse_document(src).unwrap();
    let (s, e) = doc
        .span_at(path)
        .unwrap_or_else(|| panic!("no span for path {path:?} in {src:?}"));
    doc.source()[s..e].to_owned()
}

#[test]
fn span_at_duplicate_key_selects_last_occurrence() {
    assert_eq!(slice_at("k: one\nk: two\n", "k"), "two");
}

#[test]
fn get_matches_typed_view_for_duplicate_key() {
    let doc = parse_document("k: one\nk: two\n").unwrap();
    let typed = {
        let value = doc.as_value();
        let map = value.as_mapping().expect("root is a mapping");
        map.get("k").and_then(|v| v.as_str()).unwrap().to_owned()
    };
    assert_eq!(typed, "two");
    assert_eq!(doc.get("k"), Some("two"));
}

#[test]
fn span_at_sibling_after_duplicate_is_unshifted() {
    // A duplicate earlier in the mapping must not shift the span
    // pairing of the keys that follow it.
    assert_eq!(slice_at("k: one\nk: two\nz: 3\n", "z"), "3");
}

#[test]
fn typed_fallback_stays_aligned_after_duplicate() {
    // A double-quoted key is not decodable by the green-tree walker,
    // so every path in this document resolves through the typed
    // cache — this exercises the `Value`/`SpanTree` zip alignment.
    let src = "\"q\": 0\nk: one\nk: two\nz: 3\n";
    assert_eq!(slice_at(src, "q"), "0");
    assert_eq!(slice_at(src, "k"), "two");
    assert_eq!(slice_at(src, "z"), "3");
}

#[test]
fn span_at_nested_duplicate_selects_last_occurrence() {
    assert_eq!(slice_at("m:\n  k: one\n  k: two\n", "m.k"), "two");
}

#[test]
fn span_at_flow_mapping_duplicate_selects_last_occurrence() {
    assert_eq!(slice_at("m: {k: 1, k: 2}\n", "m.k"), "2");
}

#[test]
fn span_at_duplicate_across_quote_styles() {
    // Plain and single-quoted spellings of the same key are both
    // decodable by the green-tree walker; last-wins applies across
    // spellings, in either order.
    assert_eq!(slice_at("'k': one\nk: two\n", "k"), "two");
    assert_eq!(slice_at("k: one\n'k': two\n", "k"), "two");
}

#[test]
fn span_at_double_quoted_duplicate_falls_back_to_typed_view() {
    // The double-quoted occurrence is invisible to the green-tree
    // walker. It must not silently resolve the plain occurrence —
    // the typed view decides, and it keeps the last occurrence.
    assert_eq!(slice_at("k: one\n\"k\": two\n", "k"), "two");
    assert_eq!(slice_at("\"k\": one\nk: two\n", "k"), "two");
}

#[test]
fn span_at_triple_duplicate_selects_final_occurrence() {
    assert_eq!(slice_at("k: a\nk: b\nk: c\n", "k"), "c");
}

#[test]
fn span_at_recurses_into_the_winning_occurrence() {
    // The typed view keeps the last `k`, so `k.a` (present only in
    // the shadowed first occurrence) is absent, while `k.b` resolves
    // inside the second occurrence's subtree.
    let src = "k:\n  a: 1\nk:\n  b: 2\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(doc.span_at("k.a"), None);
    assert_eq!(slice_at(src, "k.b"), "2");
}

#[test]
fn set_edits_the_occurrence_the_typed_view_selects() {
    let mut doc = parse_document("k: one\nk: two\n").unwrap();
    doc.set("k", "three").unwrap();
    assert_eq!(doc.to_string(), "k: one\nk: three\n");
}

#[test]
fn remove_sibling_after_duplicate_removes_the_right_line() {
    let mut doc = parse_document("k: 1\nk: 2\nz: 3\n").unwrap();
    doc.remove("z").unwrap();
    assert_eq!(doc.to_string(), "k: 1\nk: 2\n");
}

#[test]
fn remove_sibling_after_duplicate_is_correct_with_a_merge_key() {
    // A merge key (`<<`) is applied at mapping-end without a span
    // entry, and a duplicate key collapses in the typed map — both
    // stress the map/span-entry alignment `remove` relies on. The
    // sibling `z` must still delete its own line, not a neighbour's.
    let src = "base: &b\n  x: 1\nm:\n  <<: *b\n  k: one\n  k: two\n  z: 3\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("m.z").unwrap();
    assert_eq!(
        doc.to_string(),
        "base: &b\n  x: 1\nm:\n  <<: *b\n  k: one\n  k: two\n"
    );
}

#[test]
fn remove_of_a_duplicated_key_targets_the_typed_view_occurrence() {
    // `remove` resolves a single span, so it deletes one line — the
    // last occurrence, the one the typed view keeps. The shadowed
    // first occurrence is left in place (an inherent limit of
    // single-span addressing for duplicate keys), but the line that
    // *is* removed is the winning one.
    let mut doc = parse_document("k: one\nk: two\nz: 3\n").unwrap();
    doc.remove("k").unwrap();
    assert_eq!(doc.to_string(), "k: one\nz: 3\n");
}

#[test]
fn spanned_fields_stay_aligned_after_duplicate_key() {
    #[derive(Debug, Deserialize)]
    struct Config {
        k: Spanned<String>,
        z: Spanned<i64>,
    }

    let src = "k: one\nk: two\nz: 3\n";
    let config: Config = from_str(src).unwrap();
    // Raw event spans may include the scalar's trailing line break
    // (only `span_at` trims trailing blanks), so compare trimmed:
    // what these assertions pin is occurrence selection and
    // alignment, not the exact extent.
    assert_eq!(config.k.value, "two");
    assert_eq!(
        src[config.k.start.index()..config.k.end.index()].trim_end(),
        "two",
        "Spanned span for a duplicated key must cover the winning occurrence"
    );
    assert_eq!(config.z.value, 3);
    assert_eq!(
        src[config.z.start.index()..config.z.end.index()].trim_end(),
        "3",
        "Spanned span after a duplicate key must not be shifted"
    );
}

// ── Distinct-typed key collisions are refused, not silently collapsed ──
//
// The mapping key model is `Mapping<String, Value>`. Two DISTINCT YAML
// keys that stringify the same — the integer `1` and the string `"1"`,
// `true` and `"true"`, the null `~` and `"null"` — would otherwise
// overwrite each other, silently losing an entry. That is data loss, so
// the loader raises `Error::KeyCollision` instead. A genuine duplicate
// (the same typed key twice) keeps its `DuplicateKeyPolicy` behaviour.

#[test]
fn distinct_typed_keys_collide_loudly() {
    for src in [
        "1: a\n\"1\": b\n",          // int vs string
        "true: yes\n\"true\": no\n", // bool vs string
        "~: a\n\"null\": b\n",       // null vs string
    ] {
        let err = parse_document(src).expect_err("distinct-typed collision must error");
        assert!(
            matches!(err, noyalib::Error::KeyCollision(_)),
            "expected KeyCollision for {src:?}, got {err:?}"
        );
        assert!(format!("{err}").contains("collide"), "{err}");
    }
}

#[test]
fn genuine_duplicate_keys_are_not_a_collision() {
    // The same typed key twice is an authored duplicate: default
    // last-wins keeps one entry, no error.
    let doc = parse_document("1: a\n1: b\n").unwrap();
    let v = doc.as_value();
    let m = v.as_mapping().unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m.get("1"), Some(&noyalib::Value::String("b".into())));
}

#[test]
fn non_colliding_scalar_keys_load() {
    // A lone numeric or bool key stringifies without colliding.
    assert!(parse_document("8080: service\n").is_ok());
    assert!(parse_document("true: on\n").is_ok());
    assert!(parse_document("a: 1\nb: 2\n").is_ok());
}

#[test]
fn merge_keys_do_not_trip_the_collision_check() {
    // `<<` merge values are buffered separately and never run through
    // the ordinary-insert collision check.
    let src = "defaults: &d\n  timeout: 30\nservice:\n  <<: *d\n  name: web\n";
    assert!(parse_document(src).is_ok());
}
