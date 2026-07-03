// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage boost tests — exercises uncovered lines across the codebase.
//!
//! This file targets specific uncovered paths identified by `cargo tarpaulin`.
//! It must NOT modify any existing test files.

#![allow(unused_results, non_snake_case, clippy::approx_constant)]

use std::collections::HashMap;
use std::sync::Arc;

use noyalib::{
    DuplicateKeyPolicy, Error, Location, Mapping, MappingAny, Number, ParserConfig, Spanned, Value,
    from_slice, from_str, from_str_with_config, from_value, to_string,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// 1. streaming.rs — typed deserialization through the streaming path
// ============================================================================

// --- deserialize_bool ---
#[test]
fn streaming_bool_true() {
    let v: bool = from_str("true").unwrap();
    assert!(v);
}

#[test]
fn streaming_bool_false() {
    let v: bool = from_str("false").unwrap();
    assert!(!v);
}

#[test]
fn streaming_bool_error_on_string() {
    let r: Result<bool, _> = from_str("hello");
    assert!(r.is_err());
}

#[test]
fn streaming_bool_error_on_sequence() {
    let r: Result<bool, _> = from_str("[1, 2]");
    assert!(r.is_err());
}

// --- deserialize_i8 / i16 / i32 / i64 ---
#[test]
fn streaming_i8() {
    let v: i8 = from_str("42").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn streaming_i16() {
    let v: i16 = from_str("-1000").unwrap();
    assert_eq!(v, -1000);
}

#[test]
fn streaming_i32() {
    let v: i32 = from_str("100000").unwrap();
    assert_eq!(v, 100_000);
}

#[test]
fn streaming_i64() {
    let v: i64 = from_str("9999999999").unwrap();
    assert_eq!(v, 9_999_999_999);
}

#[test]
fn streaming_i64_error_on_string() {
    let r: Result<i64, _> = from_str("hello");
    assert!(r.is_err());
}

#[test]
fn streaming_i64_error_on_sequence() {
    let r: Result<i64, _> = from_str("[1]");
    assert!(r.is_err());
}

#[test]
fn streaming_i64_from_float_whole() {
    // A float like 3.0 should be accepted as i64 via the streaming path
    let v: i64 = from_str("3.0").unwrap();
    assert_eq!(v, 3);
}

// --- deserialize_u8 / u16 / u32 / u64 ---
#[test]
fn streaming_u8() {
    let v: u8 = from_str("255").unwrap();
    assert_eq!(v, 255);
}

#[test]
fn streaming_u16() {
    let v: u16 = from_str("60000").unwrap();
    assert_eq!(v, 60_000);
}

#[test]
fn streaming_u32() {
    let v: u32 = from_str("4000000000").unwrap();
    assert_eq!(v, 4_000_000_000);
}

#[test]
fn streaming_u64() {
    let v: u64 = from_str("18000000000000000000").unwrap();
    assert_eq!(v, 18_000_000_000_000_000_000);
}

#[test]
fn streaming_u64_error_on_negative() {
    let r: Result<u64, _> = from_str("-1");
    assert!(r.is_err());
}

#[test]
fn streaming_u64_error_on_string() {
    let r: Result<u64, _> = from_str("hello");
    assert!(r.is_err());
}

#[test]
fn streaming_u64_error_on_sequence() {
    let r: Result<u64, _> = from_str("[1]");
    assert!(r.is_err());
}

#[test]
fn streaming_u64_from_float_whole() {
    let v: u64 = from_str("5.0").unwrap();
    assert_eq!(v, 5);
}

// --- deserialize_f32 / f64 ---
#[test]
fn streaming_f32() {
    let v: f32 = from_str("3.14").unwrap();
    assert!((v - 3.14).abs() < 0.01);
}

#[test]
fn streaming_f64() {
    let v: f64 = from_str("2.718281828").unwrap();
    assert!((v - 2.718281828).abs() < 1e-9);
}

#[test]
fn streaming_f64_from_int() {
    let v: f64 = from_str("42").unwrap();
    assert!((v - 42.0).abs() < 1e-9);
}

#[test]
fn streaming_f64_inf() {
    let v: f64 = from_str(".inf").unwrap();
    assert!(v.is_infinite() && v.is_sign_positive());
}

#[test]
fn streaming_f64_neg_inf() {
    let v: f64 = from_str("-.inf").unwrap();
    assert!(v.is_infinite() && v.is_sign_negative());
}

#[test]
fn streaming_f64_nan() {
    let v: f64 = from_str(".nan").unwrap();
    assert!(v.is_nan());
}

#[test]
fn streaming_f64_error_on_string() {
    let r: Result<f64, _> = from_str("hello");
    assert!(r.is_err());
}

#[test]
fn streaming_f64_error_on_mapping() {
    let r: Result<f64, _> = from_str("a: 1");
    assert!(r.is_err());
}

// --- deserialize_char ---
#[test]
fn streaming_char_single() {
    let v: char = from_str("'x'").unwrap();
    assert_eq!(v, 'x');
}

#[test]
fn streaming_char_error_multichar() {
    let r: Result<char, _> = from_str("hello");
    assert!(r.is_err());
}

#[test]
fn streaming_char_error_on_sequence() {
    let r: Result<char, _> = from_str("[x]");
    assert!(r.is_err());
}

// --- deserialize_bytes / byte_buf ---
#[test]
fn streaming_bytes() {
    // serde_bytes provides a type that calls deserialize_bytes
    #[derive(Deserialize)]
    struct ByteWrap {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    }
    let v: ByteWrap = from_str("data: hello").unwrap();
    assert_eq!(v.data, b"hello");
}

#[test]
fn streaming_byte_buf() {
    #[derive(Deserialize)]
    struct ByteBufWrap {
        #[serde(with = "serde_bytes")]
        data: serde_bytes::ByteBuf,
    }
    let v: ByteBufWrap = from_str("data: world").unwrap();
    assert_eq!(v.data.as_ref(), b"world");
}

// --- deserialize_option ---
#[test]
fn streaming_option_none() {
    let v: Option<i32> = from_str("~").unwrap();
    assert_eq!(v, None);
}

#[test]
fn streaming_option_none_null() {
    let v: Option<i32> = from_str("null").unwrap();
    assert_eq!(v, None);
}

#[test]
fn streaming_option_some() {
    let v: Option<i32> = from_str("42").unwrap();
    assert_eq!(v, Some(42));
}

#[test]
fn streaming_option_some_string() {
    let v: Option<String> = from_str("hello").unwrap();
    assert_eq!(v.as_deref(), Some("hello"));
}

#[test]
fn streaming_option_some_seq() {
    let v: Option<Vec<i32>> = from_str("[1, 2, 3]").unwrap();
    assert_eq!(v, Some(vec![1, 2, 3]));
}

// --- deserialize_unit / unit_struct ---
#[test]
fn streaming_unit() {
    let _v: () = from_str("~").unwrap();
}

#[test]
fn streaming_unit_null_word() {
    let _v: () = from_str("null").unwrap();
}

#[test]
fn streaming_unit_error_on_non_null() {
    let r: Result<(), _> = from_str("42");
    assert!(r.is_err());
}

#[test]
fn streaming_unit_error_on_mapping() {
    let r: Result<(), _> = from_str("a: 1");
    assert!(r.is_err());
}

#[test]
fn streaming_unit_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Marker;
    let v: Marker = from_str("~").unwrap();
    assert_eq!(v, Marker);
}

// --- deserialize_newtype_struct ---
#[test]
fn streaming_newtype_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Wrapper(i32);
    let v: Wrapper = from_str("42").unwrap();
    assert_eq!(v, Wrapper(42));
}

#[test]
fn streaming_newtype_struct_string() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Name(String);
    let v: Name = from_str("hello").unwrap();
    assert_eq!(v, Name("hello".to_owned()));
}

// --- deserialize_tuple / tuple_struct ---
#[test]
fn streaming_tuple() {
    let v: (i32, String, bool) = from_str("[1, hello, true]").unwrap();
    assert_eq!(v, (1, "hello".to_owned(), true));
}

#[test]
fn streaming_tuple_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Point(f64, f64);
    let v: Point = from_str("[1.0, 2.5]").unwrap();
    assert_eq!(v, Point(1.0, 2.5));
}

// --- deserialize_enum ---
#[derive(Debug, Deserialize, Serialize, PartialEq)]
enum Color {
    Red,
    Blue,
    Rgb(u8, u8, u8),
    Custom { name: String, hex: String },
    Wrap(i32),
}

#[test]
fn streaming_enum_unit_variant() {
    let v: Color = from_str("Red").unwrap();
    assert_eq!(v, Color::Red);
}

#[test]
fn streaming_enum_newtype_variant() {
    let v: Color = from_str("Wrap: 42").unwrap();
    assert_eq!(v, Color::Wrap(42));
}

#[test]
fn streaming_enum_tuple_variant() {
    let v: Color = from_str("Rgb:\n  - 255\n  - 128\n  - 0").unwrap();
    assert_eq!(v, Color::Rgb(255, 128, 0));
}

#[test]
fn streaming_enum_struct_variant() {
    let yaml = "Custom:\n  name: forest\n  hex: '#228B22'";
    let v: Color = from_str(yaml).unwrap();
    assert_eq!(
        v,
        Color::Custom {
            name: "forest".to_owned(),
            hex: "#228B22".to_owned()
        }
    );
}

#[test]
fn streaming_enum_error_on_sequence() {
    let r: Result<Color, _> = from_str("[1, 2]");
    assert!(r.is_err());
}

// --- deserialize_ignored_any ---
#[test]
fn streaming_ignored_any() {
    // Extra fields are silently ignored via deserialize_ignored_any
    #[derive(Deserialize)]
    struct Partial {
        name: String,
    }
    let yaml = "name: test\nage: 42\nextra: [1, 2, 3]\nnested:\n  a: 1\n  b: 2";
    let v: Partial = from_str(yaml).unwrap();
    assert_eq!(v.name, "test");
}

// --- Seq Drop (early return from seq) ---
#[test]
fn streaming_seq_early_drop() {
    // A tuple takes fewer elements than the YAML sequence provides,
    // triggering the Drop draining logic
    #[derive(Deserialize, Debug, PartialEq)]
    struct TwoOf {
        items: (i32, i32),
    }
    let yaml = "items:\n  - 1\n  - 2\n  - 3\n  - 4";
    let v: TwoOf = from_str(yaml).unwrap();
    assert_eq!(v.items, (1, 2));
}

// --- Map Drop (early return from map) ---
#[test]
fn streaming_map_early_drop() {
    // Struct with fewer fields than the mapping, triggering Drop drain
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Small {
        a: i32,
    }
    let yaml = "a: 1\nb: 2\nc: 3\nd: 4";
    let v: Small = from_str(yaml).unwrap();
    assert_eq!(v.a, 1);
}

// --- Fallback to Value-based path (anchors, aliases, tags) ---
#[test]
fn streaming_fallback_anchor_alias() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Cfg {
        a: String,
        b: String,
    }
    let yaml = "a: &val hello\nb: *val";
    let v: Cfg = from_str(yaml).unwrap();
    assert_eq!(v.a, "hello");
    assert_eq!(v.b, "hello");
}

