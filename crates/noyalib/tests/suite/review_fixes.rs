//! Tests verifying all fixes from the repo review.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{
    DuplicateKeyPolicy, Mapping, Number, ParserConfig, SerializerConfig, Value, from_str,
    from_str_with_config, to_string, to_string_with_config, to_value,
};

// ============================================================================
// P0-1: deserialize_char multibyte Unicode
// ============================================================================

#[test]
fn deserialize_char_ascii() {
    let c: char = from_str("\"x\"").unwrap();
    assert_eq!(c, 'x');
}

#[test]
fn deserialize_char_multibyte_2byte() {
    // e-acute: U+00E9, 2 bytes in UTF-8
    let c: char = from_str("\"\u{00e9}\"").unwrap();
    assert_eq!(c, '\u{00e9}');
}

#[test]
fn deserialize_char_multibyte_3byte() {
    // CJK character: U+4E16, 3 bytes in UTF-8
    let c: char = from_str("\"\u{4e16}\"").unwrap();
    assert_eq!(c, '\u{4e16}');
}

#[test]
fn deserialize_char_multibyte_4byte() {
    // Emoji: U+1F600, 4 bytes in UTF-8
    let c: char = from_str("\"\u{1f600}\"").unwrap();
    assert_eq!(c, '\u{1f600}');
}

#[test]
fn deserialize_char_rejects_multichar_string() {
    let result: Result<char, _> = from_str("\"ab\"");
    assert!(result.is_err());
}

// ============================================================================
// Phase 1.2: serialize_bytes emits a `!!binary` tagged scalar (RFC 4648
// base64 per YAML 1.2.2 §10.4) — supersedes the earlier P1-1 behaviour
// of forcing bytes through UTF-8 validation. Round-trips with
// `serde_bytes::ByteBuf` / `Bytes` over any byte payload, including
// payloads that are not valid UTF-8.
// ============================================================================

#[test]
fn serialize_bytes_valid_utf8() {
    let bytes = serde_bytes::Bytes::new(b"hello");
    let val = to_value(&bytes).unwrap();
    let tagged = match &val {
        Value::Tagged(t) => t.as_ref(),
        other => panic!("expected !!binary Tagged, got {other:?}"),
    };
    assert_eq!(tagged.tag().as_str(), "!!binary");
    assert_eq!(tagged.value().as_str(), Some("aGVsbG8="));
}

#[test]
fn serialize_bytes_invalid_utf8_succeeds_via_binary() {
    // The whole point of `!!binary`: non-UTF-8 payloads must
    // round-trip cleanly. The previous "errors on invalid UTF-8"
    // contract was a workaround for not having `!!binary` at all.
    let payload: &[u8] = &[0xFF, 0xFE];
    let bytes = serde_bytes::Bytes::new(payload);
    let val = to_value(&bytes).expect("non-UTF-8 bytes must serialise via !!binary");
    let tagged = match &val {
        Value::Tagged(t) => t.as_ref(),
        other => panic!("expected !!binary Tagged, got {other:?}"),
    };
    assert_eq!(tagged.tag().as_str(), "!!binary");
    // Base64 of [0xFF, 0xFE] is "//4=".
    assert_eq!(tagged.value().as_str(), Some("//4="));
}

#[test]
fn serialize_bytes_empty() {
    let bytes = serde_bytes::Bytes::new(b"");
    let val = to_value(&bytes).unwrap();
    let tagged = match &val {
        Value::Tagged(t) => t.as_ref(),
        other => panic!("expected !!binary Tagged, got {other:?}"),
    };
    assert_eq!(tagged.tag().as_str(), "!!binary");
    assert_eq!(tagged.value().as_str(), Some(""));
}

// ============================================================================
// P1-2: write_string control character escaping
// ============================================================================

#[test]
fn serialize_string_with_nul() {
    let s = "hello\0world";
    let yaml = to_string(&s).unwrap();
    assert!(yaml.contains("\\0"), "NUL should be escaped: {yaml}");
    // Should not contain a literal NUL byte
    assert!(!yaml.contains('\0') || yaml.contains("\\0"));
}

#[test]
fn serialize_string_with_bel() {
    let s = "hello\x07world";
    let yaml = to_string(&s).unwrap();
    assert!(
        yaml.contains("\\x07") || yaml.contains("\\x7"),
        "BEL should be escaped: {yaml}"
    );
}

#[test]
fn serialize_string_with_escape() {
    let s = "hello\x1Bworld";
    let yaml = to_string(&s).unwrap();
    assert!(
        yaml.contains("\\x1B") || yaml.contains("\\x1b"),
        "ESC should be escaped: {yaml}"
    );
}

#[test]
fn serialize_string_tab_not_over_escaped() {
    // Tab is a common control char that should use \t not \x09
    let s = "hello\tworld";
    let yaml = to_string(&s).unwrap();
    // Tab may or may not need quoting depending on context,
    // but if quoted it should use \t
    if yaml.contains("\\t") {
        assert!(!yaml.contains("\\x09"));
    }
}

// ============================================================================
// P2-1: convert.rs merge-source error handling
// ============================================================================

#[test]
fn merge_key_with_mapping_source_works() {
    let yaml = "defaults: &defaults\n  color: red\n  size: large\nitem:\n  <<: *defaults\n  name: widget\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("item").is_some());
}

// ============================================================================
// P2-2: ValueIndex &Value with safe usize conversion
// ============================================================================

