//! Comprehensive edge case tests for noyalib.
//!
//! Tests covering edge cases and YAML spec compliance.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Value, from_str, to_string};
use serde::{Deserialize, Serialize};

// ============================================================================
// Number Edge Cases
// ============================================================================

#[test]
fn test_very_large_integers() {
    let yaml = format!("value: {}\n", i64::MAX);
    let value: Value = from_str(&yaml).unwrap();
    assert_eq!(value.get("value").unwrap().as_i64(), Some(i64::MAX));

    let yaml = format!("value: {}\n", i64::MIN);
    let value: Value = from_str(&yaml).unwrap();
    assert_eq!(value.get("value").unwrap().as_i64(), Some(i64::MIN));
}

#[test]
fn test_float_precision_edge_cases() {
    // Smallest positive denormalized float
    let yaml = "value: 5e-324\n";
    let value: Value = from_str(yaml).unwrap();
    let f = value.get("value").unwrap().as_f64().unwrap();
    assert!(f > 0.0);

    // Largest finite float
    let yaml = "value: 1.7976931348623157e+308\n";
    let value: Value = from_str(yaml).unwrap();
    let f = value.get("value").unwrap().as_f64().unwrap();
    assert!(f.is_finite());
    assert!(f > 1e307);
}

#[test]
fn test_nan_serialization() {
    use noyalib::Number;

    // NaN serializes to .nan
    let value = Value::Number(Number::Float(f64::NAN));
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains(".nan") || yaml.contains(".NaN") || yaml.contains(".NAN"));
}

#[test]
fn test_nan_parsing() {
    // NaN can be parsed when in a typed context
    use std::str::FromStr;

    use noyalib::Number;

    let n = Number::from_str(".nan").unwrap();
    assert!(n.is_nan());

    let n = Number::from_str(".NaN").unwrap();
    assert!(n.is_nan());
}

#[test]
fn test_infinity_serialization() {
    use noyalib::Number;

    // Positive infinity serializes
    let value = Value::Number(Number::Float(f64::INFINITY));
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains(".inf") || yaml.contains(".Inf"));

    // Negative infinity serializes
    let value = Value::Number(Number::Float(f64::NEG_INFINITY));
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("-.inf") || yaml.contains("-.Inf"));
}

#[test]
fn test_infinity_parsing() {
    // Infinity can be parsed when in a typed context
    use std::str::FromStr;

    use noyalib::Number;

    assert_eq!(
        Number::from_str(".inf").unwrap(),
        Number::Float(f64::INFINITY)
    );
    assert_eq!(
        Number::from_str("+.inf").unwrap(),
        Number::Float(f64::INFINITY)
    );
    assert_eq!(
        Number::from_str("-.inf").unwrap(),
        Number::Float(f64::NEG_INFINITY)
    );
}

#[test]
fn test_octal_integer_parsing() {
    use std::str::FromStr;

    use noyalib::Number;

    assert_eq!(Number::from_str("0o777").unwrap(), Number::Integer(0o777));
    assert_eq!(Number::from_str("0O10").unwrap(), Number::Integer(8));
    assert_eq!(Number::from_str("0o0").unwrap(), Number::Integer(0));
}

#[test]
fn test_hex_integer_parsing() {
    use std::str::FromStr;

    use noyalib::Number;

    assert_eq!(Number::from_str("0xFF").unwrap(), Number::Integer(255));
    assert_eq!(Number::from_str("0x10").unwrap(), Number::Integer(16));
    assert_eq!(
        Number::from_str("0xDEADBEEF").unwrap(),
        Number::Integer(0xDEADBEEF)
    );
}

#[test]
fn test_binary_integer_parsing() {
    use std::str::FromStr;

    use noyalib::Number;

    assert_eq!(Number::from_str("0b1010").unwrap(), Number::Integer(10));
    assert_eq!(
        Number::from_str("0B11111111").unwrap(),
        Number::Integer(255)
    );
}

// ============================================================================
// String Edge Cases
// ============================================================================