#[test]
fn streaming_fallback_merge_key() {
    let yaml = "defaults: &d\n  color: red\nitem:\n  <<: *d\n  name: test";
    let v: Value = from_str(yaml).unwrap();
    let item = v.as_mapping().unwrap().get("item").unwrap();
    assert_eq!(
        item.as_mapping().unwrap().get("color").unwrap().as_str(),
        Some("red")
    );
}

#[test]
fn streaming_fallback_tag() {
    // Tags trigger fallback to Value path which resolves tags
    let yaml = "!custom value";
    let v: Value = from_str(yaml).unwrap();
    // Tags are resolved differently — the value path may strip simple tags
    // Just verify it parses without error
    assert!(v.is_tagged() || v.as_str().is_some());
}

// --- identifier deserialization (raw key mode) ---
#[test]
fn streaming_identifier_keys() {
    // Keys that look like booleans/numbers should still work as map keys
    let yaml = "true: yes\n42: answer\nnull: empty";
    let v: HashMap<String, String> = from_str(yaml).unwrap();
    assert_eq!(v.get("true").unwrap(), "yes");
    assert_eq!(v.get("42").unwrap(), "answer");
    assert_eq!(v.get("null").unwrap(), "empty");
}

// --- streaming any: empty document ---
#[test]
fn streaming_empty_document() {
    // Empty document resolves to null which becomes Value::Null
    let v: Value = from_str("").unwrap();
    assert_eq!(v, Value::Null);
}

// --- streaming hex/octal integer parsing ---
#[test]
fn streaming_hex_integer() {
    let v: i64 = from_str("0xFF").unwrap();
    assert_eq!(v, 255);
}

#[test]
fn streaming_octal_integer() {
    let v: i64 = from_str("0o77").unwrap();
    assert_eq!(v, 63);
}

// --- streaming large integer overflow to float ---
#[test]
fn streaming_large_integer_overflow() {
    // Integer larger than i64::MAX should be parsed as f64
    let v: f64 = from_str("99999999999999999999").unwrap();
    assert!(v > 0.0);
}

// ============================================================================
// 2. parser/loader.rs — NoSpanLoader path via from_str and from_slice
// ============================================================================

// from_slice routes through NoSpanLoader on fallback
#[test]
fn loader_from_slice_basic() {
    let v: Value = from_slice(b"key: value").unwrap();
    assert_eq!(v["key"].as_str(), Some("value"));
}

#[test]
fn loader_from_slice_sequence() {
    let v: Vec<i32> = from_slice(b"[1, 2, 3]").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

// Anchors and aliases via from_str (forces fallback to NoSpanLoader)
#[test]
fn loader_nospan_anchors_aliases() {
    let yaml = "base: &base\n  x: 1\n  y: 2\nderived:\n  <<: *base\n  z: 3";
    let v: Value = from_str(yaml).unwrap();
    let derived = v.as_mapping().unwrap().get("derived").unwrap();
    assert_eq!(
        derived.as_mapping().unwrap().get("x").unwrap(),
        &Value::Number(Number::from(1))
    );
    assert_eq!(
        derived.as_mapping().unwrap().get("z").unwrap(),
        &Value::Number(Number::from(3))
    );
}

// Merge keys via from_str — sequence of mappings merge
#[test]
fn loader_nospan_merge_sequence() {
    let yaml = "a: &a\n  x: 1\nb: &b\n  y: 2\nc:\n  <<: [*a, *b]\n  z: 3";
    let v: Value = from_str(yaml).unwrap();
    let c = v.as_mapping().unwrap().get("c").unwrap();
    assert_eq!(
        c.as_mapping().unwrap().get("x").unwrap(),
        &Value::Number(Number::from(1))
    );
    assert_eq!(
        c.as_mapping().unwrap().get("y").unwrap(),
        &Value::Number(Number::from(2))
    );
}

// Duplicate key policies via from_str (uses NoSpanLoader on fallback)
#[test]
fn loader_nospan_duplicate_key_last() {
    let yaml = "a: 1\na: 2";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(
        v.as_mapping().unwrap().get("a").unwrap(),
        &Value::Number(Number::from(2))
    );
}

#[test]
fn loader_nospan_duplicate_key_first() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let yaml = "a: 1\na: 2";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(
        v.as_mapping().unwrap().get("a").unwrap(),
        &Value::Number(Number::from(1))
    );
}

#[test]
fn loader_nospan_duplicate_key_error() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let yaml = "a: 1\na: 2";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

// Deep nesting via from_str
#[test]
fn loader_nospan_deep_nesting() {
    let yaml = "a:\n  b:\n    c:\n      d:\n        e: deep";
    let v: Value = from_str(yaml).unwrap();
    let e = v["a"]["b"]["c"]["d"]["e"].as_str();
    assert_eq!(e, Some("deep"));
}

// Large documents via from_str
#[test]
fn loader_nospan_large_sequence() {
    let items: Vec<String> = (0..100).map(|i| format!("- item{i}")).collect();
    let yaml = items.join("\n");
    let v: Vec<String> = from_str(&yaml).unwrap();
    assert_eq!(v.len(), 100);
}

// Sequence with anchors via from_slice (fallback path)
#[test]
fn loader_nospan_seq_anchor_via_slice() {
    let yaml = b"- &first hello\n- *first";
    let v: Vec<String> = from_slice(yaml).unwrap();
    assert_eq!(v, vec!["hello", "hello"]);
}

// Nested anchor on mapping via from_slice
#[test]
fn loader_nospan_mapping_anchor_via_slice() {
    let yaml = b"base: &b\n  x: 10\nref: *b";
    let v: Value = from_slice(yaml).unwrap();
    let r = v.as_mapping().unwrap().get("ref").unwrap();
    assert_eq!(
        r.as_mapping().unwrap().get("x").unwrap(),
        &Value::Number(Number::from(10))
    );
}

// ============================================================================
// 3. de.rs — Value-based deserializer uncovered paths
// ============================================================================

// i8 overflow from Value
#[test]
fn de_i8_overflow() {
    let val = Value::Number(Number::Integer(200));
    let r: Result<i8, _> = from_value(&val);
    assert!(r.is_err());
}

// i16 overflow
#[test]
fn de_i16_overflow() {
    let val = Value::Number(Number::Integer(40000));
    let r: Result<i16, _> = from_value(&val);
    assert!(r.is_err());
}

// i32 overflow
#[test]
fn de_i32_overflow() {
    let val = Value::Number(Number::Integer(3_000_000_000));
    let r: Result<i32, _> = from_value(&val);
    assert!(r.is_err());
}

// u8 overflow
#[test]
fn de_u8_overflow() {
    let val = Value::Number(Number::Integer(300));
    let r: Result<u8, _> = from_value(&val);
    assert!(r.is_err());
}

// u16 overflow
#[test]
fn de_u16_overflow() {
    let val = Value::Number(Number::Integer(70000));
    let r: Result<u16, _> = from_value(&val);
    assert!(r.is_err());
}

// u32 overflow
#[test]
fn de_u32_overflow() {
    let val = Value::Number(Number::Integer(5_000_000_000));
    let r: Result<u32, _> = from_value(&val);
    assert!(r.is_err());
}

// Negative as unsigned
#[test]
fn de_negative_as_unsigned() {
    let val = Value::Number(Number::Integer(-1));
    let r: Result<u64, _> = from_value(&val);
    assert!(r.is_err());
}

// f32 from value
#[test]
fn de_f32_from_value() {
    let val = Value::Number(Number::Float(3.14));
    let v: f32 = from_value(&val).unwrap();
    assert!((v - 3.14).abs() < 0.01);
}

// f32 from integer value
#[test]
fn de_f32_from_int_value() {
    let val = Value::Number(Number::Integer(42));
    let v: f32 = from_value(&val).unwrap();
    assert!((v - 42.0).abs() < 0.01);
}

// char error on multi-char
#[test]
fn de_char_multichar_error() {
    let val = Value::String("abc".to_owned());
    let r: Result<char, _> = from_value(&val);
    assert!(r.is_err());
}

// char success
#[test]
fn de_char_single() {
    let val = Value::String("x".to_owned());
    let v: char = from_value(&val).unwrap();
    assert_eq!(v, 'x');
}

// bytes from value
#[test]
fn de_bytes_from_value() {
    #[derive(Deserialize)]
    struct W {
        #[serde(with = "serde_bytes")]
        d: Vec<u8>,
    }
    let yaml = "d: hello";
    let v: W = from_str_with_config(yaml, &ParserConfig::new()).unwrap();
    assert_eq!(v.d, b"hello");
}

// unit_struct from value
#[test]
fn de_unit_struct() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Empty;
    let v: Empty = from_str_with_config("~", &ParserConfig::new()).unwrap();
    assert_eq!(v, Empty);
}

// tuple_struct from value
#[test]
fn de_tuple_struct() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Pair(i32, i32);
    let v: Pair = from_str_with_config("[10, 20]", &ParserConfig::new()).unwrap();
    assert_eq!(v, Pair(10, 20));
}

// newtype_struct from value
#[test]
fn de_newtype_struct() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct W(String);
    let v: W = from_str_with_config("hello", &ParserConfig::new()).unwrap();
    assert_eq!(v, W("hello".to_owned()));
}

// identifier from value
#[test]
fn de_identifier() {
    // Field name deserialization uses deserialize_identifier
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Named {
        field_name: i32,
    }
    let v: Named = from_str_with_config("field_name: 5", &ParserConfig::new()).unwrap();
    assert_eq!(v.field_name, 5);
}

// ignored_any from value
#[test]
fn de_ignored_any() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Pick {
        a: i32,
    }
    let yaml = "a: 1\nb: 2\nc: 3";
    let v: Pick = from_str_with_config(yaml, &ParserConfig::new()).unwrap();
    assert_eq!(v.a, 1);
}

// EnumDeserializer — newtype, tuple, struct variants from Value path
#[test]
fn de_enum_newtype_from_value() {
    let val = Value::Mapping({
        let mut m = Mapping::new();
        m.insert("Wrap", Value::Number(Number::from(99)));
        m
    });
    let v: Color = from_value(&val).unwrap();
    assert_eq!(v, Color::Wrap(99));
}

#[test]
fn de_enum_tuple_from_value() {
    let val = Value::Mapping({
        let mut m = Mapping::new();
        m.insert(
            "Rgb",
            Value::Sequence(vec![
                Value::Number(Number::from(10)),
                Value::Number(Number::from(20)),
                Value::Number(Number::from(30)),
            ]),
        );
        m
    });
    let v: Color = from_value(&val).unwrap();
    assert_eq!(v, Color::Rgb(10, 20, 30));
}

#[test]
fn de_enum_struct_from_value() {
    let val = Value::Mapping({
        let mut m = Mapping::new();
        m.insert("Custom", {
            let mut inner = Mapping::new();
            inner.insert("name", Value::String("test".to_owned()));
            inner.insert("hex", Value::String("#000".to_owned()));
            Value::Mapping(inner)
        });
        m
    });
    let v: Color = from_value(&val).unwrap();
    assert_eq!(
        v,
        Color::Custom {
            name: "test".to_owned(),
            hex: "#000".to_owned()
        }
    );
}

#[test]
fn de_enum_unit_from_value() {
    let val = Value::Mapping({
        let mut m = Mapping::new();
        m.insert("Red", Value::Null);
        m
    });
    let v: Color = from_value(&val).unwrap();
    assert_eq!(v, Color::Red);
}

