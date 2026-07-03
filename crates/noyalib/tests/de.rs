//! Deserialization tests for noyalib.
//!
//! Ported from serde_yml test suite.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::BTreeMap;

use noyalib::{Number, Value, from_str, from_value};
#[cfg(feature = "lossless-u64")]
use noyalib::{ParserConfig, from_str_with_config};
use serde::Deserialize;

/// Helper function to test deserialization
fn test_de<T>(yaml: &str, expected: &T)
where
    T: for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug + 'static,
{
    let deserialized: T = from_str(yaml).unwrap();
    assert_eq!(*expected, deserialized);
}

// ============================================================================
// Basic Type Tests
// ============================================================================

#[test]
fn test_null() {
    let yaml = "null\n";
    let value: Value = from_str(yaml).unwrap();
    assert!(value.is_null());

    let yaml = "~\n";
    let value: Value = from_str(yaml).unwrap();
    assert!(value.is_null());
}

#[test]
fn test_bool() {
    // YAML 1.2 core schema only recognizes lowercase true/false
    let yaml = "true\n";
    let value: bool = from_str(yaml).unwrap();
    assert!(value);

    let yaml = "false\n";
    let value: bool = from_str(yaml).unwrap();
    assert!(!value);
}

#[test]
fn test_integers() {
    let yaml = "42\n";
    let value: i32 = from_str(yaml).unwrap();
    assert_eq!(value, 42);

    let yaml = "-42\n";
    let value: i32 = from_str(yaml).unwrap();
    assert_eq!(value, -42);

    let yaml = "0\n";
    let value: i32 = from_str(yaml).unwrap();
    assert_eq!(value, 0);

    // Large integers
    let yaml = "9223372036854775807\n";
    let value: i64 = from_str(yaml).unwrap();
    assert_eq!(value, i64::MAX);
}

#[cfg(feature = "lossless-u64")]
#[test]
fn test_lossless_u64_boundaries_with_config() {
    let cfg = ParserConfig::new().lossless_u64_integers(true);
    for value in [i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX] {
        let yaml = format!("{value}\n");
        let parsed: u64 = from_str_with_config(&yaml, &cfg).unwrap();
        assert_eq!(parsed, value);
    }
}

#[cfg(feature = "lossless-u64")]
#[test]
fn test_lossless_u64_rejects_overflow_with_config() {
    let cfg = ParserConfig::new().lossless_u64_integers(true);
    let err = from_str_with_config::<u64>("18446744073709551616\n", &cfg).unwrap_err();
    assert!(err.to_string().contains("unsigned integer") || err.to_string().contains("u64"));
}

#[cfg(feature = "lossless-u64")]
#[test]
fn test_from_value_unsigned_to_u64() {
    let value = Value::Number(Number::Unsigned(u64::MAX));
    let parsed: u64 = from_value(&value).unwrap();
    assert_eq!(parsed, u64::MAX);
}

#[test]
fn test_floats() {
    let yaml = "3.125\n";
    let value: f64 = from_str(yaml).unwrap();
    assert!((value - 3.125).abs() < 0.001);

    let yaml = "-3.125\n";
    let value: f64 = from_str(yaml).unwrap();
    assert!((value + 3.125).abs() < 0.001);

    let yaml = "0.0\n";
    let value: f64 = from_str(yaml).unwrap();
    assert!((value - 0.0).abs() < 0.001);
}

#[test]
fn test_strings() {
    let yaml = "hello\n";
    let value: String = from_str(yaml).unwrap();
    assert_eq!(value, "hello");

    let yaml = "\"hello world\"\n";
    let value: String = from_str(yaml).unwrap();
    assert_eq!(value, "hello world");

    let yaml = "'single quoted'\n";
    let value: String = from_str(yaml).unwrap();
    assert_eq!(value, "single quoted");
}

// ============================================================================
// Collection Tests
// ============================================================================

#[test]
fn test_sequence() {
    let yaml = "- 1\n- 2\n- 3\n";
    let expected = vec![1, 2, 3];
    test_de(yaml, &expected);
}

#[test]
fn test_sequence_inline() {
    let yaml = "[1, 2, 3]\n";
    let expected = vec![1, 2, 3];
    test_de(yaml, &expected);
}

#[test]
fn test_nested_sequence() {
    let yaml = "- - 1\n  - 2\n- - 3\n  - 4\n";
    let expected = vec![vec![1, 2], vec![3, 4]];
    test_de(yaml, &expected);
}

