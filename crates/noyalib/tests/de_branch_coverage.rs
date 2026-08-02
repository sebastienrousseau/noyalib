// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage walk for the deserialiser branches in `crates/noyalib/src/de.rs`.
//!
//! `from_str::<Value>` now goes through the skip-span fast path
//! (`is_value_target` → `parse_one_value` → `Box<dyn Any>::downcast`),
//! so the AST-fallback path is only exercised by:
//!   - typed targets (`T != Value`) that the streaming path can't
//!     handle (non-default merge policy / ignore_binary_tag_for_string
//!     / non-empty policies), and
//!   - `from_value`, `from_slice`, `from_reader` with various inputs.
//!
//! This file walks every public surface in `de.rs` to keep line
//! coverage above the CI threshold, exercising entries that no
//! other test crate happens to hit.

use std::collections::BTreeMap;

use noyalib::{Mapping, ParserConfig, Value};

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
struct Cfg {
    name: String,
    port: u16,
    enabled: bool,
}

// ── from_str_with_config: non-default merge policy ─────────────────

#[test]
fn from_str_with_explicit_merge_policy_routes_through_ast() {
    // Switching `merge_key_policy` away from `Auto` disqualifies the
    // streaming path, forcing the AST fallback.
    let yaml = "\
defaults: &defaults
  retry: 3
prod:
  <<: *defaults
  region: us-east-1
";
    let cfg = ParserConfig::default().merge_key_policy(noyalib::MergeKeyPolicy::AsOrdinary);
    let v: Value = noyalib::from_str_with_config(yaml, &cfg).expect("parse");
    let m = v.as_mapping().expect("map");
    let prod = m.get("prod").and_then(|v| v.as_mapping()).expect("prod");
    // With merge disabled, `<<: *defaults` is preserved as a literal key
    // (the `<<` key with the alias as value).
    assert!(prod.contains_key("region"));
}

// ── from_str_with_config: ignore_binary_tag_for_string ────────────

#[test]
fn from_str_with_binary_tag_ignored_for_string() {
    let yaml = "data: !!binary SGVsbG8=\n";
    let cfg = ParserConfig::default().ignore_binary_tag_for_string(true);
    let m: BTreeMap<String, String> = noyalib::from_str_with_config(yaml, &cfg).expect("parse");
    // With ignore_binary_tag_for_string, `!!binary` does NOT decode;
    // the literal base64 string is preserved.
    assert_eq!(m.get("data").map(String::as_str), Some("SGVsbG8="));
}

// ── from_value: every Value shape ─────────────────────────────────

#[test]
fn from_value_into_typed_struct() {
    let v = Value::Mapping(Mapping::from_iter([
        ("name".to_string(), Value::String("noyalib".into())),
        ("port".to_string(), Value::from(8080_i64)),
        ("enabled".to_string(), Value::Bool(true)),
    ]));
    let cfg: Cfg = noyalib::from_value(&v).expect("from_value typed");
    assert_eq!(cfg.name, "noyalib");
    assert_eq!(cfg.port, 8080);
    assert!(cfg.enabled);
}

#[test]
fn from_value_into_value_clones_directly() {
    let original = Value::Mapping(Mapping::from_iter([("a".to_string(), Value::from(1_i64))]));
    let cloned: Value = noyalib::from_value(&original).expect("from_value Value");
    assert_eq!(original, cloned);
}

#[test]
fn from_value_into_btreemap() {
    let v = Value::Mapping(Mapping::from_iter([
        ("alpha".to_string(), Value::from(1_i64)),
        ("beta".to_string(), Value::from(2_i64)),
    ]));
    let m: BTreeMap<String, i64> = noyalib::from_value(&v).expect("btreemap");
    assert_eq!(m.get("alpha"), Some(&1));
    assert_eq!(m.get("beta"), Some(&2));
}

// ── from_slice: UTF-8 boundary checks ──────────────────────────────

#[test]
fn from_slice_handles_valid_utf8() {
    let bytes = b"name: noyalib\nport: 8080\nenabled: true\n";
    let cfg: Cfg = noyalib::from_slice(bytes).expect("from_slice valid utf8");
    assert_eq!(cfg.name, "noyalib");
}