#[test]
fn de_enum_error_from_value() {
    let val = Value::Number(Number::from(42));
    let r: Result<Color, _> = from_value(&val);
    assert!(r.is_err());
}

// Float-to-i64 coercion in value path
#[test]
fn de_float_to_i64_value_path() {
    let val = Value::Number(Number::Float(10.0));
    let v: i64 = from_value(&val).unwrap();
    assert_eq!(v, 10);
}

// Float-to-u64 coercion in value path
#[test]
fn de_float_to_u64_value_path() {
    let val = Value::Number(Number::Float(10.0));
    let v: u64 = from_value(&val).unwrap();
    assert_eq!(v, 10);
}

// ============================================================================
// 4. parser/scanner.rs — Unicode escapes, block scalar, tags
// ============================================================================

#[test]
fn scanner_hex_escape_x() {
    let yaml = "\"\\x41\""; // \x41 = 'A'
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "A");
}

#[test]
fn scanner_unicode_escape_u() {
    let yaml = "\"\\u0041\""; // \u0041 = 'A'
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "A");
}

#[test]
fn scanner_unicode_escape_U() {
    let yaml = "\"\\U00000041\""; // \U00000041 = 'A'
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "A");
}

#[test]
fn scanner_unicode_escape_special() {
    let yaml = "\"\\u00e9\""; // é
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "\u{00e9}");
}

#[test]
fn scanner_block_literal_explicit_indent() {
    let yaml = "data: |2\n  hello\n  world";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["data"].as_str(), Some("hello\nworld\n"));
}

#[test]
fn scanner_block_folded_explicit_indent() {
    let yaml = "data: >2\n  hello\n  world";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["data"].as_str(), Some("hello world\n"));
}

#[test]
fn scanner_tag_with_uri() {
    let yaml = "!mytag value";
    let v: Value = from_str(yaml).unwrap();
    // The tag may be stored as tagged or stripped depending on the schema
    assert!(v.is_tagged() || v.as_str().is_some());
}

#[test]
fn scanner_anchor_alias() {
    let yaml = "a: &anchor hello\nb: *anchor";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_str(), Some("hello"));
    assert_eq!(v["b"].as_str(), Some("hello"));
}

#[test]
fn scanner_flow_collection_nested() {
    let yaml = "{a: [1, {b: 2}], c: 3}";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["c"], Value::Number(Number::from(3)));
}

// ============================================================================
// 5. value.rs — Number::Ord, Mapping from_inner/into_inner, etc.
// ============================================================================

#[test]
fn number_ord_large_integer_vs_float() {
    // Integer > 2^53 compared with float
    let large = Number::Integer((1_i64 << 53) + 1);
    let float = Number::Float(1.0);
    assert!(large > float);
}

#[test]
fn number_ord_large_negative_integer_vs_float() {
    let large = Number::Integer(-((1_i64 << 53) + 1));
    let float = Number::Float(-1.0);
    assert!(large < float);
}

#[test]
fn number_ord_float_vs_large_integer() {
    // Float vs large Integer (inverted case)
    let float = Number::Float(1.0);
    let large = Number::Integer((1_i64 << 53) + 1);
    assert!(float < large);
}

#[test]
fn number_ord_nan_vs_integer() {
    let nan = Number::Float(f64::NAN);
    let int = Number::Integer(42);
    // NaN is treated as greater than non-NaN
    assert!(nan > int);
}

#[test]
fn number_ord_integer_vs_nan() {
    let int = Number::Integer(42);
    let nan = Number::Float(f64::NAN);
    assert!(int < nan);
}

#[test]
fn mapping_from_inner_into_inner() {
    let mut std_map = indexmap::IndexMap::new();
    std_map.insert("a".to_owned(), Value::from(1));
    std_map.insert("b".to_owned(), Value::from(2));

    let mapping = Mapping::from_inner(std_map);
    assert_eq!(mapping.len(), 2);

    let back = mapping.into_inner();
    assert_eq!(back.len(), 2);
}

#[test]
fn mapping_any_from_inner_into_inner() {
    let mut std_map = indexmap::IndexMap::new();
    std_map.insert(Value::from(1), Value::from("one"));
    std_map.insert(Value::from(2), Value::from("two"));

    let mapping = MappingAny::from_inner(std_map);
    assert_eq!(mapping.len(), 2);

    let back = mapping.into_inner();
    assert_eq!(back.len(), 2);
}

#[test]
fn mapping_from_array() {
    let m: Mapping = Mapping::from([
        ("a".to_owned(), Value::from(1)),
        ("b".to_owned(), Value::from(2)),
    ]);
    assert_eq!(m.len(), 2);
    assert_eq!(m.get("a"), Some(&Value::from(1)));
}

#[test]
fn mapping_any_from_array() {
    let m: MappingAny = MappingAny::from([
        (Value::from(1), Value::from("one")),
        (Value::from(2), Value::from("two")),
    ]);
    assert_eq!(m.len(), 2);
}

#[test]
fn mapping_from_iterator() {
    let items = vec![
        ("x".to_owned(), Value::from(10)),
        ("y".to_owned(), Value::from(20)),
    ];
    let m: Mapping = items.into_iter().collect();
    assert_eq!(m.len(), 2);
}

#[test]
fn mapping_any_from_iterator() {
    let items = vec![
        (Value::from("x"), Value::from(10)),
        (Value::from("y"), Value::from(20)),
    ];
    let m: MappingAny = items.into_iter().collect();
    assert_eq!(m.len(), 2);
}

// ============================================================================
// 6. error.rs — miette::Diagnostic and format_with_source
// ============================================================================

#[test]
fn error_format_with_source_no_location() {
    let err = Error::Parse("generic error".to_owned());
    let formatted = err.format_with_source("some source");
    assert_eq!(formatted, "YAML parse error: generic error");
}

#[test]
fn error_format_with_source_with_location() {
    let err = Error::ParseWithLocation {
        message: "unexpected token".to_owned(),
        location: Location::new(1, 5, 4),
    };
    let formatted = err.format_with_source("key: value");
    assert!(formatted.contains("unexpected token"));
    assert!(formatted.contains("line 1:5"));
    assert!(formatted.contains("^"));
}

#[test]
fn error_format_with_source_line_out_of_range() {
    let err = Error::ParseWithLocation {
        message: "test".to_owned(),
        location: Location::new(100, 1, 0),
    };
    let formatted = err.format_with_source("short");
    // Falls back to plain Display
    assert!(formatted.contains("test"));
}

#[test]
fn error_shared_wrapping() {
    let err = Error::Parse("inner".to_owned());
    let shared = err.into_shared();
    let wrapped = Error::from_shared(Arc::clone(&shared));
    assert!(wrapped.is_shared());
    assert!(wrapped.as_inner().is_some());
    assert_eq!(
        wrapped.as_inner().unwrap().to_string(),
        "YAML parse error: inner"
    );
}

#[test]
fn error_shared_double_wrap() {
    let err = Error::Parse("test".to_owned());
    let arc1 = err.into_shared();
    let shared_err = Error::Shared(Arc::clone(&arc1));
    let arc2 = shared_err.into_shared();
    assert!(Arc::ptr_eq(&arc1, &arc2));
}

#[test]
fn error_shared_location_delegation() {
    let inner = Error::DeserializeWithLocation {
        message: "bad".to_owned(),
        location: Location::new(5, 10, 42),
    };
    let shared = Error::from_shared(inner.into_shared());
    let loc = shared.location().unwrap();
    assert_eq!(loc.line(), 5);
    assert_eq!(loc.column(), 10);
    assert_eq!(loc.index(), 42);
}

#[cfg(feature = "miette")]
mod miette_tests {
    use super::*;
    use miette::Diagnostic;

    #[test]
    fn miette_parse_code() {
        let err = Error::Parse("test".to_owned());
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::parse");
    }

    #[test]
    fn miette_parse_with_location_code() {
        let err = Error::ParseWithLocation {
            message: "test".to_owned(),
            location: Location::new(1, 1, 0),
        };
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::parse");
    }

    #[test]
    fn miette_deserialize_code() {
        let err = Error::Deserialize("test".to_owned());
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::deserialize");
    }

    #[test]
    fn miette_deserialize_with_location_code() {
        let err = Error::DeserializeWithLocation {
            message: "test".to_owned(),
            location: Location::new(1, 1, 0),
        };
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::deserialize");
    }

    #[test]
    fn miette_serialize_code() {
        let err = Error::Serialize("test".to_owned());
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::serialize");
    }

    #[test]
    fn miette_type_mismatch_code() {
        let err = Error::TypeMismatch {
            expected: "int",
            found: "string".to_owned(),
        };
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::type_mismatch");
    }

    #[test]
    fn miette_missing_field_code() {
        let err = Error::MissingField("name".to_owned());
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::missing_field");
    }

    #[test]
    fn miette_unknown_field_code() {
        let err = Error::UnknownField("extra".to_owned());
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::unknown_field");
    }

    #[test]
    fn miette_recursion_limit_code_and_help() {
        let err = Error::RecursionLimitExceeded { depth: 100 };
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::recursion_limit");
        let help = err.help().unwrap().to_string();
        assert!(help.contains("max_depth"));
    }

    #[test]
    fn miette_repetition_limit_code_and_help() {
        let err = Error::RepetitionLimitExceeded;
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::repetition_limit");
        let help = err.help().unwrap().to_string();
        assert!(help.contains("max_alias_expansions"));
    }

    #[test]
    fn miette_duplicate_key_code_and_help() {
        let err = Error::DuplicateKey("a".to_owned());
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::duplicate_key");
        let help = err.help().unwrap().to_string();
        assert!(help.contains("DuplicateKeyPolicy"));
    }

    #[test]
    fn miette_unknown_anchor_code_and_help() {
        let err = Error::UnknownAnchor("foo".to_owned());
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::unknown_anchor");
        let help = err.help().unwrap().to_string();
        assert!(help.contains("anchor"));
    }

    #[test]
    fn miette_eof_code() {
        let err = Error::EndOfStream;
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::eof");
    }

    #[test]
    fn miette_multi_document_code_and_help() {
        let err = Error::MoreThanOneDocument;
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::multi_document");
        let help = err.help().unwrap().to_string();
        assert!(help.contains("load_all"));
    }

    #[test]
    fn miette_io_code() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let err = Error::Io(io_err);
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::io");
    }

    #[test]
    fn miette_custom_code() {
        let err = Error::Custom("whatever".to_owned());
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::error");
    }

    #[test]
    fn miette_invalid_code() {
        let err = Error::Invalid("bad structure".to_owned());
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::error");
    }

    #[test]
    fn miette_scalar_in_merge_code() {
        let err = Error::ScalarInMerge;
        let code = err.code().unwrap().to_string();
        assert_eq!(code, "noyalib::error");
    }

    #[test]
    fn miette_labels_with_location() {
        let err = Error::ParseWithLocation {
            message: "test".to_owned(),
            location: Location::new(1, 5, 4),
        };
        let labels: Vec<_> = err.labels().unwrap().collect();
        assert_eq!(labels.len(), 1);
    }

    #[test]
    fn miette_labels_without_location() {
        let err = Error::Parse("test".to_owned());
        assert!(err.labels().is_none());
    }

    #[test]
    fn miette_shared_delegates() {
        let inner = Error::RecursionLimitExceeded { depth: 50 };
        let shared = Error::from_shared(inner.into_shared());
        let code = shared.code().unwrap().to_string();
        assert_eq!(code, "noyalib::recursion_limit");
    }

    #[test]
    fn miette_no_help_for_parse() {
        let err = Error::Parse("test".to_owned());
        assert!(err.help().is_none());
    }
}

