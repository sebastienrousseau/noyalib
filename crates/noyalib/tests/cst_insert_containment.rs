//! A fragment passed to an inserter must not reach beyond the one
//! entry it creates.
//!
//! `set` gained its containment oracle in v0.0.21
//! (`set_fragment_containment.rs`); `push_back` and `insert_after`
//! gained the outside-the-container half via `guarded_insert`. This
//! file pins the remaining holes:
//!
//! - `insert_entry`'s new-key path spliced with no oracle at all, so a
//!   lone `\r` (a YAML line break the `contains('\n')` branch test
//!   never sees) escaped the fragment into sibling territory;
//! - the key half was spliced verbatim, so a key holding `: ` or a
//!   line break restructured the mapping;
//! - the sequence inserters' oracle elides the whole container, so a
//!   fragment could smuggle *extra items into the container* — one
//!   `push_back` call appending two items;
//! - the existing-key fast path resolved `"{path}.{key}"` through the
//!   path syntax, so `insert_entry("m", "a.b", …)` with a nested
//!   `m.a.b` replaced that nested entry instead of adding the literal
//!   `a.b` key Kubernetes-style callers mean.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::Value;
use noyalib::cst::parse_document;

const MAP_SRC: &str = "m:\n  a: 1\nz: 9\n";
const SEQ_SRC: &str = "s:\n  - 1\nz: 9\n";

/// Keys of the mapping at `path` ("" for root) in the re-parsed doc.
fn keys_at(source: &str, path: &str) -> Vec<String> {
    let v: Value = noyalib::from_str(source).expect("document must re-parse");
    let target = if path.is_empty() { &v } else { &v[path] };
    let Value::Mapping(m) = target else {
        panic!("expected a mapping at {path:?}, got {target:?}")
    };
    m.keys().map(|k| k.as_str().to_owned()).collect()
}

// ── insert_entry: the fragment half ────────────────────────────────

#[test]
fn a_carriage_return_fragment_cannot_escape_the_mapping() {
    // `\r` is a YAML line break, but not `\n` — the single-line splice
    // branch never saw it, and `c: 3` landed at column 0 as a new
    // top-level key.
    let mut doc = parse_document(MAP_SRC).expect("parse");
    let err = doc
        .insert_entry("m", "k", "v\rc: 3")
        .expect_err("a fragment escaping its container must be refused");
    assert_eq!(
        doc.source(),
        MAP_SRC,
        "the document must be untouched after a refusal ({err})"
    );
}

#[test]
fn a_carriage_return_fragment_cannot_smuggle_root_keys() {
    // Root container: the outside-the-container shape check elides the
    // whole document, so only a growth oracle can see the extra key.
    let src = "a: 1\n";
    let mut doc = parse_document(src).expect("parse");
    let err = doc
        .insert_entry("", "k", "v\rc: 3")
        .expect_err("a fragment adding two root keys must be refused");
    assert_eq!(doc.source(), src, "untouched after refusal ({err})");
}

// ── insert_entry: the key half ─────────────────────────────────────

#[test]
fn a_key_with_a_line_break_is_refused() {
    // `k1: 1\nk2` as a "key" would splice two sibling entries.
    // rename_key and insert_entry_value already refuse control
    // characters in keys; insert_entry must too.
    let mut doc = parse_document(MAP_SRC).expect("parse");
    for bad in ["k1: 1\nk2", "k1\rk2: 2"] {
        let err = doc
            .insert_entry("m", bad, "2")
            .expect_err("a key holding a line break must be refused");
        assert!(
            err.to_string().contains("non-printable"),
            "diagnosis must name the character class: {err}"
        );
        assert_eq!(doc.source(), MAP_SRC);
    }
}

#[test]
fn a_key_needing_quotes_is_quoted_like_rename_key() {
    // rename_key documents: "A new key that is not plain-safe is
    // quoted automatically". The insert side gets the same courtesy
    // instead of splicing `a: b: 1`.
    let mut doc = parse_document(MAP_SRC).expect("parse");
    doc.insert_entry("m", "a: b", "1")
        .expect("a non-plain-safe key should be quoted, not spliced raw");
    assert_eq!(keys_at(doc.source(), "m"), vec!["a", "a: b"]);
    let v: Value = noyalib::from_str(doc.source()).expect("reparse");
    assert_eq!(v["m"]["a: b"], Value::from(1_i64));
}

// ── insert_entry: the existing-key fast path ───────────────────────

#[test]
fn a_dotted_key_adds_a_literal_entry_not_a_nested_edit() {
    // `insert_entry("m", "a.b", "2")` used to compose the path
    // `m.a.b`, resolve it into the *nested* mapping, and overwrite
    // `b: 1` — the caller asked for a literal `a.b` key (ubiquitous in
    // Kubernetes labels), not a write through the path syntax.
    let src = "m:\n  a:\n    b: 1\nz: 9\n";
    let mut doc = parse_document(src).expect("parse");
    doc.insert_entry("m", "a.b", "2").expect("insert");
    let v: Value = noyalib::from_str(doc.source()).expect("reparse");
    assert_eq!(
        v["m"]["a"]["b"],
        Value::from(1_i64),
        "the nested entry must be untouched: {}",
        doc.source()
    );
    assert_eq!(v["m"]["a.b"], Value::from(2_i64));
}

