// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::set_value` on a leaf inside a flow collection (issue #332).
//!
//! In flow context `,` `[` `]` `{` `}` are structural wherever they
//! appear in a plain scalar, and block scalars do not exist. The
//! formatter used the block-context rules for every site, so `x, y`
//! grew a sibling entry, `x {y} z` left the document unparseable
//! while `set_value` returned `Ok`, and a multi-line string was
//! written as a `|-` block inside `{…}`.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

fn s(v: &str) -> Value {
    Value::String(v.into())
}

fn reloads(doc: &noyalib::cst::Document) -> Value {
    noyalib::from_str(&doc.to_string()).expect("edited document must load")
}

#[test]
fn a_comma_is_quoted_in_a_flow_mapping_and_sequence() {
    let mut doc = parse_document("m: {a: 1, b: 2}\n").unwrap();
    doc.set_value("m.a", &s("x, y")).unwrap();
    assert_eq!(doc.to_string(), "m: {a: \"x, y\", b: 2}\n");
    let v = reloads(&doc);
    assert_eq!(v["m"]["a"].as_str(), Some("x, y"));
    assert_eq!(v["m"]["b"].as_i64(), Some(2));
    assert_eq!(v["m"].as_mapping().map(noyalib::Mapping::len), Some(2));

    let mut doc = parse_document("s: [a, b]\n").unwrap();
    doc.set_value("s[0]", &s("x, y")).unwrap();
    assert_eq!(doc.to_string(), "s: [\"x, y\", b]\n");
    let v = reloads(&doc);
    assert_eq!(v["s"].as_sequence().map(Vec::len), Some(2));
    assert_eq!(v["s"][0].as_str(), Some("x, y"));
}

#[test]
fn braces_and_brackets_are_quoted_anywhere_in_the_string() {
    for (val, want) in [
        ("x {y} z", "m: {a: \"x {y} z\", b: 2}\n"),
        ("x [y]", "m: {a: \"x [y]\", b: 2}\n"),
        ("x]", "m: {a: \"x]\", b: 2}\n"),
        ("x}", "m: {a: \"x}\", b: 2}\n"),
    ] {
        let mut doc = parse_document("m: {a: 1, b: 2}\n").unwrap();
        doc.set_value("m.a", &s(val)).unwrap();
        assert_eq!(doc.to_string(), want, "{val:?}");
        assert_eq!(reloads(&doc)["m"]["a"].as_str(), Some(val));
    }
}

#[test]
fn a_multi_line_string_is_double_quoted_not_a_block_scalar() {
    let mut doc = parse_document("m: {a: 1}\n").unwrap();
    doc.set_value("m.a", &s("x\ny")).unwrap();
    assert_eq!(doc.to_string(), "m: {a: \"x\\ny\"}\n");
    assert_eq!(reloads(&doc)["m"]["a"].as_str(), Some("x\ny"));

    let mut doc = parse_document("s: [a, b]\n").unwrap();
    doc.set_value("s[1]", &s("two\n\nlines\n")).unwrap();
    assert_eq!(doc.to_string(), "s: [a, \"two\\n\\nlines\\n\"]\n");
    assert_eq!(reloads(&doc)["s"][1].as_str(), Some("two\n\nlines\n"));
}

#[test]
fn a_wrapped_flow_collection_and_a_flow_collection_nested_in_block_context_count_as_flow() {
    let mut doc = parse_document("tags: [\n  a,\n  b,\n]\n").unwrap();
    doc.set_value("tags[1]", &s("x, y")).unwrap();
    assert_eq!(doc.to_string(), "tags: [\n  a,\n  \"x, y\",\n]\n");
    assert_eq!(reloads(&doc)["tags"].as_sequence().map(Vec::len), Some(2));

    let mut doc = parse_document("outer:\n  inner: {a: 1}\n  plain: 2\n").unwrap();
    doc.set_value("outer.inner.a", &s("p\nq")).unwrap();
    assert_eq!(
        doc.to_string(),
        "outer:\n  inner: {a: \"p\\nq\"}\n  plain: 2\n"
    );
    // The block-context sibling is still a block site.
    doc.set_value("outer.plain", &s("p\nq")).unwrap();
    assert_eq!(
        doc.to_string(),
        "outer:\n  inner: {a: \"p\\nq\"}\n  plain: |-\n    p\n    q\n"
    );
}

#[test]
fn the_leafs_quoting_style_is_kept_in_flow_context() {
    let mut doc = parse_document("s: ['a', \"b\", c]\n").unwrap();
    doc.set_value("s[0]", &s("x, y")).unwrap();
    doc.set_value("s[1]", &s("x, y")).unwrap();
    doc.set_value("s[2]", &s("plain")).unwrap();
    assert_eq!(doc.to_string(), "s: ['x, y', \"x, y\", plain]\n");
    // Reserved words and indicators still force quotes on a plain leaf.
    doc.set_value("s[2]", &s("NO")).unwrap();
    assert_eq!(doc.to_string(), "s: ['x, y', \"x, y\", \"NO\"]\n");
}

#[test]
fn block_context_spelling_is_unchanged() {
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    doc.set_value("a", &s("x, y")).unwrap();
    doc.set_value("b", &s("x {y} z")).unwrap();
    assert_eq!(doc.to_string(), "a: x, y\nb: x {y} z\n");
    let v = reloads(&doc);
    assert_eq!(v["a"].as_str(), Some("x, y"));
    assert_eq!(v["b"].as_str(), Some("x {y} z"));
}

#[test]
fn a_verbatim_splice_that_breaks_a_flow_collection_is_refused() {
    // `replace_span` used to repair the enclosing block entry locally,
    // which does not check flow structure, and commit an unparseable
    // source while reporting success.
    let src = "m: {a: 1, b: 2}\n";
    let mut doc = parse_document(src).unwrap();
    let (start, end) = doc.span_at("m.a").unwrap();
    let err = doc.replace_span(start, end, "x {y} z").unwrap_err();
    assert!(!err.to_string().is_empty());
    assert_eq!(doc.to_string(), src, "a refused splice must not mutate");
    assert!(noyalib::from_str::<Value>(&doc.to_string()).is_ok());
}