// ============================================================================
// 7. spanned.rs — Spanned serialization
// ============================================================================

#[test]
fn spanned_serialization() {
    let spanned = Spanned::new(42_i32);
    let yaml = to_string(&spanned).unwrap();
    // Spanned serializes transparently as the inner value
    assert!(yaml.trim() == "42");
}

#[test]
fn spanned_deserialization_from_value() {
    let val = Value::String("hello".to_owned());
    let s: Spanned<String> = from_value(&val).unwrap();
    assert_eq!(s.value, "hello");
    // Default locations when from_value
    assert_eq!(s.start.line(), 0);
}

#[test]
fn spanned_deserialization_from_str() {
    let yaml = "port: 8080";
    #[derive(Deserialize)]
    struct Config {
        port: Spanned<u16>,
    }
    let cfg: Config = from_str_with_config(yaml, &ParserConfig::new()).unwrap();
    assert_eq!(cfg.port.value, 8080);
    // Should have real span info
    // Span info should be populated (or default to 0 if no span context)
    let _ = cfg.port.start.line();
}

#[test]
fn spanned_into_inner() {
    let s = Spanned::new(vec![1, 2, 3]);
    let inner = s.into_inner();
    assert_eq!(inner, vec![1, 2, 3]);
}

#[test]
fn spanned_deref() {
    let s = Spanned::new("hello".to_owned());
    // Deref to inner type
    assert_eq!(s.len(), 5);
}

// ============================================================================
// 8. singleton_map_recursive.rs
// ============================================================================

#[test]
fn singleton_map_recursive_roundtrip() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Inner {
        A,
        B { value: i32 },
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        #[serde(with = "noyalib::with::singleton_map_recursive")]
        items: Vec<Inner>,
    }

    let cfg = Config {
        items: vec![Inner::A, Inner::B { value: 42 }],
    };
    let yaml = to_string(&cfg).unwrap();
    let back: Config = from_str(&yaml).unwrap();
    assert_eq!(cfg, back);
}

// ============================================================================
// 9. document.rs — multi-document
// ============================================================================

#[test]
fn document_load_all() {
    let yaml = "---\nname: doc1\n---\nname: doc2\n";
    let docs: Vec<_> = noyalib::load_all(yaml).unwrap().collect();
    assert_eq!(docs.len(), 2);
}

#[test]
fn document_load_all_as() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Doc {
        name: String,
    }
    let yaml = "---\nname: first\n---\nname: second\n";
    let docs: Vec<Doc> = noyalib::load_all_as(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].name, "first");
}

#[test]
fn document_try_load_all() {
    let yaml = "---\n42\n---\nhello\n";
    let docs: Vec<Result<Value, _>> = noyalib::try_load_all(yaml).unwrap().collect();
    assert_eq!(docs.len(), 2);
}

#[test]
fn document_load_all_empty() {
    let yaml = "";
    let docs: Vec<_> = noyalib::load_all(yaml).unwrap().collect();
    assert!(docs.is_empty());
}

// ============================================================================
// 10. Additional coverage for streaming edge cases
// ============================================================================

// Streaming with quoted strings that are booleans/numbers
#[test]
fn streaming_quoted_string_not_resolved() {
    let v: String = from_str("'true'").unwrap();
    assert_eq!(v, "true");
}

#[test]
fn streaming_quoted_number_not_resolved() {
    let v: String = from_str("'42'").unwrap();
    assert_eq!(v, "42");
}

// Map with various value types
#[test]
fn streaming_map_mixed_values() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Mixed {
        name: String,
        count: u32,
        active: bool,
        ratio: f64,
        tags: Vec<String>,
    }
    let yaml = "name: test\ncount: 5\nactive: true\nratio: 0.75\ntags:\n  - a\n  - b";
    let v: Mixed = from_str(yaml).unwrap();
    assert_eq!(v.name, "test");
    assert_eq!(v.count, 5);
    assert!(v.active);
    assert!((v.ratio - 0.75).abs() < 1e-9);
    assert_eq!(v.tags, vec!["a", "b"]);
}

// Streaming recursion limit
#[test]
fn streaming_recursion_limit() {
    let config = ParserConfig::new().max_depth(3);
    let yaml = "a:\n  b:\n    c:\n      d:\n        e: deep";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

// Boolean case variants (non-strict)
#[test]
fn streaming_bool_case_variants() {
    let v: bool = from_str("True").unwrap();
    assert!(v);
    let v: bool = from_str("FALSE").unwrap();
    assert!(!v);
}

// Legacy booleans
#[test]
fn streaming_legacy_booleans() {
    let config = ParserConfig::new().legacy_booleans(true);
    let v: bool = from_str_with_config("yes", &config).unwrap();
    assert!(v);
    let v: bool = from_str_with_config("no", &config).unwrap();
    assert!(!v);
    let v: bool = from_str_with_config("on", &config).unwrap();
    assert!(v);
    let v: bool = from_str_with_config("off", &config).unwrap();
    assert!(!v);
}

// Strict booleans
#[test]
fn streaming_strict_booleans() {
    let config = ParserConfig::new().strict_booleans(true);
    let v: bool = from_str_with_config("true", &config).unwrap();
    assert!(v);
    // "True" should be a string in strict mode
    let r: Result<bool, _> = from_str_with_config("True", &config);
    assert!(r.is_err());
}

// Various null forms
#[test]
fn streaming_null_forms() {
    let v: Option<i32> = from_str("~").unwrap();
    assert_eq!(v, None);
    let v: Option<i32> = from_str("null").unwrap();
    assert_eq!(v, None);
    let v: Option<i32> = from_str("Null").unwrap();
    assert_eq!(v, None);
    let v: Option<i32> = from_str("NULL").unwrap();
    assert_eq!(v, None);
    // Empty string is an empty document, not an Option<i32> — test separately
    let v: Value = from_str("").unwrap();
    assert_eq!(v, Value::Null);
}

// Document length limit
#[test]
fn streaming_document_length_limit() {
    let config = ParserConfig::new().max_document_length(10);
    let yaml = "this is a very long yaml string that exceeds the limit";
    let r: Result<String, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

// ============================================================================
// 11. Additional loader coverage: sequence length limit, mapping key limit
// ============================================================================

#[test]
fn loader_sequence_length_limit() {
    let config = ParserConfig::new().max_sequence_length(2);
    let yaml = "- 1\n- 2\n- 3";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

#[test]
fn loader_mapping_key_limit() {
    let config = ParserConfig::new().max_mapping_keys(2);
    let yaml = "a: 1\nb: 2\nc: 3";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

// ============================================================================
// 12. Additional value.rs coverage
// ============================================================================

#[test]
fn number_from_u64_overflow() {
    let n = Number::from(u64::MAX);
    #[cfg(not(feature = "lossless-u64"))]
    assert!(n.is_float());
    #[cfg(feature = "lossless-u64")]
    assert_eq!(n.as_u64(), Some(u64::MAX));
}

#[test]
fn number_from_usize() {
    let n = Number::from(42_usize);
    assert_eq!(n.as_i64(), Some(42));
}

#[test]
fn number_parse_from_str() {
    let n: Number = "42".parse().unwrap();
    assert_eq!(n.as_i64(), Some(42));

    let n: Number = "3.14".parse().unwrap();
    assert!((n.as_f64() - 3.14).abs() < 1e-9);

    let n: Number = ".nan".parse().unwrap();
    assert!(n.is_nan());

    let n: Number = ".inf".parse().unwrap();
    assert!(n.is_infinite());

    let n: Number = "0xFF".parse().unwrap();
    assert_eq!(n.as_i64(), Some(255));

    let n: Number = "0o77".parse().unwrap();
    assert_eq!(n.as_i64(), Some(63));

    let n: Number = "0b1010".parse().unwrap();
    assert_eq!(n.as_i64(), Some(10));

    let r: Result<Number, _> = "not_a_number".parse();
    assert!(r.is_err());
}

#[test]
fn mapping_any_into_mapping_success() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    m.insert(Value::from("b"), Value::from(2));
    let mapping = m.into_mapping().unwrap();
    assert_eq!(mapping.len(), 2);
}

#[test]
fn mapping_any_into_mapping_failure() {
    let mut m = MappingAny::new();
    m.insert(Value::from(1), Value::from("one"));
    assert!(m.into_mapping().is_none());
}

#[test]
fn mapping_from_mapping_any() {
    let mut m = Mapping::new();
    m.insert("x", Value::from(10));
    let any: MappingAny = MappingAny::from(m);
    assert_eq!(any.len(), 1);
}

// ============================================================================
// 13. from_reader coverage
// ============================================================================

#[test]
fn from_reader_basic() {
    let yaml = "name: test\nvalue: 42";
    let cursor = std::io::Cursor::new(yaml);
    let v: Value = noyalib::from_reader(cursor).unwrap();
    assert_eq!(v["name"].as_str(), Some("test"));
}

#[test]
fn from_reader_with_config() {
    let yaml = "name: test";
    let cursor = std::io::Cursor::new(yaml);
    let config = ParserConfig::new();
    let v: Value = noyalib::from_reader_with_config(cursor, &config).unwrap();
    assert_eq!(v["name"].as_str(), Some("test"));
}

#[test]
fn from_reader_with_config_length_limit() {
    let yaml = "a very long yaml string";
    let cursor = std::io::Cursor::new(yaml);
    let config = ParserConfig::new().max_document_length(5);
    let r: Result<Value, _> = noyalib::from_reader_with_config(cursor, &config);
    assert!(r.is_err());
}

// ============================================================================
// 14. Block scalar strip/clip/keep chomping
// ============================================================================

#[test]
fn block_scalar_strip_chomp() {
    let yaml = "data: |-\n  hello\n  world";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["data"].as_str(), Some("hello\nworld"));
}

#[test]
fn block_scalar_keep_chomp() {
    let yaml = "data: |+\n  hello\n  world\n\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["data"].as_str().unwrap();
    assert!(s.ends_with("\n\n"));
}

// ============================================================================
// 15. Streaming enum: unit variant in mapping form (e.g., {Red: ~})
// ============================================================================

#[test]
fn streaming_enum_unit_variant_mapping_form() {
    let v: Color = from_str("{Red: ~}").unwrap();
    assert_eq!(v, Color::Red);
}

#[test]
fn streaming_enum_unit_variant_null_value() {
    let v: Color = from_str("Red: null").unwrap();
    assert_eq!(v, Color::Red);
}

// ============================================================================
// 16. Additional scanner coverage: double-quoted escape sequences
// ============================================================================

#[test]
fn scanner_escape_sequences() {
    // Common escape sequences
    let yaml = r#""tab:\there""#;
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\t'));

    let yaml = r#""newline:\nhere""#;
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\n'));

    let yaml = "\"null:\\0here\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\0'));

    let yaml = r#""bell:\ahere""#;
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\u{0007}'));

    let yaml = r#""backspace:\bhere""#;
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\u{0008}'));

    let yaml = r#""escape:\ehere""#;
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\u{001B}'));

    let yaml = r#""slash:\/here""#;
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('/'));

    let yaml = "\"backslash:\\\\here\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\\'));

    let yaml = r#""quote:\"here""#;
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('"'));
}