#[test]
fn test_mapping() {
    let yaml = "a: 1\nb: 2\nc: 3\n";
    let mut expected = BTreeMap::new();
    let _ = expected.insert("a".to_string(), 1);
    let _ = expected.insert("b".to_string(), 2);
    let _ = expected.insert("c".to_string(), 3);
    test_de(yaml, &expected);
}

#[test]
fn test_mapping_inline() {
    let yaml = "{a: 1, b: 2}\n";
    let mut expected = BTreeMap::new();
    let _ = expected.insert("a".to_string(), 1);
    let _ = expected.insert("b".to_string(), 2);
    test_de(yaml, &expected);
}

// ============================================================================
// Struct Tests
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
struct SimpleStruct {
    name: String,
    value: i32,
}

#[test]
fn test_simple_struct() {
    let yaml = "name: test\nvalue: 42\n";
    let expected = SimpleStruct {
        name: "test".to_string(),
        value: 42,
    };
    test_de(yaml, &expected);
}

#[derive(Debug, Deserialize, PartialEq)]
struct NestedStruct {
    outer: String,
    inner: SimpleStruct,
}

#[test]
fn test_nested_struct() {
    let yaml = "outer: hello\ninner:\n  name: test\n  value: 42\n";
    let expected = NestedStruct {
        outer: "hello".to_string(),
        inner: SimpleStruct {
            name: "test".to_string(),
            value: 42,
        },
    };
    test_de(yaml, &expected);
}

#[derive(Debug, Deserialize, PartialEq)]
struct OptionalFields {
    required: String,
    optional: Option<String>,
}

#[test]
fn test_optional_present() {
    let yaml = "required: hello\noptional: world\n";
    let expected = OptionalFields {
        required: "hello".to_string(),
        optional: Some("world".to_string()),
    };
    test_de(yaml, &expected);
}

#[test]
fn test_optional_missing() {
    let yaml = "required: hello\n";
    let expected = OptionalFields {
        required: "hello".to_string(),
        optional: None,
    };
    test_de(yaml, &expected);
}

#[test]
fn test_optional_null() {
    let yaml = "required: hello\noptional: null\n";
    let expected = OptionalFields {
        required: "hello".to_string(),
        optional: None,
    };
    test_de(yaml, &expected);
}

// ============================================================================
// Enum Tests
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
enum UnitEnum {
    A,
    B,
    C,
}

#[test]
fn test_unit_enum() {
    let yaml = "A\n";
    let expected = UnitEnum::A;
    test_de(yaml, &expected);
}

#[derive(Debug, Deserialize, PartialEq)]
enum NewtypeEnum {
    Text(String),
    Number(i32),
}

#[test]
fn test_newtype_enum() {
    let yaml = "Text: hello\n";
    let expected = NewtypeEnum::Text("hello".to_string());
    test_de(yaml, &expected);

    let yaml = "Number: 42\n";
    let expected = NewtypeEnum::Number(42);
    test_de(yaml, &expected);
}

#[derive(Debug, Deserialize, PartialEq)]
enum StructEnum {
    Point { x: i32, y: i32 },
    Named { name: String },
}

#[test]
fn test_struct_enum() {
    let yaml = "Point:\n  x: 10\n  y: 20\n";
    let expected = StructEnum::Point { x: 10, y: 20 };
    test_de(yaml, &expected);
}

// ============================================================================
// Alias Tests (from serde_yml)
// ============================================================================

#[test]
fn test_alias() {
    let yaml = "first: &alias 1\nsecond: *alias\nthird: 3\n";
    let mut expected = BTreeMap::new();
    let _ = expected.insert("first".to_string(), 1);
    let _ = expected.insert("second".to_string(), 1);
    let _ = expected.insert("third".to_string(), 3);
    test_de(yaml, &expected);
}

// ============================================================================
// Tag Resolution Tests (from serde_yml)
// ============================================================================

#[test]
fn test_tag_resolution() {
    // YAML 1.2 core schema: only lowercase null and ~ are recognized
    let yaml = "- null\n- ~\n";
    let value: Vec<Value> = from_str(yaml).unwrap();
    assert!(value.iter().all(|v| v.is_null()));
}

