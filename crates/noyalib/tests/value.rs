//! Value type tests for noyalib.
//!
//! Ported from serde_yml test suite.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use noyalib::{
    Mapping, MaybeTag, Number, Sequence, Tag, TaggedValue, Value, check_for_tag, from_str, nobang,
    to_string,
};

// ============================================================================
// Value Construction Tests
// ============================================================================

#[test]
fn test_value_null() {
    let value = Value::Null;
    assert!(value.is_null());
    assert!(!value.is_bool());
    assert!(!value.is_number());
    assert!(!value.is_string());
    assert!(!value.is_sequence());
    assert!(!value.is_mapping());
}

#[test]
fn test_value_bool() {
    let value = Value::Bool(true);
    assert!(value.is_bool());
    assert_eq!(value.as_bool(), Some(true));

    let value = Value::Bool(false);
    assert_eq!(value.as_bool(), Some(false));
}

#[test]
fn test_value_number_integer() {
    let value = Value::Number(Number::Integer(42));
    assert!(value.is_number());
    assert_eq!(value.as_i64(), Some(42));
}

#[test]
fn test_value_number_float() {
    let value = Value::Number(Number::Float(3.125));
    assert!(value.is_number());
    let f = value.as_f64().unwrap();
    assert!((f - 3.125).abs() < 0.001);
}

#[test]
fn test_value_string() {
    let value = Value::String("hello".to_string());
    assert!(value.is_string());
    assert_eq!(value.as_str(), Some("hello"));
}

#[test]
fn test_value_sequence() {
    let seq: Sequence = vec![
        Value::Number(Number::Integer(1)),
        Value::Number(Number::Integer(2)),
        Value::Number(Number::Integer(3)),
    ];
    let value = Value::Sequence(seq);
    assert!(value.is_sequence());
    assert_eq!(value.as_sequence().unwrap().len(), 3);
}

#[test]
fn test_value_mapping() {
    let mut map = Mapping::new();
    let _ = map.insert("key".to_string(), Value::String("value".to_string()));
    let value = Value::Mapping(map);
    assert!(value.is_mapping());
    assert_eq!(value.as_mapping().unwrap().len(), 1);
}

// ============================================================================
// Value Indexing Tests
// ============================================================================

#[test]
fn test_value_index_sequence() {
    let seq: Sequence = vec![
        Value::Number(Number::Integer(10)),
        Value::Number(Number::Integer(20)),
        Value::Number(Number::Integer(30)),
    ];
    let value = Value::Sequence(seq);

    assert_eq!(value.get(0).unwrap().as_i64(), Some(10));
    assert_eq!(value.get(1).unwrap().as_i64(), Some(20));
    assert_eq!(value.get(2).unwrap().as_i64(), Some(30));
    assert!(value.get(3).is_none());
}

#[test]
fn test_value_index_mapping() {
    let mut map = Mapping::new();
    let _ = map.insert("a".to_string(), Value::Number(Number::Integer(1)));
    let _ = map.insert("b".to_string(), Value::Number(Number::Integer(2)));
    let value = Value::Mapping(map);

    assert_eq!(value.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(value.get("b").unwrap().as_i64(), Some(2));
    assert!(value.get("c").is_none());
}

#[test]
fn test_value_index_nested() {
    let yaml = r#"
outer:
  inner:
    value: 42
"#;
    let value: Value = from_str(yaml).unwrap();

    let outer = value.get("outer").unwrap();
    let inner = outer.get("inner").unwrap();
    let val = inner.get("value").unwrap();
    assert_eq!(val.as_i64(), Some(42));
}

// ============================================================================
// Number Tests
// ============================================================================

#[test]
fn test_number_integer() {
    let num = Number::Integer(42);
    assert!(num.is_integer());
    assert!(!num.is_float());
    assert_eq!(num.as_i64(), Some(42));
    assert!((num.as_f64() - 42.0).abs() < 0.001);
}

#[cfg(feature = "lossless-u64")]
#[test]
fn test_number_unsigned() {
    let num = Number::Unsigned(u64::MAX);
    assert!(num.is_integer());
    assert!(!num.is_float());
    assert_eq!(num.as_i64(), None);
    assert_eq!(num.as_u64(), Some(u64::MAX));
    assert_eq!(num.to_string(), "18446744073709551615");

    let value = Value::from(u64::MAX);
    assert_eq!(value.as_u64(), Some(u64::MAX));
    assert!(value.as_i64().is_none());
    assert!(!matches!(value, Value::Number(Number::Float(_))));
}

#[test]
fn test_number_float() {
    let num = Number::Float(3.125);
    assert!(!num.is_integer());
    assert!(num.is_float());
    assert_eq!(num.as_i64(), None);
    assert!((num.as_f64() - 3.125).abs() < 0.001);
}

#[test]
fn test_number_display() {
    assert_eq!(Number::Integer(42).to_string(), "42");
    assert_eq!(Number::Integer(-42).to_string(), "-42");
    assert!(Number::Float(3.125).to_string().contains("3.125"));
}

#[cfg(feature = "lossless-u64")]
#[test]
fn test_number_from_str_unsigned_radix() {
    assert_eq!(
        "18446744073709551615".parse::<Number>().unwrap().as_u64(),
        Some(u64::MAX)
    );
    assert_eq!(
        "0xffffffffffffffff".parse::<Number>().unwrap().as_u64(),
        Some(u64::MAX)
    );
}

// ============================================================================
// Value Display Tests
// ============================================================================

#[test]
fn test_value_display_null() {
    assert_eq!(Value::Null.to_string(), "null");
}

#[test]
fn test_value_display_bool() {
    assert_eq!(Value::Bool(true).to_string(), "true");
    assert_eq!(Value::Bool(false).to_string(), "false");
}

#[test]
fn test_value_display_number() {
    assert_eq!(Value::Number(Number::Integer(42)).to_string(), "42");
}

#[test]
fn test_value_display_string() {
    assert_eq!(Value::String("hello".to_string()).to_string(), "hello");
}

// ============================================================================
// Value Serialization Tests
// ============================================================================

#[test]
fn test_value_serialize_null() {
    let value = Value::Null;
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("null"));
}

#[test]
fn test_value_serialize_bool() {
    let value = Value::Bool(true);
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("true"));
}

#[test]
fn test_value_serialize_sequence() {
    let seq: Sequence = vec![
        Value::Number(Number::Integer(1)),
        Value::Number(Number::Integer(2)),
    ];
    let value = Value::Sequence(seq);
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("- 1"));
    assert!(yaml.contains("- 2"));
}

#[test]
fn test_value_serialize_mapping() {
    let mut map = Mapping::new();
    let _ = map.insert("key".to_string(), Value::String("value".to_string()));
    let value = Value::Mapping(map);
    let yaml = to_string(&value).unwrap();
    assert!(yaml.contains("key:"));
    assert!(yaml.contains("value"));
}

// ============================================================================
// Value Deserialization Tests
// ============================================================================

#[test]
fn test_value_deserialize_null() {
    let value: Value = from_str("null\n").unwrap();
    assert!(value.is_null());
}

#[test]
fn test_value_deserialize_bool() {
    let value: Value = from_str("true\n").unwrap();
    assert_eq!(value.as_bool(), Some(true));

    let value: Value = from_str("false\n").unwrap();
    assert_eq!(value.as_bool(), Some(false));
}

#[test]
fn test_value_deserialize_integer() {
    let value: Value = from_str("42\n").unwrap();
    assert_eq!(value.as_i64(), Some(42));
}

#[test]
fn test_value_deserialize_float() {
    let value: Value = from_str("3.125\n").unwrap();
    let f = value.as_f64().unwrap();
    assert!((f - 3.125).abs() < 0.001);
}

#[test]
fn test_value_deserialize_string() {
    let value: Value = from_str("hello\n").unwrap();
    assert_eq!(value.as_str(), Some("hello"));
}

#[test]
fn test_value_deserialize_sequence() {
    let value: Value = from_str("- 1\n- 2\n- 3\n").unwrap();
    let seq = value.as_sequence().unwrap();
    assert_eq!(seq.len(), 3);
}

#[test]
fn test_value_deserialize_mapping() {
    let value: Value = from_str("a: 1\nb: 2\n").unwrap();
    let map = value.as_mapping().unwrap();
    assert_eq!(map.len(), 2);
}

// ============================================================================
// Value Round-trip Tests
// ============================================================================

#[test]
fn test_value_roundtrip() {
    let yaml = r#"
name: test
version: 1
enabled: true
tags:
  - a
  - b
  - c
config:
  key1: value1
  key2: value2
"#;

    let value: Value = from_str(yaml).unwrap();
    let output = to_string(&value).unwrap();
    let reparsed: Value = from_str(&output).unwrap();

    // Verify structure is preserved
    assert_eq!(value.get("name"), reparsed.get("name"));
    assert_eq!(value.get("version"), reparsed.get("version"));
    assert_eq!(value.get("enabled"), reparsed.get("enabled"));
}

// ============================================================================
// Value Default Tests
// ============================================================================

#[test]
fn test_value_default() {
    let value = Value::default();
    assert!(value.is_null());
}

// ============================================================================
// Value Mutable Access Tests
// ============================================================================

#[test]
fn test_value_get_mut_sequence() {
    let mut value = Value::Sequence(vec![
        Value::Number(Number::Integer(1)),
        Value::Number(Number::Integer(2)),
    ]);

    if let Some(elem) = value.get_mut(0) {
        *elem = Value::Number(Number::Integer(100));
    }

    assert_eq!(value.get(0).unwrap().as_i64(), Some(100));
}

