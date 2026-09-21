//! Tests for Phase 1 features.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{
    ParserConfig, SerializerConfig, Value, from_slice_with_config, from_str, from_str_with_config,
    to_fmt_writer, to_fmt_writer_with_config, to_string, to_string_with_config,
};

// ── 1.1: from_slice_with_config ─────────────────────────────────────────

#[test]
fn from_slice_with_config_basic() {
    let yaml = b"key: value\n";
    let config = ParserConfig::new();
    let v: Value = from_slice_with_config(yaml, &config).unwrap();
    assert_eq!(v["key"], Value::String("value".to_string()));
}

#[test]
fn from_slice_with_config_strict() {
    let yaml = b"a: 1\nb: 2\n";
    let config = ParserConfig::strict();
    let v: Value = from_slice_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"], Value::from(1));
}

#[test]
fn from_slice_with_config_invalid_utf8_returns_error() {
    let yaml: &[u8] = &[0xFF, 0xFE, 0x00];
    let config = ParserConfig::new();
    let result: Result<Value, _> = from_slice_with_config(yaml, &config);
    assert!(result.is_err());
}

// ── 1.2: strict_booleans ────────────────────────────────────────────────

#[test]
fn strict_booleans_false_accepts_case_variants() {
    let yaml = "a: True\nb: FALSE\n";
    let config = ParserConfig::new().strict_booleans(false);
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"], Value::Bool(true));
    assert_eq!(v["b"], Value::Bool(false));
}

#[test]
fn strict_booleans_true_rejects_case_variants() {
    let yaml = "a: True\nb: FALSE\nc: true\nd: false\n";
    let config = ParserConfig::new().strict_booleans(true);
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    // Case variants become strings
    assert_eq!(v["a"], Value::String("True".to_string()));
    assert_eq!(v["b"], Value::String("FALSE".to_string()));
    // Lowercase still works
    assert_eq!(v["c"], Value::Bool(true));
    assert_eq!(v["d"], Value::Bool(false));
}

#[test]
fn strict_booleans_in_strict_config() {
    let config = ParserConfig::strict();
    assert!(config.strict_booleans);
}

#[test]
fn strict_booleans_default_is_false() {
    let config = ParserConfig::new();
    assert!(!config.strict_booleans);
}

#[test]
fn strict_booleans_true_still_resolves_null() {
    let yaml = "a: null\nb: Null\nc: NULL\nd: ~\n";
    let config = ParserConfig::new().strict_booleans(true);
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"], Value::Null);
    assert_eq!(v["b"], Value::Null);
    assert_eq!(v["c"], Value::Null);
    assert_eq!(v["d"], Value::Null);
}

// ── 1.4: to_fmt_writer ─────────────────────────────────────────────────

#[test]
fn to_fmt_writer_basic() {
    let value = Value::from(42);
    let mut output = String::new();
    to_fmt_writer(&mut output, &value).unwrap();
    assert!(output.contains("42"));
}

#[test]
fn to_fmt_writer_with_config_indent() {
    let yaml = "key: value";
    let v: Value = from_str(yaml).unwrap();
    let config = SerializerConfig::new().indent(4);
    let mut output = String::new();
    to_fmt_writer_with_config(&mut output, &v, &config).unwrap();
    assert!(output.contains("key: value"));
}

#[test]
fn to_fmt_writer_mapping() {
    let yaml = "a: 1\nb: 2\n";
    let v: Value = from_str(yaml).unwrap();
    let mut output = String::new();
    to_fmt_writer(&mut output, &v).unwrap();
    assert!(output.contains("a: 1"));
    assert!(output.contains("b: 2"));
}

// ── 1.5: SerializerConfig knobs ─────────────────────────────────────────

#[test]
fn quote_all_forces_quoting() {
    let v = Value::String("hello".to_string());
    let config = SerializerConfig::new().quote_all(true);
    let yaml = to_string_with_config(&v, &config).unwrap();
    assert!(
        yaml.contains('\'') || yaml.contains('"'),
        "quote_all should force quoting: {yaml}"
    );
}

#[test]
fn quote_all_false_no_unnecessary_quoting() {
    let v = Value::String("hello".to_string());
    let config = SerializerConfig::new().quote_all(false);
    let yaml = to_string_with_config(&v, &config).unwrap();
    assert_eq!(yaml.trim(), "hello");
}

#[test]
fn quote_all_empty_string_still_quoted() {
    let v = Value::String(String::new());
    let config = SerializerConfig::new().quote_all(true);
    let yaml = to_string_with_config(&v, &config).unwrap();
    assert!(yaml.contains("\"\"") || yaml.contains("''"));
}

#[test]
fn quote_all_roundtrip() {
    let v = Value::String("hello world".to_string());
    let config = SerializerConfig::new().quote_all(true);
    let yaml = to_string_with_config(&v, &config).unwrap();
    let parsed: Value = from_str(&yaml).unwrap();
    assert_eq!(parsed, v);
}

#[test]
fn config_builder_compact_list_indent() {
    let config = SerializerConfig::new().compact_list_indent(true);
    assert!(config.compact_list_indent);
}

#[test]
fn config_builder_folded_wrap_chars() {
    let config = SerializerConfig::new().folded_wrap_chars(120);
    assert_eq!(config.folded_wrap_chars, 120);
}

#[test]
fn config_builder_min_fold_chars() {
    let config = SerializerConfig::new().min_fold_chars(40);
    assert_eq!(config.min_fold_chars, 40);
}

#[test]
fn config_defaults() {
    let config = SerializerConfig::new();
    assert!(!config.quote_all);
    assert!(!config.compact_list_indent);
    assert_eq!(config.folded_wrap_chars, 80);
    assert_eq!(config.min_fold_chars, 80);
}

// ── Roundtrip with new configs ──────────────────────────────────────────

#[test]
fn roundtrip_with_strict_booleans() {
    let yaml = "enabled: true\ndisabled: false\n";
    let config = ParserConfig::new().strict_booleans(true);
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["enabled"], Value::Bool(true));

    let output = to_string(&v).unwrap();
    let v2: Value = from_str(&output).unwrap();
    assert_eq!(v, v2);
}