#[test]
fn a_dotted_key_that_already_exists_is_refused_not_duplicated() {
    // The path syntax cannot address `a.b` to replace its value, and
    // splicing a second entry would give the document duplicate keys.
    // Same refusal `insert_entry_value` gives.
    let src = "m:\n  a.b: 1\nz: 9\n";
    let mut doc = parse_document(src).expect("parse");
    let err = doc
        .insert_entry("m", "a.b", "2")
        .expect_err("re-inserting an unaddressable key must be refused");
    assert_eq!(doc.source(), src, "untouched after refusal ({err})");
}

// ── the sequence inserters: growth inside the container ────────────

#[test]
fn push_back_cannot_append_two_items() {
    // `v\n  - w` kept every smuggled byte *inside* the container, so
    // the outside-shape oracle waved it through and one call appended
    // two items.
    let mut doc = parse_document(SEQ_SRC).expect("parse");
    let err = doc
        .push_back("s", "v\n  - w")
        .expect_err("a fragment appending two items must be refused");
    assert_eq!(doc.source(), SEQ_SRC, "untouched after refusal ({err})");
}

#[test]
fn insert_after_cannot_insert_two_items() {
    let mut doc = parse_document(SEQ_SRC).expect("parse");
    let err = doc
        .insert_after("s[0]", "v\n  - w")
        .expect_err("a fragment inserting two items must be refused");
    assert_eq!(doc.source(), SEQ_SRC, "untouched after refusal ({err})");
}

// ── what must keep working ─────────────────────────────────────────

#[test]
fn ordinary_inserts_still_work() {
    let mut doc = parse_document(MAP_SRC).expect("parse");
    doc.insert_entry("m", "k", "v").expect("plain insert");
    assert_eq!(doc.source(), "m:\n  a: 1\n  k: v\nz: 9\n");

    let mut doc = parse_document(SEQ_SRC).expect("parse");
    doc.push_back("s", "two").expect("push_back");
    assert_eq!(doc.source(), "s:\n  - 1\n  - two\nz: 9\n");
}

#[test]
fn multiline_nested_fragments_still_insert() {
    // The multi-line branch re-indents a generated block emission
    // under the new key — the legitimate use the oracle must allow.
    let mut doc = parse_document(MAP_SRC).expect("parse");
    doc.insert_entry("m", "k", "x: 1\ny: 2")
        .expect("nested insert");
    let v: Value = noyalib::from_str(doc.source()).expect("reparse");
    assert_eq!(v["m"]["k"]["x"], Value::from(1_i64));
    assert_eq!(v["m"]["k"]["y"], Value::from(2_i64));
    assert_eq!(keys_at(doc.source(), ""), vec!["m", "z"]);
}

#[test]
fn an_existing_key_is_still_an_in_place_set() {
    let mut doc = parse_document(MAP_SRC).expect("parse");
    doc.insert_entry("m", "a", "7").expect("upsert");
    assert_eq!(doc.source(), "m:\n  a: 7\nz: 9\n");
}

#[test]
fn an_implicit_null_entry_is_an_upsert_not_a_duplicate() {
    // `a:` is an entry the mapping already has; the old fast path
    // missed it (no value span) and appended a second `a`.
    let src = "m:\n  a:\n  b: 1\nz: 9\n";
    let mut doc = parse_document(src).expect("parse");
    doc.insert_entry("m", "a", "7")
        .expect("upsert into implicit null");
    assert_eq!(keys_at(doc.source(), "m"), vec!["a", "b"]);
    let v: Value = noyalib::from_str(doc.source()).expect("reparse");
    assert_eq!(v["m"]["a"], Value::from(7_i64));
}

#[test]
fn a_merge_inherited_key_still_gets_an_explicit_override() {
    // A key the mapping only inherits through `<<` has no entry of its
    // own — the insert must append an explicit override, exactly as
    // `insert_entry_value` documents.
    let src = "base: &base\n  k: 1\nm:\n  <<: *base\n  own: 2\n";
    let mut doc = parse_document(src).expect("parse");
    doc.insert_entry("m", "k", "3").expect("override insert");
    let v: Value = noyalib::from_str(doc.source()).expect("reparse");
    assert_eq!(v["m"]["k"], Value::from(3_i64), "{}", doc.source());
    assert_eq!(v["base"]["k"], Value::from(1_i64), "anchor untouched");
}

#[test]
fn merge_key_spelling_is_refused() {
    let mut doc = parse_document(MAP_SRC).expect("parse");
    let err = doc
        .insert_entry("m", "<<", "*base")
        .expect_err("`<<` as a key name must be refused");
    assert!(err.to_string().contains("merge directive"), "{err}");
    assert_eq!(doc.source(), MAP_SRC);
}