#[test]
fn test_value_get_mut_mapping() {
    let mut map = Mapping::new();
    let _ = map.insert("key".to_string(), Value::Number(Number::Integer(1)));
    let mut value = Value::Mapping(map);

    if let Some(val) = value.get_mut("key") {
        *val = Value::Number(Number::Integer(100));
    }

    assert_eq!(value.get("key").unwrap().as_i64(), Some(100));
}

// ============================================================================
// Number Type Tests
// ============================================================================

#[test]
fn test_number_as_u64() {
    // Positive integer
    let n = Number::Integer(42);
    assert_eq!(n.as_u64(), Some(42));

    // Zero
    let n = Number::Integer(0);
    assert_eq!(n.as_u64(), Some(0));

    // Negative integer - should return None
    let n = Number::Integer(-1);
    assert_eq!(n.as_u64(), None);

    // Float - should return None
    let n = Number::Float(42.0);
    assert_eq!(n.as_u64(), None);
}

#[test]
fn test_number_is_i64_u64_f64() {
    let int_pos = Number::Integer(42);
    assert!(int_pos.is_i64());
    assert!(int_pos.is_u64());
    assert!(int_pos.is_f64());

    let int_neg = Number::Integer(-42);
    assert!(int_neg.is_i64());
    assert!(!int_neg.is_u64());
    assert!(int_neg.is_f64());

    let float = Number::Float(3.125);
    assert!(!float.is_i64());
    assert!(!float.is_u64());
    assert!(float.is_f64());
}

#[test]
fn test_number_nan_infinite_finite() {
    // Regular integer
    let n = Number::Integer(42);
    assert!(!n.is_nan());
    assert!(!n.is_infinite());
    assert!(n.is_finite());

    // Regular float
    let n = Number::Float(3.125);
    assert!(!n.is_nan());
    assert!(!n.is_infinite());
    assert!(n.is_finite());

    // NaN
    let n = Number::Float(f64::NAN);
    assert!(n.is_nan());
    assert!(!n.is_infinite());
    assert!(!n.is_finite());

    // Positive infinity
    let n = Number::Float(f64::INFINITY);
    assert!(!n.is_nan());
    assert!(n.is_infinite());
    assert!(!n.is_finite());

    // Negative infinity
    let n = Number::Float(f64::NEG_INFINITY);
    assert!(!n.is_nan());
    assert!(n.is_infinite());
    assert!(!n.is_finite());
}

#[test]
fn test_number_from_str() {
    use std::str::FromStr;

    // Integer
    assert_eq!(Number::from_str("42").unwrap(), Number::Integer(42));
    assert_eq!(Number::from_str("-42").unwrap(), Number::Integer(-42));
    assert_eq!(Number::from_str("0").unwrap(), Number::Integer(0));

    // Float
    let n = Number::from_str("3.125").unwrap();
    assert!(matches!(n, Number::Float(f) if (f - 3.125).abs() < 0.001));

    // Hex
    assert_eq!(Number::from_str("0x2A").unwrap(), Number::Integer(42));
    assert_eq!(Number::from_str("0xFF").unwrap(), Number::Integer(255));

    // Octal
    assert_eq!(Number::from_str("0o52").unwrap(), Number::Integer(42));

    // Binary
    assert_eq!(Number::from_str("0b101010").unwrap(), Number::Integer(42));

    // Special float values
    assert!(Number::from_str(".nan").unwrap().is_nan());
    assert!(Number::from_str(".NaN").unwrap().is_nan());
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

    // Invalid
    assert!(Number::from_str("not a number").is_err());
    assert!(Number::from_str("").is_err());
}

// ============================================================================
// Value Method Tests
// ============================================================================

#[test]
fn test_value_as_null() {
    let null = Value::Null;
    assert_eq!(null.as_null(), Some(()));

    let not_null = Value::Bool(true);
    assert_eq!(not_null.as_null(), None);
}

#[test]
fn test_value_as_u64() {
    // Positive integer
    let value = Value::Number(Number::Integer(42));
    assert_eq!(value.as_u64(), Some(42));

    // Zero
    let value = Value::Number(Number::Integer(0));
    assert_eq!(value.as_u64(), Some(0));

    // Negative integer - should return None
    let value = Value::Number(Number::Integer(-1));
    assert_eq!(value.as_u64(), None);

    // Float - should return None
    let value = Value::Number(Number::Float(42.0));
    assert_eq!(value.as_u64(), None);

    // Non-number - should return None
    let value = Value::String("42".to_string());
    assert_eq!(value.as_u64(), None);
}

#[test]
fn test_value_is_i64_u64_f64() {
    let int_pos = Value::Number(Number::Integer(42));
    assert!(int_pos.is_i64());
    assert!(int_pos.is_u64());
    assert!(int_pos.is_f64());

    let int_neg = Value::Number(Number::Integer(-42));
    assert!(int_neg.is_i64());
    assert!(!int_neg.is_u64());
    assert!(int_neg.is_f64());

    let float = Value::Number(Number::Float(3.125));
    assert!(!float.is_i64());
    assert!(!float.is_u64());
    assert!(float.is_f64());

    let not_number = Value::String("42".to_string());
    assert!(!not_number.is_i64());
    assert!(!not_number.is_u64());
    assert!(!not_number.is_f64());
}

#[test]
fn test_value_bracket_index_sequence() {
    let yaml = "- a\n- b\n- c\n";
    let value: Value = from_str(yaml).unwrap();

    // Bracket indexing (Index trait)
    assert_eq!(value[0].as_str(), Some("a"));
    assert_eq!(value[1].as_str(), Some("b"));
    assert_eq!(value[2].as_str(), Some("c"));
}

#[test]
fn test_value_bracket_index_mapping() {
    let yaml = "name: test\nversion: 1\n";
    let value: Value = from_str(yaml).unwrap();

    // Bracket indexing (Index trait)
    assert_eq!(value["name"].as_str(), Some("test"));
    assert_eq!(value["version"].as_i64(), Some(1));
}

#[test]
fn test_value_bracket_index_nested() {
    let yaml = r#"
users:
  - name: alice
    age: 30
  - name: bob
    age: 25
"#;
    let value: Value = from_str(yaml).unwrap();

    // Nested bracket indexing (Index trait)
    assert_eq!(value["users"][0]["name"].as_str(), Some("alice"));
    assert_eq!(value["users"][0]["age"].as_i64(), Some(30));
    assert_eq!(value["users"][1]["name"].as_str(), Some("bob"));
}

#[test]
fn test_value_bracket_index_mut() {
    let yaml = "- 1\n- 2\n- 3\n";
    let mut value: Value = from_str(yaml).unwrap();

    // IndexMut trait for sequences
    value[0] = Value::from(100);
    assert_eq!(value[0].as_i64(), Some(100));

    let yaml = "a: 1\nb: 2\n";
    let mut value: Value = from_str(yaml).unwrap();

    // IndexMut trait for mappings
    value["a"] = Value::from(100);
    assert_eq!(value["a"].as_i64(), Some(100));
}

// ============================================================================
// Hash Tests
// ============================================================================

fn hash_value<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn test_number_hash() {
    // Same integers should have same hash
    let n1 = Number::Integer(42);
    let n2 = Number::Integer(42);
    assert_eq!(hash_value(&n1), hash_value(&n2));

    // Different integers should have different hash
    let n3 = Number::Integer(43);
    assert_ne!(hash_value(&n1), hash_value(&n3));

    // Same floats should have same hash
    let f1 = Number::Float(3.125);
    let f2 = Number::Float(3.125);
    assert_eq!(hash_value(&f1), hash_value(&f2));

    // NaN values should hash consistently
    let nan1 = Number::Float(f64::NAN);
    let nan2 = Number::Float(f64::NAN);
    assert_eq!(hash_value(&nan1), hash_value(&nan2));
}

#[cfg(feature = "lossless-u64")]
#[test]
fn test_number_hash_lossless_u64_tags() {
    // Float must keep discriminant 1 when lossless-u64 is enabled.
    let f1 = Number::Float(3.125);
    let f2 = Number::Float(3.125);
    assert_eq!(hash_value(&f1), hash_value(&f2));

    let unsigned = Number::Unsigned(u64::MAX);
    assert_eq!(hash_value(&unsigned), hash_value(&unsigned));
    assert_ne!(hash_value(&f1), hash_value(&unsigned));
}

#[test]
fn test_value_hash() {
    // Same values should have same hash
    let v1 = Value::Number(Number::Integer(42));
    let v2 = Value::Number(Number::Integer(42));
    assert_eq!(hash_value(&v1), hash_value(&v2));

    // Null hash
    let null1 = Value::Null;
    let null2 = Value::Null;
    assert_eq!(hash_value(&null1), hash_value(&null2));

    // Bool hash
    let bool1 = Value::Bool(true);
    let bool2 = Value::Bool(true);
    assert_eq!(hash_value(&bool1), hash_value(&bool2));

    // String hash
    let str1 = Value::String("test".to_string());
    let str2 = Value::String("test".to_string());
    assert_eq!(hash_value(&str1), hash_value(&str2));

    // Sequence hash
    let seq1 = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let seq2 = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    assert_eq!(hash_value(&seq1), hash_value(&seq2));

    // Mapping hash
    let mut map1 = Mapping::new();
    let _ = map1.insert("a".to_string(), Value::from(1));
    let mut map2 = Mapping::new();
    let _ = map2.insert("a".to_string(), Value::from(1));
    assert_eq!(
        hash_value(&Value::Mapping(map1)),
        hash_value(&Value::Mapping(map2))
    );

    // Tagged value hash
    let tag1 = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!test"),
        Value::from(42),
    )));
    let tag2 = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!test"),
        Value::from(42),
    )));
    assert_eq!(hash_value(&tag1), hash_value(&tag2));
}