#[test]
fn from_slice_rejects_invalid_utf8() {
    let bytes = b"name: \xff\xfeinvalid";
    let r: Result<Cfg, _> = noyalib::from_slice(bytes);
    assert!(r.is_err(), "must reject non-utf8");
}

#[test]
fn from_slice_into_value() {
    let bytes = b"foo: bar\n";
    let v: Value = noyalib::from_slice(bytes).expect("from_slice value");
    assert_eq!(
        v.as_mapping()
            .and_then(|m| m.get("foo"))
            .and_then(|v| v.as_str()),
        Some("bar")
    );
}

// ── from_reader: io::Read interface ────────────────────────────────

#[test]
fn from_reader_round_trips_typed() {
    let yaml = "name: noyalib\nport: 8080\nenabled: true\n";
    let cursor = std::io::Cursor::new(yaml);
    let cfg: Cfg = noyalib::from_reader(cursor).expect("from_reader typed");
    assert_eq!(cfg.name, "noyalib");
    assert_eq!(cfg.port, 8080);
}

#[test]
fn from_reader_into_value() {
    let yaml = "foo: bar\nbaz: qux\n";
    let cursor = std::io::Cursor::new(yaml);
    let v: Value = noyalib::from_reader(cursor).expect("from_reader Value");
    assert_eq!(v.as_mapping().map(|m| m.len()), Some(2));
}

#[test]
fn from_reader_with_config() {
    let yaml = "key: value\n";
    let cursor = std::io::Cursor::new(yaml);
    let cfg = ParserConfig::default();
    let m: BTreeMap<String, String> =
        noyalib::from_reader_with_config(cursor, &cfg).expect("from_reader_with_config");
    assert_eq!(m.get("key").map(String::as_str), Some("value"));
}

// ── from_str_strict: surfaces unknown keys ─────────────────────────

#[test]
fn from_str_strict_rejects_unknown_field() {
    let yaml = "name: noyalib\nport: 8080\nenabled: true\nextra: oops\n";
    let r: Result<Cfg, _> = noyalib::from_str_strict(yaml);
    assert!(r.is_err(), "strict mode rejects unknown `extra`");
}

#[test]
fn from_str_strict_passes_when_clean() {
    let yaml = "name: noyalib\nport: 8080\nenabled: true\n";
    let cfg: Cfg = noyalib::from_str_strict(yaml).expect("strict clean");
    assert_eq!(cfg.name, "noyalib");
}

// ── from_slice_strict / from_reader_strict ──────────────────────────

#[test]
fn from_slice_strict_rejects_unknown_field() {
    let bytes = b"name: x\nport: 1\nenabled: true\nextra: oops\n";
    let r: Result<Cfg, _> = noyalib::from_slice_strict(bytes);
    assert!(r.is_err());
}

#[test]
fn from_reader_strict_rejects_unknown_field() {
    let bytes = b"name: x\nport: 1\nenabled: true\nextra: oops\n";
    let cursor = std::io::Cursor::new(bytes);
    let r: Result<Cfg, _> = noyalib::from_reader_strict(cursor);
    assert!(r.is_err());
}

// ── Error-path coverage on from_str ─────────────────────────────────

#[test]
fn from_str_invalid_yaml_returns_error() {
    let yaml = "key: [unclosed\n";
    let r: Result<Value, _> = noyalib::from_str(yaml);
    assert!(r.is_err());
}

#[test]
fn from_str_typed_into_wrong_shape_errors() {
    let yaml = "[1, 2, 3]\n";
    let r: Result<Cfg, _> = noyalib::from_str(yaml);
    assert!(r.is_err(), "sequence cannot deserialise into struct");
}

// ── Deserializer surface ────────────────────────────────────────────

#[test]
fn deserializer_new_then_deserialize_value() {
    use serde_core::Deserialize as _;
    let v = Value::from(42_i64);
    let de = noyalib::Deserializer::new(&v);
    let out: i64 = i64::deserialize(de).expect("deserialise i64");
    assert_eq!(out, 42);
}
