//! Serializer coverage tests — all config paths, flow styles, scalar styles,
//! multi-doc config.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::BTreeMap;

use noyalib::{
    FlowStyle, Mapping, ScalarStyle, SerializerConfig, Value, to_string,
    to_string_multi_with_config, to_string_with_config, to_value, to_writer_multi_with_config,
    to_writer_with_config,
};

// ============================================================================
// FlowStyle config
// ============================================================================

#[test]
fn flow_style_flow_config() {
    let mut map = BTreeMap::new();
    let _ = map.insert("a", vec![1, 2]);
    let config = SerializerConfig::new().flow_style(FlowStyle::Flow);
    let _yaml = to_string_with_config(&map, &config).unwrap();
    // Should not panic; flow mode applied globally
}

#[test]
fn flow_style_auto_small_collection() {
    let config = SerializerConfig::new()
        .flow_style(FlowStyle::Auto)
        .flow_threshold(5);
    let v = vec![1, 2, 3];
    let _yaml = to_string_with_config(&v, &config).unwrap();
}

#[test]
fn flow_style_auto_large_collection() {
    let config = SerializerConfig::new()
        .flow_style(FlowStyle::Auto)
        .flow_threshold(2);
    let v = vec![1, 2, 3, 4, 5];
    let _yaml = to_string_with_config(&v, &config).unwrap();
}

// ============================================================================
// ScalarStyle config
// ============================================================================

#[test]
fn scalar_style_double_quoted() {
    let config = SerializerConfig::new().scalar_style(ScalarStyle::DoubleQuoted);
    let yaml = to_string_with_config(&"hello", &config).unwrap();
    // The serializer should produce the string (may or may not quote based on impl)
    assert!(yaml.contains("hello"));
}

#[test]
fn scalar_style_single_quoted() {
    let config = SerializerConfig::new().scalar_style(ScalarStyle::SingleQuoted);
    let _yaml = to_string_with_config(&"hello", &config).unwrap();
}

#[test]
fn scalar_style_literal() {
    let config = SerializerConfig::new().scalar_style(ScalarStyle::Literal);
    let _yaml = to_string_with_config(&"hello", &config).unwrap();
}

#[test]
fn scalar_style_folded() {
    let config = SerializerConfig::new().scalar_style(ScalarStyle::Folded);
    let _yaml = to_string_with_config(&"hello", &config).unwrap();
}

#[test]
fn scalar_style_plain() {
    let config = SerializerConfig::new().scalar_style(ScalarStyle::Plain);
    let _yaml = to_string_with_config(&"hello", &config).unwrap();
}

// ============================================================================
// block_scalar_threshold
// ============================================================================

#[test]
fn block_scalar_threshold_high() {
    let config = SerializerConfig::new()
        .block_scalars(true)
        .block_scalar_threshold(5);
    // String with only 1 newline should NOT use block style
    let yaml = to_string_with_config(&"line1\nline2", &config).unwrap();
    assert!(!yaml.starts_with('|'));
}

#[test]
fn block_scalars_disabled() {
    let config = SerializerConfig::new().block_scalars(false);
    let yaml = to_string_with_config(&"line1\nline2\nline3", &config).unwrap();
    // Should use quoted style instead of block
    assert!(!yaml.starts_with('|'));
}

// ============================================================================
// Multi-doc with config
// ============================================================================

#[test]
fn to_string_multi_with_custom_config() {
    let config = SerializerConfig::new().indent(4);
    let docs = vec![42i64, 43];
    let yaml = to_string_multi_with_config(&docs, &config).unwrap();
    assert!(yaml.contains("---"));
    assert!(yaml.contains("42"));
    assert!(yaml.contains("43"));
}

#[test]
fn to_writer_multi_with_custom_config() {
    let config = SerializerConfig::new().indent(4);
    let docs = vec!["a", "b"];
    let mut buf = Vec::new();
    to_writer_multi_with_config(&mut buf, &docs, &config).unwrap();
    let yaml = String::from_utf8(buf).unwrap();
    assert!(yaml.contains("---"));
}

// ============================================================================
// to_writer_with_config
// ============================================================================

#[test]
fn to_writer_with_custom_config() {
    let config = SerializerConfig::new().document_start(true);
    let mut buf = Vec::new();
    to_writer_with_config(&mut buf, &42i64, &config).unwrap();
    let yaml = String::from_utf8(buf).unwrap();
    assert!(yaml.starts_with("---"));
}

// ============================================================================
// Document markers
// ============================================================================

#[test]
fn document_end_marker() {
    let config = SerializerConfig::new()
        .document_start(true)
        .document_end(true);
    let yaml = to_string_with_config(&42i64, &config).unwrap();
    assert!(yaml.starts_with("---"));
    assert!(yaml.contains("..."));
}

// ============================================================================
// Serializer edge cases
// ============================================================================