// ============================================================================
// Ord Tests
// ============================================================================

#[test]
fn test_number_ord() {
    // Integer ordering
    assert!(Number::Integer(1) < Number::Integer(2));
    assert!(Number::Integer(2) > Number::Integer(1));
    assert!(Number::Integer(1) == Number::Integer(1));

    // Float ordering
    assert!(Number::Float(1.0) < Number::Float(2.0));
    assert!(Number::Float(2.0) > Number::Float(1.0));

    // Mixed ordering (integer vs float)
    assert!(Number::Integer(1) < Number::Float(2.0));
    assert!(Number::Float(1.0) < Number::Integer(2));

    // NaN is greater than any non-NaN
    assert!(Number::Float(f64::NAN) > Number::Float(1.0));
    assert!(Number::Float(f64::NAN) > Number::Integer(1));
    assert!(Number::Integer(1) < Number::Float(f64::NAN));

    // Two NaNs are equal in Ord (but not in PartialEq due to IEEE 754)
    use std::cmp::Ordering;
    assert_eq!(
        Number::Float(f64::NAN).cmp(&Number::Float(f64::NAN)),
        Ordering::Equal
    );
}

#[test]
fn test_value_ord() {
    // Type ordering: Null < Bool < Number < String < Sequence < Mapping < Tagged
    assert!(Value::Null < Value::Bool(false));
    assert!(Value::Bool(true) < Value::Number(Number::Integer(0)));
    assert!(Value::Number(Number::Integer(0)) < Value::String("".to_string()));
    assert!(Value::String("".to_string()) < Value::Sequence(vec![]));
    assert!(Value::Sequence(vec![]) < Value::Mapping(Mapping::new()));

    // Same type ordering
    assert!(Value::Bool(false) < Value::Bool(true));
    assert!(Value::Number(Number::Integer(1)) < Value::Number(Number::Integer(2)));
    assert!(Value::String("a".to_string()) < Value::String("b".to_string()));

    // Sequence ordering (by length first, then elements)
    let seq1 = Value::Sequence(vec![Value::from(1)]);
    let seq2 = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    assert!(seq1 < seq2);

    let seq3 = Value::Sequence(vec![Value::from(1)]);
    let seq4 = Value::Sequence(vec![Value::from(2)]);
    assert!(seq3 < seq4);

    // Mapping ordering
    let mut map1 = Mapping::new();
    let _ = map1.insert("a".to_string(), Value::from(1));
    let mut map2 = Mapping::new();
    let _ = map2.insert("a".to_string(), Value::from(1));
    let _ = map2.insert("b".to_string(), Value::from(2));
    assert!(Value::Mapping(map1) < Value::Mapping(map2));
}

// ============================================================================
// From Implementation Tests
// ============================================================================

#[test]
fn test_value_from_unit() {
    let v: Value = ().into();
    assert!(v.is_null());
}

#[test]
fn test_value_from_bool() {
    let v: Value = true.into();
    assert_eq!(v.as_bool(), Some(true));

    let v: Value = false.into();
    assert_eq!(v.as_bool(), Some(false));
}

#[test]
fn test_value_from_integers() {
    let v: Value = 42i8.into();
    assert_eq!(v.as_i64(), Some(42));

    let v: Value = 42i16.into();
    assert_eq!(v.as_i64(), Some(42));

    let v: Value = 42i32.into();
    assert_eq!(v.as_i64(), Some(42));

    let v: Value = 42i64.into();
    assert_eq!(v.as_i64(), Some(42));

    let v: Value = 42u8.into();
    assert_eq!(v.as_i64(), Some(42));

    let v: Value = 42u16.into();
    assert_eq!(v.as_i64(), Some(42));

    let v: Value = 42u32.into();
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn test_value_from_floats() {
    let v: Value = 3.125f32.into();
    assert!((v.as_f64().unwrap() - 3.125).abs() < 0.01);

    let v: Value = 3.125f64.into();
    assert!((v.as_f64().unwrap() - 3.125).abs() < 0.001);
}

#[test]
fn test_value_from_string() {
    let v: Value = String::from("hello").into();
    assert_eq!(v.as_str(), Some("hello"));

    let v: Value = "world".into();
    assert_eq!(v.as_str(), Some("world"));
}

#[test]
fn test_value_from_vec() {
    let v: Value = vec![1, 2, 3].into();
    let seq = v.as_sequence().unwrap();
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_i64(), Some(1));
}

#[test]
fn test_value_from_option() {
    let v: Value = Some(42).into();
    assert_eq!(v.as_i64(), Some(42));

    let v: Value = Option::<i32>::None.into();
    assert!(v.is_null());
}

#[test]
fn test_value_from_number() {
    let v: Value = Number::Integer(42).into();
    assert_eq!(v.as_i64(), Some(42));

    let v: Value = Number::Float(3.125).into();
    assert!((v.as_f64().unwrap() - 3.125).abs() < 0.001);
}

#[test]
fn test_value_from_mapping() {
    let mut map = Mapping::new();
    let _ = map.insert("key".to_string(), Value::from(42));
    let v: Value = map.into();
    assert!(v.is_mapping());
    assert_eq!(v.get("key").unwrap().as_i64(), Some(42));
}

#[test]
fn test_value_from_tagged_value() {
    let tagged = TaggedValue::new(Tag::new("!custom"), Value::from(42));
    let v: Value = tagged.into();
    assert!(v.is_tagged());
}

// ============================================================================
// Tag and TaggedValue Tests
// ============================================================================

#[test]
fn test_tag_creation() {
    let tag = Tag::new("!custom");
    assert_eq!(tag.as_str(), "!custom");

    let tag2: Tag = "!another".into();
    assert_eq!(tag2.as_str(), "!another");

    let tag3: Tag = String::from("!third").into();
    assert_eq!(tag3.as_str(), "!third");

    assert_eq!(tag.as_ref(), "!custom");
    assert_eq!(format!("{}", tag), "!custom");

    let owned = tag.into_string();
    assert_eq!(owned, "!custom");
}

#[test]
fn test_tagged_value() {
    let mut tagged = TaggedValue::new(
        Tag::new("!timestamp"),
        Value::String("2024-01-01".to_string()),
    );

    assert_eq!(tagged.tag().as_str(), "!timestamp");
    assert_eq!(tagged.value().as_str(), Some("2024-01-01"));

    // Mutable access
    *tagged.value_mut() = Value::String("2024-12-31".to_string());
    assert_eq!(tagged.value().as_str(), Some("2024-12-31"));

    // Display
    let display = format!("{}", tagged);
    assert!(display.contains("!timestamp"));

    // Into parts
    let (tag, value) = tagged.into_parts();
    assert_eq!(tag.as_str(), "!timestamp");
    assert_eq!(value.as_str(), Some("2024-12-31"));
}

// ============================================================================
// ParseNumberError Tests
// ============================================================================

#[test]
fn test_parse_number_error() {
    use std::error::Error;
    use std::str::FromStr;

    let err = Number::from_str("invalid").unwrap_err();
    assert_eq!(format!("{}", err), "invalid number");

    // Test that it implements Error trait
    let _: &dyn Error = &err;
}

// ============================================================================
// Number Display Tests (additional)
// ============================================================================

#[test]
fn test_number_display_negative() {
    assert_eq!(format!("{}", Number::Integer(-42)), "-42");
    assert_eq!(format!("{}", Number::Integer(0)), "0");
}

// ============================================================================
// Value Display Tests
// ============================================================================

#[test]
fn test_value_display() {
    assert_eq!(format!("{}", Value::Null), "null");
    assert_eq!(format!("{}", Value::Bool(true)), "true");
    assert_eq!(format!("{}", Value::Bool(false)), "false");
    assert_eq!(format!("{}", Value::Number(Number::Integer(42))), "42");
    assert_eq!(format!("{}", Value::String("hello".to_string())), "hello");

    let seq = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    assert_eq!(format!("{}", seq), "[1, 2]");

    let mut map = Mapping::new();
    let _ = map.insert("a".to_string(), Value::from(1));
    let map_val = Value::Mapping(map);
    assert_eq!(format!("{}", map_val), "{a: 1}");

    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!test"),
        Value::from(42),
    )));
    let display = format!("{}", tagged);
    assert!(display.contains("!test"));
}

// ============================================================================
// Additional Number FromStr Tests (edge cases)
// ============================================================================

#[test]
fn test_number_from_str_edge_cases() {
    use std::str::FromStr;

    // Whitespace trimming
    assert_eq!(Number::from_str("  42  ").unwrap(), Number::Integer(42));

    // Case variations for special values
    assert!(Number::from_str(".NAN").unwrap().is_nan());
    assert_eq!(
        Number::from_str(".INF").unwrap(),
        Number::Float(f64::INFINITY)
    );
    assert_eq!(
        Number::from_str("+.INF").unwrap(),
        Number::Float(f64::INFINITY)
    );
    assert_eq!(
        Number::from_str("-.INF").unwrap(),
        Number::Float(f64::NEG_INFINITY)
    );
    assert_eq!(
        Number::from_str(".Inf").unwrap(),
        Number::Float(f64::INFINITY)
    );
    assert_eq!(
        Number::from_str("+.Inf").unwrap(),
        Number::Float(f64::INFINITY)
    );
    assert_eq!(
        Number::from_str("-.Inf").unwrap(),
        Number::Float(f64::NEG_INFINITY)
    );

    // Uppercase hex/octal/binary
    assert_eq!(Number::from_str("0X10").unwrap(), Number::Integer(16));
    assert_eq!(Number::from_str("0O10").unwrap(), Number::Integer(8));
    assert_eq!(Number::from_str("0B10").unwrap(), Number::Integer(2));

    // Scientific notation
    let n = Number::from_str("1e10").unwrap();
    assert!(matches!(n, Number::Float(_)));

    // Invalid hex/octal/binary (should fall through to error)
    assert!(Number::from_str("0xGG").is_err());
    assert!(Number::from_str("0o99").is_err());
    assert!(Number::from_str("0b22").is_err());
}

