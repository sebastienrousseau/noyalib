// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Flow and empty collections in the insertion mutators, flow-mapping
//! renames, and the one anchored-node policy (#338, ADR-0011).
//!
//! The reporting corpus writes lists as `tags: [a, b]` and small
//! mappings as `menu: {visible: false}`; every mutator that used to
//! refuse those sites now splices in the collection's own style, and
//! every refusal that remains (multi-line flow, anchored values)
//! leaves the source byte-identical.

#![cfg(feature = "std")]

use noyalib::Value;
use noyalib::cst::{Document, parse_document};

fn apply(
    src: &str,
    f: impl FnOnce(&mut Document) -> noyalib::Result<()>,
) -> Result<String, (noyalib::Error, String)> {
    let mut doc = parse_document(src).unwrap();
    match f(&mut doc) {
        Ok(()) => Ok(doc.to_string()),
        Err(e) => Err((e, doc.to_string())),
    }
}

// ── insert_entry_value ─────────────────────────────────────────────

#[test]
fn insert_into_flow_mapping() {
    let out = apply("menu: {visible: false}\n", |d| {
        d.insert_entry_value("menu", "order", &3_i64)
    })
    .unwrap();
    assert_eq!(out, "menu: {visible: false, order: 3}\n");
}

#[test]
fn insert_into_empty_flow_mapping() {
    let out = apply("menu: {}\n", |d| {
        d.insert_entry_value("menu", "order", &3_i64)
    })
    .unwrap();
    assert_eq!(out, "menu: {order: 3}\n");
}

#[test]
fn insert_into_root_flow_mapping() {
    let out = apply("{a: 1}\n", |d| d.insert_entry_value("", "k", &2_i64)).unwrap();
    assert_eq!(out, "{a: 1, k: 2}\n");
}

#[test]
fn insert_collection_value_into_flow_mapping() {
    let list: Value = noyalib::from_str("[1, 2]").unwrap();
    let out = apply("menu: {a: 1}\n", |d| {
        d.insert_entry_value("menu", "list", &list)
    })
    .unwrap();
    assert_eq!(out, "menu: {a: 1, list: [1, 2]}\n");
}

#[test]
fn insert_key_needing_quotes_into_flow_mapping() {
    let out = apply("menu: {a: 1}\n", |d| {
        d.insert_entry_value("menu", "true", &2_i64)
    })
    .unwrap();
    // Plain `true:` would read as a boolean key.
    assert_eq!(out, "menu: {a: 1, \"true\": 2}\n");
    let loaded: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(loaded["menu"]["true"], Value::from(2_i64));
}

#[test]
fn upsert_of_existing_flow_key_still_replaces_in_place() {
    let out = apply("menu: {visible: false}\n", |d| {
        d.insert_entry_value("menu", "visible", &true)
    })
    .unwrap();
    assert_eq!(out, "menu: {visible: true}\n");
}

#[test]
fn multiline_flow_mapping_refuses_byte_identical() {
    let src = "menu: {a: 1,\n  b: 2}\n";
    let (err, out) = apply(src, |d| d.insert_entry_value("menu", "c", &3_i64)).unwrap_err();
    assert_eq!(out, src);
    assert!(err.to_string().contains("single-line"), "got: {err}");
}

// ── push_back_value ────────────────────────────────────────────────

#[test]
fn push_into_flow_sequence() {
    let out = apply("tags: [a, b]\n", |d| d.push_back_value("tags", "c")).unwrap();
    assert_eq!(out, "tags: [a, b, c]\n");
}

#[test]
fn push_into_empty_flow_sequence() {
    let out = apply("tags: []\n", |d| d.push_back_value("tags", "a")).unwrap();
    assert_eq!(out, "tags: [a]\n");
}

#[test]
fn push_flow_member_needing_quotes() {
    // `b, c` plain would read as two members; a multi-line string
    // has no plain or single-quoted spelling in flow at all.
    let out = apply("tags: [a]\n", |d| d.push_back_value("tags", "b, c")).unwrap();
    assert_eq!(out, "tags: [a, \"b, c\"]\n");
    let out = apply("tags: [a]\n", |d| d.push_back_value("tags", "two\nlines")).unwrap();
    assert_eq!(out, "tags: [a, \"two\\nlines\"]\n");
}

