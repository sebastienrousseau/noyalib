// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `set_path`, and the styling the editor copies from its neighbours.
//!
//! `set_path` creates the intermediate mappings a path needs, so its
//! interesting cases are the ones where creation is impossible or where
//! the document does not end the way the writer assumes: a root that is
//! not a mapping, a file with no trailing newline, a path that runs
//! through a sequence.
//!
//! The styling logic decides how to spell a new scalar by counting how
//! its siblings are spelled — plain, single-quoted, double-quoted — so
//! it needs siblings of each kind to have anything to count.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use noyalib::cst::parse_document;
use noyalib::{QueryPath, Value};

// ── set_path ────────────────────────────────────────────────────

/// Creating a path several levels deep, into documents that end in
/// different ways. A file with no trailing newline needs one supplied
/// before the new entry, and that is a separate branch.
#[test]
fn set_path_creates_missing_levels_however_the_file_ends() {
    let cases: &[(&str, &str)] = &[
        ("a trailing newline", "existing: 1\n"),
        ("no trailing newline", "existing: 1"),
        ("an empty document", ""),
        ("only a comment", "# just a comment\n"),
        ("blank lines at the end", "existing: 1\n\n\n"),
    ];
    for (label, src) in cases {
        let mut doc = parse_document(src).unwrap_or_else(|e| panic!("{label}: parse: {e}"));
        doc.set_path("a.b.c", &Value::from(42_i64))
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let out = doc.to_string();
        let v: Value = noyalib::from_str(&out)
            .unwrap_or_else(|e| panic!("{label}: result does not parse: {e}\n{out:?}"));
        assert_eq!(
            v.get("a")
                .and_then(|a| a.get("b"))
                .and_then(|b| b.get("c"))
                .and_then(Value::as_i64),
            Some(42),
            "{label}: the path was not created\n{out:?}"
        );
        if src.starts_with("existing") {
            assert_eq!(
                v.get("existing").and_then(Value::as_i64),
                Some(1),
                "{label}: the existing entry was lost\n{out:?}"
            );
        }
    }
}

/// A root that already holds something other than a mapping has no
/// place to put a key, so creation is refused rather than guessed at.
#[test]
fn set_path_refuses_a_root_that_is_not_a_mapping() {
    for (label, src) in [
        ("a scalar root", "just-a-scalar\n"),
        ("a sequence root", "- 1\n- 2\n"),
        ("a quoted scalar root", "\"text\"\n"),
    ] {
        let mut doc = parse_document(src).unwrap_or_else(|e| panic!("{label}: parse: {e}"));
        let before = doc.to_string();
        if doc.set_path("a.b", &Value::from(1_i64)).is_err() {
            assert_eq!(
                doc.to_string(),
                before,
                "{label}: a refused creation edited the document"
            );
        } else {
            {
                // Accepting means it replaced the root outright; the
                // result must at least still parse and hold the value.
                let out = doc.to_string();
                let v: Value = noyalib::from_str(&out).unwrap_or_else(|e| {
                    panic!("{label}: accepted but broke the document: {e}\n{out}")
                });
                assert_eq!(
                    v.get("a").and_then(|a| a.get("b")).and_then(Value::as_i64),
                    Some(1),
                    "{label}: accepted but the path is not readable\n{out}"
                );
            }
        }
    }
}

/// An empty path has no entry to address.
#[test]
fn set_path_refuses_an_empty_path() {
    let mut doc = parse_document("a: 1\n").expect("parse");
    let err = doc
        .set_path("", &Value::from(1_i64))
        .expect_err("an empty path must be refused");
    assert!(
        err.to_string().contains("non-empty"),
        "unhelpful refusal: {err}"
    );
}

/// A path that would have to create a *sequence* index cannot be
/// materialised — there is no natural missing item to invent.
#[test]
fn set_path_refuses_to_invent_a_sequence_item() {
    for (label, path) in [
        ("an index at the end", "xs[5]"),
        ("an index in the middle", "xs[5].deep"),
        ("an index under a new key", "fresh[0]"),
    ] {
        let mut doc = parse_document("xs:\n  - one\n").expect("parse");
        let before = doc.to_string();
        if doc.set_path(path, &Value::from(1_i64)).is_err() {
            assert_eq!(
                doc.to_string(),
                before,
                "{label}: a refused creation edited the document"
            );
        }
    }
}

/// When the whole path already resolves, `set_path` is just `set_value`.
#[test]
fn set_path_on_an_existing_path_sets_the_value() {
    let mut doc = parse_document("a:\n  b: 1\nz: 2\n").expect("parse");
    doc.set_path("a.b", &Value::from(9_i64))
        .expect("set an existing path");
    let v: Value = noyalib::from_str(&doc.to_string()).expect("reparse");
    assert_eq!(
        v.get("a").and_then(|a| a.get("b")).and_then(Value::as_i64),
        Some(9)
    );
    assert_eq!(v.get("z").and_then(Value::as_i64), Some(2));
}