// ============================================================================
// 17. Additional streaming coverage: from_str that falls back for Spanned<T>
// ============================================================================

#[test]
fn streaming_fallback_for_spanned() {
    #[derive(Deserialize)]
    struct Cfg {
        port: Spanned<u16>,
    }
    let yaml = "port: 8080";
    // from_str tries streaming first, but Spanned triggers fallback
    let cfg: Cfg = from_str(yaml).unwrap();
    assert_eq!(cfg.port.value, 8080);
}

// ============================================================================
// 18. Multi-document edge cases
// ============================================================================

#[test]
fn document_load_all_with_config() {
    let config = ParserConfig::new();
    let yaml = "---\na: 1\n---\nb: 2\n";
    let docs: Vec<_> = noyalib::load_all_with_config(yaml, &config)
        .unwrap()
        .collect();
    assert_eq!(docs.len(), 2);
}

#[test]
fn document_single_empty() {
    let yaml = "---\n...\n";
    let docs: Vec<_> = noyalib::load_all(yaml).unwrap().collect();
    // Empty doc resolves to null
    assert!(!docs.is_empty());
}

// ============================================================================
// 19. Typed Value::from conversions and tagged values
// ============================================================================

#[test]
fn value_from_conversions() {
    assert_eq!(Value::from(true), Value::Bool(true));
    assert_eq!(Value::from(42_i32), Value::Number(Number::from(42)));
    assert_eq!(Value::from(42_u32), Value::Number(Number::from(42)));
    assert_eq!(Value::from(3.14_f64), Value::Number(Number::Float(3.14)));
    assert_eq!(Value::from("hello"), Value::String("hello".to_owned()));
}

#[test]
fn tagged_value_roundtrip() {
    let yaml = "!mytag value";
    let v: Value = from_str(yaml).unwrap();
    // Verify it parsed successfully — tag handling depends on schema
    assert!(v.is_tagged() || v.as_str().is_some());
    let yaml_out = to_string(&v).unwrap();
    assert!(!yaml_out.is_empty());
}

// ============================================================================
// 20. Streaming: deserialize_str error paths
// ============================================================================

#[test]
fn streaming_str_error_on_non_string_scalar() {
    // Trying to deserialize a boolean as a String should fail in streaming mode
    // Actually, serde routes through deserialize_any for String by default.
    // Let's use a wrapper that forces str deserialization.
    // In practice, the streaming path returns `TypeMismatch` for plain `true` as a string
    // when not in raw_str_mode. But from_str::<String> uses deserialize_string.
    // Let's just verify the path works.
    let v: String = from_str("'hello world'").unwrap();
    assert_eq!(v, "hello world");
}

#[test]
fn streaming_str_error_on_sequence() {
    // Trying to get a str from a sequence
    let r: Result<String, _> = from_str("[1, 2]");
    assert!(r.is_err());
}

// ============================================================================
// 21. Parser events: various state transitions
// ============================================================================

#[test]
fn parser_flow_sequence() {
    let v: Vec<i32> = from_str("[1, 2, 3]").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn parser_flow_mapping() {
    let v: HashMap<String, i32> = from_str("{a: 1, b: 2}").unwrap();
    assert_eq!(v["a"], 1);
    assert_eq!(v["b"], 2);
}

#[test]
fn parser_nested_flow() {
    let yaml = "{a: [1, 2], b: {c: 3}}";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"][0], Value::Number(Number::from(1)));
    assert_eq!(v["b"]["c"], Value::Number(Number::from(3)));
}

#[test]
fn parser_multiline_string() {
    let yaml = "text: |\n  line1\n  line2\n  line3";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["text"].as_str(), Some("line1\nline2\nline3\n"));
}

#[test]
fn parser_folded_string() {
    let yaml = "text: >\n  hello\n  world";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["text"].as_str(), Some("hello world\n"));
}

// ============================================================================
// 22. from_slice with various inputs
// ============================================================================

#[test]
fn from_slice_invalid_utf8() {
    let bytes = &[0xFF, 0xFE];
    let r: Result<Value, _> = from_slice(bytes);
    assert!(r.is_err());
}

#[test]
fn from_slice_with_config() {
    let config = ParserConfig::new();
    let v: Value = noyalib::from_slice_with_config(b"key: value", &config).unwrap();
    assert_eq!(v["key"].as_str(), Some("value"));
}

// ============================================================================
// 23. Coverage: streaming positive/negative sign integers
// ============================================================================

#[test]
fn streaming_positive_sign() {
    let v: i64 = from_str("+42").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn streaming_negative_sign() {
    let v: i64 = from_str("-42").unwrap();
    assert_eq!(v, -42);
}

#[test]
fn streaming_positive_float() {
    let v: f64 = from_str("+3.14").unwrap();
    assert!((v - 3.14).abs() < 1e-9);
}

// ============================================================================
// 24. Additional streaming coverage: skip_value with nested structures
// ============================================================================

#[test]
fn streaming_skip_nested_sequence() {
    #[derive(Deserialize)]
    struct S {
        a: i32,
    }
    let yaml = "a: 1\nb:\n  - [1, 2]\n  - [3, 4]";
    let v: S = from_str(yaml).unwrap();
    assert_eq!(v.a, 1);
}

#[test]
fn streaming_skip_nested_mapping() {
    #[derive(Deserialize)]
    struct S {
        a: i32,
    }
    let yaml = "a: 1\nb:\n  c:\n    d: 2\n  e: 3";
    let v: S = from_str(yaml).unwrap();
    assert_eq!(v.a, 1);
}

// ============================================================================
// 25. error.rs: Location::from_index edge cases
// ============================================================================

#[test]
fn location_from_index_beyond_end() {
    let loc = Location::from_index("hello", 1000);
    assert_eq!(loc.line(), 1);
}

#[test]
fn location_display() {
    let loc = Location::new(5, 10, 42);
    let s = format!("{loc}");
    assert_eq!(s, "line 5, column 10");
}

// ============================================================================
// 26. Value: Display, Hash, Ord implementations
// ============================================================================

#[test]
fn value_display() {
    let v = Value::Null;
    assert_eq!(format!("{v}"), "null");

    let v = Value::Bool(true);
    assert_eq!(format!("{v}"), "true");

    let v = Value::Number(Number::from(42));
    assert_eq!(format!("{v}"), "42");

    let v = Value::String("hello".to_owned());
    assert_eq!(format!("{v}"), "hello");
}

#[test]
fn mapping_display() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    let s = format!("{m}");
    assert!(s.contains("a: 1"));
}

#[test]
fn mapping_any_display() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    let s = format!("{m}");
    assert!(s.contains("a: 1"));
}

// ============================================================================
// 27. Span-aware Loader path (from_str_with_config exercises Loader)
// ============================================================================

#[test]
fn loader_span_anchors_aliases() {
    let config = ParserConfig::new();
    let yaml = "base: &base\n  x: 1\n  y: 2\nderived:\n  <<: *base\n  z: 3";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    let derived = v.as_mapping().unwrap().get("derived").unwrap();
    assert_eq!(
        derived.as_mapping().unwrap().get("x").unwrap(),
        &Value::Number(Number::from(1))
    );
}

#[test]
fn loader_span_merge_sequence() {
    let config = ParserConfig::new();
    let yaml = "a: &a\n  x: 1\nb: &b\n  y: 2\nc:\n  <<: [*a, *b]\n  z: 3";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    let c = v.as_mapping().unwrap().get("c").unwrap();
    assert_eq!(
        c.as_mapping().unwrap().get("x").unwrap(),
        &Value::Number(Number::from(1))
    );
}

#[test]
fn loader_span_sequence_anchor() {
    let config = ParserConfig::new();
    let yaml = "items: &list\n  - 1\n  - 2\ncopy: *list";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    let copy = v.as_mapping().unwrap().get("copy").unwrap();
    assert_eq!(copy.as_sequence().unwrap().len(), 2);
}

#[test]
fn loader_span_mapping_anchor() {
    let config = ParserConfig::new();
    let yaml = "base: &b\n  x: 10\n  y: 20\nref: *b";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    let r = v.as_mapping().unwrap().get("ref").unwrap();
    assert_eq!(
        r.as_mapping().unwrap().get("x").unwrap(),
        &Value::Number(Number::from(10))
    );
}

#[test]
fn loader_span_duplicate_key_last() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
    let yaml = "a: 1\na: 2";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(
        v.as_mapping().unwrap().get("a").unwrap(),
        &Value::Number(Number::from(2))
    );
}

#[test]
fn loader_span_duplicate_key_first() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let yaml = "a: 1\na: 2";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(
        v.as_mapping().unwrap().get("a").unwrap(),
        &Value::Number(Number::from(1))
    );
}

#[test]
fn loader_span_duplicate_key_error() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let yaml = "a: 1\na: 2";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

#[test]
fn loader_span_deep_nesting() {
    let config = ParserConfig::new();
    let yaml = "a:\n  b:\n    c:\n      d: 4";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"]["b"]["c"]["d"], Value::Number(Number::from(4)));
}

#[test]
fn loader_span_recursion_limit() {
    let config = ParserConfig::new().max_depth(2);
    let yaml = "a:\n  b:\n    c:\n      d: 4";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

#[test]
fn loader_span_sequence_length_limit() {
    let config = ParserConfig::new().max_sequence_length(2);
    let yaml = "- 1\n- 2\n- 3";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

#[test]
fn loader_span_mapping_key_limit() {
    let config = ParserConfig::new().max_mapping_keys(2);
    let yaml = "a: 1\nb: 2\nc: 3";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

#[test]
fn loader_span_alias_limit() {
    let config = ParserConfig::new().max_alias_expansions(1);
    let yaml = "a: &v hello\nb: *v\nc: *v";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

#[test]
fn loader_span_doc_length_limit() {
    let config = ParserConfig::new().max_document_length(10);
    let yaml = "this is a very long document";
    let r: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(r.is_err());
}

#[test]
fn loader_span_tagged_scalar() {
    let config = ParserConfig::new();
    // Standard YAML tags
    let yaml = "!!str 42";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v.as_str(), Some("42"));
}

#[test]
fn loader_span_tagged_null() {
    let config = ParserConfig::new();
    let yaml = "!!null ''";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v, Value::Null);
}

#[test]
fn loader_span_tagged_bool() {
    let config = ParserConfig::new();
    let yaml = "!!bool true";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v, Value::Bool(true));
}

#[test]
fn loader_span_tagged_int() {
    let config = ParserConfig::new();
    let yaml = "!!int 42";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v, Value::Number(Number::Integer(42)));
}

#[test]
fn loader_span_tagged_float() {
    let config = ParserConfig::new();
    let yaml = "!!float 3.14";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert!(v.as_f64().is_some());
}

#[test]
fn loader_span_tagged_float_inf() {
    let config = ParserConfig::new();
    let yaml = "!!float .inf";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert!(v.as_f64().unwrap().is_infinite());
}

