// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Error-path regressions for the CST edit mutators.
//!
//! The mutators are guarded: an unresolvable path, a wrong container
//! kind, an out-of-bounds index, or a duplicate key must be **refused**
//! with the document left byte-for-byte unchanged — never a panic and
//! never a partial edit. Happy paths are covered elsewhere; this file
//! drives the rejection arms (the `ok_or_else` / `map_err` diagnostics)
//! so a refusal always reports a clear error and preserves the source.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

const SRC: &str = "m:\n  a: 1\n  nested:\n    x: 1\nseq:\n  - one\n  - two\n";

fn doc() -> noyalib::cst::Document {
    parse_document(SRC).unwrap()
}

// ── set / set_value ─────────────────────────────────────────────────

#[test]
fn set_rejects_unresolvable_paths() {
    let mut d = doc();
    assert!(d.set("nope", "1").is_err());
    assert!(d.set("m.missing", "1").is_err());
    assert!(d.set_value("nope", &Value::from(1_i64)).is_err());
    assert_eq!(d.to_string(), SRC, "a refused set must not mutate");
}

// ── swap_items / move_item: out-of-bounds and wrong kind ────────────

#[test]
fn swap_items_rejects_out_of_bounds_and_non_sequences() {
    let mut d = doc();
    assert!(d.swap_items("seq", 0, 9).is_err(), "j out of bounds");
    assert!(d.swap_items("seq", 9, 0).is_err(), "i out of bounds");
    assert!(d.swap_items("m", 0, 1).is_err(), "not a sequence");
    assert!(d.swap_items("nope", 0, 1).is_err(), "missing path");
    assert_eq!(d.to_string(), SRC);
}

#[test]
fn move_item_rejects_out_of_bounds_and_non_sequences() {
    let mut d = doc();
    assert!(d.move_item("seq", 0, 9).is_err(), "to out of bounds");
    assert!(d.move_item("seq", 9, 0).is_err(), "from out of bounds");
    assert!(d.move_item("m", 0, 1).is_err(), "not a sequence");
    assert_eq!(d.to_string(), SRC);
}

// ── push_back / insert_after (verbatim fragment) ────────────────────

#[test]
fn push_back_rejects_missing_and_non_sequence_paths() {
    let mut d = doc();
    assert!(d.push_back("nope", "x").is_err(), "missing path");
    assert!(d.push_back("m", "x").is_err(), "not a sequence");
    assert_eq!(d.to_string(), SRC);
}

#[test]
fn insert_after_rejects_non_index_and_out_of_bounds() {
    let mut d = doc();
    assert!(d.insert_after("m", "x").is_err(), "not an index path");
    assert!(
        d.insert_after("seq[9]", "x").is_err(),
        "index out of bounds"
    );
    assert!(d.insert_after("nope[0]", "x").is_err(), "missing sequence");
    assert_eq!(d.to_string(), SRC);
}

// ── insert_entry / insert_entry_value ───────────────────────────────

#[test]
fn insert_entry_rejects_bad_targets() {
    let mut d = doc();
    assert!(d.insert_entry("nope", "k", "1").is_err(), "missing mapping");
    assert!(d.insert_entry("seq", "k", "1").is_err(), "not a mapping");
    assert_eq!(d.to_string(), SRC);
}

#[test]
fn insert_entry_value_rejects_bad_targets() {
    let mut d = doc();
    let v = Value::from(1_i64);
    assert!(
        d.insert_entry_value("nope", "k", &v).is_err(),
        "missing mapping"
    );
    assert!(
        d.insert_entry_value("seq", "k", &v).is_err(),
        "not a mapping"
    );
    assert!(
        d.insert_entry_value("m.a", "k", &v).is_err(),
        "target is a scalar"
    );
    assert_eq!(d.to_string(), SRC);
}

// ── push_back_value / insert_after_value (typed) ────────────────────

#[test]
fn push_back_value_rejects_missing_and_non_sequence_paths() {
    let mut d = doc();
    let v = Value::from(1_i64);
    assert!(d.push_back_value("nope", &v).is_err(), "missing path");
    assert!(d.push_back_value("m", &v).is_err(), "not a sequence");
    assert_eq!(d.to_string(), SRC);
}

#[test]
fn insert_after_value_rejects_non_index_and_out_of_bounds() {
    let mut d = doc();
    let v = Value::from(1_i64);
    assert!(d.insert_after_value("m", &v).is_err(), "not an index path");
    assert!(
        d.insert_after_value("seq[9]", &v).is_err(),
        "index out of bounds"
    );
    assert!(
        d.insert_after_value("nope[0]", &v).is_err(),
        "missing sequence"
    );
    assert_eq!(d.to_string(), SRC);
}

// ── rename_key / remove ─────────────────────────────────────────────

#[test]
fn rename_key_rejects_missing_and_colliding() {
    let mut d = doc();
    assert!(d.rename_key("nope", "x").is_err(), "missing key");
    assert!(d.rename_key("m", "seq").is_err(), "collides with sibling");
    assert_eq!(d.to_string(), SRC);
}

