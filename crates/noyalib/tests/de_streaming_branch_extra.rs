// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Targeted line-coverage push for `de.rs` and `streaming.rs`.
//!
//! Both files sit in the high-80s for line coverage even after the
//! existing `de_branch_coverage.rs` and `coverage_de.rs` suites,
//! because the dominant uncovered code is in:
//!   - error paths on `Deserializer::deserialize_*` (type-mismatch
//!     arms that only fire on shape disagreement)
//!   - tag-preserving / binary-tag migration paths
//!   - merge-key edge cases on the streaming deserialiser
//!     (sequence-of-aliases, late `<<` keys)
//!   - duplicate-key policies on the streaming path
//!
//! Each test below targets a specific named branch identified by
//! coverage analysis. Test names follow the pattern
//! `branch_<file>_<short_description>` so that future coverage
//! drift is traceable to the specific path being exercised.

use std::collections::BTreeMap;

use noyalib::{Mapping, ParserConfig, Value, from_str, from_str_with_config, from_value};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Cfg {
    name: String,
    port: u16,
}

// ── de.rs: deserialize_any with tagged Value (preserve_tags=true) ───

#[test]
fn branch_de_tagged_through_from_value() {
    let inner = Value::String("foo".into());
    let tag = noyalib::Tag::new("!!str");
    let tagged = Value::Tagged(Box::new(noyalib::TaggedValue::new(tag, inner)));
    let v: Value = from_value(&tagged).expect("from_value Tagged");
    assert!(v.as_tagged().is_some());
}

// ── de.rs: float-to-int coercion when fract() == 0.0 ───────────────

#[test]
fn branch_de_float_to_i64_when_whole() {
    let v = Value::from(42.0_f64);
    let n: i64 = from_value(&v).expect("whole-number float to i64");
    assert_eq!(n, 42);
}

#[test]
fn branch_de_float_to_u64_when_whole_nonneg() {
    let v = Value::from(7.0_f64);
    let n: u64 = from_value(&v).expect("whole-number nonneg float to u64");
    assert_eq!(n, 7);
}

#[test]
fn branch_de_float_to_i64_with_fract_errors() {
    let v = Value::from(42.5_f64);
    let r: Result<i64, _> = from_value(&v);
    assert!(r.is_err(), "fractional float must reject i64 deserialise");
}

#[test]
fn branch_de_float_to_u64_negative_errors() {
    let v = Value::from(-1.0_f64);
    let r: Result<u64, _> = from_value(&v);
    assert!(r.is_err(), "negative float must reject u64 deserialise");
}

// ── de.rs: enum from single-key mapping (struct variant) ───────────

#[derive(Debug, Deserialize, PartialEq)]
enum Choice {
    Plain,
    Pair { a: i32, b: i32 },
    Wrapped(String),
}

#[test]
fn branch_de_enum_unit_variant() {
    let yaml = "Plain\n";
    let c: Choice = from_str(yaml).expect("unit variant");
    assert_eq!(c, Choice::Plain);
}

#[test]
fn branch_de_enum_struct_variant_via_single_key_mapping() {
    let yaml = "Pair:\n  a: 1\n  b: 2\n";
    let c: Choice = from_str(yaml).expect("struct variant");
    assert_eq!(c, Choice::Pair { a: 1, b: 2 });
}

#[test]
fn branch_de_enum_newtype_variant() {
    let yaml = "Wrapped: hello\n";
    let c: Choice = from_str(yaml).expect("newtype variant");
    assert_eq!(c, Choice::Wrapped("hello".into()));
}

// ── de.rs: typed shape mismatch errors ──────────────────────────────

#[test]
fn branch_de_seq_into_struct_errors() {
    let yaml = "[1, 2, 3]\n";
    let r: Result<Cfg, _> = from_str(yaml);
    assert!(r.is_err(), "sequence cannot deserialise into struct");
}

#[test]
fn branch_de_string_into_i64_errors() {
    let r: Result<i64, _> = from_str("not_a_number\n");
    assert!(r.is_err());
}

#[test]
fn branch_de_null_into_required_struct_field_errors() {
    let yaml = "name: ~\nport: 8080\n";
    let r: Result<Cfg, _> = from_str(yaml);
    assert!(r.is_err(), "null cannot deserialise into String");
}

// ── de.rs: from_str_with_config with policy on Value target
//        (skip-span fast path bypasses policy walk) ─────────────────

#[test]
fn branch_de_value_fast_path_skips_policy_walk() {
    let yaml = "a: 1\n";
    // The Value fast path doesn't run policies; the test verifies it
    // still succeeds even with a no-op config carrying a policy.
    let cfg = ParserConfig::default();
    let v: Value = from_str_with_config(yaml, &cfg).expect("Value fast path");
    assert!(v.is_mapping());
}

// ── de.rs: from_value into BTreeMap<String, Value> ──────────────────

