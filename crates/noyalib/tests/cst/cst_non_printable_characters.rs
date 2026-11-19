// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! CST edits writing strings that carry a C1 control character (U+0080
//! to U+009F) or one of the non-characters U+FFFE and U+FFFF (issue
//! #379): `set_value` and the typed inserts must spell them with
//! double-quoted escapes, never plain or single-quoted, and a key
//! holding one is refused like a key holding any other control
//! character.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

fn s(v: &str) -> Value {
    Value::String(v.into())
}

/// `c-printable`, YAML 1.2 section 5.1.
fn is_printable(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\r'
            | ' '..='~'
            | '\u{85}'
            | '\u{a0}'..='\u{d7ff}'
            | '\u{e000}'..='\u{fffd}'
            | '\u{10000}'..='\u{10ffff}'
    )
}

#[test]
fn set_value_double_quotes_every_non_printable() {
    for c in ('\u{80}'..='\u{9f}').chain(['\u{fffe}', '\u{ffff}']) {
        let val = format!("a{c}b");
        let mut doc = parse_document("title: old\nnext: 1\n").unwrap();
        doc.set_value("title", &s(&val)).unwrap();
        let out = doc.to_string();
        let cp = c as u32;
        assert!(
            out.starts_with("title: \"") && out.chars().all(is_printable),
            "U+{cp:04X} -> {out:?}"
        );
        let re: Value = noyalib::from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
        assert_eq!(re["title"].as_str(), Some(val.as_str()), "{out:?}");
        assert_eq!(re["next"].as_i64(), Some(1));
    }
}

#[test]
fn the_spellings_match_the_existing_control_escapes() {
    let write = |val: &str| {
        let mut doc = parse_document("title: old\n").unwrap();
        doc.set_value("title", &s(val)).unwrap();
        doc.to_string()
    };
    assert_eq!(write("a\u{9f}b"), "title: \"a\\u009Fb\"\n");
    assert_eq!(write("a\u{80}b"), "title: \"a\\u0080b\"\n");
    assert_eq!(write("a\u{fffe}b"), "title: \"a\\uFFFEb\"\n");
    assert_eq!(write("a\u{ffff}b"), "title: \"a\\uFFFFb\"\n");
    // NEL keeps its named escape (#335); DEL its existing spelling.
    assert_eq!(write("a\u{85}b"), "title: \"a\\Nb\"\n");
    assert_eq!(write("a\u{7f}b"), "title: \"a\\u007Fb\"\n");
}

#[test]
fn a_single_quoted_site_falls_back_to_double_quotes() {
    let mut doc = parse_document("title: 'old'\nnext: 'kept'\n").unwrap();
    doc.set_value("title", &s("a\u{9f}b")).unwrap();
    assert_eq!(doc.to_string(), "title: \"a\\u009Fb\"\nnext: 'kept'\n");
}

#[test]
fn a_flow_site_falls_back_to_double_quotes() {
    let mut doc = parse_document("s: [old, kept]\n").unwrap();
    doc.set_value("s[0]", &s("a\u{9f}b")).unwrap();
    assert_eq!(doc.to_string(), "s: [\"a\\u009Fb\", kept]\n");
}

#[test]
fn a_typed_insert_double_quotes_the_value() {
    let mut doc = parse_document("a: 1\n").unwrap();
    doc.set_path("b", &s("a\u{80}b")).unwrap();
    assert_eq!(doc.to_string(), "a: 1\nb: \"a\\u0080b\"\n");
    let re = parse_document(&doc.to_string()).unwrap();
    assert_eq!(re.as_value()["b"].as_str(), Some("a\u{80}b"));
}

#[test]
fn a_key_holding_a_non_character_is_refused_like_a_control_character() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.set_path("x\u{fffe}y", &s("v")).unwrap_err();
    assert!(err.to_string().contains("U+FFFE"), "{err}");
    assert_eq!(doc.to_string(), "a: 1\n");

    let err = doc.rename_key("a", "x\u{ffff}y").unwrap_err();
    assert!(err.to_string().contains("U+FFFF"), "{err}");
    assert_eq!(doc.to_string(), "a: 1\n");
}

/// A multi-line string holding a character that the scanner refuses in
/// a block scalar: a C1 control other than NEL, U+FFFE, U+FFFF, or a
/// byte order mark (issue #533).
fn excluded_from_block_scalars() -> impl Iterator<Item = char> {
    ('\u{80}'..='\u{9f}')
        .filter(|&c| c != '\u{85}')
        .chain(['\u{fffe}', '\u{ffff}', '\u{feff}'])
}

fn assert_reads_back(out: &str, key: &str, val: &str, cp: u32) {
    assert!(
        !out.contains('|'),
        "U+{cp:04X} was written as a block literal: {out:?}"
    );
    let re: Value = noyalib::from_str(out).unwrap_or_else(|e| panic!("U+{cp:04X} {out:?}: {e}"));
    assert_eq!(re[key].as_str(), Some(val), "U+{cp:04X} {out:?}");
}

#[test]
fn insert_entry_value_double_quotes_a_multi_line_non_printable() {
    for c in excluded_from_block_scalars() {
        for val in [format!("a{c}b\nc\n"), format!("a\nb{c}"), format!("{c}\n")] {
            let mut doc = parse_document("title: T\n").unwrap();
            doc.insert_entry_value("", "k", &s(&val))
                .unwrap_or_else(|e| panic!("U+{:04X} {val:?}: {e}", c as u32));
            assert_reads_back(&doc.to_string(), "k", &val, c as u32);
        }
    }
}

#[test]
fn set_value_double_quotes_a_multi_line_non_printable() {
    for c in excluded_from_block_scalars() {
        let val = format!("a{c}b\nc\n");
        let mut doc = parse_document("title: T\nk: old\n").unwrap();
        doc.set_value("k", &s(&val)).unwrap();
        assert_reads_back(&doc.to_string(), "k", &val, c as u32);
    }
}

#[test]
fn a_multi_line_string_with_nel_stays_a_block_literal() {
    let mut doc = parse_document("title: T\n").unwrap();
    doc.insert_entry_value("", "k", &s("x\ny\u{85}")).unwrap();
    let out = doc.to_string();
    assert_eq!(out, "title: T\nk: |-\n  x\n  y\u{85}\n");
    let re: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(re["k"].as_str(), Some("x\ny\u{85}"));
}
