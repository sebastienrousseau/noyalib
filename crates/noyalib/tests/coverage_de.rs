//! Deserializer coverage tests — security limits, config paths, edge cases.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::io::Cursor;

use noyalib::{
    DuplicateKeyPolicy, ParserConfig, Value, from_reader_with_config, from_str,
    from_str_with_config, from_value,
};

// ============================================================================
// ParserConfig security limits
// ============================================================================

#[test]
fn max_depth_exceeded() {
    let yaml = "a:\n  b:\n    c:\n      d:\n        e:\n          f: 1\n";
    let config = ParserConfig::new().max_depth(3);
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("recursion") || err.contains("depth"),
        "got: {err}"
    );
}

#[test]
fn max_document_length_exceeded() {
    let config = ParserConfig::new().max_document_length(10);
    let yaml = "this string is definitely more than 10 bytes long";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("maximum length"), "got: {err}");
}

#[test]
fn max_sequence_length_exceeded() {
    let mut yaml = String::new();
    for i in 0..20 {
        yaml.push_str(&format!("- {i}\n"));
    }
    let config = ParserConfig::new().max_sequence_length(5);
    let result: Result<Value, _> = from_str_with_config(&yaml, &config);
    assert!(result.is_err());
}

#[test]
fn max_mapping_keys_exceeded() {
    let mut yaml = String::new();
    for i in 0..20 {
        yaml.push_str(&format!("key{i}: {i}\n"));
    }
    let config = ParserConfig::new().max_mapping_keys(5);
    let result: Result<Value, _> = from_str_with_config(&yaml, &config);
    assert!(result.is_err());
}

// ============================================================================
// from_reader_with_config
// ============================================================================

