// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Targeted coverage tests for regression-prone paths surfaced during
//! the v0.0.1 cleanup:
//!   * Tagged-scalar resolution in the loader (`!!int`, `!!float`, `!!bool`,
//!     `!!null`, `!!str`, custom tags).
//!   * Complex-key coercion to string for the AST Mapping.
//!   * Alias expansion byte limit in the streaming path.
//!   * Merge-key injection edge cases.
//!   * Duplicate-key policy `First` / `Last` / `Error` in the streaming path.

use noyalib::{DuplicateKeyPolicy, ParserConfig, Value, from_str, from_str_with_config};
use serde::Deserialize;
use std::collections::BTreeMap;

// ── Tagged-scalar resolution via AST fallback ────────────────────────────

#[test]
fn loader_tagged_int_decimal() {
    let v: Value = from_str("!!int 42\n").unwrap();
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn loader_tagged_int_hex() {
    let v: Value = from_str("!!int 0xff\n").unwrap();
    assert_eq!(v.as_i64(), Some(255));
}

#[test]
fn loader_tagged_int_octal() {
    let v: Value = from_str("!!int 0o17\n").unwrap();
    assert_eq!(v.as_i64(), Some(15));
}

#[test]
fn loader_tagged_int_negative() {
    let v: Value = from_str("!!int -42\n").unwrap();
    assert_eq!(v.as_i64(), Some(-42));
}

#[test]
fn loader_tagged_int_invalid_errors() {
    assert!(from_str::<Value>("!!int notanumber\n").is_err());
}

#[test]
fn loader_tagged_float_basic() {
    let v: Value = from_str("!!float 2.5\n").unwrap();
    assert!((v.as_f64().unwrap() - 2.5).abs() < 0.001);
}

#[test]
fn loader_tagged_float_inf() {
    let v: Value = from_str("!!float .inf\n").unwrap();
    assert!(v.as_f64().unwrap().is_infinite() && v.as_f64().unwrap().is_sign_positive());
}

#[test]
fn loader_tagged_float_neg_inf() {
    let v: Value = from_str("!!float -.inf\n").unwrap();
    assert!(v.as_f64().unwrap().is_infinite() && v.as_f64().unwrap().is_sign_negative());
}

#[test]
fn loader_tagged_float_nan() {
    let v: Value = from_str("!!float .nan\n").unwrap();
    assert!(v.as_f64().unwrap().is_nan());
}

#[test]
fn loader_tagged_float_invalid_errors() {
    assert!(from_str::<Value>("!!float not-a-float\n").is_err());
}

#[test]
fn loader_tagged_bool_true() {
    let v: Value = from_str("!!bool true\n").unwrap();
    assert_eq!(v.as_bool(), Some(true));
}

#[test]
fn loader_tagged_bool_false() {
    let v: Value = from_str("!!bool False\n").unwrap();
    assert_eq!(v.as_bool(), Some(false));
}

#[test]
fn loader_tagged_bool_invalid_errors() {
    assert!(from_str::<Value>("!!bool banana\n").is_err());
}

#[test]
fn loader_tagged_null_variants() {
    for y in ["!!null null", "!!null ~", "!!null \"\"", "!!null Null"] {
        let v: Value = from_str(y).unwrap();
        assert!(v.is_null(), "input {y:?} should parse as null");
    }
}

#[test]
fn loader_tagged_null_invalid_errors() {
    assert!(from_str::<Value>("!!null NOT_NULL\n").is_err());
}

#[test]
fn loader_tagged_str_preserves_numeric_text() {
    let v: Value = from_str("!!str 42\n").unwrap();
    assert_eq!(v.as_str(), Some("42"));
}

// ── Complex-key coercion (non-string scalar keys become strings) ────────
//
// Sequence / mapping keys (`? - a` style) are handled upstream in the
// block-sequence parser, which has open edge cases — those are captured
// in the `official_suite` SKIP_LIST. These tests cover the scalar-key
// coercion path only.

#[test]
fn integer_key_coerced_to_string() {
    let yaml = "1: one\n2: two\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("1").and_then(|v| v.as_str()), Some("one"));
    assert_eq!(v.get("2").and_then(|v| v.as_str()), Some("two"));
}

#[test]
fn bool_key_coerced_to_string() {
    let yaml = "true: t\nfalse: f\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("true").and_then(|v| v.as_str()), Some("t"));
    assert_eq!(v.get("false").and_then(|v| v.as_str()), Some("f"));
}

// (Null scalar keys like `~: value` depend on block-key disambiguation
// and are covered through the AST path when they arise; not a stable
// public expectation.)

// ── Alias expansion byte limit ──────────────────────────────────────────

#[test]
fn alias_expansion_respects_document_length_limit() {
    let config = ParserConfig::new().max_document_length(1024);
    // 200-byte value × 10 aliases → 2000+ bytes expanded.
    let long = "x".repeat(200);
    let mut yaml = format!("anchor: &a {long}\n");
    for i in 0..10 {
        yaml.push_str(&format!("ref{i}: *a\n"));
    }
    let result: Result<Value, _> = from_str_with_config(&yaml, &config);
    assert!(result.is_err(), "expected alias-bytes limit to trip");
}

// ── Duplicate-key policy through streaming ───────────────────────────────

#[test]
fn duplicate_policy_first_keeps_first_value() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let m: BTreeMap<String, i32> = from_str_with_config("a: 1\na: 2\n", &config).unwrap();
    assert_eq!(m["a"], 1);
}