#[test]
fn push_into_flow_sequence_nested_in_flow_mapping() {
    let out = apply("m: {list: [1]}\n", |d| d.push_back_value("m.list", &2_i64)).unwrap();
    assert_eq!(out, "m: {list: [1, 2]}\n");
}

// ── insert_after_value ─────────────────────────────────────────────

#[test]
fn insert_after_flow_item() {
    let out = apply("tags: [a, c]\n", |d| d.insert_after_value("tags[0]", "b")).unwrap();
    assert_eq!(out, "tags: [a, b, c]\n");
}

#[test]
fn insert_after_last_flow_item() {
    let out = apply("tags: [a]\n", |d| d.insert_after_value("tags[0]", "z")).unwrap();
    assert_eq!(out, "tags: [a, z]\n");
}

// ── rename_key ─────────────────────────────────────────────────────

#[test]
fn rename_flow_mapping_key() {
    let out = apply("menu: {visible: false, order: 1}\n", |d| {
        d.rename_key("menu.visible", "shown")
    })
    .unwrap();
    assert_eq!(out, "menu: {shown: false, order: 1}\n");
}

#[test]
fn rename_flow_key_to_flow_unsafe_spelling_quotes_it() {
    let out = apply("menu: {a: 1}\n", |d| d.rename_key("menu.a", "x,y")).unwrap();
    assert_eq!(out, "menu: {\"x,y\": 1}\n");
    let loaded: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(loaded["menu"]["x,y"], Value::from(1_i64));
}

#[test]
fn rename_flow_key_same_name_is_a_noop() {
    let src = "menu: {a: 1}\n";
    assert_eq!(apply(src, |d| d.rename_key("menu.a", "a")).unwrap(), src);
}

#[test]
fn rename_block_key_unchanged_by_the_flow_support() {
    let out = apply("a: 1\nb: 2\n", |d| d.rename_key("a", "z")).unwrap();
    assert_eq!(out, "z: 1\nb: 2\n");
}

// ── one policy for anchored nodes ──────────────────────────────────

#[test]
fn insert_inside_aliased_anchor_refuses_with_guidance() {
    let src = "base: &b {a: 1}\nother: *b\n";
    let (err, out) = apply(src, |d| d.insert_entry_value("base", "c", &2_i64)).unwrap_err();
    assert_eq!(out, src);
    assert!(
        err.to_string().contains("materialise_aliases_of"),
        "got: {err}"
    );
}

#[test]
fn set_value_inside_aliased_anchor_refuses_like_rename_key() {
    // The policy #338 asked for: set_value used to silently change
    // `other.a` and every merge site too.
    let src = "base: &b\n  a: 1\nother: *b\n";
    let (err, out) = apply(src, |d| d.set_value("base.a", &Value::from(2_i64))).unwrap_err();
    assert_eq!(out, src);
    assert!(
        err.to_string().contains("materialise_aliases_of"),
        "got: {err}"
    );
}

#[test]
fn remove_inside_aliased_anchor_refuses_like_rename_key() {
    let src = "base: &b\n  a: 1\n  b: 2\nother: *b\n";
    let (err, out) = apply(src, |d| d.remove("base.a")).unwrap_err();
    assert_eq!(out, src);
    assert!(
        err.to_string().contains("materialise_aliases_of"),
        "got: {err}"
    );
}

#[test]
fn set_value_equal_value_stays_a_noop_inside_an_anchor() {
    let src = "base: &b\n  a: 1\nother: *b\n";
    assert_eq!(
        apply(src, |d| d.set_value("base.a", &Value::from(1_i64))).unwrap(),
        src
    );
}

#[test]
fn edits_outside_any_anchor_are_unaffected_by_the_policy() {
    let out = apply("base: &b\n  a: 1\nother: *b\nfree: 1\n", |d| {
        d.set_value("free", &Value::from(2_i64))
    })
    .unwrap();
    assert_eq!(out, "base: &b\n  a: 1\nother: *b\nfree: 2\n");
}