// ============================================================================
// Value Mutation Tests
// ============================================================================

#[test]
fn test_value_get_returns_none_for_wrong_type() {
    let value = Value::String("not a sequence".to_string());
    assert!(value.get(0).is_none());
    assert!(value.get("key").is_none());

    let value = Value::Sequence(vec![]);
    assert!(value.get("key").is_none());

    let value = Value::Mapping(Mapping::new());
    assert!(value.get(0).is_none());
}

#[test]
fn test_value_insert_on_non_mapping() {
    let mut value = Value::String("not a mapping".to_string());
    let result = value.insert("key", Value::from(42));
    assert!(result.is_none());
    // Value should be unchanged
    assert!(value.is_string());
}

#[test]
fn test_value_remove_on_non_mapping() {
    let mut value = Value::String("not a mapping".to_string());
    let result = value.remove("key");
    assert!(result.is_none());
}

// ============================================================================
// Value Accessor None Cases
// ============================================================================

#[test]
fn test_value_as_str_none() {
    let value = Value::Number(Number::Integer(42));
    assert!(value.as_str().is_none());

    let value = Value::Bool(true);
    assert!(value.as_str().is_none());

    let value = Value::Null;
    assert!(value.as_str().is_none());
}

#[test]
fn test_value_as_sequence_none() {
    let value = Value::String("not a sequence".to_string());
    assert!(value.as_sequence().is_none());

    let value = Value::Number(Number::Integer(42));
    assert!(value.as_sequence().is_none());
}

#[test]
fn test_value_as_sequence_mut_none() {
    let mut value = Value::String("not a sequence".to_string());
    assert!(value.as_sequence_mut().is_none());
}

#[test]
fn test_value_as_sequence_mut_some() {
    let mut value = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let seq = value.as_sequence_mut().unwrap();
    seq.push(Value::from(3));
    assert_eq!(seq.len(), 3);
}

#[test]
fn test_value_as_f64_on_string() {
    // Test as_f64 returning None on String value
    let value = Value::String("hello".to_string());
    assert!(value.as_f64().is_none());
}

#[test]
fn test_value_as_f64_on_null() {
    // Test as_f64 returning None on Null value
    let value = Value::Null;
    assert!(value.as_f64().is_none());
}

#[test]
fn test_value_as_mapping_none() {
    let value = Value::String("not a mapping".to_string());
    assert!(value.as_mapping().is_none());

    let value = Value::Sequence(vec![]);
    assert!(value.as_mapping().is_none());
}

#[test]
fn test_value_as_mapping_mut_none() {
    let mut value = Value::String("not a mapping".to_string());
    assert!(value.as_mapping_mut().is_none());
}

#[test]
fn test_value_as_mapping_mut_some() {
    let mut map = Mapping::new();
    let _ = map.insert("a".to_string(), Value::from(1));
    let mut value = Value::Mapping(map);
    let m = value.as_mapping_mut().unwrap();
    let _ = m.insert("b".to_string(), Value::from(2));
    assert_eq!(m.len(), 2);
}

#[test]
fn test_value_as_tagged() {
    let tagged = TaggedValue::new(Tag::new("!custom"), Value::from(42));
    let value = Value::Tagged(Box::new(tagged));

    let result = value.as_tagged();
    assert!(result.is_some());
    assert_eq!(result.unwrap().tag().as_str(), "!custom");
}

#[test]
fn test_value_as_tagged_none() {
    let value = Value::String("not tagged".to_string());
    assert!(value.as_tagged().is_none());

    let value = Value::Null;
    assert!(value.as_tagged().is_none());
}

#[test]
fn test_value_as_tagged_mut() {
    let tagged = TaggedValue::new(Tag::new("!old"), Value::from(1));
    let mut value = Value::Tagged(Box::new(tagged));

    if let Some(t) = value.as_tagged_mut() {
        *t.value_mut() = Value::from(999);
    }

    assert_eq!(value.as_tagged().unwrap().value().as_i64(), Some(999));
}

#[test]
fn test_value_as_tagged_mut_none() {
    let mut value = Value::String("not tagged".to_string());
    assert!(value.as_tagged_mut().is_none());
}

// ============================================================================
// Value Ord Edge Cases
// ============================================================================

#[test]
fn test_value_ord_tagged() {
    let tag1 = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!a"), Value::from(1))));
    let tag2 = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!b"), Value::from(1))));
    let tag3 = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!a"), Value::from(2))));

    // Same tag, different values
    assert!(tag1 < tag3);

    // Different tags
    assert!(tag1 < tag2);

    // Tagged values are greater than all other types
    assert!(tag1 > Value::Mapping(Mapping::new()));
}

#[test]
fn test_value_ord_mapping_values() {
    let mut map1 = Mapping::new();
    let _ = map1.insert("a".to_string(), Value::from(1));
    let mut map2 = Mapping::new();
    let _ = map2.insert("a".to_string(), Value::from(2));

    // Same keys, different values
    assert!(Value::Mapping(map1) < Value::Mapping(map2));
}

#[test]
fn test_value_ord_sequence_elements() {
    let seq1 = Value::Sequence(vec![Value::from(1), Value::from(1)]);
    let seq2 = Value::Sequence(vec![Value::from(1), Value::from(2)]);

    // Same length, different elements
    assert!(seq1 < seq2);
}

// ============================================================================
// Additional Coverage Tests - Hash, Ord, Display, ValueIndex
// ============================================================================

#[test]
fn test_value_hash_null() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let _ = set.insert(Value::Null);
    assert!(set.contains(&Value::Null));
}

#[test]
fn test_value_hash_sequence() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let seq = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let _ = set.insert(seq.clone());
    assert!(set.contains(&seq));
}

#[test]
fn test_value_hash_mapping() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let mut map = Mapping::new();
    let _ = map.insert("key".to_string(), Value::from("value"));
    let value = Value::Mapping(map);
    let _ = set.insert(value.clone());
    assert!(set.contains(&value));
}

#[test]
fn test_value_hash_tagged() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!tag"),
        Value::from(42),
    )));
    let _ = set.insert(tagged.clone());
    assert!(set.contains(&tagged));
}

#[test]
fn test_value_ord_null_equal() {
    assert_eq!(Value::Null.cmp(&Value::Null), std::cmp::Ordering::Equal);
}

#[test]
fn test_value_ord_sequence_equal_length_equal_elements() {
    let seq1 = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let seq2 = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    assert_eq!(seq1.cmp(&seq2), std::cmp::Ordering::Equal);
}

#[test]
fn test_value_ord_mapping_equal() {
    let mut map1 = Mapping::new();
    let _ = map1.insert("a".to_string(), Value::from(1));
    let mut map2 = Mapping::new();
    let _ = map2.insert("a".to_string(), Value::from(1));
    assert_eq!(
        Value::Mapping(map1).cmp(&Value::Mapping(map2)),
        std::cmp::Ordering::Equal
    );
}

#[test]
fn test_value_ord_mapping_different_keys() {
    let mut map1 = Mapping::new();
    let _ = map1.insert("a".to_string(), Value::from(1));
    let mut map2 = Mapping::new();
    let _ = map2.insert("b".to_string(), Value::from(1));
    // "a" < "b" in string comparison
    assert!(Value::Mapping(map1) < Value::Mapping(map2));
}

#[test]
fn test_value_display_mapping_multiple() {
    let mut map = Mapping::new();
    let _ = map.insert("a".to_string(), Value::from(1));
    let _ = map.insert("b".to_string(), Value::from(2));
    let value = Value::Mapping(map);
    let display = format!("{}", value);
    assert!(display.contains("a: 1"));
    assert!(display.contains("b: 2"));
    assert!(display.contains(", "));
}

#[test]
fn test_value_display_sequence_multiple() {
    let seq = Value::Sequence(vec![Value::from(1), Value::from(2), Value::from(3)]);
    let display = format!("{}", seq);
    assert!(display.contains("1"));
    assert!(display.contains("2"));
    assert!(display.contains("3"));
    assert!(display.contains(", "));
}

#[test]
fn test_value_index_usize_on_non_sequence() {
    let value = Value::String("not a sequence".to_string());
    assert!(value.get(0usize).is_none());
}

#[test]
fn test_value_index_usize_mut_on_non_sequence() {
    let mut value = Value::String("not a sequence".to_string());
    assert!(value.get_mut(0usize).is_none());
}

#[test]
fn test_value_index_str_on_non_mapping() {
    let value = Value::from(42);
    assert!(value.get("key").is_none());
}

#[test]
fn test_value_index_str_mut_on_non_mapping() {
    let mut value = Value::from(42);
    assert!(value.get_mut("key").is_none());
}

#[test]
fn test_number_ord_float_nan_with_normal() {
    // Test the case where b is NaN and a is normal
    let normal = Number::Float(1.0);
    let nan = Number::Float(f64::NAN);

    // NaN is considered greater than any non-NaN
    assert!(normal < nan);
    assert!(nan > normal);
}

#[test]
fn test_number_ord_integer_with_nan() {
    let int = Number::Integer(42);
    let nan = Number::Float(f64::NAN);

    // Integer compared with NaN: NaN is greater
    assert!(int < nan);
}

