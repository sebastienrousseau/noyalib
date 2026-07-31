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