#[test]
fn remove_rejects_unresolvable_paths() {
    let mut d = doc();
    assert!(d.remove("nope").is_err(), "missing key");
    assert!(d.remove("m.missing").is_err(), "missing nested key");
    assert!(d.remove("seq[9]").is_err(), "index out of bounds");
    assert_eq!(d.to_string(), SRC);
}

// ── Paths and targets the editing API must refuse ─────────────────────
//
// Each of these is an error branch a caller can reach through the public
// API. They were reachable and untested, which meant the message a user
// would see had never been read by anyone.

#[test]
fn set_path_rejects_query_segments_that_address_more_than_one_entry() {
    for path in ["a.*", "a..b", "a[*]", "$..b"] {
        let mut d = doc();
        let err = d
            .set_path(path, &Value::Bool(true))
            .expect_err(&format!("`{path}` addresses more than one entry"));
        let msg = err.to_string();
        assert!(msg.contains("set_path"), "{path}: {msg}");
        // The document is left exactly as it was.
        assert_eq!(d.to_string(), doc().to_string(), "{path}: document changed");
    }
}

#[test]
fn rename_key_rejects_a_path_that_is_not_a_mapping_entry() {
    // A sequence item has no key to rename.
    let mut d = parse_document("items:\n  - one\n  - two\n").unwrap();
    let before = d.to_string();
    let err = d
        .rename_key("items[0]", "renamed")
        .expect_err("a sequence item has no key");
    assert!(err.to_string().contains("rename_key"), "{err}");
    assert_eq!(d.to_string(), before, "the document was modified");
}

#[test]
fn rename_key_rejects_the_document_root() {
    let mut d = doc();
    let before = d.to_string();
    let err = d
        .rename_key("", "renamed")
        .expect_err("the root has no key");
    assert!(err.to_string().contains("rename_key"), "{err}");
    assert_eq!(d.to_string(), before);
}

#[test]
fn remove_rejects_the_document_root() {
    let mut d = doc();
    let before = d.to_string();
    let err = d.remove("").expect_err("the root cannot be removed");
    assert!(!err.to_string().is_empty());
    assert_eq!(d.to_string(), before);
}

#[test]
fn set_value_rejects_a_path_through_a_scalar() {
    let mut d = parse_document("a: 1\n").unwrap();
    let before = d.to_string();
    let err = d
        .set_value("a.b", &Value::Bool(true))
        .expect_err("`a` is a scalar, so `a.b` addresses nothing");
    assert!(!err.to_string().is_empty());
    assert_eq!(d.to_string(), before);
}

#[test]
fn swap_items_rejects_equal_and_reversed_indices_consistently() {
    let mut d = parse_document("items:\n  - one\n  - two\n  - three\n").unwrap();
    let before = d.to_string();
    // Swapping an item with itself is a no-op, not an error.
    d.swap_items("items", 1, 1)
        .expect("swapping an item with itself");
    assert_eq!(d.to_string(), before);
    // Reversed order addresses the same pair.
    d.swap_items("items", 2, 0).expect("reversed indices");
    let after: Value = noyalib::from_str(&d.to_string()).unwrap();
    assert_eq!(after["items"][0].as_str(), Some("three"));
    assert_eq!(after["items"][2].as_str(), Some("one"));
}

#[test]
fn editing_a_sequence_index_past_the_end_is_refused_by_every_operation() {
    // Each of these resolves the path through a different helper, and
    // each has its own "index out of bounds" message. A caller reaches
    // all of them with the same mistake.
    let src = "items:\n  - one\n  - two\n";
    for (name, result) in [
        (
            "set_value",
            parse_document(src)
                .unwrap()
                .set_value("items[9]", &Value::Bool(true)),
        ),
        ("set", parse_document(src).unwrap().set("items[9]", "x")),
        (
            "rename_key",
            parse_document(src).unwrap().rename_key("items[9]", "k"),
        ),
        ("remove", parse_document(src).unwrap().remove("items[9]")),
        (
            "push_back_value",
            parse_document(src)
                .unwrap()
                .push_back_value("items[9]", "x"),
        ),
        (
            "insert_after_value",
            parse_document(src)
                .unwrap()
                .insert_after_value("items[9]", "x"),
        ),
    ] {
        let err = result.expect_err(&format!("{name}: index 9 of a two-item sequence"));
        assert!(!err.to_string().is_empty(), "{name}: empty message");
    }
}

#[test]
fn a_nested_sequence_index_past_the_end_is_refused() {
    let src = "outer:\n  items:\n    - one\n";
    for path in ["outer.items[5]", "outer.items[5].deeper", "outer[0]"] {
        let mut d = parse_document(src).unwrap();
        let before = d.to_string();
        let err = d
            .set_value(path, &Value::Bool(true))
            .expect_err(&format!("`{path}` addresses nothing"));
        assert!(!err.to_string().is_empty());
        assert_eq!(d.to_string(), before, "{path}: the document was modified");
    }
}
