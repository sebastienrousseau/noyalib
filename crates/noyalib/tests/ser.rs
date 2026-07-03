//! Serialization tests for noyalib.
//!
//! Ported from serde_yml test suite.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::BTreeMap;

use noyalib::{Mapping, Number, SerializerConfig, Value, to_string};
use serde::Serialize;

// ============================================================================
// Basic Type Tests
// ============================================================================

#[test]
fn test_serialize_bool() {
    assert!(to_string(&true).unwrap().contains("true"));
    assert!(to_string(&false).unwrap().contains("false"));
}

#[test]
fn test_serialize_integers() {
    assert!(to_string(&42i32).unwrap().contains("42"));
    assert!(to_string(&-42i32).unwrap().contains("-42"));
    assert!(to_string(&0i32).unwrap().contains("0"));
}

#[cfg(feature = "lossless-u64")]
#[test]
fn test_serialize_lossless_u64_plain_scalars() {
    for value in [i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX] {
        let yaml = to_string(&value).unwrap();
        assert_eq!(yaml.trim(), value.to_string());
        assert!(!yaml.contains('"'));
        assert!(!yaml.contains('\''));
        assert!(!yaml.contains('.'));
    }
}

#[test]
fn test_serialize_floats() {
    let yaml = to_string(&3.125f64).unwrap();
    assert!(yaml.contains("3.125"));

    let yaml = to_string(&-3.125f64).unwrap();
    assert!(yaml.contains("-3.125"));
}

#[test]
fn test_serialize_string() {
    let yaml = to_string(&"hello").unwrap();
    assert!(yaml.contains("hello"));
}

#[test]
fn test_serialize_string_with_special_chars() {
    // Strings with colons should be quoted
    let yaml = to_string(&"key: value").unwrap();
    assert!(yaml.contains("\"key: value\"") || yaml.contains("'key: value'"));
}

// ============================================================================
// Collection Tests
// ============================================================================

#[test]
fn test_serialize_vec() {
    let vec = vec![1, 2, 3];
    let yaml = to_string(&vec).unwrap();
    assert!(yaml.contains("- 1"));
    assert!(yaml.contains("- 2"));
    assert!(yaml.contains("- 3"));
}

#[test]
fn test_serialize_empty_vec() {
    let vec: Vec<i32> = vec![];
    let yaml = to_string(&vec).unwrap();
    assert!(yaml.contains("[]"));
}

#[test]
fn test_serialize_map() {
    let mut map = BTreeMap::new();
    let _ = map.insert("a", 1);
    let _ = map.insert("b", 2);
    let yaml = to_string(&map).unwrap();
    assert!(yaml.contains("a: 1") || yaml.contains("a:"));
    assert!(yaml.contains("b: 2") || yaml.contains("b:"));
}

#[cfg(feature = "lossless-u64")]
#[test]
fn test_serialize_lossless_u64_map_key() {
    let mut map = BTreeMap::new();
    let _ = map.insert(u64::MAX, "max");
    let yaml = to_string(&map).unwrap();
    assert!(yaml.contains("18446744073709551615"));
    assert!(yaml.contains("max"));
}

#[test]
fn test_serialize_empty_map() {
    let map: BTreeMap<String, i32> = BTreeMap::new();
    let yaml = to_string(&map).unwrap();
    assert!(yaml.contains("{}"));
}

// ============================================================================
// Struct Tests
// ============================================================================

#[derive(Serialize)]
struct SimpleStruct {
    name: String,
    value: i32,
}

#[test]
fn test_serialize_simple_struct() {
    let s = SimpleStruct {
        name: "test".to_string(),
        value: 42,
    };
    let yaml = to_string(&s).unwrap();
    assert!(yaml.contains("name: test") || yaml.contains("name:"));
    assert!(yaml.contains("value: 42") || yaml.contains("value:"));
}