#[test]
fn test_bool_tag_resolution() {
    // YAML 1.2 core schema: only lowercase true/false are recognized
    let yaml = "- true\n- false\n";
    let value: Vec<bool> = from_str(yaml).unwrap();
    assert_eq!(value, vec![true, false]);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_empty_document_null() {
    let result: Result<Value, _> = from_str("");
    assert_eq!(result.unwrap(), Value::Null);
}

#[test]
fn test_type_mismatch_error() {
    let yaml = "hello\n";
    let result: Result<i32, _> = from_str(yaml);
    assert!(result.is_err());
}

// ============================================================================
// Complex Structure Tests
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
struct Config {
    name: String,
    version: u32,
    features: Vec<String>,
    settings: BTreeMap<String, i32>,
}

#[test]
fn test_complex_config() {
    let yaml = r#"
name: my-app
version: 1
features:
  - auth
  - api
  - logging
settings:
  timeout: 30
  retries: 3
"#;

    let expected = Config {
        name: "my-app".to_string(),
        version: 1,
        features: vec!["auth".to_string(), "api".to_string(), "logging".to_string()],
        settings: {
            let mut s = BTreeMap::new();
            let _ = s.insert("timeout".to_string(), 30);
            let _ = s.insert("retries".to_string(), 3);
            s
        },
    };

    test_de(yaml, &expected);
}

// ============================================================================
// Borrowed String Tests (from serde_yml)
// ============================================================================

#[test]
fn test_quoted_strings() {
    let yaml = "- 'single quoted'\n- \"double quoted\"\n";
    let expected = vec!["single quoted".to_string(), "double quoted".to_string()];
    test_de(yaml, &expected);
}

// ============================================================================
// Empty Collection Tests
// ============================================================================

#[test]
fn test_empty_sequence() {
    let yaml = "[]\n";
    let expected: Vec<i32> = vec![];
    test_de(yaml, &expected);
}

#[test]
fn test_empty_mapping() {
    let yaml = "{}\n";
    let expected: BTreeMap<String, i32> = BTreeMap::new();
    test_de(yaml, &expected);
}

// ============================================================================
// Multiline String Tests
// ============================================================================

#[test]
fn test_multiline_literal() {
    let yaml = "text: |\n  line1\n  line2\n";
    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        text: String,
    }
    let doc: Doc = from_str(yaml).unwrap();
    assert!(doc.text.contains("line1"));
    assert!(doc.text.contains("line2"));
}

// ============================================================================
// Unicode Tests
// ============================================================================

#[test]
fn test_unicode() {
    let yaml = "text: 日本語\n";
    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        text: String,
    }
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.text, "日本語");
}

#[test]
fn test_emoji() {
    let yaml = "emoji: 🎉\n";
    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        emoji: String,
    }
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.emoji, "🎉");
}

// ============================================================================
// Additional Coverage Tests
// ============================================================================

#[test]
fn test_from_slice() {
    let yaml = b"name: test\nvalue: 42\n";
    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        name: String,
        value: i32,
    }
    let doc: Doc = noyalib::from_slice(yaml).unwrap();
    assert_eq!(doc.name, "test");
    assert_eq!(doc.value, 42);
}

#[test]
fn test_from_slice_invalid_utf8() {
    let invalid_utf8: &[u8] = &[0xFF, 0xFE];
    let result: Result<Value, _> = noyalib::from_slice(invalid_utf8);
    assert!(result.is_err());
}

#[test]
fn test_from_reader() {
    use std::io::Cursor;
    let yaml = "key: value\n";
    let reader = Cursor::new(yaml);
    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        key: String,
    }
    let doc: Doc = noyalib::from_reader(reader).unwrap();
    assert_eq!(doc.key, "value");
}

#[test]
fn test_empty_yaml() {
    let yaml = "";
    let result: Result<Value, _> = from_str(yaml);
    assert_eq!(result.unwrap(), Value::Null);
}

#[test]
fn test_char_deserialization() {
    let yaml = "c: a\n";
    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        c: char,
    }
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.c, 'a');
}

#[test]
fn test_unit_deserialization() {
    let yaml = "null\n";
    let _: () = from_str(yaml).unwrap();
}

#[test]
fn test_unit_struct_deserialization() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct UnitStruct;

    let yaml = "null\n";
    let _: UnitStruct = from_str(yaml).unwrap();
}