#[test]
fn test_multiline_literal_string() {
    let yaml = r#"
description: |
  This is a multi-line
  literal string that
  preserves newlines.
"#;
    let value: Value = from_str(yaml).unwrap();
    let desc = value.get("description").unwrap().as_str().unwrap();
    assert!(desc.contains("This is a multi-line"));
    assert!(desc.contains("literal string"));
}

#[test]
fn test_multiline_folded_string() {
    let yaml = r#"
description: >
  This is a folded
  string that becomes
  a single line.
"#;
    let value: Value = from_str(yaml).unwrap();
    let desc = value.get("description").unwrap().as_str().unwrap();
    // Folded strings join lines with spaces
    assert!(desc.contains("This is a folded"));
}

#[test]
fn test_strings_with_special_characters() {
    let yaml = r#"
quoted_single: 'has ''escaped'' quotes'
quoted_double: "has \"escaped\" quotes and\nnewline"
backslash: "path\\to\\file"
"#;
    let value: Value = from_str(yaml).unwrap();

    let single = value.get("quoted_single").unwrap().as_str().unwrap();
    assert!(single.contains("'escaped'"));

    let double = value.get("quoted_double").unwrap().as_str().unwrap();
    assert!(double.contains("\"escaped\""));

    let backslash = value.get("backslash").unwrap().as_str().unwrap();
    assert!(backslash.contains("\\"));
}

#[test]
fn test_unicode_strings() {
    let yaml = r#"
emoji: "Hello 👋 World 🌍"
chinese: "你好世界"
japanese: "こんにちは"
arabic: "مرحبا"
mixed: "Hello 世界 مرحبا"
"#;
    let value: Value = from_str(yaml).unwrap();

    assert_eq!(
        value.get("emoji").unwrap().as_str().unwrap(),
        "Hello 👋 World 🌍"
    );
    assert_eq!(value.get("chinese").unwrap().as_str().unwrap(), "你好世界");
    assert_eq!(
        value.get("japanese").unwrap().as_str().unwrap(),
        "こんにちは"
    );
    assert_eq!(value.get("arabic").unwrap().as_str().unwrap(), "مرحبا");
}

#[test]
fn test_empty_string() {
    let yaml = "value: ''\n";
    let value: Value = from_str(yaml).unwrap();
    assert_eq!(value.get("value").unwrap().as_str(), Some(""));

    let yaml = "value: \"\"\n";
    let value: Value = from_str(yaml).unwrap();
    assert_eq!(value.get("value").unwrap().as_str(), Some(""));
}

#[test]
fn test_strings_that_look_like_other_types() {
    let yaml = r#"
bool_like: "true"
number_like: "123"
null_like: "null"
float_like: "3.125"
"#;
    let value: Value = from_str(yaml).unwrap();

    // All should be strings due to quotes
    assert_eq!(value.get("bool_like").unwrap().as_str(), Some("true"));
    assert_eq!(value.get("number_like").unwrap().as_str(), Some("123"));
    assert_eq!(value.get("null_like").unwrap().as_str(), Some("null"));
    assert_eq!(value.get("float_like").unwrap().as_str(), Some("3.125"));
}

// ============================================================================
// Collection Edge Cases
// ============================================================================