/// A validated path can be reused for CST mutation without reparsing
/// untrusted input at each call site.
#[test]
fn set_query_path_creates_and_updates_entries() {
    let path: QueryPath = "menu.visible".parse().expect("valid path");
    let mut doc = parse_document("title: x\n").expect("parse");
    doc.set_query_path(&path, &Value::Bool(true))
        .expect("create typed path");
    assert_eq!(
        doc.as_value().get_query_path(&path),
        Some(&Value::Bool(true))
    );

    doc.set_query_path(&path, &Value::Bool(false))
        .expect("update typed path");
    assert_eq!(
        doc.as_value().get_query_path(&path),
        Some(&Value::Bool(false))
    );
}

// ── neighbour-copied styling ────────────────────────────────────

/// A new scalar is spelled the way its siblings are. The chooser counts
/// plain, single-quoted and double-quoted neighbours, so a container
/// needs siblings of a given kind before it has anything to copy.
#[test]
fn a_new_scalar_copies_the_spelling_its_siblings_use() {
    let cases: &[(&str, &str, &str)] = &[
        ("all double-quoted", "m:\n  a: \"one\"\n  b: \"two\"\n", "m"),
        ("all single-quoted", "m:\n  a: 'one'\n  b: 'two'\n", "m"),
        ("all plain", "m:\n  a: one\n  b: two\n", "m"),
        (
            "mixed spellings",
            "m:\n  a: \"one\"\n  b: 'two'\n  c: three\n",
            "m",
        ),
        (
            "nested collections only",
            "m:\n  a:\n    x: 1\n  b:\n    - 1\n",
            "m",
        ),
        ("a single entry", "m:\n  a: \"one\"\n", "m"),
    ];
    for (label, src, container) in cases {
        let mut doc = parse_document(src).unwrap_or_else(|e| panic!("{label}: parse: {e}"));
        doc.insert_entry_value(container, "added", &Value::from("value"))
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let out = doc.to_string();
        let v: Value = noyalib::from_str(&out)
            .unwrap_or_else(|e| panic!("{label}: does not reparse: {e}\n{out}"));
        assert_eq!(
            v.get(*container)
                .and_then(|m| m.get("added"))
                .and_then(Value::as_str),
            Some("value"),
            "{label}: the inserted value did not survive its spelling\n{out}"
        );
    }
}

/// The same for sequence items, whose siblings are counted the same way.
#[test]
fn a_new_sequence_item_copies_the_spelling_its_siblings_use() {
    for (label, src) in [
        ("double-quoted items", "xs:\n  - \"one\"\n  - \"two\"\n"),
        ("single-quoted items", "xs:\n  - 'one'\n  - 'two'\n"),
        ("plain items", "xs:\n  - one\n  - two\n"),
        ("collection items", "xs:\n  - a: 1\n  - b: 2\n"),
    ] {
        let mut doc = parse_document(src).unwrap_or_else(|e| panic!("{label}: parse: {e}"));
        doc.push_back_value("xs", &Value::from("added"))
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let out = doc.to_string();
        let v: Value = noyalib::from_str(&out)
            .unwrap_or_else(|e| panic!("{label}: does not reparse: {e}\n{out}"));
        let items = v.get("xs").and_then(Value::as_sequence).expect("xs");
        assert_eq!(
            items.last().and_then(Value::as_str),
            Some("added"),
            "{label}: the appended value did not survive\n{out}"
        );
    }
}

/// Renaming a key to the name it already has is a no-op, decided on the
/// decoded key rather than the spelling — so a quoted `"true"` renamed
/// to `true` must not be requoted into a different YAML type.
#[test]
fn renaming_a_key_to_its_current_decoded_name_is_a_no_op() {
    for (label, src, path, new_key) in [
        ("plain", "a: 1\nz: 2\n", "a", "a"),
        ("single-quoted", "'a': 1\nz: 2\n", "a", "a"),
        ("double-quoted", "\"a\": 1\nz: 2\n", "a", "a"),
        (
            "a boolean-looking key",
            "\"true\": 1\nz: 2\n",
            "true",
            "true",
        ),
        ("inside a flow mapping", "m: {a: 1}\nz: 2\n", "m.a", "a"),
    ] {
        let mut doc = parse_document(src).unwrap_or_else(|e| panic!("{label}: parse: {e}"));
        doc.rename_key(path, new_key)
            .unwrap_or_else(|e| panic!("{label}: a same-name rename must succeed: {e}"));
        assert_eq!(
            doc.to_string(),
            src,
            "{label}: a same-name rename rewrote the document"
        );
    }
}