#[test]
fn duplicate_policy_last_keeps_last_value() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
    let m: BTreeMap<String, i32> = from_str_with_config("a: 1\na: 2\n", &config).unwrap();
    assert_eq!(m["a"], 2);
}

#[test]
fn duplicate_policy_error_rejects_duplicates() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let err = from_str_with_config::<BTreeMap<String, i32>>("a: 1\na: 2\n", &config).unwrap_err();
    assert!(err.to_string().contains("duplicate") || err.to_string().contains("a"));
}

// ── Merge-key edge cases (streaming) ─────────────────────────────────────

#[test]
fn merge_key_multi_anchor_sequence_precedence() {
    // Left source wins in a sequence merge: `<<: [*first, *second]`.
    let yaml = r#"
first: &f
  a: 1
  b: 2
second: &s
  b: 20
  c: 30
target:
  <<: [*f, *s]
"#;
    #[derive(Deserialize)]
    struct Doc {
        target: BTreeMap<String, i64>,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.target["a"], 1);
    assert_eq!(d.target["b"], 2, "first source wins on overlap");
    assert_eq!(d.target["c"], 30);
}

#[test]
fn merge_key_with_empty_anchor_target() {
    let yaml = "empty: &e {}\ntarget:\n  <<: *e\n  only: here\n";
    #[derive(Deserialize)]
    struct Doc {
        target: BTreeMap<String, String>,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.target.len(), 1);
    assert_eq!(d.target["only"], "here");
}

// ── Special float resolution (plain) ─────────────────────────────────────

#[test]
fn streaming_resolves_inf_variants() {
    for input in [".inf", ".Inf", ".INF", "+.inf", "+.Inf"] {
        let v: f64 = from_str(input).unwrap();
        assert!(v.is_infinite() && v.is_sign_positive(), "input {input}");
    }
}

#[test]
fn streaming_resolves_neg_inf_variants() {
    for input in ["-.inf", "-.Inf", "-.INF"] {
        let v: f64 = from_str(input).unwrap();
        assert!(v.is_infinite() && v.is_sign_negative(), "input {input}");
    }
}

#[test]
fn streaming_resolves_nan_variants() {
    for input in [".nan", ".NaN", ".NAN"] {
        let v: f64 = from_str(input).unwrap();
        assert!(v.is_nan(), "input {input}");
    }
}

// ── Legacy booleans ─────────────────────────────────────────────────────