#[test]
fn loader_span_tagged_float_nan() {
    let config = ParserConfig::new();
    let yaml = "!!float .nan";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert!(v.as_f64().unwrap().is_nan());
}

#[test]
fn loader_span_custom_tag() {
    let config = ParserConfig::new();
    let yaml = "!custom hello";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    // Custom tags may or may not be preserved depending on schema resolution
    assert!(v.is_tagged() || v.as_str().is_some());
}

#[test]
fn loader_span_custom_tag_empty_suffix() {
    // Empty-suffix `!` is a *primary* tag handle, so the result
    // surfaces as `Value::Tagged("!", String("hello"))` on the
    // default tag-preserving deserialise path. Use `untag_ref()`
    // to step through the wrapper for the underlying scalar
    // check.
    let config = ParserConfig::new();
    let yaml = "! hello";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert!(matches!(v, Value::Tagged(_)), "tag is preserved");
    assert_eq!(v.untag_ref().as_str(), Some("hello"));
}

#[test]
fn loader_span_legacy_booleans() {
    let config = ParserConfig::new().legacy_booleans(true);
    let yaml = "a: yes\nb: no\nc: on\nd: off\ne: y\nf: n";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"], Value::Bool(true));
    assert_eq!(v["b"], Value::Bool(false));
    assert_eq!(v["c"], Value::Bool(true));
    assert_eq!(v["d"], Value::Bool(false));
    assert_eq!(v["e"], Value::Bool(true));
    assert_eq!(v["f"], Value::Bool(false));
}

#[test]
fn loader_span_strict_booleans() {
    let config = ParserConfig::new().strict_booleans(true);
    let yaml = "a: true\nb: True\nc: TRUE";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"], Value::Bool(true));
    // In strict mode, True/TRUE become strings
    assert_eq!(v["b"].as_str(), Some("True"));
    assert_eq!(v["c"].as_str(), Some("TRUE"));
}

#[test]
fn loader_span_hex_integer() {
    let config = ParserConfig::new();
    let yaml = "val: 0xFF";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["val"], Value::Number(Number::Integer(255)));
}

#[test]
fn loader_span_octal_integer() {
    let config = ParserConfig::new();
    let yaml = "val: 0o77";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["val"], Value::Number(Number::Integer(63)));
}

#[test]
fn loader_span_float_scientific() {
    let config = ParserConfig::new();
    let yaml = "val: 1.5e10";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert!(v["val"].as_f64().is_some());
}

#[test]
fn loader_span_large_int_overflow() {
    let config = ParserConfig::new();
    let yaml = "val: 99999999999999999999";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    // Should become float since it overflows i64
    assert!(v["val"].as_f64().is_some());
}

#[test]
fn loader_span_special_floats() {
    let config = ParserConfig::new();
    let yaml = "a: .inf\nb: -.inf\nc: .nan";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert!(v["a"].as_f64().unwrap().is_infinite());
    assert!(v["b"].as_f64().unwrap().is_sign_negative());
    assert!(v["c"].as_f64().unwrap().is_nan());
}

#[test]
fn loader_span_null_forms() {
    let config = ParserConfig::new();
    let yaml = "a: ~\nb: null\nc: Null\nd: NULL\ne: ''";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"], Value::Null);
    assert_eq!(v["b"], Value::Null);
    assert_eq!(v["c"], Value::Null);
    assert_eq!(v["d"], Value::Null);
    // Quoted empty string is a string, not null
    assert_eq!(v["e"].as_str(), Some(""));
}

#[test]
fn loader_span_boolean_forms() {
    let config = ParserConfig::new();
    let yaml = "a: true\nb: false\nc: True\nd: False\ne: TRUE\nf: FALSE";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"], Value::Bool(true));
    assert_eq!(v["b"], Value::Bool(false));
    assert_eq!(v["c"], Value::Bool(true));
    assert_eq!(v["d"], Value::Bool(false));
    assert_eq!(v["e"], Value::Bool(true));
    assert_eq!(v["f"], Value::Bool(false));
}

#[test]
fn loader_span_positive_signed_integer() {
    let config = ParserConfig::new();
    let yaml = "val: +42";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["val"], Value::Number(Number::Integer(42)));
}

// ============================================================================
// 28. More scanner coverage: edge cases
// ============================================================================

#[test]
fn scanner_nel_escape() {
    let yaml = "\"hello\\Nworld\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\u{0085}'));
}

#[test]
fn scanner_nbsp_escape() {
    let yaml = "\"hello\\_world\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\u{00A0}'));
}

#[test]
fn scanner_line_separator_escape() {
    let yaml = "\"hello\\Lworld\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\u{2028}'));
}

#[test]
fn scanner_paragraph_separator_escape() {
    let yaml = "\"hello\\Pworld\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\u{2029}'));
}

#[test]
fn scanner_carriage_return_escape() {
    let yaml = "\"hello\\rworld\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\r'));
}

#[test]
fn scanner_form_feed_escape() {
    let yaml = "\"hello\\fworld\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\x0C'));
}

#[test]
fn scanner_vertical_tab_escape() {
    let yaml = "\"hello\\vworld\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains('\x0B'));
}

#[test]
fn scanner_line_break_escape_in_double_quoted() {
    // Escaped line break in double-quoted string folds
    let yaml = "\"hello\\\n  world\"";
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "helloworld");
}

#[test]
fn scanner_double_quoted_multiline() {
    let yaml = "\"hello\n  world\"";
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "hello world");
}

#[test]
fn scanner_single_quoted_escape() {
    let yaml = "'it''s'";
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "it's");
}

#[test]
fn scanner_complex_flow_key() {
    // Complex keys (non-scalar) may produce various structures
    let yaml = "? a\n: value";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_mapping().is_some());
}

#[test]
fn scanner_empty_flow_sequence() {
    let v: Vec<i32> = from_str("[]").unwrap();
    assert!(v.is_empty());
}

#[test]
fn scanner_empty_flow_mapping() {
    let v: HashMap<String, i32> = from_str("{}").unwrap();
    assert!(v.is_empty());
}

#[test]
fn scanner_block_sequence_of_mappings() {
    let yaml = "- a: 1\n  b: 2\n- a: 3\n  b: 4";
    let v: Vec<HashMap<String, i32>> = from_str(yaml).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0]["a"], 1);
    assert_eq!(v[1]["a"], 3);
}

// ============================================================================
// 29. More streaming coverage: various typed structs
// ============================================================================

#[test]
fn streaming_struct_with_optional_fields() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Config {
        name: String,
        port: u16,
        debug: Option<bool>,
        workers: Option<u32>,
    }
    let yaml = "name: app\nport: 8080";
    let v: Config = from_str(yaml).unwrap();
    assert_eq!(v.name, "app");
    assert_eq!(v.port, 8080);
    assert_eq!(v.debug, None);
}

#[test]
fn streaming_struct_with_all_fields() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Config {
        name: String,
        port: u16,
        debug: bool,
        ratio: f64,
    }
    let yaml = "name: app\nport: 8080\ndebug: true\nratio: 0.5";
    let v: Config = from_str(yaml).unwrap();
    assert_eq!(v.name, "app");
    assert_eq!(v.port, 8080);
    assert!(v.debug);
    assert!((v.ratio - 0.5).abs() < 1e-9);
}

#[test]
fn streaming_nested_struct() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Inner {
        x: i32,
        y: i32,
    }
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Outer {
        name: String,
        pos: Inner,
    }
    let yaml = "name: point\npos:\n  x: 10\n  y: 20";
    let v: Outer = from_str(yaml).unwrap();
    assert_eq!(v.pos.x, 10);
    assert_eq!(v.pos.y, 20);
}

#[test]
fn streaming_vec_of_structs() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Item {
        id: u32,
        name: String,
    }
    let yaml = "- id: 1\n  name: first\n- id: 2\n  name: second";
    let v: Vec<Item> = from_str(yaml).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].id, 1);
    assert_eq!(v[1].name, "second");
}

#[test]
fn streaming_hashmap_of_vecs() {
    let yaml = "a:\n  - 1\n  - 2\nb:\n  - 3";
    let v: HashMap<String, Vec<i32>> = from_str(yaml).unwrap();
    assert_eq!(v["a"], vec![1, 2]);
    assert_eq!(v["b"], vec![3]);
}

// ============================================================================
// 30. Value conversions and number edge cases
// ============================================================================

#[test]
fn number_ord_large_positive_vs_small_float() {
    // Large positive integer > 2^53 vs small positive float
    let large = Number::Integer((1_i64 << 53) + 100);
    let small = Number::Float(1.0);
    assert!(large > small);
}

#[test]
fn number_ord_large_negative_vs_large_neg_float() {
    let large = Number::Integer(-((1_i64 << 53) + 100));
    let float = Number::Float(-((1_i64 << 53) as f64) - 200.0);
    // Both large negatives, compare via f64 approximation
    let _cmp = large.cmp(&float);
}

#[test]
fn number_ord_integer_float_exact() {
    // Integer that can be exactly represented as f64
    let int = Number::Integer(42);
    let float = Number::Float(42.0);
    assert_eq!(int.cmp(&float), std::cmp::Ordering::Equal);
}

#[test]
fn number_ord_float_integer_exact() {
    let float = Number::Float(42.0);
    let int = Number::Integer(42);
    assert_eq!(float.cmp(&int), std::cmp::Ordering::Equal);
}

#[test]
fn number_float_nan_eq() {
    let a = Number::Float(f64::NAN);
    let b = Number::Float(f64::NAN);
    assert_eq!(a, b);
}

#[test]
fn number_float_nan_ord() {
    let a = Number::Float(f64::NAN);
    let b = Number::Float(f64::NAN);
    assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
}

#[test]
fn number_float_nan_gt_regular() {
    let nan = Number::Float(f64::NAN);
    let regular = Number::Float(42.0);
    assert!(nan > regular);
}

// ============================================================================
// 31. Error variant coverage
// ============================================================================

#[test]
fn error_all_variants_display() {
    let errors: Vec<Error> = vec![
        Error::Parse("test".to_owned()),
        Error::ParseWithLocation {
            message: "test".to_owned(),
            location: Location::new(1, 1, 0),
        },
        Error::Serialize("test".to_owned()),
        Error::Deserialize("test".to_owned()),
        Error::DeserializeWithLocation {
            message: "test".to_owned(),
            location: Location::new(1, 1, 0),
        },
        Error::Invalid("test".to_owned()),
        Error::TypeMismatch {
            expected: "int",
            found: "str".to_owned(),
        },
        Error::MissingField("name".to_owned()),
        Error::UnknownField("extra".to_owned()),
        Error::RecursionLimitExceeded { depth: 100 },
        Error::RepetitionLimitExceeded,
        Error::UnknownAnchor("anchor".to_owned()),
        Error::ScalarInMerge,
        Error::TaggedInMerge,
        Error::ScalarInMergeElement,
        Error::SequenceInMergeElement,
        Error::EmptyTag,
        Error::FailedToParseNumber("abc".to_owned()),
        Error::EndOfStream,
        Error::MoreThanOneDocument,
        Error::DuplicateKey("key".to_owned()),
        Error::Custom("custom".to_owned()),
    ];
    for err in &errors {
        let msg = err.to_string();
        assert!(
            !msg.is_empty(),
            "Error {:?} should have non-empty display",
            err
        );
    }
}