#[test]
fn test_value_as_bool_none() {
    let value = Value::String("not a bool".to_string());
    assert!(value.as_bool().is_none());
}

#[test]
fn test_value_as_i64_none() {
    let value = Value::String("not a number".to_string());
    assert!(value.as_i64().is_none());
}

#[test]
fn test_value_merge_concat_insert_new_key() {
    let mut base: Value = from_str("a: 1\n").unwrap();
    let other: Value = from_str("b: 2\n").unwrap();
    base.merge_concat(other);
    assert_eq!(base.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(base.get("b").unwrap().as_i64(), Some(2));
}

#[test]
fn test_value_merge_concat_scalar_replace() {
    let mut base = Value::from(42);
    let other = Value::from(100);
    base.merge_concat(other);
    assert_eq!(base.as_i64(), Some(100));
}

#[test]
fn test_value_as_null_some() {
    let value = Value::Null;
    assert!(value.as_null().is_some());
}

#[test]
fn test_value_as_null_none() {
    let value = Value::from(42);
    assert!(value.as_null().is_none());
}

// ============================================================================
// Value Visitor Coverage Tests (triggered via serde_json)
// ============================================================================

#[test]
fn test_value_visitor_bool() {
    let json = "true";
    let value: Value = serde_json::from_str(json).unwrap();
    assert_eq!(value.as_bool(), Some(true));
}

#[test]
fn test_value_visitor_i64() {
    let json = "42";
    let value: Value = serde_json::from_str(json).unwrap();
    assert_eq!(value.as_i64(), Some(42));
}

#[test]
fn test_value_visitor_u64() {
    let json = "18446744073709551615";
    let value: Value = serde_json::from_str(json).unwrap();
    // Large u64 stored as i64, may overflow
    assert!(value.is_number());
}

#[test]
fn test_value_visitor_f64() {
    let json = "3.125";
    let value: Value = serde_json::from_str(json).unwrap();
    assert!(value.as_f64().is_some());
}

#[test]
fn test_value_visitor_str() {
    let json = "\"hello\"";
    let value: Value = serde_json::from_str(json).unwrap();
    assert_eq!(value.as_str(), Some("hello"));
}

#[test]
fn test_value_visitor_string_owned() {
    let json = "\"world\"";
    let value: Value = serde_json::from_str(json).unwrap();
    assert_eq!(value.as_str(), Some("world"));
}

#[test]
fn test_value_visitor_none() {
    let json = "null";
    let value: Value = serde_json::from_str(json).unwrap();
    assert!(value.is_null());
}

#[test]
fn test_value_visitor_unit() {
    // JSON doesn't have a unit type, but this tests the visitor path
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct UnitWrapper(Value);

    let json = "null";
    let wrapper: UnitWrapper = serde_json::from_str(json).unwrap();
    assert!(wrapper.0.is_null());
}

#[test]
fn test_value_visitor_seq() {
    let json = "[1, 2, 3]";
    let value: Value = serde_json::from_str(json).unwrap();
    assert!(value.is_sequence());
    assert_eq!(value.as_sequence().unwrap().len(), 3);
}

#[test]
fn test_value_visitor_map() {
    let json = "{\"a\": 1, \"b\": 2}";
    let value: Value = serde_json::from_str(json).unwrap();
    assert!(value.is_mapping());
}

// ============================================================================
// Mapping Struct Tests
// ============================================================================

#[test]
fn test_mapping_new() {
    let map = Mapping::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn test_mapping_with_capacity() {
    let map = Mapping::with_capacity(10);
    assert!(map.is_empty());
    assert!(map.capacity() >= 10);
}

#[test]
fn test_mapping_insert_and_get() {
    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(42));

    assert_eq!(map.len(), 1);
    assert!(map.contains_key("key"));
    assert_eq!(map.get("key").unwrap().as_i64(), Some(42));
}

#[test]
fn test_mapping_insert_string() {
    let mut map = Mapping::new();
    let _ = map.insert("key".to_string(), Value::from("value"));
    assert_eq!(map.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn test_mapping_get_mut() {
    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(1));

    if let Some(v) = map.get_mut("key") {
        *v = Value::from(2);
    }

    assert_eq!(map.get("key").unwrap().as_i64(), Some(2));
}

#[test]
fn test_mapping_remove() {
    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(42));

    let removed = map.remove("key");
    assert_eq!(removed.unwrap().as_i64(), Some(42));
    assert!(!map.contains_key("key"));
    assert!(map.is_empty());
}

#[test]
fn test_mapping_remove_entry() {
    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(42));

    let (k, v) = map.remove_entry("key").unwrap();
    assert_eq!(k, "key");
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn test_mapping_clear() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    map.clear();
    assert!(map.is_empty());
}

#[test]
fn test_mapping_reserve_and_shrink() {
    let mut map = Mapping::new();
    map.reserve(100);
    assert!(map.capacity() >= 100);

    let _ = map.insert("key", Value::from(1));
    map.shrink_to_fit();
    // Just verify it doesn't panic
}

#[test]
fn test_mapping_get_index() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    let (k, v) = map.get_index(0).unwrap();
    assert_eq!(k, "a");
    assert_eq!(v.as_i64(), Some(1));

    let (k, v) = map.get_index(1).unwrap();
    assert_eq!(k, "b");
    assert_eq!(v.as_i64(), Some(2));

    assert!(map.get_index(2).is_none());
}

#[test]
fn test_mapping_first_last() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));
    let _ = map.insert("c", Value::from(3));

    let (k, v) = map.first().unwrap();
    assert_eq!(k, "a");
    assert_eq!(v.as_i64(), Some(1));

    let (k, v) = map.last().unwrap();
    assert_eq!(k, "c");
    assert_eq!(v.as_i64(), Some(3));
}

#[test]
fn test_mapping_pop_first_last() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));
    let _ = map.insert("c", Value::from(3));

    let (k, v) = map.pop_first().unwrap();
    assert_eq!(k, "a");
    assert_eq!(v.as_i64(), Some(1));

    let (k, v) = map.pop_last().unwrap();
    assert_eq!(k, "c");
    assert_eq!(v.as_i64(), Some(3));

    assert_eq!(map.len(), 1);
}

#[test]
fn test_mapping_entry() {
    let mut map = Mapping::new();

    let _ = map.entry("key").or_insert(Value::from(1));
    assert_eq!(map.get("key").unwrap().as_i64(), Some(1));

    let _ = map.entry("key").or_insert(Value::from(2));
    assert_eq!(map.get("key").unwrap().as_i64(), Some(1)); // Not overwritten
}

#[test]
fn test_mapping_retain() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));
    let _ = map.insert("c", Value::from(3));

    map.retain(|k, _| k != "b");

    assert_eq!(map.len(), 2);
    assert!(map.contains_key("a"));
    assert!(!map.contains_key("b"));
    assert!(map.contains_key("c"));
}

#[test]
fn test_mapping_iter() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys, vec!["a", "b"]);

    let values: Vec<_> = map.values().map(|v| v.as_i64().unwrap()).collect();
    assert_eq!(values, vec![1, 2]);
}

#[test]
fn test_mapping_iter_mut() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    for (_, v) in map.iter_mut() {
        if let Some(n) = v.as_i64() {
            *v = Value::from(n * 10);
        }
    }

    assert_eq!(map.get("a").unwrap().as_i64(), Some(10));
    assert_eq!(map.get("b").unwrap().as_i64(), Some(20));
}

#[test]
fn test_mapping_sort_keys() {
    let mut map = Mapping::new();
    let _ = map.insert("c", Value::from(3));
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    map.sort_keys();

    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn test_mapping_reverse() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));
    let _ = map.insert("c", Value::from(3));

    map.reverse();

    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys, vec!["c", "b", "a"]);
}

#[test]
fn test_mapping_extend() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));

    map.extend(vec![
        ("b".to_string(), Value::from(2)),
        ("c".to_string(), Value::from(3)),
    ]);

    assert_eq!(map.len(), 3);
}

#[test]
fn test_mapping_index() {
    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(42));

    assert_eq!(map["key"].as_i64(), Some(42));
}

#[test]
fn test_mapping_index_mut() {
    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(1));

    map["key"] = Value::from(2);

    assert_eq!(map["key"].as_i64(), Some(2));
}

#[test]
fn test_mapping_into_iter() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    let pairs: Vec<_> = map.into_iter().collect();
    assert_eq!(pairs.len(), 2);
}

#[test]
fn test_mapping_from_iter() {
    let map: Mapping = vec![
        ("a".to_string(), Value::from(1)),
        ("b".to_string(), Value::from(2)),
    ]
    .into_iter()
    .collect();

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("a").unwrap().as_i64(), Some(1));
}

#[test]
fn test_mapping_from_array() {
    let map: Mapping = [
        ("a".to_string(), Value::from(1)),
        ("b".to_string(), Value::from(2)),
    ]
    .into();

    assert_eq!(map.len(), 2);
}

#[test]
fn test_mapping_from_indexmap() {
    let mut inner = indexmap::IndexMap::new();
    let _ = inner.insert("key".to_string(), Value::from(42));

    let map = Mapping::from(inner);
    assert_eq!(map.get("key").unwrap().as_i64(), Some(42));
}

#[test]
fn test_mapping_into_indexmap() {
    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(42));

    let inner: indexmap::IndexMap<String, Value> = map.into();
    assert_eq!(inner.get("key").unwrap().as_i64(), Some(42));
}