#[test]
fn legacy_booleans_opt_in() {
    let cfg = ParserConfig::new().legacy_booleans(true);
    let v: bool = from_str_with_config("yes", &cfg).unwrap();
    assert!(v);
    let v: bool = from_str_with_config("no", &cfg).unwrap();
    assert!(!v);
    let v: bool = from_str_with_config("y", &cfg).unwrap();
    assert!(v);
    let v: bool = from_str_with_config("n", &cfg).unwrap();
    assert!(!v);
    let v: bool = from_str_with_config("on", &cfg).unwrap();
    assert!(v);
    let v: bool = from_str_with_config("off", &cfg).unwrap();
    assert!(!v);
}

#[test]
fn legacy_booleans_off_by_default() {
    // Default ParserConfig has legacy_booleans=false, so `yes` is a string.
    let v: Value = from_str("yes").unwrap();
    assert_eq!(v.as_str(), Some("yes"));
}

#[test]
fn strict_booleans_rejects_case_variants() {
    let cfg = ParserConfig::new().strict_booleans(true);
    // Only lowercase `true` / `false` recognised.
    let v: Value = from_str_with_config("True", &cfg).unwrap();
    assert_eq!(v.as_str(), Some("True"));
    let v: Value = from_str_with_config("TRUE", &cfg).unwrap();
    assert_eq!(v.as_str(), Some("TRUE"));
    let v: Value = from_str_with_config("true", &cfg).unwrap();
    assert_eq!(v.as_bool(), Some(true));
}

// ── Integer coercion from whole-number float ─────────────────────────────

#[test]
fn integer_from_whole_float_is_accepted() {
    let v: i64 = from_str("42.0").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn u64_from_whole_float_is_accepted() {
    let v: u64 = from_str("42.0").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn string_from_integer_rejected() {
    // Typed String field must not silently coerce an integer.
    let result: Result<String, _> = from_str("42\n");
    assert!(result.is_err());
}

// ── Depth limits ────────────────────────────────────────────────────────

#[test]
fn max_depth_limit_enforced_on_mapping() {
    let config = ParserConfig::new().max_depth(3);
    let yaml = "a:\n  b:\n    c:\n      d: too deep\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("depth") || msg.contains("recursion"),
        "got: {msg}"
    );
}

#[test]
fn max_depth_limit_enforced_on_sequence() {
    let config = ParserConfig::new().max_depth(3);
    let yaml = "- - - - deep\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

// ── Document length limit ────────────────────────────────────────────────

#[test]
fn document_length_limit_reports_maximum() {
    let config = ParserConfig::new().max_document_length(10);
    let yaml = "this string is definitely more than 10 bytes long";
    let err = from_str_with_config::<Value>(yaml, &config).unwrap_err();
    assert!(err.to_string().contains("maximum length"));
}

// ── Sequence / mapping size limits ───────────────────────────────────────

#[test]
fn sequence_length_limit_enforced() {
    let config = ParserConfig::new().max_sequence_length(3);
    let mut yaml = String::new();
    for i in 0..10 {
        yaml.push_str(&format!("- {i}\n"));
    }
    assert!(from_str_with_config::<Value>(&yaml, &config).is_err());
}

#[test]
fn mapping_keys_limit_enforced() {
    let config = ParserConfig::new().max_mapping_keys(3);
    let mut yaml = String::new();
    for i in 0..10 {
        yaml.push_str(&format!("k{i}: {i}\n"));
    }
    assert!(from_str_with_config::<Value>(&yaml, &config).is_err());
}

// ── Empty YAML documents are valid (Value::Null) ─────────────────────────

#[test]
fn empty_stream_parses_as_null() {
    let v: Value = from_str("").unwrap();
    assert!(v.is_null());
}

#[test]
fn comment_only_stream_parses_as_null() {
    let v: Value = from_str("# just a comment\n").unwrap();
    assert!(v.is_null());
}

#[test]
fn directive_only_stream_parses_as_null() {
    let v: Value = from_str("---\n").unwrap();
    assert!(v.is_null());
}
