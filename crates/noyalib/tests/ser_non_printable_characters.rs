// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Serializing strings that carry a C1 control character (U+0080 to
//! U+009F) or one of the non-characters U+FFFE and U+FFFF (issue #379).
//!
//! YAML 1.2 section 5.1 `c-printable` admits none of them (NEL, U+0085,
//! is the one exception, and #335 already spells it `\N`). Written raw
//! they pass this crate's own reader but no libyaml-based tool, so they
//! now force double-quoted style with a hex escape, in key and value
//! position, whatever the configured quote preference.

#![allow(missing_docs)]

use noyalib::{Mapping, SerializerConfig, Value};

fn value_doc(s: &str) -> Value {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::String(s.into()));
    Value::Mapping(m)
}

fn key_doc(s: &str) -> Value {
    let mut m = Mapping::new();
    let _ = m.insert(s, Value::String("v".into()));
    Value::Mapping(m)
}

/// Everything section 5.1 leaves out of `c-printable` that a Rust `str`
/// can hold: the C1 block and the two non-characters. Surrogates cannot
/// occur in a `str`; C0 and DEL were escaped before #379.
fn non_printables() -> impl Iterator<Item = char> {
    ('\u{80}'..='\u{9f}').chain(['\u{fffe}', '\u{ffff}'])
}

/// `c-printable`, section 5.1.
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
fn every_non_printable_is_escaped_in_value_position() {
    for c in non_printables() {
        let s = format!("a{c}b");
        let out = noyalib::to_string(&value_doc(&s)).unwrap();
        let cp = c as u32;
        assert!(out.chars().all(is_printable), "U+{cp:04X}: emitted {out:?}");
        assert!(out.starts_with("k: \""), "U+{cp:04X}: emitted {out:?}");
        let back: Value = noyalib::from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
        assert_eq!(
            back["k"].as_str(),
            Some(s.as_str()),
            "U+{cp:04X}: emitted {out:?}"
        );
    }
}

#[test]
fn every_non_printable_is_escaped_in_key_position() {
    for c in non_printables() {
        let s = format!("a{c}b");
        let doc = key_doc(&s);
        let out = noyalib::to_string(&doc).unwrap();
        let cp = c as u32;
        assert!(out.chars().all(is_printable), "U+{cp:04X}: emitted {out:?}");
        assert!(out.starts_with('"'), "U+{cp:04X}: emitted {out:?}");
        let back: Value = noyalib::from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
        assert_eq!(back, doc, "U+{cp:04X}: emitted {out:?}");
    }
}

#[test]
fn the_spellings_are_the_two_digit_hex_escape_and_the_named_nel() {
    let one = |s: &str| noyalib::to_string(&value_doc(s)).unwrap();
    assert_eq!(one("a\u{80}b"), "k: \"a\\x80b\"");
    assert_eq!(one("a\u{9f}b"), "k: \"a\\x9Fb\"");
    assert_eq!(one("a\u{85}b"), "k: \"a\\Nb\"");
    assert_eq!(one("a\u{fffe}b"), "k: \"a\\uFFFEb\"");
    assert_eq!(one("a\u{ffff}b"), "k: \"a\\uFFFFb\"");
    // Unchanged from before #379: C0 and DEL.
    assert_eq!(one("a\u{1}b"), "k: \"a\\x01b\"");
    assert_eq!(one("a\u{7f}b"), "k: \"a\\x7Fb\"");
}

#[test]
fn a_single_quote_preference_and_quote_all_still_fall_back_to_double_quotes() {
    let single = SerializerConfig::new().prefer_single_quotes(true);
    let out = noyalib::to_string_with_config(&value_doc("a\u{9f}b"), &single).unwrap();
    assert_eq!(out, "k: \"a\\x9Fb\"");

    // `quote_all` quotes the key as well; the value still cannot take
    // single quotes.
    let all = SerializerConfig::new().quote_all(true);
    let out = noyalib::to_string_with_config(&value_doc("a\u{fffe}b"), &all).unwrap();
    assert!(out.ends_with(": \"a\\uFFFEb\""), "emitted {out:?}");
    assert!(out.chars().all(is_printable), "emitted {out:?}");
}

#[test]
fn a_multi_line_string_holding_a_non_printable_is_not_a_block_scalar() {
    let s = "one\ntwo\u{9f}\nthree";
    let out = noyalib::to_string(&value_doc(s)).unwrap();
    assert_eq!(out, "k: \"one\\ntwo\\x9F\\nthree\"");
    let back: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(back["k"].as_str(), Some(s));
}