#[test]
fn test_mapping_into_inner() {
    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(42));

    let inner = map.into_inner();
    assert_eq!(inner.get("key").unwrap().as_i64(), Some(42));
}

#[test]
fn test_mapping_from_inner() {
    let mut inner = indexmap::IndexMap::new();
    let _ = inner.insert("key".to_string(), Value::from(42));

    let map = Mapping::from_inner(inner);
    assert_eq!(map.get("key").unwrap().as_i64(), Some(42));
}

#[test]
fn test_mapping_display() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    let s = format!("{}", map);
    assert!(s.contains("a: 1"));
    assert!(s.contains("b: 2"));
}

#[test]
fn test_mapping_clone() {
    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(42));

    let cloned = map.clone();
    assert_eq!(cloned.get("key").unwrap().as_i64(), Some(42));
}

#[test]
fn test_mapping_eq() {
    let mut map1 = Mapping::new();
    let _ = map1.insert("key", Value::from(42));

    let mut map2 = Mapping::new();
    let _ = map2.insert("key", Value::from(42));

    assert_eq!(map1, map2);
}

#[test]
fn test_mapping_ord() {
    let mut map1 = Mapping::new();
    let _ = map1.insert("a", Value::from(1));

    let mut map2 = Mapping::new();
    let _ = map2.insert("a", Value::from(1));
    let _ = map2.insert("b", Value::from(2));

    assert!(map1 < map2); // Shorter is less
}

#[test]
fn test_mapping_serde_roundtrip() {
    use noyalib::{from_str, to_string};

    let mut map = Mapping::new();
    let _ = map.insert("name", Value::from("test"));
    let _ = map.insert("value", Value::from(42));

    let yaml = to_string(&map).unwrap();
    let parsed: Mapping = from_str(&yaml).unwrap();

    assert_eq!(map, parsed);
}

#[test]
fn test_mapping_swap_remove() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));
    let _ = map.insert("c", Value::from(3));

    let removed = map.swap_remove("b");
    assert_eq!(removed.unwrap().as_i64(), Some(2));
    assert_eq!(map.len(), 2);
}

#[test]
fn test_mapping_shift_remove() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));
    let _ = map.insert("c", Value::from(3));

    let removed = map.shift_remove("b");
    assert_eq!(removed.unwrap().as_i64(), Some(2));

    // Order should be preserved
    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys, vec!["a", "c"]);
}

#[test]
fn test_mapping_get_index_mut() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));

    if let Some((_, v)) = map.get_index_mut(0) {
        *v = Value::from(100);
    }

    assert_eq!(map.get("a").unwrap().as_i64(), Some(100));
}

#[test]
fn test_mapping_first_mut() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    if let Some((_, v)) = map.first_mut() {
        *v = Value::from(100);
    }

    assert_eq!(map.get("a").unwrap().as_i64(), Some(100));
}

#[test]
fn test_mapping_last_mut() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    if let Some((_, v)) = map.last_mut() {
        *v = Value::from(100);
    }

    assert_eq!(map.get("b").unwrap().as_i64(), Some(100));
}

#[test]
fn test_mapping_values_mut() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    for v in map.values_mut() {
        if let Some(n) = v.as_i64() {
            *v = Value::from(n * 10);
        }
    }

    assert_eq!(map.get("a").unwrap().as_i64(), Some(10));
    assert_eq!(map.get("b").unwrap().as_i64(), Some(20));
}

#[test]
fn test_mapping_into_iter_mut() {
    let mut map = Mapping::new();
    let _ = map.insert("a", Value::from(1));
    let _ = map.insert("b", Value::from(2));

    // Test &mut Mapping IntoIterator
    for (_, v) in &mut map {
        if let Some(n) = v.as_i64() {
            *v = Value::from(n * 100);
        }
    }

    assert_eq!(map.get("a").unwrap().as_i64(), Some(100));
    assert_eq!(map.get("b").unwrap().as_i64(), Some(200));
}

#[test]
fn test_mapping_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut map1 = Mapping::new();
    let _ = map1.insert("a", Value::from(1));
    let _ = map1.insert("b", Value::from(2));

    let mut map2 = Mapping::new();
    let _ = map2.insert("a", Value::from(1));
    let _ = map2.insert("b", Value::from(2));

    let mut hasher1 = DefaultHasher::new();
    map1.hash(&mut hasher1);
    let hash1 = hasher1.finish();

    let mut hasher2 = DefaultHasher::new();
    map2.hash(&mut hasher2);
    let hash2 = hasher2.finish();

    assert_eq!(hash1, hash2);
}

#[test]
fn test_mapping_ord_different_keys() {
    let mut map1 = Mapping::new();
    let _ = map1.insert("a", Value::from(1));

    let mut map2 = Mapping::new();
    let _ = map2.insert("b", Value::from(1));

    // "a" < "b" lexicographically
    assert!(map1 < map2);
}

#[test]
fn test_mapping_ord_different_values() {
    let mut map1 = Mapping::new();
    let _ = map1.insert("a", Value::from(1));

    let mut map2 = Mapping::new();
    let _ = map2.insert("a", Value::from(2));

    // Same key, different values
    assert!(map1 < map2);
}

#[test]
fn test_mapping_ord_equal() {
    let mut map1 = Mapping::new();
    let _ = map1.insert("a", Value::from(1));
    let _ = map1.insert("b", Value::from(2));

    let mut map2 = Mapping::new();
    let _ = map2.insert("a", Value::from(1));
    let _ = map2.insert("b", Value::from(2));

    use std::cmp::Ordering;
    assert_eq!(map1.cmp(&map2), Ordering::Equal);
}

#[test]
fn test_mapping_partial_ord() {
    let mut map1 = Mapping::new();
    let _ = map1.insert("a", Value::from(1));

    let mut map2 = Mapping::new();
    let _ = map2.insert("a", Value::from(2));

    assert!(map1.partial_cmp(&map2) == Some(std::cmp::Ordering::Less));
}

#[test]
fn test_mapping_in_hashset() {
    use std::collections::HashSet;

    let mut map = Mapping::new();
    let _ = map.insert("key", Value::from(42));

    let mut set: HashSet<Mapping> = HashSet::new();
    let _ = set.insert(map.clone());

    assert!(set.contains(&map));
}

#[test]
fn test_value_hash_null_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let null1 = Value::Null;
    let null2 = Value::Null;

    let mut hasher1 = DefaultHasher::new();
    null1.hash(&mut hasher1);
    let hash1 = hasher1.finish();

    let mut hasher2 = DefaultHasher::new();
    null2.hash(&mut hasher2);
    let hash2 = hasher2.finish();

    assert_eq!(hash1, hash2);
}

#[test]
fn test_mapping_deserialize_from_yaml() {
    use noyalib::from_str;

    let yaml = "a: 1\nb: 2\n";
    let map: Mapping = from_str(yaml).unwrap();

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(map.get("b").unwrap().as_i64(), Some(2));
}

#[test]
fn test_mapping_deserialize_nested() {
    use noyalib::from_str;

    let yaml = r#"
outer:
  inner: value
  number: 42
"#;
    let map: Mapping = from_str(yaml).unwrap();

    assert!(map.contains_key("outer"));
    let outer = map.get("outer").unwrap().as_mapping().unwrap();
    assert_eq!(outer.get("inner").unwrap().as_str(), Some("value"));
}

// ============================================================================
// apply_merge tests
// ============================================================================

#[test]
fn test_apply_merge_basic() {
    use noyalib::from_str;

    let yaml = r#"
defaults: &defaults
  timeout: 30
  retries: 3

server:
  <<: *defaults
  host: localhost
"#;

    let mut value: Value = from_str(yaml).unwrap();
    value.apply_merge().unwrap();

    // Check that server has merged values
    assert_eq!(value["server"]["host"].as_str(), Some("localhost"));
    assert_eq!(value["server"]["timeout"].as_i64(), Some(30));
    assert_eq!(value["server"]["retries"].as_i64(), Some(3));

    // The << key should be removed after merge
    assert!(value["server"].get("<<").is_none());
}

#[test]
fn test_apply_merge_no_override() {
    use noyalib::from_str;

    let yaml = r#"
defaults: &defaults
  timeout: 30
  retries: 3

server:
  <<: *defaults
  host: localhost
  timeout: 60
"#;

    let mut value: Value = from_str(yaml).unwrap();
    value.apply_merge().unwrap();

    // Explicit value should NOT be overridden by merge
    assert_eq!(value["server"]["timeout"].as_i64(), Some(60));
    // Merged value should appear
    assert_eq!(value["server"]["retries"].as_i64(), Some(3));
}

#[test]
fn test_apply_merge_multiple_sources() {
    use noyalib::from_str;

    let yaml = r#"
a: &a
  x: 1
  common: from_a

b: &b
  y: 2
  common: from_b

merged:
  <<: [*a, *b]
  z: 3
"#;

    let mut value: Value = from_str(yaml).unwrap();
    value.apply_merge().unwrap();

    // z is direct value
    assert_eq!(value["merged"]["z"].as_i64(), Some(3));
    // x from *a
    assert_eq!(value["merged"]["x"].as_i64(), Some(1));
    // y from *b
    assert_eq!(value["merged"]["y"].as_i64(), Some(2));
    // common: first source (*a) takes precedence
    assert_eq!(value["merged"]["common"].as_str(), Some("from_a"));
}

#[test]
fn test_apply_merge_nested() {
    use noyalib::from_str;

    let yaml = r#"
base: &base
  nested:
    a: 1

level1:
  <<: *base
  extra: value
"#;

    let mut value: Value = from_str(yaml).unwrap();
    value.apply_merge().unwrap();

    assert_eq!(value["level1"]["extra"].as_str(), Some("value"));
    assert_eq!(value["level1"]["nested"]["a"].as_i64(), Some(1));
}