#[test]
fn serialize_u64_max() {
    let result = to_value(&u64::MAX);
    #[cfg(not(feature = "lossless-u64"))]
    {
        // Legacy model cannot represent u64::MAX losslessly.
        assert!(result.is_err());
    }
    #[cfg(feature = "lossless-u64")]
    {
        assert_eq!(result.unwrap().as_u64(), Some(u64::MAX));
    }
}

#[test]
fn serialize_bytes() {
    // Phase 1.2 contract: bytes serialise as a `!!binary` tagged
    // scalar carrying the RFC 4648 base64 encoding (YAML 1.2.2
    // §10.4) — not a UTF-8 string. The string-encoding form was
    // wrong both for non-UTF-8 payloads and for round-trip with
    // `serde_bytes::ByteBuf`.
    let bytes = serde_bytes::Bytes::new(b"hello");
    let val = to_value(&bytes).unwrap();
    let tagged = match &val {
        Value::Tagged(t) => t.as_ref(),
        other => panic!("expected Tagged !!binary value, got {other:?}"),
    };
    assert_eq!(tagged.tag().as_str(), "!!binary");
    // "hello" in standard base64 is "aGVsbG8=".
    assert_eq!(tagged.value().as_str(), Some("aGVsbG8="));
}

#[test]
fn serialize_char() {
    let val = to_value(&'x').unwrap();
    assert_eq!(val.as_str(), Some("x"));
}

#[test]
fn serialize_unit() {
    let val = to_value(&()).unwrap();
    assert!(val.is_null());
}

#[test]
fn serialize_none() {
    let val = to_value(&Option::<i64>::None).unwrap();
    assert!(val.is_null());
}

#[test]
fn serialize_some() {
    let val = to_value(&Some(42i64)).unwrap();
    assert_eq!(val.as_i64(), Some(42));
}

#[test]
fn serialize_map_with_non_string_key() {
    // boolean keys should work
    let mut map = BTreeMap::new();
    let _ = map.insert(true, "yes");
    let val = to_value(&map).unwrap();
    assert!(val.is_mapping());
}

#[test]
fn serialize_map_with_integer_key() {
    let mut map = BTreeMap::new();
    let _ = map.insert(42i64, "value");
    let val = to_value(&map).unwrap();
    assert!(val.is_mapping());
}

#[test]
fn serialize_tagged_value() {
    use noyalib::{Tag, TaggedValue};
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        Value::from(42),
    )));
    let yaml = to_string(&tagged).unwrap();
    assert!(yaml.contains("!custom"));
    assert!(yaml.contains("42"));
}

#[test]
fn serialize_tagged_value_in_sequence() {
    use noyalib::{Tag, TaggedValue};
    let seq = vec![
        Value::from(1),
        Value::Tagged(Box::new(TaggedValue::new(Tag::new("!t"), Value::from(2)))),
        Value::from(3),
    ];
    let yaml = to_string(&seq).unwrap();
    assert!(yaml.contains("!t"));
}

#[test]
fn serialize_f32() {
    let val = to_value(&2.75f32).unwrap();
    assert!(val.as_f64().is_some());
}

#[test]
fn serialize_i8_i16_i32() {
    let _ = to_value(&1i8).unwrap();
    let _ = to_value(&2i16).unwrap();
    let _ = to_value(&3i32).unwrap();
}

#[test]
fn serialize_u8_u16_u32() {
    let _ = to_value(&1u8).unwrap();
    let _ = to_value(&2u16).unwrap();
    let _ = to_value(&3u32).unwrap();
}

#[test]
fn write_string_needs_quotes() {
    // Strings that need quoting: empty, starts with special chars, etc.
    for s in [
        "", "true", "false", "null", "~", "42", "3.14", ": colon", "# hash",
    ] {
        let yaml = to_string(&s).unwrap();
        let parsed: String = noyalib::from_str(&yaml).unwrap();
        assert_eq!(s, parsed, "roundtrip failed for {s:?}");
    }
}

#[test]
fn write_empty_mapping() {
    let m = Mapping::new();
    let yaml = to_string(&Value::Mapping(m)).unwrap();
    assert_eq!(yaml.trim(), "{}");
}

#[test]
fn write_empty_sequence() {
    let s: Vec<i64> = vec![];
    let yaml = to_string(&s).unwrap();
    assert_eq!(yaml.trim(), "[]");
}

#[test]
fn write_nan() {
    let yaml = to_string(&f64::NAN).unwrap();
    assert!(yaml.contains(".nan"));
}

#[test]
fn write_infinity() {
    let yaml = to_string(&f64::INFINITY).unwrap();
    assert!(yaml.contains(".inf"));
}

#[test]
fn write_neg_infinity() {
    let yaml = to_string(&f64::NEG_INFINITY).unwrap();
    assert!(yaml.contains("-.inf"));
}