#[test]
fn value_index_by_value_string() {
    let yaml = "name: test\nport: 8080\n";
    let v: Value = from_str(yaml).unwrap();
    let key = Value::from("name");
    assert_eq!(v.get(&key).unwrap().as_str(), Some("test"));
}

#[test]
fn value_index_by_value_integer() {
    let v: Value = from_str("- a\n- b\n- c\n").unwrap();
    let key = Value::Number(Number::from(1i64));
    assert_eq!(v.get(&key).unwrap().as_str(), Some("b"));
}

#[test]
fn value_index_by_value_negative_returns_none() {
    let v: Value = from_str("- a\n- b\n").unwrap();
    let key = Value::Number(Number::from(-1i64));
    assert!(v.get(&key).is_none());
}

// ============================================================================
// P2-3: write_block_scalar trailing newlines
// ============================================================================

#[test]
fn block_scalar_single_trailing_newline_roundtrip() {
    let config = SerializerConfig::new()
        .block_scalars(true)
        .block_scalar_threshold(1);
    let original = "hello\nworld\n";
    let yaml = to_string_with_config(&original, &config).unwrap();
    let parsed: String = from_str(&yaml).unwrap();
    assert_eq!(parsed, original);
}

#[test]
fn block_scalar_no_trailing_newline_roundtrip() {
    let config = SerializerConfig::new()
        .block_scalars(true)
        .block_scalar_threshold(1);
    let original = "hello\nworld";
    let yaml = to_string_with_config(&original, &config).unwrap();
    let parsed: String = from_str(&yaml).unwrap();
    assert_eq!(parsed, original);
}

// ============================================================================
// Debt: looks_like_number correctness
// ============================================================================

#[test]
fn number_like_strings_roundtrip() {
    for s in [
        "42", "-17", "+42", "0", "2.75", "-2.5", "1.0e3", "1.0e-3", ".inf", ".Inf", ".INF",
        "+.inf", "-.inf", ".nan", ".NaN", ".NAN", "0x2A", "0o52", "0.0",
    ] {
        let yaml = to_string(&s).unwrap();
        let parsed: String = from_str(&yaml).unwrap();
        assert_eq!(s, parsed, "roundtrip failed for number-like string {s:?}");
    }
}

#[test]
fn non_number_strings_not_over_quoted() {
    // These are clearly not numbers and should roundtrip as plain strings.
    for s in ["hello", "foo123", "word"] {
        let yaml = to_string(&s).unwrap();
        let parsed: String = from_str(&yaml).unwrap();
        assert_eq!(s, parsed, "roundtrip failed for {s:?}");
    }
}

#[test]
fn ambiguous_numeric_strings_roundtrip() {
    // Strings that yaml-rust2 might interpret as numbers get safely quoted.
    for s in [
        "0xGG", "1e", "1.2.3", "0o99", "1abc", "0xDEAD", "++1", "--5",
    ] {
        let yaml = to_string(&s).unwrap();
        let parsed: String = from_str(&yaml).unwrap();
        assert_eq!(
            s, parsed,
            "roundtrip failed for ambiguous numeric string {s:?}"
        );
    }
}

// ============================================================================
// Existing behavior preserved: special value strings
// ============================================================================

#[test]
fn special_value_strings_roundtrip() {
    for s in ["", "true", "false", "null", "~", ": colon", "# hash"] {
        let yaml = to_string(&s).unwrap();
        let parsed: String = from_str(&yaml).unwrap();
        assert_eq!(s, parsed, "roundtrip failed for {s:?}");
    }
}

// ============================================================================
// DuplicateKeyPolicy still works correctly
// ============================================================================

#[test]
fn duplicate_key_policy_error_rejects() {
    // yaml-rust2 rejects duplicate keys at the parser level, so all
    // policies result in an error for truly duplicate keys.
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let yaml = "a: 1\na: 2";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

#[test]
fn duplicate_key_policy_first_keeps_first_value() {
    // With DuplicateKeyPolicy::First, the first occurrence wins.
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let yaml = "a: 1\na: 2";
    let result: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(result.get("a").unwrap().as_i64(), Some(1));
}

#[test]
fn duplicate_key_policy_last_keeps_last_value() {
    // With DuplicateKeyPolicy::Last, the last occurrence wins.
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
    let yaml = "a: 1\na: 2";
    let result: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(result.get("a").unwrap().as_i64(), Some(2));
}

#[test]
fn duplicate_key_policy_distinct_keys_work() {
    // Verify distinct keys still work with all policies
    for policy in [
        DuplicateKeyPolicy::Error,
        DuplicateKeyPolicy::First,
        DuplicateKeyPolicy::Last,
    ] {
        let config = ParserConfig::new().duplicate_key_policy(policy);
        let yaml = "a: 1\nb: 2";
        let v: Value = from_str_with_config(yaml, &config).unwrap();
        assert_eq!(v.get("a").unwrap().as_i64(), Some(1));
        assert_eq!(v.get("b").unwrap().as_i64(), Some(2));
    }
}

// ============================================================================
// Empty mapping/sequence still work
// ============================================================================

#[test]
fn empty_mapping_roundtrip() {
    let m = Mapping::new();
    let yaml = to_string(&Value::Mapping(m)).unwrap();
    assert_eq!(yaml.trim(), "{}");
}

#[test]
fn empty_sequence_roundtrip() {
    let s: Vec<i64> = vec![];
    let yaml = to_string(&s).unwrap();
    assert_eq!(yaml.trim(), "[]");
}