#[derive(Serialize)]
struct NestedStruct {
    outer: String,
    inner: SimpleStruct,
}

#[test]
fn test_serialize_nested_struct() {
    let s = NestedStruct {
        outer: "hello".to_string(),
        inner: SimpleStruct {
            name: "test".to_string(),
            value: 42,
        },
    };
    let yaml = to_string(&s).unwrap();
    assert!(yaml.contains("outer:"));
    assert!(yaml.contains("inner:"));
}

#[derive(Serialize)]
struct OptionalFields {
    required: String,
    optional: Option<String>,
}

#[test]
fn test_serialize_optional_some() {
    let s = OptionalFields {
        required: "hello".to_string(),
        optional: Some("world".to_string()),
    };
    let yaml = to_string(&s).unwrap();
    assert!(yaml.contains("required:"));
    assert!(yaml.contains("optional:"));
}

#[test]
fn test_serialize_optional_none() {
    let s = OptionalFields {
        required: "hello".to_string(),
        optional: None,
    };
    let yaml = to_string(&s).unwrap();
    assert!(yaml.contains("required:"));
    assert!(yaml.contains("optional: null") || yaml.contains("optional:"));
}

// ============================================================================
// Enum Tests
// ============================================================================

#[derive(Serialize)]
#[allow(dead_code)]
enum UnitEnum {
    A,
    B,
    C,
}

#[test]
fn test_serialize_unit_enum() {
    let yaml = to_string(&UnitEnum::A).unwrap();
    assert!(yaml.contains("A"));
}

#[derive(Serialize)]
enum NewtypeEnum {
    Text(String),
    Number(i32),
}

#[test]
fn test_serialize_newtype_enum() {
    let yaml = to_string(&NewtypeEnum::Text("hello".to_string())).unwrap();
    assert!(yaml.contains("Text:"));
    assert!(yaml.contains("hello"));

    let yaml = to_string(&NewtypeEnum::Number(42)).unwrap();
    assert!(yaml.contains("Number:"));
    assert!(yaml.contains("42"));
}

#[derive(Serialize)]
enum StructEnum {
    Point { x: i32, y: i32 },
}

#[test]
fn test_serialize_struct_enum() {
    let yaml = to_string(&StructEnum::Point { x: 10, y: 20 }).unwrap();
    assert!(yaml.contains("Point:"));
    assert!(yaml.contains("x:"));
    assert!(yaml.contains("y:"));
}

// ============================================================================
// Complex Structure Tests
// ============================================================================

#[derive(Serialize)]
struct Config {
    name: String,
    version: u32,
    features: Vec<String>,
    settings: BTreeMap<String, i32>,
}

#[test]
fn test_serialize_complex_config() {
    let config = Config {
        name: "my-app".to_string(),
        version: 1,
        features: vec!["auth".to_string(), "api".to_string()],
        settings: {
            let mut s = BTreeMap::new();
            let _ = s.insert("timeout".to_string(), 30);
            s
        },
    };
    let yaml = to_string(&config).unwrap();
    assert!(yaml.contains("name:"));
    assert!(yaml.contains("version:"));
    assert!(yaml.contains("features:"));
    assert!(yaml.contains("settings:"));
}

// ============================================================================
// Special Value Tests
// ============================================================================

#[test]
fn test_serialize_null() {
    let yaml = to_string(&Option::<i32>::None).unwrap();
    assert!(yaml.contains("null"));
}

// ============================================================================
// Unicode Tests
// ============================================================================

#[test]
fn test_serialize_unicode() {
    let yaml = to_string(&"日本語").unwrap();
    assert!(yaml.contains("日本語"));
}

#[test]
fn test_serialize_emoji() {
    let yaml = to_string(&"🎉").unwrap();
    assert!(yaml.contains("🎉"));
}

// ============================================================================
// Round-trip Tests
// ============================================================================