#[test]
fn from_reader_with_config_basic() {
    let yaml = "key: value\n";
    let reader = Cursor::new(yaml);
    let config = ParserConfig::new();
    let v: Value = from_reader_with_config(reader, &config).unwrap();
    assert_eq!(v.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn from_reader_with_config_length_exceeded() {
    let yaml = "this is a long string that exceeds the limit";
    let reader = Cursor::new(yaml);
    let config = ParserConfig::new().max_document_length(10);
    let result: Result<Value, _> = from_reader_with_config(reader, &config);
    assert!(result.is_err());
}

// ============================================================================
// DuplicateKeyPolicy variants
// ============================================================================

#[test]
fn duplicate_key_policy_default_is_last() {
    let config = ParserConfig::new();
    assert_eq!(config.duplicate_key_policy, DuplicateKeyPolicy::Last);
}

#[test]
fn duplicate_key_policy_strict_is_error() {
    let config = ParserConfig::strict();
    assert_eq!(config.duplicate_key_policy, DuplicateKeyPolicy::Error);
}

#[test]
fn parser_config_builder_all_fields() {
    let config = ParserConfig::new()
        .max_depth(10)
        .max_document_length(1000)
        .max_alias_expansions(50)
        .max_mapping_keys(100)
        .max_sequence_length(100)
        .duplicate_key_policy(DuplicateKeyPolicy::First);

    assert_eq!(config.max_depth, 10);
    assert_eq!(config.max_document_length, 1000);
    assert_eq!(config.max_alias_expansions, 50);
    assert_eq!(config.max_mapping_keys, 100);
    assert_eq!(config.max_sequence_length, 100);
    assert_eq!(config.duplicate_key_policy, DuplicateKeyPolicy::First);
}

// ============================================================================
// Deserializer edge cases
// ============================================================================

#[test]
fn deserialize_empty_document() {
    let result: Result<i64, _> = from_str("");
    assert!(result.is_err());
}

#[test]
fn deserialize_unit_from_null() {
    let v: () = from_str("~").unwrap();
    assert_eq!(v, ());
}

#[test]
fn deserialize_char() {
    let c: char = from_str("x").unwrap();
    assert_eq!(c, 'x');
}

#[test]
fn deserialize_char_too_long() {
    let result: Result<char, _> = from_str("ab");
    assert!(result.is_err());
}

#[test]
fn deserialize_bytes() {
    // Deserialize via Value
    let v: Value = from_str("hello").unwrap();
    assert_eq!(v.as_str(), Some("hello"));
}

#[test]
fn deserialize_option_none() {
    let v: Option<i64> = from_str("~").unwrap();
    assert!(v.is_none());
}

#[test]
fn deserialize_option_some() {
    let v: Option<i64> = from_str("42").unwrap();
    assert_eq!(v, Some(42));
}

#[test]
fn deserialize_ignored_any() {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Partial {
        name: String,
    }

    let v: Partial = from_str("name: test\nextra: ignored\nmore: also_ignored\n").unwrap();
    assert_eq!(v.name, "test");
}

#[test]
fn deserialize_enum_unit() {
    use serde::Deserialize;
    #[derive(Debug, Deserialize, PartialEq)]
    enum Color {
        Red,
        Green,
        Blue,
    }

    let v: Color = from_str("Red").unwrap();
    assert_eq!(v, Color::Red);
}

#[test]
fn deserialize_enum_newtype() {
    use serde::Deserialize;
    #[derive(Debug, Deserialize, PartialEq)]
    enum Wrapper {
        Int(i64),
    }

    let v: Wrapper = from_str("Int: 42\n").unwrap();
    assert_eq!(v, Wrapper::Int(42));
}

#[test]
fn deserialize_enum_struct() {
    use serde::Deserialize;
    #[derive(Debug, Deserialize, PartialEq)]
    enum Shape {
        Circle { radius: f64 },
    }

    let v: Shape = from_str("Circle:\n  radius: 2.75\n").unwrap();
    match v {
        Shape::Circle { radius } => assert!((radius - 2.75).abs() < 0.01),
    }
}

#[test]
fn deserialize_enum_tuple() {
    use serde::Deserialize;
    #[derive(Debug, Deserialize, PartialEq)]
    enum Pair {
        Point(f64, f64),
    }

    let v: Pair = from_str("Point:\n  - 1.0\n  - 2.0\n").unwrap();
    assert_eq!(v, Pair::Point(1.0, 2.0));
}

#[test]
fn from_value_struct() {
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Config {
        name: String,
        port: u16,
    }

    let mut m = noyalib::Mapping::new();
    let _ = m.insert("name", Value::from("test"));
    let _ = m.insert("port", Value::from(8080));
    let v = Value::Mapping(m);

    let config: Config = from_value(&v).unwrap();
    assert_eq!(config.name, "test");
    assert_eq!(config.port, 8080);
}

#[test]
fn type_mismatch_errors() {
    assert!(from_str::<bool>("42").is_err());
    assert!(from_str::<i64>("hello").is_err());
    assert!(from_str::<f64>("not_a_number").is_err());
    assert!(from_str::<String>("- 1\n- 2\n").is_err());
    assert!(from_str::<Vec<i64>>("key: value\n").is_err());
    assert!(from_str::<std::collections::HashMap<String, i64>>("- 1\n- 2\n").is_err());
    assert!(from_str::<()>("42").is_err());
}

#[test]
fn float_from_integer() {
    // Integer value should be coercible to f64
    let v: f64 = from_str("42").unwrap();
    assert!((v - 42.0).abs() < 0.001);
}

#[test]
fn integer_from_float_whole() {
    // Float with no fractional part should be coercible to integer
    let v: i64 = from_str("42.0").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn u64_from_positive_integer() {
    let v: u64 = from_str("42").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn u64_from_negative_integer_fails() {
    let result: Result<u64, _> = from_str("-1");
    assert!(result.is_err());
}

#[test]
fn u64_from_float() {
    let v: u64 = from_str("42.0").unwrap();
    assert_eq!(v, 42);
}