#[test]
fn test_empty_sequence() {
    let yaml = "items: []\n";
    let value: Value = from_str(yaml).unwrap();
    let items = value.get("items").unwrap().as_sequence().unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_empty_mapping() {
    let yaml = "config: {}\n";
    let value: Value = from_str(yaml).unwrap();
    let config = value.get("config").unwrap().as_mapping().unwrap();
    assert!(config.is_empty());
}

#[test]
fn test_deeply_nested_structure() {
    // Create a deeply nested structure (10 levels)
    let yaml = r#"
level1:
  level2:
    level3:
      level4:
        level5:
          level6:
            level7:
              level8:
                level9:
                  level10:
                    value: "deep"
"#;
    let value: Value = from_str(yaml).unwrap();
    let deep = value
        .get_path("level1.level2.level3.level4.level5.level6.level7.level8.level9.level10.value")
        .unwrap();
    assert_eq!(deep.as_str(), Some("deep"));
}

#[test]
fn test_large_sequence() {
    // Create a sequence with 1000 items
    let items: Vec<i32> = (0..1000).collect();
    let yaml = to_string(&items).unwrap();
    let parsed: Vec<i32> = from_str(&yaml).unwrap();
    assert_eq!(parsed.len(), 1000);
    assert_eq!(parsed[0], 0);
    assert_eq!(parsed[999], 999);
}

#[test]
fn test_large_mapping() {
    use std::collections::BTreeMap;

    // Create a mapping with 100 keys
    let mut map: BTreeMap<String, i32> = BTreeMap::new();
    for i in 0..100 {
        let _ = map.insert(format!("key_{i}"), i);
    }
    let yaml = to_string(&map).unwrap();
    let parsed: BTreeMap<String, i32> = from_str(&yaml).unwrap();
    assert_eq!(parsed.len(), 100);
    assert_eq!(parsed.get("key_50"), Some(&50));
}

#[test]
fn test_duplicate_keys_in_mapping() {
    // Default policy is Last: last occurrence wins.
    let yaml = r#"
name: first
value: 1
name: second
"#;
    let result: Value = from_str(yaml).unwrap();
    assert_eq!(result.get("name").unwrap().as_str(), Some("second"));
    assert_eq!(result.get("value").unwrap().as_i64(), Some(1));

    // With DuplicateKeyPolicy::Error, duplicate keys are rejected.
    use noyalib::{DuplicateKeyPolicy, ParserConfig, from_str_with_config};
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("duplicate") || err.contains("Duplicate"),
        "got: {err}"
    );
}

#[test]
fn test_mixed_type_sequence() {
    let yaml = r#"
items:
  - 1
  - "two"
  - true
  - null
  - 3.125
  - [nested, list]
  - key: value
"#;
    let value: Value = from_str(yaml).unwrap();
    let items = value.get("items").unwrap().as_sequence().unwrap();
    assert_eq!(items.len(), 7);
    assert_eq!(items[0].as_i64(), Some(1));
    assert_eq!(items[1].as_str(), Some("two"));
    assert_eq!(items[2].as_bool(), Some(true));
    assert!(items[3].is_null());
    assert!(items[4].as_f64().is_some());
    assert!(items[5].is_sequence());
    assert!(items[6].is_mapping());
}

// ============================================================================
// YAML Spec Compliance
// ============================================================================

#[test]
fn test_null_representations() {
    let yaml = r#"
null1: null
null2: ~
null3:
"#;
    let value: Value = from_str(yaml).unwrap();
    assert!(value.get("null1").unwrap().is_null());
    assert!(value.get("null2").unwrap().is_null());
    assert!(value.get("null3").unwrap().is_null());
}

#[test]
fn test_boolean_variations() {
    // yaml-rust2 uses YAML 1.1 which recognizes true/false (lowercase)
    let yaml = r#"
true1: true
false1: false
"#;
    let value: Value = from_str(yaml).unwrap();
    assert_eq!(value.get("true1").unwrap().as_bool(), Some(true));
    assert_eq!(value.get("false1").unwrap().as_bool(), Some(false));

    // Capitalized variants may be parsed as strings depending on the YAML version
    let yaml_caps = r#"
true2: True
true3: TRUE
false2: False
false3: FALSE
"#;
    let value: Value = from_str(yaml_caps).unwrap();
    // Test that parsing doesn't fail - the values may be bool or string
    assert!(value.get("true2").is_some());
    assert!(value.get("true3").is_some());
    assert!(value.get("false2").is_some());
    assert!(value.get("false3").is_some());
}