#[test]
fn test_apply_merge_recursive() {
    use noyalib::from_str;

    let yaml = r#"
base1: &base1
  a: 1

base2: &base2
  <<: *base1
  b: 2

final:
  <<: *base2
  c: 3
"#;

    let mut value: Value = from_str(yaml).unwrap();
    value.apply_merge().unwrap();

    // final should have all values through recursive merge
    assert_eq!(value["final"]["a"].as_i64(), Some(1));
    assert_eq!(value["final"]["b"].as_i64(), Some(2));
    assert_eq!(value["final"]["c"].as_i64(), Some(3));
}

#[test]
fn test_apply_merge_in_sequence() {
    use noyalib::from_str;

    let yaml = r#"
defaults: &defaults
  timeout: 30

servers:
  - <<: *defaults
    name: server1
  - <<: *defaults
    name: server2
    timeout: 60
"#;

    let mut value: Value = from_str(yaml).unwrap();
    value.apply_merge().unwrap();

    let servers = value["servers"].as_sequence().unwrap();
    assert_eq!(servers[0]["name"].as_str(), Some("server1"));
    assert_eq!(servers[0]["timeout"].as_i64(), Some(30));
    assert_eq!(servers[1]["name"].as_str(), Some("server2"));
    assert_eq!(servers[1]["timeout"].as_i64(), Some(60)); // Overridden
}

#[test]
fn test_apply_merge_error_scalar_in_merge() {
    // Create a value where << points to a scalar (not a mapping)
    let mut mapping = Mapping::new();
    let _ = mapping.insert("<<", Value::from("not_a_mapping")); // Scalar string
    let _ = mapping.insert("value", Value::from(1));

    let mut value = Value::Mapping(mapping);
    let result = value.apply_merge();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("scalar"));
}

#[test]
fn test_apply_merge_error_sequence_in_merge_element() {
    // Create a value where << points to a sequence (not a mapping)
    let mut mapping = Mapping::new();
    let _ = mapping.insert(
        "<<",
        Value::Sequence(vec![
            Value::Sequence(vec![Value::from(1)]), // This is a sequence, not a mapping
        ]),
    );
    let _ = mapping.insert("value", Value::from(1));

    let mut value = Value::Mapping(mapping);
    let result = value.apply_merge();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("sequence"));
}

#[test]
fn test_apply_merge_no_merge_key() {
    use noyalib::from_str;

    let yaml = r#"
simple:
  a: 1
  b: 2
"#;

    let mut value: Value = from_str(yaml).unwrap();
    let original = value.clone();
    value.apply_merge().unwrap();

    // Value should be unchanged when there's no merge key
    assert_eq!(value, original);
}

#[test]
fn test_apply_merge_empty_merge_source() {
    use noyalib::from_str;

    let yaml = r#"
empty: &empty {}

test:
  <<: *empty
  value: 1
"#;

    let mut value: Value = from_str(yaml).unwrap();
    value.apply_merge().unwrap();

    assert_eq!(value["test"]["value"].as_i64(), Some(1));
    assert!(value["test"].get("<<").is_none());
}

#[test]
fn test_apply_merge_idempotent() {
    use noyalib::from_str;

    let yaml = r#"
defaults: &defaults
  timeout: 30

server:
  <<: *defaults
  host: localhost
"#;

    let mut value: Value = from_str(yaml).unwrap();
    value.apply_merge().unwrap();
    let after_first = value.clone();

    // Applying merge again should not change anything
    value.apply_merge().unwrap();
    assert_eq!(value, after_first);
}

// ============================================================================
// ValueIndex enhancement tests
// ============================================================================

#[test]
fn test_value_index_string_type() {
    use noyalib::from_str;

    let yaml = "name: test\nvalue: 42\n";
    let value: Value = from_str(yaml).unwrap();

    // Test with String
    let key = String::from("name");
    assert_eq!(value.get(key).unwrap().as_str(), Some("test"));

    // Test with &String
    let key2 = String::from("value");
    assert_eq!(value.get(&key2).unwrap().as_i64(), Some(42));
}

#[test]
fn test_value_index_tagged_value() {
    // Test that indexing works through tagged values
    let inner_mapping = {
        let mut m = Mapping::new();
        let _ = m.insert("key", Value::from("value"));
        Value::Mapping(m)
    };

    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        inner_mapping,
    )));

    // Should be able to index through the tag
    assert_eq!(tagged.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn test_value_index_or_insert_creates_key() {
    use noyalib::ValueIndex;

    let mut value = Value::Mapping(Mapping::new());

    // index_or_insert should create the key if it doesn't exist
    {
        let entry = "new_key".index_or_insert(&mut value);
        *entry = Value::from(42);
    }

    assert_eq!(value.get("new_key").unwrap().as_i64(), Some(42));
}

#[test]
fn test_value_index_or_insert_existing_key() {
    use noyalib::ValueIndex;

    let mut mapping = Mapping::new();
    let _ = mapping.insert("existing", Value::from(10));
    let mut value = Value::Mapping(mapping);

    // index_or_insert should return existing key, not overwrite
    {
        let entry = "existing".index_or_insert(&mut value);
        assert_eq!(entry.as_i64(), Some(10));
        *entry = Value::from(20);
    }

    assert_eq!(value.get("existing").unwrap().as_i64(), Some(20));
}

#[test]
fn test_value_index_or_insert_null_to_mapping() {
    use noyalib::ValueIndex;

    let mut value = Value::Null;

    // index_or_insert on null should convert to mapping
    {
        let entry = "key".index_or_insert(&mut value);
        *entry = Value::from("hello");
    }

    assert!(value.is_mapping());
    assert_eq!(value.get("key").unwrap().as_str(), Some("hello"));
}

#[test]
fn test_value_index_or_insert_chained() {
    use noyalib::ValueIndex;

    let mut value = Value::Null;

    // Chain multiple index_or_insert calls to build nested structure
    {
        let a = "a".index_or_insert(&mut value);
        let b = "b".index_or_insert(a);
        let c = "c".index_or_insert(b);
        *c = Value::from(123);
    }

    assert_eq!(
        value
            .get("a")
            .unwrap()
            .get("b")
            .unwrap()
            .get("c")
            .unwrap()
            .as_i64(),
        Some(123)
    );
}

#[test]
#[should_panic(expected = "cannot access index")]
fn test_value_index_or_insert_sequence_out_of_bounds() {
    use noyalib::ValueIndex;

    let mut value = Value::Sequence(vec![Value::from(1), Value::from(2)]);

    // Should panic because index 5 is out of bounds
    let _ = 5usize.index_or_insert(&mut value);
}

#[test]
#[should_panic(expected = "cannot access index")]
fn test_value_index_or_insert_wrong_type_for_usize() {
    use noyalib::ValueIndex;

    let mut value = Value::from("not a sequence");

    // Should panic because strings can't be indexed by usize
    let _ = 0usize.index_or_insert(&mut value);
}

#[test]
#[should_panic(expected = "cannot access key")]
fn test_value_index_or_insert_wrong_type_for_str() {
    use noyalib::ValueIndex;

    let mut value = Value::Sequence(vec![Value::from(1)]);

    // Should panic because sequences can't be indexed by string
    let _ = "key".index_or_insert(&mut value);
}

#[test]
fn test_value_index_sequence_usize() {
    use noyalib::ValueIndex;

    let value = Value::Sequence(vec![
        Value::from("first"),
        Value::from("second"),
        Value::from("third"),
    ]);

    assert_eq!(0usize.index_into(&value).unwrap().as_str(), Some("first"));
    assert_eq!(1usize.index_into(&value).unwrap().as_str(), Some("second"));
    assert_eq!(2usize.index_into(&value).unwrap().as_str(), Some("third"));
    assert!(3usize.index_into(&value).is_none());
}

#[test]
fn test_value_index_sequence_tagged() {
    use noyalib::ValueIndex;

    let seq = Value::Sequence(vec![Value::from("item")]);
    let tagged = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!list"), seq)));

    // Should index through the tag
    assert_eq!(0usize.index_into(&tagged).unwrap().as_str(), Some("item"));
}

// ============================================================================
// serde_yml Parity Tests
// ============================================================================

#[test]
fn test_number_from_impls() {
    assert_eq!(Number::from(42_i8), Number::Integer(42));
    assert_eq!(Number::from(1000_i16), Number::Integer(1000));
    assert_eq!(Number::from(100_000_i32), Number::Integer(100_000));
    assert_eq!(Number::from(1_000_000_i64), Number::Integer(1_000_000));
    assert_eq!(Number::from(42_isize), Number::Integer(42));

    assert_eq!(Number::from(255_u8), Number::Integer(255));
    assert_eq!(Number::from(65535_u16), Number::Integer(65535));
    assert_eq!(Number::from(4_000_000_u32), Number::Integer(4_000_000));
    assert_eq!(Number::from(100_u64), Number::Integer(100));
    assert_eq!(Number::from(42_usize), Number::Integer(42));

    let big = u64::MAX;
    #[cfg(not(feature = "lossless-u64"))]
    assert_eq!(Number::from(big), Number::Float(big as f64));
    #[cfg(feature = "lossless-u64")]
    assert_eq!(Number::from(big), Number::Unsigned(big));

    assert_eq!(Number::from(1.5_f32), Number::Float(1.5));
    assert_eq!(Number::from(2.5_f64), Number::Float(2.5));
}

#[test]
fn test_value_from_u64() {
    let v = Value::from(42_u64);
    assert_eq!(v.as_i64(), Some(42));

    let v = Value::from(u64::MAX);
    #[cfg(not(feature = "lossless-u64"))]
    assert!(v.as_f64().is_some());
    #[cfg(feature = "lossless-u64")]
    assert_eq!(v.as_u64(), Some(u64::MAX));
}

#[test]
fn test_value_from_isize() {
    let v = Value::from(-10_isize);
    assert_eq!(v.as_i64(), Some(-10));
}

#[test]
fn test_value_from_usize() {
    let v = Value::from(99_usize);
    assert_eq!(v.as_i64(), Some(99));
}

#[test]
fn test_value_from_cow() {
    let owned: Cow<'_, str> = Cow::Owned("hello".to_string());
    let v = Value::from(owned);
    assert_eq!(v.as_str(), Some("hello"));

    let borrowed: Cow<'_, str> = Cow::Borrowed("world");
    let v = Value::from(borrowed);
    assert_eq!(v.as_str(), Some("world"));
}

#[test]
fn test_value_from_slice() {
    let nums: &[i64] = &[1, 2, 3];
    let v = Value::from(nums);
    assert!(v.is_sequence());
    assert_eq!(v[0].as_i64(), Some(1));
    assert_eq!(v[1].as_i64(), Some(2));
    assert_eq!(v[2].as_i64(), Some(3));
}

#[test]
fn test_value_from_iterator() {
    let v: Value = (1..=3).map(Value::from).collect();
    assert!(v.is_sequence());
    assert_eq!(v[0].as_i64(), Some(1));
    assert_eq!(v[2].as_i64(), Some(3));
}

#[test]
fn test_tag_comparison_ignores_bang() {
    assert_eq!(Tag::new("!foo"), Tag::new("foo"));
    assert_eq!(Tag::new("!foo"), Tag::new("!foo"));
    assert_eq!(Tag::new("bar"), Tag::new("bar"));

    // Hashes should match
    fn hash_of(t: &Tag) -> u64 {
        let mut h = DefaultHasher::new();
        t.hash(&mut h);
        h.finish()
    }
    assert_eq!(hash_of(&Tag::new("!foo")), hash_of(&Tag::new("foo")));
}

#[test]
fn test_tag_ordering() {
    assert!(Tag::new("!a") < Tag::new("!b"));
    assert!(Tag::new("a") < Tag::new("b"));
    assert_eq!(
        Tag::new("!a").cmp(&Tag::new("a")),
        std::cmp::Ordering::Equal
    );
}

#[test]
fn test_nobang() {
    assert_eq!(nobang("!foo"), "foo");
    assert_eq!(nobang("foo"), "foo");
    assert_eq!(nobang("!!int"), "!int");
    assert_eq!(nobang("!"), "");
    assert_eq!(nobang(""), "");
}

#[test]
fn test_tag_nobang_method() {
    assert_eq!(Tag::new("!foo").nobang(), "foo");
    assert_eq!(Tag::new("bar").nobang(), "bar");
}

#[test]
fn test_check_for_tag() {
    match check_for_tag(&"!mytag") {
        MaybeTag::Tag(s) => assert_eq!(s, "!mytag"),
        MaybeTag::NotTag(_) => panic!("expected Tag"),
    }

    match check_for_tag(&"plain") {
        MaybeTag::NotTag(s) => assert_eq!(s, "plain"),
        MaybeTag::Tag(_) => panic!("expected NotTag"),
    }
}

#[test]
fn test_maybe_tag_variants() {
    let tag: MaybeTag<String> = MaybeTag::Tag("!x".to_string());
    let not: MaybeTag<String> = MaybeTag::NotTag("y".to_string());
    assert_ne!(tag, not);
}

#[test]
fn test_tag_try_from_bytes() {
    let tag = Tag::try_from(b"!custom".as_slice()).unwrap();
    assert_eq!(tag.as_str(), "!custom");

    // Invalid UTF-8
    assert!(Tag::try_from(&[0xFF, 0xFE][..]).is_err());
}

#[test]
fn test_untag_consuming() {
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!int"),
        Value::from(42),
    )));
    let untagged = tagged.untag();
    assert_eq!(untagged.as_i64(), Some(42));
}