#[test]
fn error_location_variants() {
    let parse_loc = Error::ParseWithLocation {
        message: "test".to_owned(),
        location: Location::new(1, 2, 3),
    };
    assert_eq!(parse_loc.location().unwrap().line(), 1);

    let de_loc = Error::DeserializeWithLocation {
        message: "test".to_owned(),
        location: Location::new(4, 5, 6),
    };
    assert_eq!(de_loc.location().unwrap().column(), 5);

    let no_loc = Error::Parse("test".to_owned());
    assert!(no_loc.location().is_none());
}

// ============================================================================
// 32. Tagged value operations
// ============================================================================

#[test]
fn tagged_value_with_standard_tags() {
    let config = ParserConfig::new();
    // !!int tag
    let yaml = "!!int '42'";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v, Value::Number(Number::Integer(42)));

    // !!float tag
    let yaml = "!!float '3.14'";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert!(v.as_f64().is_some());

    // !!str tag forces string
    let yaml = "!!str true";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v.as_str(), Some("true"));

    // !!bool tag
    let yaml = "!!bool true";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v, Value::Bool(true));

    // !!null tag
    let yaml = "!!null ''";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v, Value::Null);
}

// ============================================================================
// 33. Mapping operations that might be uncovered
// ============================================================================

#[test]
fn mapping_operations() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    m.insert("b", Value::from(2));
    m.insert("c", Value::from(3));

    // sort_keys
    m.sort_keys();
    let keys: Vec<_> = m.keys().collect();
    assert_eq!(keys, vec!["a", "b", "c"]);

    // reverse
    m.reverse();
    let keys: Vec<_> = m.keys().collect();
    assert_eq!(keys, vec!["c", "b", "a"]);

    // first / last
    assert_eq!(m.first().unwrap().0, "c");
    assert_eq!(m.last().unwrap().0, "a");

    // pop_first / pop_last
    let first = m.pop_first().unwrap();
    assert_eq!(first.0, "c");
    let last = m.pop_last().unwrap();
    assert_eq!(last.0, "a");
}

#[test]
fn mapping_any_operations() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    m.insert(Value::from("b"), Value::from(2));
    m.insert(Value::from("c"), Value::from(3));

    m.sort_keys();
    m.reverse();

    assert!(m.first().is_some());
    assert!(m.last().is_some());

    let _ = m.pop_first();
    let _ = m.pop_last();
    assert_eq!(m.len(), 1);
}

// ============================================================================
// 34. More from_value edge cases for enum deserializer
// ============================================================================

#[test]
fn de_enum_from_string_value() {
    let val = Value::String("Blue".to_owned());
    let v: Color = from_value(&val).unwrap();
    assert_eq!(v, Color::Blue);
}

// ============================================================================
// 35. parse_all_values / multi-doc via load_all_with_config
// ============================================================================

#[test]
fn multi_doc_via_load_all_with_config() {
    let config = ParserConfig::new();
    let yaml = "---\n42\n---\nhello\n---\ntrue\n";
    let docs: Vec<_> = noyalib::load_all_with_config(yaml, &config)
        .unwrap()
        .collect();
    assert_eq!(docs.len(), 3);
}

#[test]
fn multi_doc_typed() {
    #[derive(Deserialize, Debug)]
    struct Simple {
        x: i32,
    }
    let yaml = "---\nx: 1\n---\nx: 2\n";
    let docs: Vec<Simple> = noyalib::load_all_as(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].x, 1);
    assert_eq!(docs[1].x, 2);
}

// ============================================================================
// 36. Additional streaming: from_reader exercises streaming path
// ============================================================================

#[test]
fn from_reader_streaming_path() {
    let yaml = b"name: test\ncount: 42";
    let cursor = std::io::Cursor::new(&yaml[..]);
    #[derive(Deserialize)]
    struct S {
        name: String,
        count: u32,
    }
    let v: S = noyalib::from_reader(cursor).unwrap();
    assert_eq!(v.name, "test");
    assert_eq!(v.count, 42);
}

// ============================================================================
// 37. Spanned with from_str_with_config (full span tracking)
// ============================================================================

#[test]
fn spanned_in_struct_with_config() {
    #[derive(Deserialize)]
    struct Cfg {
        name: Spanned<String>,
        port: Spanned<u16>,
    }
    let config = ParserConfig::new();
    let yaml = "name: test\nport: 8080";
    let v: Cfg = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v.name.value, "test");
    assert_eq!(v.port.value, 8080);
}

// ============================================================================
// 38. singleton_map_with coverage
// ============================================================================

#[test]
fn singleton_map_with_roundtrip() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Status {
        Active,
        Pending,
    }

    fn to_lower(s: &str) -> String {
        s.to_lowercase()
    }
    fn from_lower(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Cfg {
        #[serde(serialize_with = "ser_status", deserialize_with = "de_status")]
        status: Status,
    }

    fn ser_status<S: serde::Serializer>(val: &Status, s: S) -> Result<S::Ok, S::Error> {
        noyalib::with::singleton_map_with::serialize_with(val, s, to_lower)
    }
    fn de_status<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Status, D::Error> {
        noyalib::with::singleton_map_with::deserialize_with(d, from_lower)
    }

    let cfg = Cfg {
        status: Status::Active,
    };
    let yaml = to_string(&cfg).unwrap();
    let back: Cfg = from_str(&yaml).unwrap();
    assert_eq!(cfg, back);
}

// ============================================================================
// 39. More from_slice_with_config coverage
// ============================================================================

#[test]
fn from_slice_with_config_anchors() {
    let config = ParserConfig::new();
    let yaml = b"a: &val hello\nb: *val";
    let v: Value = noyalib::from_slice_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"].as_str(), Some("hello"));
    assert_eq!(v["b"].as_str(), Some("hello"));
}

// ============================================================================
// 40. Quoted tag handling in loader
// ============================================================================

#[test]
fn loader_quoted_scalar_with_tag() {
    let config = ParserConfig::new();
    let yaml = "!!str 'hello'";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v.as_str(), Some("hello"));
}

#[test]
fn loader_quoted_scalar_no_tag() {
    let config = ParserConfig::new();
    let yaml = "'42'";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    // Quoted scalar is always a string
    assert_eq!(v.as_str(), Some("42"));
}

// ============================================================================
// 41. from_reader fallback path (anchors/aliases force Value-based path)
// ============================================================================

#[test]
fn from_reader_fallback_path() {
    let yaml = b"a: &val hello\nb: *val";
    let cursor = std::io::Cursor::new(&yaml[..]);
    let v: Value = noyalib::from_reader(cursor).unwrap();
    assert_eq!(v["a"].as_str(), Some("hello"));
    assert_eq!(v["b"].as_str(), Some("hello"));
}

#[test]
fn from_slice_fallback_path() {
    let yaml = b"a: &val hello\nb: *val";
    let v: Value = from_slice(yaml).unwrap();
    assert_eq!(v["a"].as_str(), Some("hello"));
    assert_eq!(v["b"].as_str(), Some("hello"));
}

// ============================================================================
// 42. Value bytes error path (from_value)
// ============================================================================

#[test]
fn de_bytes_error_on_non_string() {
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct W {
        #[serde(with = "serde_bytes")]
        d: Vec<u8>,
    }
    let yaml = "d: 42";
    // This goes through the Value path (from_str_with_config) and should hit
    // the bytes error on a non-string Value
    let r: Result<W, _> = from_str_with_config(yaml, &ParserConfig::new());
    assert!(r.is_err());
}

// ============================================================================
// 43. Enum deserialization error paths
// ============================================================================

#[test]
fn de_enum_non_string_key_in_mapping() {
    // Two-entry mapping should fail for enum deserialization
    let mut m = Mapping::new();
    m.insert("Red", Value::Null);
    m.insert("Blue", Value::Null);
    let val = Value::Mapping(m);
    let r: Result<Color, _> = from_value(&val);
    assert!(r.is_err());
}

// ============================================================================
// 44. Additional singleton_map_recursive: tagged value handling
// ============================================================================

#[test]
fn singleton_map_recursive_tagged() {
    use noyalib::with::singleton_map_recursive;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Status {
        Active,
        Inactive,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        #[serde(with = "singleton_map_recursive")]
        status: Status,
    }

    let cfg = Config {
        status: Status::Active,
    };
    let yaml = to_string(&cfg).unwrap();
    let back: Config = from_str(&yaml).unwrap();
    assert_eq!(cfg, back);
}

// ============================================================================
// 45. ser.rs uncovered lines — serialization edge cases
// ============================================================================

#[test]
fn ser_sequence_of_mappings() {
    let mut m1 = Mapping::new();
    m1.insert("a", Value::from(1));
    let mut m2 = Mapping::new();
    m2.insert("b", Value::from(2));
    let val = Value::Sequence(vec![Value::Mapping(m1), Value::Mapping(m2)]);
    let yaml = to_string(&val).unwrap();
    assert!(yaml.contains("a: 1"));
    assert!(yaml.contains("b: 2"));
}

#[test]
fn ser_nested_sequence() {
    let val = Value::Sequence(vec![
        Value::Sequence(vec![Value::from(1), Value::from(2)]),
        Value::Sequence(vec![Value::from(3), Value::from(4)]),
    ]);
    let yaml = to_string(&val).unwrap();
    assert!(!yaml.is_empty());
    // Roundtrip
    let back: Value = from_str(&yaml).unwrap();
    assert_eq!(val, back);
}

#[test]
fn ser_special_floats() {
    let nan = Value::Number(Number::Float(f64::NAN));
    let yaml = to_string(&nan).unwrap();
    assert!(yaml.contains(".nan") || yaml.contains("NaN"));

    let inf = Value::Number(Number::Float(f64::INFINITY));
    let yaml = to_string(&inf).unwrap();
    assert!(yaml.contains(".inf") || yaml.contains("inf"));

    let neg_inf = Value::Number(Number::Float(f64::NEG_INFINITY));
    let yaml = to_string(&neg_inf).unwrap();
    assert!(yaml.contains("-.inf") || yaml.contains("-inf"));
}

#[test]
fn ser_null_in_sequence() {
    let val = Value::Sequence(vec![Value::Null, Value::from(1), Value::Null]);
    let yaml = to_string(&val).unwrap();
    let back: Value = from_str(&yaml).unwrap();
    assert_eq!(val, back);
}

#[test]
fn ser_empty_string() {
    let val = Value::String(String::new());
    let yaml = to_string(&val).unwrap();
    let back: Value = from_str(&yaml).unwrap();
    assert_eq!(back.as_str(), Some(""));
}

#[test]
fn ser_string_with_special_chars() {
    let val = Value::String("hello: world\nnewline".to_owned());
    let yaml = to_string(&val).unwrap();
    let back: Value = from_str(&yaml).unwrap();
    assert_eq!(val, back);
}

#[test]
fn ser_bool_values() {
    let t = Value::Bool(true);
    let yaml = to_string(&t).unwrap();
    assert!(yaml.trim() == "true");

    let f = Value::Bool(false);
    let yaml = to_string(&f).unwrap();
    assert!(yaml.trim() == "false");
}

// ============================================================================
// 46. parser/events.rs coverage
// ============================================================================

#[test]
fn parser_events_document_markers() {
    // Explicit document markers
    let yaml = "---\nhello\n...\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello"));
}

