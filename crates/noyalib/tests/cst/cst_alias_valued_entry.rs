// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! An entry whose value is an alias reference (`j: *x`) could be
//! neither removed nor overwritten: the span tree resolved the alias
//! through to the anchor's bytes (issue #149, the right answer for a
//! read), and the writers, seeing bytes that belong to a different
//! key, refused. The entry's own value byte in the source is the
//! `*name` token; the alias node now records that token's span, so
//! `remove` deletes the entry through it and `set_value` replaces the
//! reference with a scalar. Everything that resolves *through* an
//! alias to the anchor's interior stays refused.

use noyalib::Value;
use noyalib::cst::parse_document;

#[test]
fn remove_deletes_a_block_entry_whose_value_is_an_alias() {
    let src = "a: &x 1\nj: *x\nz: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("j").unwrap();
    assert_eq!(doc.source(), "a: &x 1\nz: 2\n");
}

#[test]
fn remove_takes_the_alias_entrys_trailing_comment_with_it() {
    let src = "a: &x 1\nj: *x  # retired\nz: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("j").unwrap();
    assert_eq!(doc.source(), "a: &x 1\nz: 2\n");
}

#[test]
fn remove_deletes_a_sequence_item_that_is_an_alias() {
    let src = "l:\n  - &x 1\n  - *x\n  - 3\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("l[1]").unwrap();
    assert_eq!(doc.source(), "l:\n  - &x 1\n  - 3\n");
}

#[test]
fn remove_deletes_a_flow_member_whose_value_is_an_alias() {
    let src = "a: &x 1\nm: {j: *x, z: 2}\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("m.j").unwrap();
    assert_eq!(doc.source(), "a: &x 1\nm: {z: 2}\n");
}

#[test]
fn set_value_replaces_the_reference_with_a_scalar() {
    let src = "a: &x 1\nj: *x\nz: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("j", &Value::from(5)).unwrap();
    assert_eq!(doc.source(), "a: &x 1\nj: 5\nz: 2\n");
    assert_eq!(doc.get("a"), Some("&x 1"));
}

#[test]
fn set_value_replaces_an_alias_sequence_item() {
    let src = "l:\n  - &x 1\n  - *x\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("l[1]", &Value::from(5)).unwrap();
    assert_eq!(doc.source(), "l:\n  - &x 1\n  - 5\n");
}

#[test]
fn set_value_with_the_loaded_value_stays_a_no_op_and_keeps_the_alias() {
    // The alias resolves to 1; writing 1 is a no-op, and a no-op must
    // not rewrite the reference into a literal.
    let src = "a: &x 1\nj: *x\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("j", &Value::from(1)).unwrap();
    assert_eq!(doc.source(), src);
}

#[test]
fn a_path_through_an_alias_is_still_refused() {
    // `j.k` resolves through `*x` into the anchor's value: those bytes
    // belong to `a`, not to `j`, at every depth.
    let src = "a: &x\n  k: 1\nj: *x\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.set_value("j.k", &Value::from(5)).unwrap_err();
    assert!(
        err.to_string().contains("resolves through an alias"),
        "{err}"
    );
    assert_eq!(doc.source(), src);
}

#[test]
fn a_collection_cannot_replace_the_reference_in_place() {
    let src = "a: &x 1\nj: *x\n";
    let mut doc = parse_document(src).unwrap();
    let mut m = noyalib::Mapping::new();
    let _ = m.insert("k", Value::from(1));
    let err = doc.set_value("j", &Value::Mapping(m)).unwrap_err();
    assert!(err.to_string().contains("alias reference"), "{err}");
    assert_eq!(doc.source(), src);
}

#[test]
fn replacing_the_reference_does_not_disturb_other_alias_sites() {
    let src = "a: &x 1\nj: *x\nk: *x\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("j", &Value::from(9)).unwrap();
    assert_eq!(doc.source(), "a: &x 1\nj: 9\nk: *x\n");
}

#[test]
fn an_alias_entry_inside_a_shared_anchor_is_still_policy_guarded() {
    // Replacing `j`'s token would edit inside the value `&o` shares
    // with `c`; the #338 policy applies before the token is touched.
    let src = "a: &x 1\no: &o\n  j: *x\nc: *o\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.set_value("o.j", &Value::from(5)).unwrap_err();
    assert!(err.to_string().contains("anchored by `&o`"), "{err}");
    assert_eq!(doc.source(), src);
}