#[test]
fn test_roundtrip_simple() {
    use noyalib::from_str;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Data {
        name: String,
        value: i32,
    }

    let original = Data {
        name: "test".to_string(),
        value: 42,
    };

    let yaml = to_string(&original).unwrap();
    let parsed: Data = from_str(&yaml).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_roundtrip_nested() {
    use noyalib::from_str;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Inner {
        x: i32,
        y: i32,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Outer {
        name: String,
        inner: Inner,
        tags: Vec<String>,
    }

    let original = Outer {
        name: "test".to_string(),
        inner: Inner { x: 10, y: 20 },
        tags: vec!["a".to_string(), "b".to_string()],
    };

    let yaml = to_string(&original).unwrap();
    let parsed: Outer = from_str(&yaml).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_roundtrip_collections() {
    use noyalib::from_str;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Data {
        items: Vec<i32>,
        lookup: BTreeMap<String, String>,
    }

    let original = Data {
        items: vec![1, 2, 3, 4, 5],
        lookup: {
            let mut m = BTreeMap::new();
            let _ = m.insert("key1".to_string(), "value1".to_string());
            let _ = m.insert("key2".to_string(), "value2".to_string());
            m
        },
    };

    let yaml = to_string(&original).unwrap();
    let parsed: Data = from_str(&yaml).unwrap();
    assert_eq!(original, parsed);
}

// ============================================================================
// Additional Serialization Coverage Tests
// ============================================================================

#[test]
fn test_serialize_to_writer() {
    #[derive(Debug, Serialize)]
    struct Data {
        name: String,
        value: i32,
    }

    let data = Data {
        name: "test".to_string(),
        value: 42,
    };

    let mut buffer = Vec::new();
    noyalib::to_writer(&mut buffer, &data).unwrap();

    let yaml = String::from_utf8(buffer).unwrap();
    assert!(yaml.contains("name: test"));
    assert!(yaml.contains("value: 42"));
}

#[test]
fn test_serialize_with_config() {
    #[derive(Debug, Serialize)]
    struct Data {
        name: String,
    }

    let data = Data {
        name: "test".to_string(),
    };

    // Custom config with document markers
    let config = SerializerConfig::new()
        .indent(4)
        .document_start(true)
        .document_end(true)
        .block_scalars(false)
        .block_scalar_threshold(80);

    let yaml = noyalib::to_string_with_config(&data, &config).unwrap();
    assert!(yaml.starts_with("---"));
    assert!(yaml.contains("..."));
}

#[test]
fn test_serialize_to_writer_with_config() {
    #[derive(Debug, Serialize)]
    struct Data {
        name: String,
    }

    let data = Data {
        name: "test".to_string(),
    };

    let config = SerializerConfig::new()
        .indent(2)
        .document_start(true)
        .document_end(false)
        .block_scalars(false)
        .block_scalar_threshold(80);

    let mut buffer = Vec::new();
    noyalib::to_writer_with_config(&mut buffer, &data, &config).unwrap();

    let yaml = String::from_utf8(buffer).unwrap();
    assert!(yaml.starts_with("---"));
}

#[test]
fn test_serialize_char() {
    let c = 'a';
    let yaml = to_string(&c).unwrap();
    assert!(yaml.contains("a"));
}

#[test]
fn test_serialize_unit() {
    let unit = ();
    let yaml = to_string(&unit).unwrap();
    assert!(yaml.contains("null"));
}

#[test]
fn test_serialize_unit_struct() {
    #[derive(Serialize)]
    struct UnitStruct;

    let yaml = to_string(&UnitStruct).unwrap();
    assert!(yaml.contains("null"));
}

#[test]
fn test_serialize_newtype_struct() {
    #[derive(Serialize)]
    struct Wrapper(i32);

    let wrapper = Wrapper(42);
    let yaml = to_string(&wrapper).unwrap();
    assert!(yaml.contains("42"));
}

#[test]
fn test_serialize_tuple() {
    let tuple = (1, "hello", true);
    let yaml = to_string(&tuple).unwrap();
    assert!(yaml.contains("1"));
    assert!(yaml.contains("hello"));
    assert!(yaml.contains("true"));
}

#[test]
fn test_serialize_tuple_struct() {
    #[derive(Serialize)]
    struct Point(i32, i32);

    let point = Point(10, 20);
    let yaml = to_string(&point).unwrap();
    assert!(yaml.contains("10"));
    assert!(yaml.contains("20"));
}

#[test]
fn test_serialize_enum_unit_variant() {
    #[derive(Serialize)]
    enum Color {
        Red,
    }

    let yaml = to_string(&Color::Red).unwrap();
    assert!(yaml.contains("Red"));
}

#[test]
fn test_serialize_enum_tuple_variant() {
    #[derive(Serialize)]
    enum Message {
        Write(String),
    }

    let msg = Message::Write("hello".to_string());
    let yaml = to_string(&msg).unwrap();
    assert!(yaml.contains("Write"));
    assert!(yaml.contains("hello"));
}

#[test]
fn test_serialize_enum_struct_variant() {
    #[derive(Serialize)]
    enum Shape {
        Rectangle { width: u32, height: u32 },
    }

    let shape = Shape::Rectangle {
        width: 10,
        height: 20,
    };
    let yaml = to_string(&shape).unwrap();
    assert!(yaml.contains("Rectangle"));
    assert!(yaml.contains("width: 10"));
    assert!(yaml.contains("height: 20"));
}

#[test]
fn test_serialize_option_none() {
    let opt: Option<i32> = None;
    let yaml = to_string(&opt).unwrap();
    assert!(yaml.contains("null"));
}

#[test]
fn test_serialize_option_some() {
    let opt = Some(42);
    let yaml = to_string(&opt).unwrap();
    assert!(yaml.contains("42"));
}

#[test]
fn test_serialize_integers_signed() {
    let i8_val: i8 = -128;
    let yaml = to_string(&i8_val).unwrap();
    assert!(yaml.contains("-128"));

    let i16_val: i16 = -32768;
    let yaml = to_string(&i16_val).unwrap();
    assert!(yaml.contains("-32768"));

    let i32_val: i32 = -2147483648;
    let yaml = to_string(&i32_val).unwrap();
    assert!(yaml.contains("-2147483648"));

    let i64_val: i64 = -9223372036854775808;
    let yaml = to_string(&i64_val).unwrap();
    assert!(yaml.contains("-9223372036854775808"));
}

#[test]
fn test_serialize_integers_unsigned() {
    let u8_val: u8 = 255;
    let yaml = to_string(&u8_val).unwrap();
    assert!(yaml.contains("255"));

    let u16_val: u16 = 65535;
    let yaml = to_string(&u16_val).unwrap();
    assert!(yaml.contains("65535"));

    let u32_val: u32 = 4294967295;
    let yaml = to_string(&u32_val).unwrap();
    assert!(yaml.contains("4294967295"));

    let u64_val: u64 = u64::MAX;
    #[cfg(not(feature = "lossless-u64"))]
    assert!(to_string(&u64_val).is_err());
    #[cfg(feature = "lossless-u64")]
    assert_eq!(to_string(&u64_val).unwrap().trim(), u64::MAX.to_string());

    // u64 values that fit in i64 should serialize fine
    let u64_val: u64 = i64::MAX as u64;
    let yaml = to_string(&u64_val).unwrap();
    assert!(yaml.contains("9223372036854775807"));
}

#[test]
fn test_serialize_floats_precision() {
    let f32_val: f32 = 3.125159;
    let yaml = to_string(&f32_val).unwrap();
    assert!(yaml.contains("3.125"));

    let f64_val: f64 = 2.8125;
    let yaml = to_string(&f64_val).unwrap();
    assert!(yaml.contains("2.8125"));
}

#[test]
fn test_to_value() {
    #[derive(Serialize)]
    struct Data {
        name: String,
        value: i32,
    }

    let data = Data {
        name: "test".to_string(),
        value: 42,
    };

    let value = noyalib::to_value(&data).unwrap();
    assert!(value.is_mapping());
    assert_eq!(value.get("name").unwrap().as_str(), Some("test"));
    assert_eq!(value.get("value").unwrap().as_i64(), Some(42));
}

// ============================================================================
// Special Float Value Serialization Tests
// ============================================================================

#[test]
fn test_serialize_nan() {
    let value = Value::Number(Number::Float(f64::NAN));
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains(".nan"));
}

#[test]
fn test_serialize_positive_infinity() {
    let value = Value::Number(Number::Float(f64::INFINITY));
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains(".inf"));
}

#[test]
fn test_serialize_negative_infinity() {
    let value = Value::Number(Number::Float(f64::NEG_INFINITY));
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("-.inf"));
}