#[test]
fn test_untag_nested() {
    let inner = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!inner"),
        Value::from("hello"),
    )));
    let outer = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!outer"), inner)));
    let untagged = outer.untag();
    assert_eq!(untagged.as_str(), Some("hello"));
}

#[test]
fn test_untag_sequence() {
    let seq = Value::Sequence(vec![
        Value::Tagged(Box::new(TaggedValue::new(Tag::new("!x"), Value::from(1)))),
        Value::from(2),
    ]);
    let untagged = seq.untag();
    if let Value::Sequence(s) = &untagged {
        assert_eq!(s[0].as_i64(), Some(1));
        assert_eq!(s[1].as_i64(), Some(2));
    } else {
        panic!("expected sequence");
    }
}

#[test]
fn test_untag_ref() {
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!int"),
        Value::from(42),
    )));
    let inner = tagged.untag_ref();
    assert_eq!(inner.as_i64(), Some(42));
}

#[test]
fn test_untag_mut() {
    let mut tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!int"),
        Value::from(42),
    )));
    let inner = tagged.untag_mut();
    *inner = Value::from(99);
    assert_eq!(tagged.untag_ref().as_i64(), Some(99));
}

#[test]
fn test_tagged_value_serialize() {
    let tv = TaggedValue::new(Tag::new("!color"), Value::from("red"));
    let yaml = to_string(&tv).unwrap();
    assert!(yaml.contains("!color"));
    assert!(yaml.contains("red"));
}

#[test]
fn test_tagged_value_deserialize() {
    // TaggedValue serializes as a single-entry map: {tag: value}
    // Deserialize from that format
    let yaml = "color: red\n";
    let tv: TaggedValue = from_str(yaml).unwrap();
    assert_eq!(tv.tag().as_str(), "color");
    assert_eq!(tv.value().as_str(), Some("red"));
}

#[test]
fn test_tagged_value_roundtrip() {
    let original = TaggedValue::new(Tag::new("!mytype"), Value::from(42));
    let yaml = to_string(&original).unwrap();
    let deserialized: TaggedValue = from_str(&yaml).unwrap();
    assert_eq!(deserialized.tag(), original.tag());
}

#[test]
fn test_mapping_from_vec() {
    let pairs = vec![
        ("name".to_string(), Value::from("test")),
        ("version".to_string(), Value::from(1)),
    ];
    let map = Mapping::from(pairs);
    assert_eq!(map.len(), 2);
    assert_eq!(map.get("name").unwrap().as_str(), Some("test"));
    assert_eq!(map.get("version").unwrap().as_i64(), Some(1));
}

// ============================================================================
// DuplicateKeyPolicy Tests
// ============================================================================

#[test]
fn test_duplicate_key_policy_error() {
    use noyalib::{DuplicateKeyPolicy, ParserConfig};

    // yaml-rust2 rejects duplicate keys at the parser level before our
    // policy is checked, so all three policies result in an error for
    // raw duplicate keys. Verify the enum exists and the config builder works.
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    assert_eq!(config.duplicate_key_policy, DuplicateKeyPolicy::Error);

    let yaml = "a: 1\na: 2";
    let result: Result<Value, _> = noyalib::from_str_with_config(yaml, &config);
    // Parser rejects before our policy gets invoked
    assert!(result.is_err());
}

#[test]
fn test_duplicate_key_policy_first() {
    use noyalib::{DuplicateKeyPolicy, ParserConfig};

    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    assert_eq!(config.duplicate_key_policy, DuplicateKeyPolicy::First);

    // Non-duplicate keys work fine
    let yaml = "a: 1\nb: 2";
    let value: Value = noyalib::from_str_with_config(yaml, &config).unwrap();
    assert_eq!(value.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(value.get("b").unwrap().as_i64(), Some(2));
}

#[test]
fn test_duplicate_key_policy_last() {
    use noyalib::{DuplicateKeyPolicy, ParserConfig};

    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
    assert_eq!(config.duplicate_key_policy, DuplicateKeyPolicy::Last);

    // Default behavior: last occurrence wins (when parser allows it)
    let yaml = "a: 1\nb: 2";
    let value: Value = noyalib::from_str_with_config(yaml, &config).unwrap();
    assert_eq!(value.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(value.get("b").unwrap().as_i64(), Some(2));
}

#[test]
fn test_duplicate_key_policy_strict_config() {
    use noyalib::{DuplicateKeyPolicy, ParserConfig};

    // strict() should set Error policy
    let strict = ParserConfig::strict();
    assert_eq!(strict.duplicate_key_policy, DuplicateKeyPolicy::Error);
}

// ============================================================================
// ValueIndex for &Value Tests
// ============================================================================

#[test]
fn test_value_index_by_value() {
    let mut map = Mapping::new();
    let _ = map.insert("name", Value::from("test"));
    let _ = map.insert("age", Value::from(30));
    let value = Value::Mapping(map);

    // Index by Value::String
    let key = Value::from("name");
    assert_eq!(value.get(&key).unwrap().as_str(), Some("test"));

    // Index by Value::String that doesn't exist
    let missing_key = Value::from("missing");
    assert!(value.get(&missing_key).is_none());

    // Index sequence by Value::Number
    let seq = Value::Sequence(vec![Value::from(10), Value::from(20), Value::from(30)]);
    let idx = Value::from(1i64);
    assert_eq!(seq.get(&idx).unwrap().as_i64(), Some(20));

    // Negative index should return None
    let neg = Value::Number(Number::Integer(-1));
    assert!(seq.get(&neg).is_none());
}