#[test]
fn branch_de_from_value_into_btreemap_value() {
    let v = Value::Mapping(Mapping::from_iter([
        ("a".to_string(), Value::from(1_i64)),
        ("b".to_string(), Value::from("two")),
    ]));
    let m: BTreeMap<String, Value> = from_value(&v).expect("btreemap value");
    assert_eq!(m.get("a"), Some(&Value::from(1_i64)));
    assert_eq!(m.get("b").and_then(|v| v.as_str()), Some("two"));
}

#[test]
fn branch_de_from_value_into_vec_value() {
    let v = Value::Sequence(vec![Value::from(1_i64), Value::from(2_i64)]);
    let xs: Vec<Value> = from_value(&v).expect("vec value");
    assert_eq!(xs.len(), 2);
}

// ── de.rs: from_value into Option ─────────────────────────────────

#[test]
fn branch_de_from_value_into_option_some() {
    let v = Value::from(42_i64);
    let n: Option<i64> = from_value(&v).expect("option some");
    assert_eq!(n, Some(42));
}

#[test]
fn branch_de_from_value_into_option_none() {
    let v = Value::Null;
    let n: Option<i64> = from_value(&v).expect("option none");
    assert_eq!(n, None);
}

// ── streaming.rs: merge key after another key (rare ordering) ─────

#[test]
fn branch_streaming_merge_key_late_position() {
    // YAML 1.2 places no requirement on `<<` being first; many real
    // documents have it later. The streaming path must still
    // resolve the merge correctly.
    let yaml = "\
defaults: &defaults
  retry: 3
  timeout: 30
prod:
  region: us-east-1
  <<: *defaults
";
    let v: Value = from_str(yaml).expect("late << works");
    let prod = v
        .get_path("prod")
        .and_then(|v| v.as_mapping())
        .expect("prod");
    assert_eq!(
        prod.get("region").and_then(|v| v.as_str()),
        Some("us-east-1")
    );
    assert_eq!(prod.get("retry").and_then(|v| v.as_i64()), Some(3));
}

// ── streaming.rs: merge sequence of aliases ───────────────────────

#[test]
fn branch_streaming_merge_sequence_of_aliases() {
    let yaml = "\
defaults1: &d1
  x: 1
defaults2: &d2
  y: 2
combined:
  <<: [*d1, *d2]
  z: 3
";
    let v: Value = from_str(yaml).expect("sequence-of-aliases merge");
    let combined = v
        .get_path("combined")
        .and_then(|v| v.as_mapping())
        .expect("combined");
    assert_eq!(combined.get("x").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(combined.get("y").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(combined.get("z").and_then(|v| v.as_i64()), Some(3));
}

// ── streaming.rs: typed deserialise with anchor + alias ───────────

#[derive(Debug, Deserialize, PartialEq)]
struct WithAnchor {
    base: String,
    alias_to_base: String,
}

#[test]
fn branch_streaming_typed_anchor_alias() {
    let yaml = "base: &b hello\nalias_to_base: *b\n";
    let v: WithAnchor = from_str(yaml).expect("typed anchor+alias");
    assert_eq!(v.base, "hello");
    assert_eq!(v.alias_to_base, "hello");
}

// ── streaming.rs: typed deserialise into BTreeMap (alphabetical) ──

#[test]
fn branch_streaming_typed_btreemap_orders() {
    let yaml = "z: 1\na: 2\nm: 3\n";
    let m: BTreeMap<String, i64> = from_str(yaml).expect("btreemap from yaml");
    let keys: Vec<&str> = m.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["a", "m", "z"]); // BTreeMap is alphabetical
}

// ── from_value into a deeply-nested Value structure ────────────────

#[test]
fn branch_de_deep_nested_value_via_from_value() {
    let yaml = "\
a:
  b:
    c:
      d:
        e: deep
";
    let v: Value = from_str(yaml).expect("deep parse");
    let cloned: Value = from_value(&v).expect("deep clone via from_value");
    assert_eq!(v, cloned);
}

// ── streaming.rs: empty mapping / empty sequence typed ────────────

#[test]
fn branch_streaming_empty_mapping() {
    let yaml = "{}\n";
    let m: BTreeMap<String, i64> = from_str(yaml).expect("empty map");
    assert!(m.is_empty());
}

#[test]
fn branch_streaming_empty_sequence() {
    let yaml = "[]\n";
    let v: Vec<i64> = from_str(yaml).expect("empty seq");
    assert!(v.is_empty());
}

// ── streaming.rs: scalar shapes (bool, float, negative int) ────────

#[test]
fn branch_streaming_negative_int() {
    let n: i64 = from_str("-42\n").expect("negative int");
    assert_eq!(n, -42);
}

#[test]
fn branch_streaming_float_with_exponent() {
    let f: f64 = from_str("1.5e2\n").expect("scientific notation");
    assert!((f - 150.0).abs() < 1e-9);
}

#[test]
fn branch_streaming_bool_uppercase() {
    let b: bool = from_str("True\n").expect("True");
    assert!(b);
    let b: bool = from_str("FALSE\n").expect("FALSE");
    assert!(!b);
}