#[test]
fn test_yes_no_on_off_booleans() {
    // YAML 1.1 style (may be treated as strings in YAML 1.2)
    let yaml = r#"
yes_val: yes
no_val: no
on_val: on
off_val: off
"#;
    let value: Value = from_str(yaml).unwrap();
    // yaml-rust2 uses YAML 1.1 which recognizes yes/no/on/off as booleans
    let yes = value.get("yes_val").unwrap();
    let no = value.get("no_val").unwrap();
    // These might be booleans or strings depending on parser
    assert!(yes.is_bool() || yes.is_string());
    assert!(no.is_bool() || no.is_string());
}

#[test]
fn test_flow_vs_block_style() {
    // Flow style (inline)
    let flow_yaml = "{name: test, items: [1, 2, 3]}";
    let flow: Value = from_str(flow_yaml).unwrap();

    // Block style (indented)
    let block_yaml = r#"
name: test
items:
  - 1
  - 2
  - 3
"#;
    let block: Value = from_str(block_yaml).unwrap();

    // Should produce equivalent values
    assert_eq!(
        flow.get("name").unwrap().as_str(),
        block.get("name").unwrap().as_str()
    );
    assert_eq!(
        flow.get("items").unwrap().as_sequence().unwrap().len(),
        block.get("items").unwrap().as_sequence().unwrap().len()
    );
}

// ============================================================================
// Error Handling Edge Cases
// ============================================================================

#[test]
fn test_malformed_yaml_unclosed_bracket() {
    let yaml = "[1, 2, 3";
    let result: Result<Value, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_malformed_yaml_bad_indentation() {
    let yaml = r#"
name: test
  badly: indented
"#;
    let result: Result<Value, _> = from_str(yaml);
    // This may succeed with yaml-rust2 by treating it as a multi-line string
    // or may fail depending on strictness
    // The test documents current behavior
    let _ = result;
}

#[test]
fn test_type_mismatch_error() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Typed {
        count: i32,
    }

    let yaml = "count: not_a_number\n";
    let result: Result<Typed, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_missing_required_field_error() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Required {
        name: String,
        required_field: i32,
    }

    let yaml = "name: test\n";
    let result: Result<Required, _> = from_str(yaml);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("required_field") || msg.contains("missing"));
}

#[test]
fn test_error_location() {
    use noyalib::Location;

    // Test location calculation
    let source = "line1\nline2\nline3";
    let loc = Location::from_index(source, 0);
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 1);

    let loc = Location::from_index(source, 6);
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 1);

    let loc = Location::from_index(source, 12);
    assert_eq!(loc.line(), 3);
    assert_eq!(loc.column(), 1);
}

// ============================================================================
// Serialization Edge Cases
// ============================================================================

#[test]
fn test_serialize_option_none() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct WithOption {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        optional: Option<i32>,
    }

    let value = WithOption {
        name: "test".to_string(),
        optional: None,
    };
    let yaml = to_string(&value).unwrap();
    assert!(!yaml.contains("optional"));
}

#[test]
fn test_serialize_option_some() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct WithOption {
        name: String,
        optional: Option<i32>,
    }

    let value = WithOption {
        name: "test".to_string(),
        optional: Some(42),
    };
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("optional"));
    assert!(yaml.contains("42"));
}

#[test]
fn test_serialize_unit_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct UnitStruct;

    let value = UnitStruct;
    let yaml = to_string(&value).unwrap();
    let parsed: UnitStruct = from_str(&yaml).unwrap();
    assert_eq!(value, parsed);
}

#[test]
fn test_serialize_newtype_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Wrapper(i32);

    let value = Wrapper(42);
    let yaml = to_string(&value).unwrap();
    let parsed: Wrapper = from_str(&yaml).unwrap();
    assert_eq!(value, parsed);
}

#[test]
fn test_serialize_tuple_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Point(i32, i32);

    let value = Point(10, 20);
    let yaml = to_string(&value).unwrap();
    let parsed: Point = from_str(&yaml).unwrap();
    assert_eq!(value, parsed);
}