#[test]
fn test_newtype_struct() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Wrapper(i32);

    let yaml = "42\n";
    let wrapper: Wrapper = from_str(yaml).unwrap();
    assert_eq!(wrapper.0, 42);
}

#[test]
fn test_tuple_deserialization() {
    let yaml = "- 1\n- hello\n- true\n";
    let tuple: (i32, String, bool) = from_str(yaml).unwrap();
    assert_eq!(tuple, (1, "hello".to_string(), true));
}

#[test]
fn test_tuple_struct() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Point(i32, i32);

    let yaml = "- 10\n- 20\n";
    let point: Point = from_str(yaml).unwrap();
    assert_eq!(point, Point(10, 20));
}

#[test]
fn test_ignored_any() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Partial {
        keep: String,
        #[serde(skip)]
        _ignored: (),
    }

    let yaml = "keep: value\nextra: ignored\n";
    let partial: Partial = from_str(yaml).unwrap();
    assert_eq!(partial.keep, "value");
}

#[test]
fn test_sequence_of_bytes() {
    let yaml = "data: [104, 101, 108, 108, 111]\n";
    #[derive(Debug, Deserialize)]
    struct Doc {
        data: Vec<u8>,
    }
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.data, vec![104, 101, 108, 108, 111]);
}

#[test]
fn test_enum_unit_variant() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Color {
        Red,
        Green,
        Blue,
    }

    let yaml = "Red\n";
    let color: Color = from_str(yaml).unwrap();
    assert_eq!(color, Color::Red);
}

#[test]
fn test_enum_tuple_variant() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Message {
        Move { x: i32, y: i32 },
        Write(String),
        Quit,
    }

    let yaml = "Write: hello\n";
    let msg: Message = from_str(yaml).unwrap();
    assert_eq!(msg, Message::Write("hello".to_string()));
}

#[test]
fn test_enum_struct_variant() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Shape {
        Rectangle { width: u32, height: u32 },
        Circle { radius: u32 },
    }

    let yaml = "Rectangle:\n  width: 10\n  height: 20\n";
    let shape: Shape = from_str(yaml).unwrap();
    assert_eq!(
        shape,
        Shape::Rectangle {
            width: 10,
            height: 20
        }
    );
}

#[test]
fn test_option_none() {
    let yaml = "~\n";
    let opt: Option<i32> = from_str(yaml).unwrap();
    assert_eq!(opt, None);
}

#[test]
fn test_option_some() {
    let yaml = "42\n";
    let opt: Option<i32> = from_str(yaml).unwrap();
    assert_eq!(opt, Some(42));
}

#[test]
fn test_large_float() {
    let yaml = "1.7976931348623157e308\n";
    let value: f64 = from_str(yaml).unwrap();
    assert!(value.is_finite());
}

#[test]
fn test_small_float() {
    let yaml = "2.2250738585072014e-308\n";
    let value: f64 = from_str(yaml).unwrap();
    assert!(value.is_finite());
}

#[test]
fn test_float_variations() {
    let yaml = "1.0e10\n";
    let value: f64 = from_str(yaml).unwrap();
    assert!((value - 1.0e10).abs() < 1.0);

    let yaml = "1.5\n";
    let value: f64 = from_str(yaml).unwrap();
    assert!((value - 1.5).abs() < 0.001);
}

#[test]
fn test_unsigned_integers() {
    let yaml = "255\n";
    let value: u8 = from_str(yaml).unwrap();
    assert_eq!(value, 255);

    let yaml = "65535\n";
    let value: u16 = from_str(yaml).unwrap();
    assert_eq!(value, 65535);

    let yaml = "4294967295\n";
    let value: u32 = from_str(yaml).unwrap();
    assert_eq!(value, 4294967295);

    let yaml = "18446744073709551615\n";
    let value: u64 = from_str(yaml).unwrap();
    assert_eq!(value, 18446744073709551615);
}

#[test]
fn test_signed_integers() {
    let yaml = "-128\n";
    let value: i8 = from_str(yaml).unwrap();
    assert_eq!(value, -128);

    let yaml = "-32768\n";
    let value: i16 = from_str(yaml).unwrap();
    assert_eq!(value, -32768);

    let yaml = "-2147483648\n";
    let value: i32 = from_str(yaml).unwrap();
    assert_eq!(value, -2147483648);
}

#[test]
fn test_deserialize_any_bool() {
    let yaml = "true\n";
    let value: Value = from_str(yaml).unwrap();
    assert!(value.is_bool());
}

