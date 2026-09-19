//! Regression coverage for plain-scalar quoting edge cases (Refs #345).
//!
//! `write_string`'s fast path used to combine its four guard conditions
//! with a stray `||` where every condition should have been `&&`-ed
//! together, so any newline-free string took the fast path regardless of
//! its first byte. That let a bare `-` (a block-sequence indicator, not a
//! scalar plain-style-safe character), and any leading/trailing tab,
//! through unquoted. yaml-rust2 and noyalib itself refuse to re-parse
//! `k: -` as a scalar, and a leading/trailing tab silently changes the
//! string's boundary content once re-parsed as plain.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Value, from_str, to_string};

fn mapping_yaml(value: &str) -> String {
    let mut m = noyalib::Mapping::new();
    let _ = m.insert("k", Value::String(value.to_owned()));
    to_string(&Value::Mapping(m)).unwrap()
}

#[test]
fn dash_only_string_is_quoted() {
    let out = mapping_yaml("-");
    assert_eq!(out, "k: \"-\"");
    let back: Value = from_str(&out).unwrap();
    assert_eq!(back.get_path("k").and_then(Value::as_str), Some("-"));
}

#[test]
fn tab_leading_string_is_quoted() {
    let out = mapping_yaml("\tNote");
    assert_eq!(out, "k: \"\\tNote\"");
    let back: Value = from_str(&out).unwrap();
    assert_eq!(back.get_path("k").and_then(Value::as_str), Some("\tNote"));
}

#[test]
fn tab_trailing_string_is_quoted() {
    let out = mapping_yaml("Note\t");
    assert_eq!(out, "k: \"Note\\t\"");
    let back: Value = from_str(&out).unwrap();
    assert_eq!(back.get_path("k").and_then(Value::as_str), Some("Note\t"));
}

#[test]
fn plain_scalars_stay_plain() {
    for s in ["x", "hello world", "a-b", "it's"] {
        let out = mapping_yaml(s);
        assert_eq!(out, format!("k: {s}"), "expected {s:?} to stay plain");
        let back: Value = from_str(&out).unwrap();
        assert_eq!(back.get_path("k").and_then(Value::as_str), Some(s));
    }
}