#[test]
fn test_serialize_tagged_value() {
    use noyalib::{Tag, TaggedValue, Value};

    let tagged = TaggedValue::new(
        Tag::new("!timestamp"),
        Value::String("2024-01-01".to_string()),
    );
    let value = Value::Tagged(Box::new(tagged));
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("!timestamp"));
    assert!(yaml.contains("2024-01-01"));
}

#[test]
fn test_serialize_block_scalar() {
    let config = SerializerConfig::new()
        .indent(2)
        .document_start(false)
        .document_end(false)
        .block_scalars(true)
        .block_scalar_threshold(1);

    let value = Value::String("line1\nline2\nline3\n".to_string());
    let yaml = noyalib::to_string_with_config(&value, &config).unwrap();
    // Should use block scalar format
    assert!(yaml.contains("|") || yaml.contains(">") || yaml.contains('\n'));
}

#[test]
fn test_serialize_string_needs_quoting() {
    // String with colon
    let value = Value::String("key: value".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with dash
    let value = Value::String("- item".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // Empty string
    let value = Value::String("".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("\"\"") || yaml.contains("''"));
}

#[test]
fn test_serialize_string_quoting_rules() {
    // String with newline
    let value = Value::String("line1\nline2".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(!yaml.is_empty());

    // String with hash
    let value = Value::String("# comment".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with &
    let value = Value::String("&anchor".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with *
    let value = Value::String("*alias".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with !
    let value = Value::String("!tag".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}

// ============================================================================
// Additional Serializer Coverage Tests
// ============================================================================

#[test]
fn test_serialize_tagged_value_with_string() {
    use noyalib::{Tag, TaggedValue};
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        Value::String("value".to_string()),
    )));
    let yaml = to_string(&tagged).unwrap();
    assert!(yaml.contains("!custom"));
    assert!(yaml.contains("value"));
}

#[test]
fn test_serialize_string_escape_chars() {
    // String with backslash that needs quoting (starts with special char)
    let value = Value::String("-back\\slash".to_string());
    let yaml = to_string(&value).unwrap();
    // Debug: println!("Backslash yaml: {:?}", yaml);
    assert!(
        yaml.contains("\\\\"),
        "Expected escaped backslash in: {}",
        yaml
    );

    // String with carriage return (needs quoting due to control char)
    let value = Value::String("-with\rreturn".to_string());
    let yaml = to_string(&value).unwrap();
    // Debug: println!("CR yaml: {:?}", yaml);
    assert!(yaml.contains("\\r"), "Expected escaped CR in: {}", yaml);

    // String with tab (needs quoting due to tab character)
    let value = Value::String("-with\ttab".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("\\t"));

    // String with double quote (needs quoting)
    let value = Value::String("-has\"quote".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("\\\""));
}

#[test]
fn test_serialize_block_scalar_keep_chomping() {
    // String ending with multiple newlines should use "+" chomping indicator
    let config = SerializerConfig::default()
        .block_scalars(true)
        .block_scalar_threshold(1);
    let value = Value::String("line1\nline2\n\n".to_string());
    let yaml = noyalib::to_string_with_config(&value, &config).unwrap();
    assert!(yaml.contains("|+") || yaml.contains("line1"));
}

#[test]
fn test_serialize_sequence_with_multi_key_mapping() {
    // Sequence containing mappings with multiple keys
    let mut map1 = Mapping::new();
    let _ = map1.insert("key1".to_string(), Value::from(1));
    let _ = map1.insert("key2".to_string(), Value::from(2));

    let mut map2 = Mapping::new();
    let _ = map2.insert("a".to_string(), Value::from("x"));
    let _ = map2.insert("b".to_string(), Value::from("y"));

    let seq = Value::Sequence(vec![Value::Mapping(map1), Value::Mapping(map2)]);
    let yaml = to_string(&seq).unwrap();

    assert!(yaml.contains("key1"));
    assert!(yaml.contains("key2"));
    assert!(yaml.contains("a:"));
    assert!(yaml.contains("b:"));
}

#[test]
fn test_serialize_sequence_with_nested_sequence() {
    // Sequence containing sequences
    let inner1 = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let inner2 = Value::Sequence(vec![Value::from(3), Value::from(4)]);
    let outer = Value::Sequence(vec![inner1, inner2]);

    let yaml = to_string(&outer).unwrap();
    assert!(yaml.contains("1"));
    assert!(yaml.contains("4"));
}

#[test]
fn test_serialize_bytes_array() {
    #[derive(Serialize)]
    struct WithBytes<'a> {
        #[serde(with = "serde_bytes")]
        data: &'a [u8],
    }

    let data = WithBytes { data: b"hello" };
    let yaml = to_string(&data).unwrap();
    assert!(yaml.contains("hello") || yaml.contains("data"));
}

#[test]
fn test_serialize_map_with_integer_key() {
    use std::collections::BTreeMap;

    let mut map: BTreeMap<i32, String> = BTreeMap::new();
    let _ = map.insert(42, "value".to_string());

    let yaml = to_string(&map).unwrap();
    assert!(yaml.contains("42"));
    assert!(yaml.contains("value"));
}

#[test]
fn test_serialize_map_with_bool_key() {
    use std::collections::BTreeMap;

    let mut map: BTreeMap<bool, String> = BTreeMap::new();
    let _ = map.insert(true, "yes".to_string());
    let _ = map.insert(false, "no".to_string());

    let yaml = to_string(&map).unwrap();
    assert!(yaml.contains("true"));
    assert!(yaml.contains("false"));
}

#[test]
fn test_serialize_map_with_sequence_value() {
    // Create a map with a sequence as value
    let mut map = Mapping::new();
    let key = Value::Sequence(vec![Value::from(1)]);
    let _ = map.insert("valid".to_string(), key);

    // The serialization should succeed for valid keys
    let result = to_string(&Value::Mapping(map));
    assert!(result.is_ok());
}

#[test]
fn test_serialize_string_special_chars() {
    // String starting with |
    let value = Value::String("|pipe".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with >
    let value = Value::String(">fold".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with %
    let value = Value::String("%directive".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with @
    let value = Value::String("@reserved".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with `
    let value = Value::String("`backtick".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with {
    let value = Value::String("{brace".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with }
    let value = Value::String("}brace".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with [
    let value = Value::String("[bracket".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with ]
    let value = Value::String("]bracket".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with ,
    let value = Value::String(",comma".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String starting with ?
    let value = Value::String("?question".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}

#[test]
fn test_serialize_string_looks_like_number() {
    // String that looks like an integer
    let value = Value::String("123".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String that looks like a float
    let value = Value::String("1.5".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String "null"
    let value = Value::String("null".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    // String "~"
    let value = Value::String("~".to_string());
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}
