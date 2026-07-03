//! Serde integration tests for noyalib.
//!
//! Comprehensive round-trip tests ported from serde_yml.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::BTreeMap;

#[cfg(feature = "lossless-u64")]
use noyalib::{ParserConfig, from_str_with_config};
use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

/// Helper for round-trip testing
fn test_serde<T>(value: &T, expected_yaml_contains: &[&str])
where
    T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug + 'static,
{
    let yaml = to_string(value).unwrap();
    for expected in expected_yaml_contains {
        assert!(
            yaml.contains(expected),
            "Expected YAML to contain '{}', but got:\n{}",
            expected,
            yaml
        );
    }
    let parsed: T = from_str(&yaml).unwrap();
    assert_eq!(*value, parsed);
}

// ============================================================================
// Primitive Types
// ============================================================================

#[test]
fn test_serde_bool() {
    test_serde(&true, &["true"]);
    test_serde(&false, &["false"]);
}

#[test]
fn test_serde_integers() {
    test_serde(&42i8, &["42"]);
    test_serde(&42i16, &["42"]);
    test_serde(&42i32, &["42"]);
    test_serde(&42i64, &["42"]);
    test_serde(&-42i32, &["-42"]);
}

#[test]
fn test_serde_unsigned() {
    test_serde(&42u8, &["42"]);
    test_serde(&42u16, &["42"]);
    test_serde(&42u32, &["42"]);
    test_serde(&42u64, &["42"]);
}

#[cfg(feature = "lossless-u64")]
#[test]
fn test_serde_lossless_u64_struct_field() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Record {
        id: u64,
    }

    let cfg = ParserConfig::new().lossless_u64_integers(true);
    for id in [i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX] {
        let value = Record { id };
        let yaml = to_string(&value).unwrap();
        assert!(yaml.contains(&format!("id: {id}")));
        let parsed: Record = from_str_with_config(&yaml, &cfg).unwrap();
        assert_eq!(parsed, value);
    }
}

#[test]
fn test_serde_floats() {
    test_serde(&3.125f32, &["3.125"]);
    test_serde(&3.125f64, &["3.125"]);
    test_serde(&-3.125f64, &["-3.125"]);
}

#[test]
fn test_serde_char() {
    test_serde(&'a', &["a"]);
    test_serde(&'Z', &["Z"]);
}

#[test]
fn test_serde_string() {
    test_serde(&"hello".to_string(), &["hello"]);
    test_serde(&"hello world".to_string(), &["hello world"]);
}

// ============================================================================
// Collections
// ============================================================================

#[test]
fn test_serde_vec() {
    let vec = vec![1, 2, 3];
    test_serde(&vec, &["- 1", "- 2", "- 3"]);
}

#[test]
fn test_serde_vec_strings() {
    let vec = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    test_serde(&vec, &["- a", "- b", "- c"]);
}

#[test]
fn test_serde_nested_vec() {
    let vec = vec![vec![1, 2], vec![3, 4]];
    let yaml = to_string(&vec).unwrap();
    let parsed: Vec<Vec<i32>> = from_str(&yaml).unwrap();
    assert_eq!(vec, parsed);
}

#[test]
fn test_serde_btreemap() {
    let mut map = BTreeMap::new();
    let _ = map.insert("a".to_string(), 1);
    let _ = map.insert("b".to_string(), 2);
    test_serde(&map, &["a:", "b:"]);
}

// ============================================================================
// Structs
// ============================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn test_serde_simple_struct() {
    let point = Point { x: 10, y: 20 };
    test_serde(&point, &["x:", "y:"]);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Rectangle {
    top_left: Point,
    bottom_right: Point,
}