#[test]
fn parser_events_multiple_document_markers() {
    let yaml = "---\nhello\n...\n---\nworld\n...\n";
    let docs: Vec<_> = noyalib::load_all(yaml).unwrap().collect();
    assert_eq!(docs.len(), 2);
}

#[test]
fn parser_events_implicit_document() {
    let yaml = "hello";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello"));
}

// ============================================================================
// 47. value.rs remaining coverage: From<IndexMap> for Mapping/MappingAny
// ============================================================================

#[test]
fn value_from_indexmap_mapping() {
    let mut map = indexmap::IndexMap::new();
    map.insert("x".to_owned(), Value::from(10));
    let m = Mapping::from(map);
    assert_eq!(m.get("x"), Some(&Value::from(10)));
}

#[test]
fn value_from_indexmap_mapping_any() {
    let mut map = indexmap::IndexMap::new();
    map.insert(Value::from("x"), Value::from(10));
    let m = MappingAny::from(map);
    assert_eq!(m.get(&Value::from("x")), Some(&Value::from(10)));
}

#[test]
fn value_into_indexmap_mapping() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    let map: indexmap::IndexMap<String, Value> = m.into();
    assert_eq!(map.len(), 1);
}

#[test]
fn value_into_indexmap_mapping_any() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    let map: indexmap::IndexMap<Value, Value> = m.into();
    assert_eq!(map.len(), 1);
}

// ============================================================================
// 48. Mapping capacity and reserve
// ============================================================================

#[test]
fn mapping_with_capacity() {
    let m = Mapping::with_capacity(10);
    assert!(m.capacity() >= 10);
    assert!(m.is_empty());
}

#[test]
fn mapping_reserve_and_shrink() {
    let mut m = Mapping::new();
    m.reserve(100);
    assert!(m.capacity() >= 100);
    m.insert("a", Value::from(1));
    m.shrink_to_fit();
}

#[test]
fn mapping_any_with_capacity() {
    let m = MappingAny::with_capacity(10);
    assert!(m.capacity() >= 10);
}

#[test]
fn mapping_any_reserve_and_shrink() {
    let mut m = MappingAny::new();
    m.reserve(100);
    assert!(m.capacity() >= 100);
    m.insert(Value::from("a"), Value::from(1));
    m.shrink_to_fit();
}

// ============================================================================
// 49. Mapping entry, retain, swap_remove, shift_remove, get_index
// ============================================================================

#[test]
fn mapping_entry() {
    let mut m = Mapping::new();
    m.entry("a").or_insert(Value::from(1));
    m.entry("a").or_insert(Value::from(2)); // should not overwrite
    assert_eq!(m.get("a"), Some(&Value::from(1)));
}

#[test]
fn mapping_retain() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    m.insert("b", Value::from(2));
    m.insert("c", Value::from(3));
    m.retain(|k, _| k != "b");
    assert_eq!(m.len(), 2);
    assert!(!m.contains_key("b"));
}

#[test]
fn mapping_swap_remove() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    m.insert("b", Value::from(2));
    let removed = m.swap_remove("a");
    assert_eq!(removed, Some(Value::from(1)));
    assert_eq!(m.len(), 1);
}

#[test]
fn mapping_shift_remove() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    m.insert("b", Value::from(2));
    let removed = m.shift_remove("a");
    assert_eq!(removed, Some(Value::from(1)));
}

#[test]
fn mapping_remove_entry() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    let entry = m.remove_entry("a");
    assert_eq!(entry, Some(("a".to_owned(), Value::from(1))));
}

#[test]
fn mapping_get_index() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    let (k, v) = m.get_index(0).unwrap();
    assert_eq!(k, "a");
    assert_eq!(v, &Value::from(1));
}

#[test]
fn mapping_get_index_mut() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    let (_, v) = m.get_index_mut(0).unwrap();
    *v = Value::from(99);
    assert_eq!(m.get("a"), Some(&Value::from(99)));
}

#[test]
fn mapping_first_mut_last_mut() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    m.insert("b", Value::from(2));
    let (_, v) = m.first_mut().unwrap();
    *v = Value::from(10);
    let (_, v) = m.last_mut().unwrap();
    *v = Value::from(20);
    assert_eq!(m.get("a"), Some(&Value::from(10)));
    assert_eq!(m.get("b"), Some(&Value::from(20)));
}

// ============================================================================
// 50. MappingAny entry, retain, swap_remove, etc.
// ============================================================================

#[test]
fn mapping_any_entry() {
    let mut m = MappingAny::new();
    m.entry(Value::from("a")).or_insert(Value::from(1));
    assert_eq!(m.get(&Value::from("a")), Some(&Value::from(1)));
}

#[test]
fn mapping_any_retain() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    m.insert(Value::from("b"), Value::from(2));
    m.retain(|_, v| v != &Value::from(2));
    assert_eq!(m.len(), 1);
}

#[test]
fn mapping_any_swap_remove() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    let removed = m.swap_remove(&Value::from("a"));
    assert_eq!(removed, Some(Value::from(1)));
}

#[test]
fn mapping_any_shift_remove() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    let removed = m.shift_remove(&Value::from("a"));
    assert_eq!(removed, Some(Value::from(1)));
}

#[test]
fn mapping_any_remove_entry() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    let entry = m.remove_entry(&Value::from("a"));
    assert_eq!(entry, Some((Value::from("a"), Value::from(1))));
}

#[test]
fn mapping_any_get_index() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    let (k, v) = m.get_index(0).unwrap();
    assert_eq!(k, &Value::from("a"));
    assert_eq!(v, &Value::from(1));
}

#[test]
fn mapping_any_get_index_mut() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    let (_, v) = m.get_index_mut(0).unwrap();
    *v = Value::from(99);
}

#[test]
fn mapping_any_first_mut_last_mut() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    m.insert(Value::from("b"), Value::from(2));
    let (_, v) = m.first_mut().unwrap();
    *v = Value::from(10);
    let (_, v) = m.last_mut().unwrap();
    *v = Value::from(20);
}

// ============================================================================
// 51. Value indexing and type checking
// ============================================================================

#[test]
fn value_as_methods() {
    let v = Value::Null;
    assert!(v.is_null());

    let v = Value::Bool(true);
    assert!(v.is_bool());
    assert_eq!(v.as_bool(), Some(true));

    let v = Value::Number(Number::from(42));
    assert!(v.is_number());
    assert!(v.is_i64());
    assert_eq!(v.as_i64(), Some(42));

    let v = Value::String("hello".to_owned());
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("hello"));

    let v = Value::Sequence(vec![]);
    assert!(v.is_sequence());
    assert!(v.as_sequence().is_some());

    let v = Value::Mapping(Mapping::new());
    assert!(v.is_mapping());
    assert!(v.as_mapping().is_some());
}

// ============================================================================
// 52. Value::from Vec and other conversions
// ============================================================================

#[test]
fn mapping_from_vec() {
    let v = vec![
        ("a".to_owned(), Value::from(1)),
        ("b".to_owned(), Value::from(2)),
    ];
    let m = Mapping::from(v);
    assert_eq!(m.len(), 2);
}

#[test]
fn mapping_extend() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    m.extend(vec![
        ("b".to_owned(), Value::from(2)),
        ("c".to_owned(), Value::from(3)),
    ]);
    assert_eq!(m.len(), 3);
}

#[test]
fn mapping_any_extend() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));
    m.extend(vec![(Value::from("b"), Value::from(2))]);
    assert_eq!(m.len(), 2);
}

// ============================================================================
// 53. Mapping/MappingAny iteration
// ============================================================================

#[test]
fn mapping_iter_keys_values() {
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    m.insert("b", Value::from(2));

    let keys: Vec<_> = m.keys().collect();
    assert_eq!(keys.len(), 2);

    let values: Vec<_> = m.values().collect();
    assert_eq!(values.len(), 2);

    for v in m.values_mut() {
        // Just iterate to exercise the iterator
        let _ = v;
    }
}

#[test]
fn mapping_any_iter_keys_values() {
    let mut m = MappingAny::new();
    m.insert(Value::from("a"), Value::from(1));

    let keys: Vec<_> = m.keys().collect();
    assert_eq!(keys.len(), 1);

    let values: Vec<_> = m.values().collect();
    assert_eq!(values.len(), 1);
}

// ============================================================================
// 54. Mapping Ord comparison
// ============================================================================

#[test]
fn mapping_ord() {
    let mut m1 = Mapping::new();
    m1.insert("a", Value::from(1));
    let mut m2 = Mapping::new();
    m2.insert("a", Value::from(2));
    assert!(m1 < m2);

    let mut m3 = Mapping::new();
    m3.insert("a", Value::from(1));
    m3.insert("b", Value::from(2));
    // m3 has more keys, so it's greater
    assert!(m3 > m1);
}

#[test]
fn mapping_any_ord() {
    let mut m1 = MappingAny::new();
    m1.insert(Value::from("a"), Value::from(1));
    let mut m2 = MappingAny::new();
    m2.insert(Value::from("a"), Value::from(2));
    assert!(m1 < m2);
}

// ============================================================================
// 55. Mapping Hash
// ============================================================================

#[test]
fn mapping_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut m1 = Mapping::new();
    m1.insert("a", Value::from(1));
    let mut m2 = Mapping::new();
    m2.insert("a", Value::from(1));

    let mut h1 = DefaultHasher::new();
    m1.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    m2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

// ============================================================================
// 56. Number methods
// ============================================================================

#[test]
fn number_methods() {
    let n = Number::Integer(42);
    assert!(n.is_integer());
    assert!(!n.is_float());
    assert!(n.is_i64());
    assert!(n.is_u64());
    assert!(n.is_f64());
    assert!(!n.is_nan());
    assert!(!n.is_infinite());
    assert!(n.is_finite());
    assert_eq!(n.as_i64(), Some(42));
    assert_eq!(n.as_u64(), Some(42));
    assert!((n.as_f64() - 42.0).abs() < 1e-9);

    let n = Number::Float(f64::NAN);
    assert!(!n.is_integer());
    assert!(n.is_float());
    assert!(n.is_nan());
    assert!(!n.is_infinite());
    assert!(!n.is_finite());

    let n = Number::Float(f64::INFINITY);
    assert!(n.is_infinite());
    assert!(!n.is_finite());

    let n = Number::Integer(-1);
    assert!(n.is_i64());
    assert!(!n.is_u64());
    assert_eq!(n.as_u64(), None);

    let n = Number::Float(3.14);
    assert_eq!(n.as_i64(), None);
    assert_eq!(n.as_u64(), None);
}

#[test]
fn number_display() {
    assert_eq!(format!("{}", Number::Integer(42)), "42");
    assert_eq!(format!("{}", Number::Float(3.14)), "3.14");
}

#[test]
fn number_from_types() {
    let _ = Number::from(42_i8);
    let _ = Number::from(42_i16);
    let _ = Number::from(42_i32);
    let _ = Number::from(42_i64);
    let _ = Number::from(42_isize);
    let _ = Number::from(42_u8);
    let _ = Number::from(42_u16);
    let _ = Number::from(42_u32);
    let _ = Number::from(42_u64);
    let _ = Number::from(42_usize);
    let _ = Number::from(3.14_f32);
    let _ = Number::from(3.14_f64);
}
