// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Anchor declarations shadow earlier bindings in source order.

use noyalib::{Value, from_str};

#[test]
fn nested_anchor_redefinition_uses_declaration_order() {
    for (yaml, expected) in [
        ("a: &x [ &x inner ]\nb: *x\n", "a: [inner]\nb: inner\n"),
        (
            "a: &x [&x inner, *x]\nb: *x\n",
            "a: [inner, inner]\nb: inner\n",
        ),
        ("a: &x [&x [inner]]\nb: *x\n", "a: [[inner]]\nb: [inner]\n"),
        (
            "a: &x {nested: &x {leaf: inner}}\nb: *x\n",
            "a: {nested: {leaf: inner}}\nb: {leaf: inner}\n",
        ),
        (
            "a: &x {nested: &x inner}\nb: *x\n",
            "a: {nested: inner}\nb: inner\n",
        ),
        (
            "a: &outer {nested: &inner leaf}\nb: *inner\n",
            "a: {nested: leaf}\nb: leaf\n",
        ),
        (
            "first: &x old\nsecond: &x {nested: &y leaf}\ncopy: *x\n",
            "first: old\nsecond: {nested: leaf}\ncopy: {nested: leaf}\n",
        ),
    ] {
        let want: Value = from_str(expected).unwrap();
        let value: Value = from_str(yaml).unwrap();
        assert_eq!(value, want, "Value: {yaml:?}");
        let span_value = noyalib::load_all(yaml).unwrap().next().unwrap().unwrap();
        assert_eq!(span_value, want, "span loader: {yaml:?}");
        let typed: serde_json::Value = from_str(yaml).unwrap();
        let want_typed: serde_json::Value = from_str(expected).unwrap();
        assert_eq!(typed, want_typed, "typed: {yaml:?}");
        let document = noyalib::cst::parse_document(yaml).unwrap();
        assert_eq!(*document.as_value(), want, "CST: {yaml:?}");
    }
}

#[test]
fn redefined_open_anchor_cannot_resolve_to_the_previous_binding() {
    let yaml = "previous: &x old\ncurrent: &x {self: *x}\n";
    assert!(from_str::<Value>(yaml).is_err());
    assert!(from_str::<serde_json::Value>(yaml).is_err());
    assert!(noyalib::load_all(yaml).is_err());
    assert!(noyalib::cst::parse_document(yaml).is_err());
}

#[test]
fn original_nested_anchor_fuzz_input_keeps_the_latest_null_binding() {
    let yaml = include_str!("../fixtures/nested-anchor-redefinition.yaml");
    let value: Value = from_str(yaml).unwrap();
    assert!(value["dbvelopment"]["<<0"].is_null());
    let span_value = noyalib::load_all(yaml).unwrap().next().unwrap().unwrap();
    assert_eq!(span_value, value);
    let typed: serde_json::Value = from_str(yaml).unwrap();
    assert_eq!(typed["dbvelopment"]["<<0"], serde_json::Value::Null);
    let document = noyalib::cst::parse_document(yaml).unwrap();
    assert_eq!(*document.as_value(), value);
}