#[test]
fn test_deserialize_type_mismatch() {
    let yaml = "hello\n";
    let result: Result<i32, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_missing_field() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Required {
        name: String,
        value: i32,
    }

    let yaml = "name: test\n";
    let result: Result<Required, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_unknown_field_ignored() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Known {
        name: String,
    }

    let yaml = "name: test\nextra: ignored\n";
    let doc: Known = from_str(yaml).unwrap();
    assert_eq!(doc.name, "test");
}

#[test]
fn test_string_from_integer() {
    // Integer should not be coerced to string
    let yaml = "42\n";
    let result: Result<String, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_from_value() {
    let value = Value::Number(Number::Integer(42));
    let result: i32 = from_value(&value).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_from_value_struct() {
    use noyalib::Mapping;

    let mut map = Mapping::new();
    let _ = map.insert("name".to_string(), Value::String("test".to_string()));
    let _ = map.insert("value".to_string(), Value::Number(Number::Integer(42)));
    let value = Value::Mapping(map);

    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        name: String,
        value: i32,
    }

    let doc: Doc = from_value(&value).unwrap();
    assert_eq!(doc.name, "test");
    assert_eq!(doc.value, 42);
}

// ============================================================================
// Type Mismatch Error Coverage Tests
// ============================================================================

#[test]
fn test_bool_type_mismatch() {
    let value = Value::String("not a bool".to_string());
    let result: Result<bool, _> = from_value(&value);
    assert!(result.is_err());
}

#[test]
fn test_u64_type_mismatch() {
    // Negative number should fail for u64
    let value = Value::Number(Number::Integer(-1));
    let result: Result<u64, _> = from_value(&value);
    assert!(result.is_err());

    // String should fail for u64
    let value = Value::String("not a number".to_string());
    let result: Result<u64, _> = from_value(&value);
    assert!(result.is_err());
}

#[test]
fn test_f64_type_mismatch() {
    let value = Value::String("not a float".to_string());
    let result: Result<f64, _> = from_value(&value);
    assert!(result.is_err());
}

#[test]
fn test_char_type_mismatch() {
    // Multi-char string should fail
    let value = Value::String("abc".to_string());
    let result: Result<char, _> = from_value(&value);
    assert!(result.is_err());

    // Number should fail for char
    let value = Value::Number(Number::Integer(65));
    let result: Result<char, _> = from_value(&value);
    assert!(result.is_err());
}

#[test]
fn test_str_type_mismatch() {
    let value = Value::Number(Number::Integer(42));
    let result: Result<String, _> = from_value(&value);
    assert!(result.is_err());
}

#[test]
fn test_seq_type_mismatch() {
    let value = Value::String("not a sequence".to_string());
    let result: Result<Vec<i32>, _> = from_value(&value);
    assert!(result.is_err());
}

#[test]
fn test_map_type_mismatch() {
    let value = Value::String("not a map".to_string());
    let result: Result<BTreeMap<String, i32>, _> = from_value(&value);
    assert!(result.is_err());
}

#[test]
fn test_deserialize_tagged_value() {
    // Custom-tag scalars now surface as `Value::Tagged(tag, inner)`
    // on the default deserialise path so the tag is queryable
    // downstream. The inner is the unresolved string scalar
    // (per YAML 1.2: tags suppress plain-scalar resolution).
    let yaml = "!custom value\n";
    let value: Value = from_str(yaml).unwrap();
    assert!(value.is_tagged(), "tag is preserved");
    assert_eq!(value.untag_ref().as_str(), Some("value"));
}

#[test]
fn test_deserialize_float_from_integer() {
    let value = Value::Number(Number::Integer(42));
    let result: f64 = from_value(&value).unwrap();
    assert!((result - 42.0).abs() < 0.001);
}

#[test]
fn test_deserialize_u64_from_float() {
    let value = Value::Number(Number::Float(42.0));
    let result: u64 = from_value(&value).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_deserialize_negative_float_to_u64_fails() {
    let value = Value::Number(Number::Float(-1.0));
    let result: Result<u64, _> = from_value(&value);
    assert!(result.is_err());
}

#[test]
fn test_deserialize_identifier() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "lowercase")]
    enum Status {
        Active,
        Inactive,
    }

    let yaml = "active\n";
    let status: Status = from_str(yaml).unwrap();
    assert_eq!(status, Status::Active);
}