#[test]
fn test_serde_nested_struct() {
    let rect = Rectangle {
        top_left: Point { x: 0, y: 0 },
        bottom_right: Point { x: 100, y: 100 },
    };
    test_serde(&rect, &["top_left:", "bottom_right:"]);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WithOptional {
    required: String,
    optional: Option<String>,
}

#[test]
fn test_serde_optional_some() {
    let value = WithOptional {
        required: "req".to_string(),
        optional: Some("opt".to_string()),
    };
    test_serde(&value, &["required:", "optional:"]);
}

#[test]
fn test_serde_optional_none() {
    let value = WithOptional {
        required: "req".to_string(),
        optional: None,
    };
    let yaml = to_string(&value).unwrap();
    let parsed: WithOptional = from_str(&yaml).unwrap();
    assert_eq!(value, parsed);
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
struct WithDefault {
    value: i32,
    #[serde(default)]
    defaulted: i32,
}

#[test]
fn test_serde_default() {
    let yaml = "value: 42\n";
    let parsed: WithDefault = from_str(yaml).unwrap();
    assert_eq!(parsed.value, 42);
    assert_eq!(parsed.defaulted, 0);
}

// ============================================================================
// Enums
// ============================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn test_serde_unit_enum() {
    test_serde(&Color::Red, &["Red"]);
    test_serde(&Color::Green, &["Green"]);
    test_serde(&Color::Blue, &["Blue"]);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Wrapper {
    Int(i32),
    Str(String),
}

#[test]
fn test_serde_newtype_enum() {
    test_serde(&Wrapper::Int(42), &["Int:", "42"]);
    test_serde(&Wrapper::Str("hello".to_string()), &["Str:", "hello"]);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Shape {
    Circle { radius: f64 },
    Rectangle { width: i32, height: i32 },
}

#[test]
fn test_serde_struct_enum() {
    test_serde(&Shape::Circle { radius: 5.0 }, &["Circle:", "radius:"]);
    test_serde(
        &Shape::Rectangle {
            width: 10,
            height: 20,
        },
        &["Rectangle:", "width:", "height:"],
    );
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Tuple {
    Pair(i32, i32),
    Triple(i32, i32, i32),
}

#[test]
fn test_serde_tuple_enum() {
    let pair = Tuple::Pair(1, 2);
    let yaml = to_string(&pair).unwrap();
    let parsed: Tuple = from_str(&yaml).unwrap();
    assert_eq!(pair, parsed);

    let triple = Tuple::Triple(1, 2, 3);
    let yaml = to_string(&triple).unwrap();
    let parsed: Tuple = from_str(&yaml).unwrap();
    assert_eq!(triple, parsed);
}

// ============================================================================
// Tuple and Newtype Structs
// ============================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Newtype(i32);

#[test]
fn test_serde_newtype_struct() {
    test_serde(&Newtype(42), &["42"]);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TupleStruct(i32, String, bool);

#[test]
fn test_serde_tuple_struct() {
    let ts = TupleStruct(42, "hello".to_string(), true);
    let yaml = to_string(&ts).unwrap();
    let parsed: TupleStruct = from_str(&yaml).unwrap();
    assert_eq!(ts, parsed);
}

// ============================================================================
// Complex Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Config {
    name: String,
    version: u32,
    debug: bool,
    features: Vec<String>,
    limits: BTreeMap<String, u32>,
}

#[test]
fn test_serde_complex_config() {
    let config = Config {
        name: "my-app".to_string(),
        version: 1,
        debug: true,
        features: vec!["auth".to_string(), "api".to_string()],
        limits: {
            let mut m = BTreeMap::new();
            let _ = m.insert("max_connections".to_string(), 100);
            let _ = m.insert("timeout".to_string(), 30);
            m
        },
    };
    test_serde(
        &config,
        &["name:", "version:", "debug:", "features:", "limits:"],
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_serde_empty_string() {
    test_serde(&"".to_string(), &[]);
}

#[test]
fn test_serde_empty_vec() {
    let vec: Vec<i32> = vec![];
    let yaml = to_string(&vec).unwrap();
    assert!(yaml.contains("[]"));
    let parsed: Vec<i32> = from_str(&yaml).unwrap();
    assert_eq!(vec, parsed);
}

#[test]
fn test_serde_empty_map() {
    let map: BTreeMap<String, i32> = BTreeMap::new();
    let yaml = to_string(&map).unwrap();
    assert!(yaml.contains("{}"));
    let parsed: BTreeMap<String, i32> = from_str(&yaml).unwrap();
    assert_eq!(map, parsed);
}

// ============================================================================
// Unicode
// ============================================================================

#[test]
fn test_serde_unicode() {
    test_serde(&"日本語".to_string(), &["日本語"]);
    test_serde(&"Ελληνικά".to_string(), &["Ελληνικά"]);
    test_serde(&"العربية".to_string(), &["العربية"]);
}

#[test]
fn test_serde_emoji() {
    test_serde(&"🎉🎊🎈".to_string(), &["🎉🎊🎈"]);
}

// ============================================================================
// Special Strings (YAML edge cases)
// ============================================================================

#[test]
fn test_serde_string_with_colon() {
    let s = "key: value".to_string();
    let yaml = to_string(&s).unwrap();
    let parsed: String = from_str(&yaml).unwrap();
    assert_eq!(s, parsed);
}

#[test]
fn test_serde_numeric_string() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Data {
        value: String,
    }

    let data = Data {
        value: "123".to_string(),
    };
    let yaml = to_string(&data).unwrap();
    let parsed: Data = from_str(&yaml).unwrap();
    assert_eq!(data, parsed);
}

#[test]
fn test_serde_bool_like_string() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Data {
        value: String,
    }

    let data = Data {
        value: "true".to_string(),
    };
    let yaml = to_string(&data).unwrap();
    let parsed: Data = from_str(&yaml).unwrap();
    assert_eq!(data, parsed);
}