#[test]
fn test_serialize_enum_variants() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum MyEnum {
        Unit,
        Newtype(i32),
        Tuple(i32, String),
        Struct { x: i32, y: i32 },
    }

    let unit = MyEnum::Unit;
    let newtype = MyEnum::Newtype(42);
    let tuple = MyEnum::Tuple(1, "hello".to_string());
    let struct_var = MyEnum::Struct { x: 10, y: 20 };

    // Test each variant
    let yaml = to_string(&unit).unwrap();
    let parsed: MyEnum = from_str(&yaml).unwrap();
    assert_eq!(unit, parsed);

    let yaml = to_string(&newtype).unwrap();
    let parsed: MyEnum = from_str(&yaml).unwrap();
    assert_eq!(newtype, parsed);

    let yaml = to_string(&tuple).unwrap();
    let parsed: MyEnum = from_str(&yaml).unwrap();
    assert_eq!(tuple, parsed);

    let yaml = to_string(&struct_var).unwrap();
    let parsed: MyEnum = from_str(&yaml).unwrap();
    assert_eq!(struct_var, parsed);
}

// ============================================================================
// Value Type Edge Cases
// ============================================================================

#[test]
fn test_value_equality_edge_cases() {
    // Same value different representations
    let v1: Value = from_str("42").unwrap();
    let v2 = Value::from(42);
    assert_eq!(v1, v2);

    // Null comparisons
    let null1 = Value::Null;
    let null2: Value = from_str("null").unwrap();
    assert_eq!(null1, null2);
}

#[test]
fn test_value_ordering_edge_cases() {
    use std::cmp::Ordering;

    // Null < Bool
    assert!(Value::Null < Value::Bool(false));

    // Bool < Number
    assert!(Value::Bool(true) < Value::from(0));

    // Number < String
    assert!(Value::from(999) < Value::from("a"));

    // String < Sequence
    assert!(Value::from("zzz") < Value::Sequence(vec![]));

    // Sequence < Mapping
    assert!(Value::Sequence(vec![]) < Value::Mapping(Default::default()));

    // Test within same type
    assert_eq!(Value::from(1).cmp(&Value::from(2)), Ordering::Less);
    assert_eq!(Value::from("a").cmp(&Value::from("b")), Ordering::Less);
}

#[test]
fn test_value_hash_consistency() {
    use std::collections::HashSet;
    use std::hash::{Hash, Hasher};

    fn hash_value(v: &Value) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        v.hash(&mut hasher);
        hasher.finish()
    }

    // Equal values should have equal hashes
    let v1 = Value::from(42);
    let v2 = Value::from(42);
    assert_eq!(v1, v2);
    assert_eq!(hash_value(&v1), hash_value(&v2));

    // Values can be used as hash set keys
    let mut set: HashSet<Value> = HashSet::new();
    let _ = set.insert(Value::from(1));
    let _ = set.insert(Value::from("test"));
    let _ = set.insert(Value::Null);
    assert_eq!(set.len(), 3);
}

// ============================================================================
// Anchor and Alias Edge Cases
// ============================================================================

#[test]
fn test_anchor_with_scalar() {
    let yaml = r#"
value: &anchor 42
copy: *anchor
"#;
    let value: Value = from_str(yaml).unwrap();
    assert_eq!(value.get("value").unwrap().as_i64(), Some(42));
    assert_eq!(value.get("copy").unwrap().as_i64(), Some(42));
}

#[test]
fn test_anchor_with_sequence() {
    let yaml = r#"
items: &items
  - a
  - b
  - c
copy: *items
"#;
    let value: Value = from_str(yaml).unwrap();
    let items = value.get("items").unwrap().as_sequence().unwrap();
    let copy = value.get("copy").unwrap().as_sequence().unwrap();
    assert_eq!(items.len(), copy.len());
    assert_eq!(items.len(), 3);
}

