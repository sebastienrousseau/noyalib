// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! CST edits writing strings that carry CR, NEL, LS, or PS (issue
//! #335): `set_value` and the typed inserts must spell them with
//! double-quoted escapes, never plain, single-quoted, or as a block
//! literal.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

fn s(v: &str) -> Value {
    Value::String(v.into())
}

#[test]
fn set_value_double_quotes_the_four_characters() {
    for (val, escape) in [
        ("line\r\nwin", "\\r"),
        ("x\u{0085}y", "\\N"),
        ("x\u{2028}y", "\\L"),
        ("x\u{2029}y", "\\P"),
    ] {
        let mut doc = parse_document("title: old\nnext: 1\n").unwrap();
        doc.set_value("title", &s(val)).unwrap();
        let out = doc.to_string();
        assert!(
            out.starts_with("title: \"") && out.contains(escape),
            "{val:?} -> {out:?}"
        );
        let re: Value = noyalib::from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
        assert_eq!(re["title"].as_str(), Some(val), "{out:?}");
        assert_eq!(re["next"].as_i64(), Some(1));
    }
}

#[test]
fn a_single_quoted_site_falls_back_to_double_quotes() {
    let mut doc = parse_document("a: 'q'\n").unwrap();
    doc.set_value("a", &s("x\u{2028}y")).unwrap();
    let out = doc.to_string();
    assert!(out.contains("\\L"), "{out:?}");
    let re: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(re["a"].as_str(), Some("x\u{2028}y"));
}

#[test]
fn a_cr_string_is_not_written_as_a_block_literal() {
    let mut doc = parse_document("a: old\n").unwrap();
    doc.set_value("a", &s("line\r\nwin")).unwrap();
    let out = doc.to_string();
    assert!(!out.contains('|'), "{out:?}");
    let re: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(re["a"].as_str(), Some("line\r\nwin"));
}

#[test]
fn the_typed_inserts_quote_them_too() {
    let mut doc = parse_document("a: 1\n").unwrap();
    doc.insert_entry_value("", "k", &s("x\u{0085}y")).unwrap();
    let out = doc.to_string();
    let re: Value = noyalib::from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
    assert_eq!(re["k"].as_str(), Some("x\u{0085}y"), "{out:?}");

    let mut doc = parse_document("tags:\n  - a\n").unwrap();
    doc.push_back_value("tags", &s("x\u{2029}y")).unwrap();
    let re: Value = noyalib::from_str(&doc.to_string()).unwrap();
    assert_eq!(re["tags"][1].as_str(), Some("x\u{2029}y"));
}
