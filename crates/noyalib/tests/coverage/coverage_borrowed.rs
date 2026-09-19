// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage tests for `BorrowedValue<'a>` — the zero-copy YAML value.
//!
//! Before these tests the `borrowed` module had zero exercised paths.
//! The suite here covers the public API surface: construction, accessors
//! (`is_null`, `as_str`, `as_i64`, `as_bool`, `as_sequence`, `as_mapping`),
//! path queries (`get_path`, `query`), and conversion to owned `Value`
//! via `into_owned`.

use noyalib::Value;
#[cfg(feature = "lossless-u64")]
use noyalib::borrowed::from_str_borrowed_with_config;
use noyalib::borrowed::{BorrowedValue, from_str_borrowed};
#[cfg(feature = "lossless-u64")]
use noyalib::{Number, ParserConfig};

// ── Construction & accessors ────────────────────────────────────────────

#[test]
fn null_parses_as_null() {
    let v: BorrowedValue<'_> = from_str_borrowed("~").unwrap();
    assert!(v.is_null());
    assert_eq!(v.as_str(), None);
    assert_eq!(v.as_i64(), None);
    assert_eq!(v.as_bool(), None);
    assert!(v.as_sequence().is_none());
    assert!(v.as_mapping().is_none());
}

#[test]
fn string_scalar() {
    let v: BorrowedValue<'_> = from_str_borrowed("hello").unwrap();
    assert_eq!(v.as_str(), Some("hello"));
    assert!(!v.is_null());
}

#[test]
fn integer_scalar() {
    let v: BorrowedValue<'_> = from_str_borrowed("42").unwrap();
    assert_eq!(v.as_i64(), Some(42));
}

#[cfg(feature = "lossless-u64")]
#[test]
fn lossless_u64_integer_scalar() {
    let cfg = ParserConfig::new().lossless_u64_integers(true);
    let v = from_str_borrowed_with_config("18446744073709551615\n", &cfg).unwrap();
    let owned = v.into_owned();
    assert_eq!(owned.as_u64(), Some(u64::MAX));
    assert!(!matches!(owned, Value::Number(Number::Float(_))));
}

#[test]
fn bool_scalar_true() {
    let v: BorrowedValue<'_> = from_str_borrowed("true").unwrap();
    assert_eq!(v.as_bool(), Some(true));
}

#[test]
fn bool_scalar_false() {
    let v: BorrowedValue<'_> = from_str_borrowed("false").unwrap();
    assert_eq!(v.as_bool(), Some(false));
}

// ── Sequence ────────────────────────────────────────────────────────────

#[test]
fn flow_sequence() {
    let v: BorrowedValue<'_> = from_str_borrowed("[1, 2, 3]").unwrap();
    let seq = v.as_sequence().expect("expected sequence");
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_i64(), Some(1));
    assert_eq!(seq[2].as_i64(), Some(3));
}

#[test]
fn block_sequence() {
    let v: BorrowedValue<'_> = from_str_borrowed("- a\n- b\n- c\n").unwrap();
    let seq = v.as_sequence().expect("seq");
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_str(), Some("a"));
}

// ── Mapping ─────────────────────────────────────────────────────────────

#[test]
fn flow_mapping() {
    let v: BorrowedValue<'_> = from_str_borrowed("{a: 1, b: 2}").unwrap();
    let m = v.as_mapping().expect("map");
    assert_eq!(m.len(), 2);
    assert_eq!(m.get("a").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(m.get("b").and_then(|v| v.as_i64()), Some(2));
}

#[test]
fn block_mapping() {
    let v: BorrowedValue<'_> = from_str_borrowed("name: noyalib\nversion: 1\n").unwrap();
    let m = v.as_mapping().expect("map");
    assert_eq!(m.get("name").and_then(|v| v.as_str()), Some("noyalib"));
    assert_eq!(m.get("version").and_then(|v| v.as_i64()), Some(1));
}

// ── Nested structures ───────────────────────────────────────────────────

#[test]
fn nested_mapping_sequence() {
    let yaml = "servers:\n  - name: a\n    port: 8080\n  - name: b\n    port: 9090\n";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let servers = v
        .as_mapping()
        .and_then(|m| m.get("servers"))
        .and_then(|s| s.as_sequence())
        .expect("sequence");
    assert_eq!(servers.len(), 2);
    assert_eq!(
        servers[0]
            .as_mapping()
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str()),
        Some("a")
    );
    assert_eq!(
        servers[1]
            .as_mapping()
            .and_then(|m| m.get("port"))
            .and_then(|v| v.as_i64()),
        Some(9090)
    );
}

// ── Path queries ────────────────────────────────────────────────────────

#[test]
fn get_path_simple() {
    let yaml = "config:\n  host: db\n  port: 5432\n";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let port = v.get_path("config.port").expect("path hit");
    assert_eq!(port.as_i64(), Some(5432));
}

#[test]
fn get_path_missing_returns_none() {
    let v: BorrowedValue<'_> = from_str_borrowed("a: 1\n").unwrap();
    assert!(v.get_path("a.b.c").is_none());
}

#[test]
fn query_wildcard() {
    let yaml = "items:\n  - name: a\n    v: 1\n  - name: b\n    v: 2\n";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let results = v.query("items[*].name");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].as_str(), Some("a"));
    assert_eq!(results[1].as_str(), Some("b"));
}

// ── Conversion to owned ─────────────────────────────────────────────────

#[test]
fn into_owned_preserves_structure() {
    let yaml = "a:\n  b: [1, 2, 3]\n  c: hello\n";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let owned = v.into_owned();
    // Owned Value should roundtrip via to_string / from_str.
    let yaml_out = noyalib::to_string(&owned).unwrap();
    let reparsed: Value = noyalib::from_str(&yaml_out).unwrap();
    assert_eq!(owned, reparsed);
}

#[test]
fn into_owned_scalar() {
    let v: BorrowedValue<'_> = from_str_borrowed("42").unwrap();
    let owned = v.into_owned();
    assert_eq!(owned.as_i64(), Some(42));
}

// ── Invalid input ───────────────────────────────────────────────────────

#[test]
fn invalid_syntax_errors() {
    let err = from_str_borrowed("a: [unclosed\nnext: v\n");
    assert!(err.is_err());
}

// ── Zero-copy borrows directly from input ───────────────────────────────

#[test]
fn string_borrow_points_into_input() {
    let yaml = String::from("hello world");
    let v: BorrowedValue<'_> = from_str_borrowed(&yaml).unwrap();
    let s = v.as_str().expect("string");
    // Borrowed slice pointer should lie inside `yaml`'s buffer.
    let yaml_ptr_range = yaml.as_ptr() as usize..(yaml.as_ptr() as usize + yaml.len());
    assert!(yaml_ptr_range.contains(&(s.as_ptr() as usize)));
}