#[test]
fn test_deserialize_enum_invalid_variant() {
    #[derive(Debug, Deserialize)]
    enum Color {
        Red,
        Green,
        Blue,
    }

    let yaml = "Purple\n";
    let result: Result<Color, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_deserialize_struct_with_extra_fields() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Simple {
        name: String,
    }

    let yaml = "name: test\nextra: ignored\n";
    let simple: Simple = from_str(yaml).unwrap();
    assert_eq!(simple.name, "test");
}

#[test]
fn test_deserialize_option_explicit_null() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        value: Option<i32>,
    }

    let yaml = "value: null\n";
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.value, None);
}

#[test]
fn test_deserialize_bytes_from_sequence() {
    let yaml = "[1, 2, 3, 4, 5]\n";
    let bytes: Vec<u8> = from_str(yaml).unwrap();
    assert_eq!(bytes, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_deserialize_i8_bounds() {
    let yaml = "127\n";
    let val: i8 = from_str(yaml).unwrap();
    assert_eq!(val, 127);

    let yaml = "-128\n";
    let val: i8 = from_str(yaml).unwrap();
    assert_eq!(val, -128);
}

#[test]
fn test_deserialize_u8_bounds() {
    let yaml = "255\n";
    let val: u8 = from_str(yaml).unwrap();
    assert_eq!(val, 255);

    let yaml = "0\n";
    let val: u8 = from_str(yaml).unwrap();
    assert_eq!(val, 0);
}

// ============================================================================
// Additional Coverage Tests - Type Name, Bytes, Unit, Enum
// ============================================================================

#[test]
fn test_unit_type_mismatch() {
    // Try to deserialize non-null value as unit
    let value = Value::String("not null".to_string());
    let result: Result<(), _> = from_value(&value);
    assert!(result.is_err());
}

#[test]
fn test_enum_type_mismatch_on_sequence() {
    // Enum requires string or single-key map
    #[derive(Debug, Deserialize)]
    enum Status {
        Active,
        Inactive,
    }

    let yaml = "- item1\n- item2\n";
    let result: Result<Status, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_bytes_deserialization_from_string() {
    use serde_bytes::ByteBuf;

    let yaml = "hello\n";
    let bytes: ByteBuf = from_str(yaml).unwrap();
    assert_eq!(&*bytes, b"hello");
}

#[test]
fn test_bytes_type_mismatch() {
    use serde_bytes::ByteBuf;

    // Integer should fail for bytes
    let yaml = "42\n";
    let result: Result<ByteBuf, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_deserialize_tagged_inner_value() {
    // Tagged values should deserialize their inner content through deserialize_any
    let yaml = "!custom 42\n";
    let value: Value = from_str(yaml).unwrap();
    // The inner value should be accessible
    if let Some(tagged) = value.as_tagged() {
        assert_eq!(tagged.tag().as_str(), "!custom");
    }
}

#[test]
fn test_deserialize_enum_tuple_variant_from_map() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Data {
        Pair(i32, i32),
    }

    let yaml = "Pair:\n  - 1\n  - 2\n";
    let data: Data = from_str(yaml).unwrap();
    assert_eq!(data, Data::Pair(1, 2));
}

#[test]
fn test_type_name_coverage() {
    // Ensure all type names are tested by deserializing wrong types
    // Null type name
    let value = Value::Null;
    let result: Result<i32, _> = from_value(&value);
    assert!(result.is_err());

    // Bool type name
    let value = Value::Bool(true);
    let result: Result<String, _> = from_value(&value);
    assert!(result.is_err());

    // Sequence type name
    let value = Value::Sequence(vec![Value::Number(Number::Integer(1))]);
    let result: Result<String, _> = from_value(&value);
    assert!(result.is_err());

    // Mapping type name
    let mut map = noyalib::Mapping::new();
    let _ = map.insert("key".to_string(), Value::String("value".to_string()));
    let value = Value::Mapping(map);
    let result: Result<String, _> = from_value(&value);
    assert!(result.is_err());

    // Tagged type name
    let tagged = Value::Tagged(Box::new(noyalib::TaggedValue::new(
        noyalib::Tag::new("!custom"),
        Value::from(42),
    )));
    let result: Result<String, _> = from_value(&tagged);
    assert!(result.is_err());
}