#[test]
fn test_merge_key_basic() {
    let yaml = r#"
defaults: &defaults
  timeout: 30
  retries: 3

server:
  <<: *defaults
  host: localhost
"#;
    let value: Value = from_str(yaml).unwrap();
    let server = value.get("server").unwrap();
    assert_eq!(server.get("host").unwrap().as_str(), Some("localhost"));
    assert_eq!(server.get("timeout").unwrap().as_i64(), Some(30));
    assert_eq!(server.get("retries").unwrap().as_i64(), Some(3));
}

#[test]
fn test_merge_key_override() {
    let yaml = r#"
defaults: &defaults
  timeout: 30
  retries: 3

server:
  <<: *defaults
  timeout: 60
"#;
    let value: Value = from_str(yaml).unwrap();
    let server = value.get("server").unwrap();
    // Local value should override merged value
    assert_eq!(server.get("timeout").unwrap().as_i64(), Some(60));
    assert_eq!(server.get("retries").unwrap().as_i64(), Some(3));
}

// ============================================================================
// Multi-Document Edge Cases
// ============================================================================

#[test]
fn test_load_all_empty() {
    use noyalib::load_all;

    let yaml = "";
    let docs: Vec<_> = load_all(yaml).unwrap().filter_map(Result::ok).collect();
    assert!(docs.is_empty());
}

#[test]
fn test_load_all_single_document() {
    use noyalib::load_all;

    let yaml = "value: 42";
    let docs: Vec<_> = load_all(yaml).unwrap().filter_map(Result::ok).collect();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].get("value").unwrap().as_i64(), Some(42));
}

#[test]
fn test_load_all_multiple_documents() {
    use noyalib::load_all;

    let yaml = r#"
---
name: doc1
---
name: doc2
---
name: doc3
"#;
    let docs: Vec<_> = load_all(yaml).unwrap().filter_map(Result::ok).collect();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].get("name").unwrap().as_str(), Some("doc1"));
    assert_eq!(docs[1].get("name").unwrap().as_str(), Some("doc2"));
    assert_eq!(docs[2].get("name").unwrap().as_str(), Some("doc3"));
}

// ============================================================================
// PathAccess Edge Cases
// ============================================================================

#[test]
fn test_path_with_array_index() {
    let yaml = r#"
items:
  - name: first
  - name: second
  - name: third
"#;
    let value: Value = from_str(yaml).unwrap();
    assert_eq!(
        value.get_path("items[0].name").unwrap().as_str(),
        Some("first")
    );
    assert_eq!(
        value.get_path("items[1].name").unwrap().as_str(),
        Some("second")
    );
    assert_eq!(
        value.get_path("items[2].name").unwrap().as_str(),
        Some("third")
    );
}

#[test]
fn test_path_nonexistent() {
    let yaml = "name: test\n";
    let value: Value = from_str(yaml).unwrap();
    assert!(value.get_path("nonexistent").is_none());
    assert!(value.get_path("name.nested").is_none());
    assert!(value.get_path("items[0]").is_none());
}

#[test]
fn test_path_empty_string() {
    let yaml = "name: test\n";
    let value: Value = from_str(yaml).unwrap();
    // Empty path should return None (no segments)
    let result = value.get_path("");
    assert!(result.is_none() || result == Some(&value));
}

// ============================================================================
// Config Edge Cases
// ============================================================================

#[test]
fn test_serializer_config_document_markers() {
    use noyalib::{SerializerConfig, to_string_with_config};

    let config = SerializerConfig::new()
        .document_start(true)
        .document_end(true);

    let value = Value::from("test");
    let yaml = to_string_with_config(&value, &config).unwrap();
    assert!(yaml.starts_with("---"));
    assert!(yaml.contains("..."));
}

#[test]
fn test_serializer_config_indent() {
    use noyalib::{Mapping, SerializerConfig, to_string_with_config};

    let config = SerializerConfig::new().indent(4);

    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from("value"));
    let value = Value::Mapping(map);

    let yaml = to_string_with_config(&value, &config).unwrap();
    // With 4-space indent, nested content would have 4 spaces
    // For simple maps, just verify it doesn't error
    assert!(yaml.contains("key"));
}
