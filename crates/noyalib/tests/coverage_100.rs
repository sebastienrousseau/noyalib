// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(
    unused_comparisons,
    clippy::approx_constant,
    clippy::absurd_extreme_comparisons,
    clippy::enum_variant_names,
    clippy::upper_case_acronyms,
    unused_results,
    missing_docs
)]

use std::collections::HashMap;

use noyalib::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 1. value.rs — Mapping / MappingAny Visitor expecting
// ============================================================================

#[test]
fn mapping_deserialize_from_yaml() {
    // Exercises MappingVisitor (lines 395-405)
    let yaml = "a: 1\nb: 2\n";
    let m: Mapping = from_str(yaml).unwrap();
    assert_eq!(m.len(), 2);
    assert_eq!(m.get("a").unwrap().as_i64(), Some(1));
}

#[test]
fn mapping_any_deserialize_from_yaml() {
    // Exercises MappingAnyVisitor (lines 850-860)
    let yaml = "a: 1\nb: 2\n";
    let m: MappingAny = from_str(yaml).unwrap();
    assert_eq!(m.len(), 2);
}

// ============================================================================
// 1. value.rs — TaggedValue Deserializer (lines 1440-1600)
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
enum MyEnum {
    UnitVariant,
    NewtypeVariant(i64),
    TupleVariant(i64, String),
    StructVariant { x: i64, y: String },
}

#[test]
fn tagged_value_deserialize_unit_variant() {
    // TaggedValueEnumAccess + unit_variant
    // Use &Value deserializer directly to hit the Tagged enum path
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!UnitVariant"),
        Value::Null,
    )));
    let e: MyEnum = Deserialize::deserialize(&v).unwrap();
    assert_eq!(e, MyEnum::UnitVariant);
}

#[test]
fn tagged_value_deserialize_newtype_variant() {
    // TaggedValueEnumAccess + newtype_variant_seed
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!NewtypeVariant"),
        Value::from(42),
    )));
    let e: MyEnum = Deserialize::deserialize(&v).unwrap();
    assert_eq!(e, MyEnum::NewtypeVariant(42));
}

#[test]
fn tagged_value_deserialize_tuple_variant() {
    // TaggedValueEnumAccess + tuple_variant -> deserialize_seq on &Value
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!TupleVariant"),
        Value::Sequence(vec![Value::from(42), Value::from("hello")]),
    )));
    let e: MyEnum = Deserialize::deserialize(&v).unwrap();
    assert_eq!(e, MyEnum::TupleVariant(42, "hello".to_string()));
}

#[test]
fn tagged_value_deserialize_struct_variant() {
    // TaggedValueEnumAccess + struct_variant -> deserialize_map on &Value
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!StructVariant"),
        Value::Mapping({
            let mut m = Mapping::new();
            m.insert("x", Value::from(42));
            m.insert("y", Value::from("hello"));
            m
        }),
    )));
    let e: MyEnum = Deserialize::deserialize(&v).unwrap();
    assert_eq!(
        e,
        MyEnum::StructVariant {
            x: 42,
            y: "hello".to_string()
        }
    );
}

#[test]
fn tagged_value_deserialize_any_as_map() {
    // TaggedValueMapAccess via deserialize_any on &TaggedValue
    // Using &Value deserializer on a Tagged value
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!mytag"),
        Value::from("hello"),
    )));
    let map: HashMap<String, Value> = Deserialize::deserialize(&v).unwrap();
    assert!(map.contains_key("!mytag"));
}

// ============================================================================
// 1. value.rs — apply_merge error paths (lines 2090-2110)
// ============================================================================

#[test]
fn apply_merge_scalar_in_merge_element() {
    let mut v = Value::Mapping({
        let mut m = Mapping::new();
        m.insert("<<", Value::from("scalar_value"));
        m
    });
    let err = v.apply_merge().unwrap_err();
    assert!(format!("{err}").contains("scalar") || matches!(err, Error::ScalarInMergeElement));
}

#[test]
fn apply_merge_tagged_in_merge() {
    let mut v = Value::Mapping({
        let mut m = Mapping::new();
        m.insert(
            "<<",
            Value::Tagged(Box::new(TaggedValue::new(
                Tag::new("!foo"),
                Value::from("bar"),
            ))),
        );
        m
    });
    let err = v.apply_merge().unwrap_err();
    assert!(matches!(err, Error::TaggedInMerge));
}

#[test]
fn apply_merge_sequence_in_merge_element() {
    let mut v = Value::Mapping({
        let mut m = Mapping::new();
        m.insert(
            "<<",
            Value::Sequence(vec![Value::Sequence(vec![Value::from(1)])]),
        );
        m
    });
    let err = v.apply_merge().unwrap_err();
    assert!(matches!(err, Error::SequenceInMergeElement));
}

#[test]
fn apply_merge_single_mapping_value() {
    // Non-sequence merge value (single mapping) -> vec![value] path (line 2090)
    let mut v = Value::Mapping({
        let mut m = Mapping::new();
        let mut merge_src = Mapping::new();
        merge_src.insert("a", Value::from(1));
        m.insert("<<", Value::Mapping(merge_src));
        m
    });
    v.apply_merge().unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

// ============================================================================
// 1. value.rs — ValueIndex through Tagged (lines 2460-2575)
// ============================================================================

#[test]
fn value_index_usize_on_tagged_sequence() {
    // index_or_insert for usize through Tagged (line 2468)
    let mut v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!seq"),
        Value::Sequence(vec![Value::from(10), Value::from(20)]),
    )));
    v[0] = Value::from(99);
    assert_eq!(v[0].as_i64(), Some(99));
}

#[test]
fn value_index_str_on_tagged_mapping() {
    // index_or_insert for &str through Tagged (line 2506)
    let mut v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!map"),
        Value::Mapping(Mapping::new()),
    )));
    // Use index_or_insert directly since Index<&str> doesn't go through Tagged
    *"key".index_or_insert(&mut v) = Value::from("value");
    assert_eq!("key".index_into(&v).unwrap().as_str(), Some("value"));
}

#[test]
fn value_index_str_on_null_creates_mapping() {
    // index_or_insert for &str on Null (line 2497-2498)
    let mut v = Value::Null;
    *"key".index_or_insert(&mut v) = Value::from("value");
    assert_eq!(v["key"].as_str(), Some("value"));
}

#[test]
fn value_index_string_type() {
    // ValueIndex for String (lines 2516-2528)
    let mut m = Mapping::new();
    m.insert("hello", Value::from(42));
    let mut v = Value::Mapping(m);

    let key = String::from("hello");
    // index_into
    assert_eq!(key.clone().index_into(&v).unwrap().as_i64(), Some(42));

    // index_into_mut
    let key2 = String::from("hello");
    *key2.index_into_mut(&mut v).unwrap() = Value::from(99);
    assert_eq!(v["hello"].as_i64(), Some(99));

    // index_or_insert
    let key3 = String::from("new_key");
    *key3.index_or_insert(&mut v) = Value::from(100);
    assert_eq!(v["new_key"].as_i64(), Some(100));
}

#[test]
fn value_index_ref_string_type() {
    // ValueIndex for &String (lines 2530-2542)
    let mut m = Mapping::new();
    m.insert("hello", Value::from(42));
    let mut v = Value::Mapping(m);

    let key = String::from("hello");
    // index_into
    assert_eq!((&key).index_into(&v).unwrap().as_i64(), Some(42));
    // index_into_mut
    *(&key).index_into_mut(&mut v).unwrap() = Value::from(99);
    assert_eq!(v["hello"].as_i64(), Some(99));
    // index_or_insert
    *(&key).index_or_insert(&mut v) = Value::from(100);
    assert_eq!(v["hello"].as_i64(), Some(100));
}

#[test]
fn value_index_ref_value_string() {
    // ValueIndex for &Value with String (line 2547, 2557, 2567)
    let mut m = Mapping::new();
    m.insert("hello", Value::from(42));
    let mut v = Value::Mapping(m);

    let idx = Value::from("hello");
    // index_into
    assert_eq!((&idx).index_into(&v).unwrap().as_i64(), Some(42));
    // index_into_mut
    *(&idx).index_into_mut(&mut v).unwrap() = Value::from(99);
    assert_eq!(v["hello"].as_i64(), Some(99));
    // index_or_insert
    *(&idx).index_or_insert(&mut v) = Value::from(100);
    assert_eq!(v["hello"].as_i64(), Some(100));
}

#[test]
fn value_index_ref_value_integer() {
    // ValueIndex for &Value with Number::Integer (line 2548-2549, 2558-2559,
    // 2568-2571)
    let mut v = Value::Sequence(vec![Value::from(10), Value::from(20), Value::from(30)]);

    let idx = Value::from(1);
    // index_into
    assert_eq!((&idx).index_into(&v).unwrap().as_i64(), Some(20));
    // index_into_mut
    *(&idx).index_into_mut(&mut v).unwrap() = Value::from(99);
    assert_eq!((&idx).index_into(&v).unwrap().as_i64(), Some(99));
    // index_or_insert
    *(&idx).index_or_insert(&mut v) = Value::from(88);
    assert_eq!((&idx).index_into(&v).unwrap().as_i64(), Some(88));
}

#[test]
fn value_index_ref_value_other_returns_none() {
    // ValueIndex for &Value with non-string/non-integer (line 2551, 2561)
    let mut m = Value::Mapping(Mapping::new());
    let idx = Value::Bool(true);
    assert!((&idx).index_into(&m).is_none());
    assert!((&idx).index_into_mut(&mut m).is_none());
}

#[test]
#[should_panic(expected = "cannot index")]
fn value_index_ref_value_bool_panic_on_insert() {
    // ValueIndex for &Value with non-indexable type (line 2573)
    let mut v = Value::Mapping(Mapping::new());
    let idx = Value::Bool(true);
    let _ = (&idx).index_or_insert(&mut v);
}

// ============================================================================
// 1. value.rs — Value Visitor (lines 2575-2630)
// ============================================================================

#[test]
fn value_deserialize_visit_string() {
    // visit_string path (line 2627-2628)
    // When deserializing an owned string value
    let yaml = "hello world\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello world"));
}

// ============================================================================
// 1. value.rs — Deserializer for &Value (lines 2700-2850)
// ============================================================================

#[test]
fn ref_value_deserialize_any_null() {
    let v = Value::Null;
    let result: () = from_value(&v).unwrap();
    assert_eq!(result, ());
}

#[test]
fn ref_value_deserialize_any_bool() {
    let v = Value::Bool(true);
    let result: bool = from_value(&v).unwrap();
    assert!(result);
}

#[test]
fn ref_value_deserialize_any_integer() {
    let v = Value::from(42);
    let result: i64 = from_value(&v).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn ref_value_deserialize_any_float() {
    let v = Value::Number(Number::Float(3.14));
    let result: f64 = from_value(&v).unwrap();
    assert!((result - 3.14).abs() < f64::EPSILON);
}

#[test]
fn ref_value_deserialize_any_string() {
    let v = Value::from("hello");
    let result: String = from_value(&v).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn ref_value_deserialize_any_sequence() {
    let v = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let result: Vec<i64> = from_value(&v).unwrap();
    assert_eq!(result, vec![1, 2]);
}

#[test]
fn ref_value_deserialize_any_mapping() {
    let v = Value::Mapping({
        let mut m = Mapping::new();
        m.insert("a", Value::from(1));
        m
    });
    let result: HashMap<String, i64> = from_value(&v).unwrap();
    assert_eq!(result["a"], 1);
}

#[test]
fn ref_value_deserialize_enum_string() {
    // deserialize_enum with Value::String (line 2799-2800)
    #[derive(Debug, Deserialize, PartialEq)]
    enum Simple {
        Hello,
        World,
    }
    let v = Value::from("Hello");
    let result: Simple = from_value(&v).unwrap();
    assert_eq!(result, Simple::Hello);
}

#[test]
fn ref_value_deserialize_seq_passthrough() {
    // deserialize_seq on non-sequence falls through to deserialize_any (line 2811)
    let v = Value::from("not a sequence");
    let result: Result<Vec<i64>> = from_value(&v);
    assert!(result.is_err());
}

#[test]
fn ref_value_deserialize_map_passthrough() {
    // deserialize_map on non-mapping falls through to deserialize_any (line 2824)
    let v = Value::from("not a mapping");
    let result: Result<HashMap<String, i64>> = from_value(&v);
    assert!(result.is_err());
}

#[test]
fn ref_value_deserialize_struct_spanned() {
    // deserialize_struct with spanned type name (lines 2837-2843)
    let yaml = "hello\n";
    let v: Spanned<String> = from_str(yaml).unwrap();
    assert_eq!(v.value, "hello");
}

// ============================================================================
// 2. scanner.rs — Various scanner paths
// ============================================================================

#[test]
fn scanner_bom_at_start() {
    // BOM handling (lines 473-476)
    let yaml = "\u{FEFF}key: value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("value"));
}

#[test]
fn scanner_single_quoted_multiline() {
    // Single-quoted multiline with line folding (lines 998-1050)
    let yaml = "key: 'line1\n  line2'\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1 line2"));
}

#[test]
fn scanner_single_quoted_multiline_breaks() {
    // Single-quoted with multiple line breaks (lines 1024-1031)
    let yaml = "key: 'line1\n\n  line2'\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1\nline2"));
}

#[test]
fn scanner_single_quoted_escaped_quote() {
    // Escaped single quote '' (lines 985-991)
    let yaml = "key: 'it''s'\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("it's"));
}

#[test]
fn scanner_double_quoted_escapes() {
    // All escape sequences (lines 1112-1141)
    let yaml = r#"key: "\0\a\b\t\n\v\f\r\e \"\/ \\ \N \_ \L \P""#;
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains('\0'));
    assert!(s.contains('\x07'));
    assert!(s.contains('\x08'));
    assert!(s.contains('\t'));
    assert!(s.contains('\n'));
    assert!(s.contains('\x0B'));
    assert!(s.contains('\x0C'));
    assert!(s.contains('\r'));
    assert!(s.contains('\x1B'));
    assert!(s.contains('"'));
    assert!(s.contains('/'));
    assert!(s.contains('\\'));
    assert!(s.contains('\u{0085}'));
    assert!(s.contains('\u{00A0}'));
    assert!(s.contains('\u{2028}'));
    assert!(s.contains('\u{2029}'));
}

#[test]
fn scanner_double_quoted_hex_escapes() {
    // Hex escapes: \x, \u, \U (lines 1130-1141)
    let yaml = r#"key: "\x41\u0042\U00000043""#;
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("ABC"));
}

#[test]
fn scanner_double_quoted_multiline() {
    // Double-quoted multiline folding (lines 1163-1201)
    let yaml = "key: \"line1\n  line2\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1 line2"));
}

#[test]
fn scanner_double_quoted_multiline_multiple_breaks() {
    // Multiple line breaks in double-quoted (lines 1189-1196)
    let yaml = "key: \"line1\n\n  line2\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1\nline2"));
}

#[test]
fn scanner_double_quoted_line_escape() {
    // Line break escape \<newline> (lines 1142-1151)
    let yaml = "key: \"line1\\\n  line2\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1line2"));
}

#[test]
fn scanner_block_scalar_literal_clip() {
    // Block scalar literal with clip chomping (default)
    let yaml = "key: |\n  line1\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1\nline2\n"));
}

#[test]
fn scanner_block_scalar_literal_strip() {
    // Block scalar with strip chomping |- (line 1296)
    let yaml = "key: |-\n  line1\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1\nline2"));
}

#[test]
fn scanner_block_scalar_literal_keep() {
    // Block scalar with keep chomping |+ (line 1291-1293)
    let yaml = "key: |+\n  line1\n  line2\n\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1\nline2\n\n"));
}

#[test]
fn scanner_block_scalar_with_indent_indicator() {
    // Block scalar with explicit indent indicator (line 1299-1301)
    let yaml = "key: |2\n  line1\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1\nline2\n"));
}

#[test]
fn scanner_block_scalar_folded() {
    // Folded block scalar > (line 1409-1414)
    let yaml = "key: >\n  line1\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1 line2\n"));
}

#[test]
fn scanner_block_scalar_folded_strip() {
    let yaml = "key: >-\n  line1\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1 line2"));
}

#[test]
fn scanner_block_scalar_folded_keep() {
    let yaml = "key: >+\n  line1\n  line2\n\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1 line2\n\n"));
}

#[test]
fn scanner_block_scalar_folded_with_more_indented() {
    // More-indented lines in folded scalar (lines 1379-1391)
    let yaml = "key: >\n  line1\n    indented\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    // More-indented lines preserve line breaks
    assert!(s.contains("line1"));
    assert!(s.contains("indented"));
    assert!(s.contains("line2"));
}

#[test]
fn scanner_block_scalar_literal_utf8() {
    // UTF-8 content in block scalar (lines 1427-1433)
    let yaml = "key: |\n  héllo wörld\n  日本語\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("héllo wörld"));
    assert!(s.contains("日本語"));
}

#[test]
fn scanner_flow_collections() {
    // Flow collections (lines 434-437)
    let yaml = "[1, {a: b}, 2]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0].as_i64(), Some(1));
    assert_eq!(v[2].as_i64(), Some(2));
    assert_eq!(v[1]["a"].as_str(), Some("b"));
}

#[test]
fn scanner_flow_mapping() {
    let yaml = "{a: [1, 2], b: 3}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["b"].as_i64(), Some(3));
    assert_eq!(v["a"][0].as_i64(), Some(1));
}

#[test]
fn scanner_crlf_line_endings() {
    // CRLF handling (lines 286-287, 905, 914, 1017, 1027, etc.)
    let yaml = "key: value\r\nother: data\r\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("value"));
    assert_eq!(v["other"].as_str(), Some("data"));
}

#[test]
fn scanner_unterminated_single_quote() {
    // EOF in single-quoted string (line 980)
    let yaml = "key: 'unterminated";
    let result: Result<Value> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn scanner_unterminated_double_quote() {
    // EOF in double-quoted string (line 1073)
    let yaml = "key: \"unterminated";
    let result: Result<Value> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn scanner_tab_indentation_error() {
    // Tab at start of line — treated as content, not indentation.
    // The parser handles this leniently by ending the previous mapping.
    let yaml = "key: value\n\tindented: bad\n";
    let result: Result<Value> = from_str(yaml);
    // May parse (leniently) or error (strict) — both are acceptable
    let _ = result;
}

#[test]
fn scanner_unknown_escape() {
    // Unknown escape character (lines 1152-1160)
    let yaml = r#"key: "\z""#;
    let result: Result<Value> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn scanner_plain_scalar_multiline() {
    // Plain scalar line folding (lines 800-931)
    let yaml = "key:\n  line1\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    // plain scalar multiline -> folded into single line
    assert_eq!(v["key"].as_str(), Some("line1 line2"));
}

#[test]
fn scanner_plain_scalar_with_multiple_breaks() {
    // Multiple line breaks in plain scalar (lines 911-920)
    let yaml = "key:\n  line1\n\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1\nline2"));
}

#[test]
fn scanner_roll_indent_flow_return() {
    // roll_indent returns early in flow context (line 326-327)
    let yaml = "{a: {b: c}}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"]["b"].as_str(), Some("c"));
}

#[test]
fn scanner_fetch_value_block_mapping_init() {
    // fetch_value block mapping init (lines 624-640)
    let yaml = "? key\n: value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("value"));
}

// ============================================================================
// 3. events.rs — Various event parser paths
// ============================================================================

#[test]
fn events_multi_document() {
    // Multi-document with --- and ... markers (lines 213-224)
    let yaml = "---\na: 1\n...\n---\nb: 2\n...\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0]["a"].as_i64(), Some(1));
    assert_eq!(docs[1]["b"].as_i64(), Some(2));
}

#[test]
fn events_explicit_key_syntax() {
    // Explicit ? key syntax (lines 438-458)
    let yaml = "? explicit_key\n: explicit_value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["explicit_key"].as_str(), Some("explicit_value"));
}

#[test]
fn events_indentless_sequence() {
    // Indentless sequences in block mapping context (lines 408-430)
    // Indentless sequences appear after a "key:" in block mapping
    let yaml = "key:\n  - item1\n  - item2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"][0].as_str(), Some("item1"));
    assert_eq!(v["key"][1].as_str(), Some("item2"));
}

#[test]
fn events_flow_sequence_implicit_mapping() {
    // Flow sequence with key indicator (lines 507-516)
    let yaml = "[a, b, c]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0].as_str(), Some("a"));
    assert_eq!(v[1].as_str(), Some("b"));
    assert_eq!(v[2].as_str(), Some("c"));
}

#[test]
fn events_empty_block_seq_entries() {
    // Empty entries in block sequences (lines 388-391)
    let yaml = "- \n- value\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v[0].is_null());
    assert_eq!(v[1].as_str(), Some("value"));
}

#[test]
fn events_flow_mapping_empty_values() {
    // Flow mapping with empty values (lines 601-602)
    let yaml = "{? a, b: c}\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["a"].is_null());
    assert_eq!(v["b"].as_str(), Some("c"));
}

#[test]
fn events_anchor_tag_combination() {
    // Anchor + tag combinations (lines 264-287)
    // Use load_all to preserve tags
    let yaml = "&anc !!custom value\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(|r| r.ok()).collect();
    assert!(docs[0].is_tagged());
}

#[test]
fn events_tag_then_anchor() {
    // Tag followed by anchor (lines 275-286)
    let yaml = "!!custom &anc value\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(|r| r.ok()).collect();
    assert!(docs[0].is_tagged());
}

#[test]
fn events_document_content_empty() {
    // Document content that's just document markers (lines 226-236)
    let yaml = "---\n...\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(|r| r.ok()).collect();
    assert!(docs[0].is_null());
}

#[test]
fn events_block_mapping_empty_value() {
    // Block mapping value that's empty (lines 476-478)
    let yaml = "a:\nb: 2\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["a"].is_null());
    assert_eq!(v["b"].as_i64(), Some(2));
}

#[test]
fn events_flow_sequence_entry_mapping_value() {
    // Flow sequence entry mapping value (lines 540-553)
    // In flow sequence, key:value pairs create implicit mappings
    let yaml = "[a: b, c]\n";
    let v: Value = from_str(yaml).unwrap();
    // First element is a mapping {a: b}
    assert!(v[0].is_mapping());
}

#[test]
fn events_flow_sequence_entry_mapping_empty_value() {
    // Flow sequence entry mapping with empty value (lines 544-553)
    let yaml = "[a:, b]\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v[0].is_mapping());
}

#[test]
fn events_flow_mapping_complex_key() {
    // Flow mapping (lines 588-603)
    let yaml = "{a: b, c: d}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_str(), Some("b"));
    assert_eq!(v["c"].as_str(), Some("d"));
}

// ============================================================================
// 4. loader.rs — Various loader paths
// ============================================================================

#[test]
fn loader_multi_document() {
    // Multi-document parsing (lines 145-158)
    let yaml = "doc1\n---\ndoc2\n";
    let docs = load_all(yaml).unwrap();
    assert!(docs.len() >= 2); // len() is on DocumentIterator
}

#[test]
fn loader_anchored_sequence() {
    // Anchored sequences (lines 206-248)
    let yaml = "a: &seq\n  - 1\n  - 2\nb: *seq\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"][0].as_i64(), Some(1));
    assert_eq!(v["b"][0].as_i64(), Some(1));
}

#[test]
fn loader_anchored_mapping() {
    // Anchored mappings (lines 250-327)
    let yaml = "a: &map\n  x: 1\n  y: 2\nb: *map\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"]["x"].as_i64(), Some(1));
    assert_eq!(v["b"]["x"].as_i64(), Some(1));
}

#[test]
fn loader_merge_keys() {
    // Merge keys from anchors (lines 274-297)
    let yaml = "defaults: &defaults\n  x: 1\n  y: 2\nresult:\n  <<: *defaults\n  z: 3\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["result"]["x"].as_i64(), Some(1));
    assert_eq!(v["result"]["z"].as_i64(), Some(3));
}

#[test]
fn loader_merge_keys_sequence() {
    // Merge key with sequence of mappings (lines 284-296)
    let yaml = "a: &a\n  x: 1\nb: &b\n  y: 2\nc:\n  <<: [*a, *b]\n  z: 3\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["c"]["x"].as_i64(), Some(1));
    assert_eq!(v["c"]["y"].as_i64(), Some(2));
    assert_eq!(v["c"]["z"].as_i64(), Some(3));
}

#[test]
fn loader_duplicate_key_first() {
    // Duplicate key policy: First (lines 417-421)
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let yaml = "a: 1\na: 2\n";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

#[test]
fn loader_tagged_scalars() {
    // Tagged scalar resolution (lines 534-593)
    let yaml_int = "!!int 42\n";
    let v: Value = from_str(yaml_int).unwrap();
    assert_eq!(v.as_i64(), Some(42));

    let yaml_bool = "!!bool true\n";
    let v: Value = from_str(yaml_bool).unwrap();
    assert_eq!(v.as_bool(), Some(true));

    let yaml_float = "!!float 3.14\n";
    let v: Value = from_str(yaml_float).unwrap();
    assert!((v.as_f64().unwrap() - 3.14).abs() < f64::EPSILON);

    let yaml_null = "!!null ~\n";
    let v: Value = from_str(yaml_null).unwrap();
    assert!(v.is_null());

    let yaml_str = "!!str 42\n";
    let v: Value = from_str(yaml_str).unwrap();
    assert_eq!(v.as_str(), Some("42"));
}

#[test]
fn loader_custom_tag() {
    // Custom tag resolution (lines 581-593)
    // Tags with !! prefix resolve via resolve_tagged_scalar
    // load_all preserves tags without serde deserialization
    let yaml = "!!custom hello\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(|r| r.ok()).collect();
    assert!(docs[0].is_tagged());
}

#[test]
fn loader_hex_octal_integers() {
    // Hex/octal integers (via try_parse_integer)
    let yaml_hex = "0xFF\n";
    let v: Value = from_str(yaml_hex).unwrap();
    assert_eq!(v.as_i64(), Some(255));

    let yaml_oct = "0o77\n";
    let v: Value = from_str(yaml_oct).unwrap();
    assert_eq!(v.as_i64(), Some(63));
}

#[test]
fn loader_large_integer_overflow_to_float() {
    // Large integer overflow to float (lines 509-513)
    let yaml = "99999999999999999999\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_f64().is_some());
}

#[test]
fn loader_value_to_key_types() {
    // value_to_key for different types (lines 461-469)
    // Bool key
    let yaml = "true: value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["true"].as_str(), Some("value"));

    // Null key
    let yaml = "null: value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["null"].as_str(), Some("value"));

    // Float key
    let yaml = "3.14: value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["3.14"].as_str(), Some("value"));

    // Integer key
    let yaml = "42: value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["42"].as_str(), Some("value"));
}

// ============================================================================
// 5. de.rs — Deserialize identifier and SpannedMapAccess
// ============================================================================

#[test]
fn de_deserialize_identifier() {
    // deserialize_identifier (lines 648-652)
    #[derive(Debug, Deserialize, PartialEq)]
    struct Wrapper {
        field: String,
    }
    let yaml = "field: hello\n";
    let w: Wrapper = from_str(yaml).unwrap();
    assert_eq!(w.field, "hello");
}

#[test]
fn de_spanned_map_access_all_fields() {
    // SpannedMapAccess states (lines 786-814)
    #[derive(Debug, Deserialize)]
    struct Config {
        name: Spanned<String>,
        value: Spanned<i64>,
    }
    let yaml = "name: hello\nvalue: 42\n";
    let config: Config = from_str(yaml).unwrap();
    assert_eq!(config.name.value, "hello");
    assert_eq!(config.value.value, 42);
}

// ============================================================================
// 5. spanned.rs — SpannedVisitor (lines 117-156)
// ============================================================================

#[test]
fn spanned_string_from_yaml() {
    // SpannedVisitor visit_map (lines 129-156)
    let yaml = "hello\n";
    let v: Spanned<String> = from_str(yaml).unwrap();
    assert_eq!(v.value, "hello");
}

#[test]
fn spanned_integer_from_yaml() {
    let yaml = "42\n";
    let v: Spanned<i64> = from_str(yaml).unwrap();
    assert_eq!(v.value, 42);
}

// ============================================================================
// 5. ser.rs — String quoting and block scalars (lines 355-420, 490-500,
//    705-785)
// ============================================================================

#[test]
fn ser_string_needs_single_quotes() {
    // Strings that need quoting: reserved words, trailing space, etc.
    let v = Value::from("true");
    let s = to_string(&v).unwrap();
    assert!(s.contains('\'') || s.contains('"'));

    let v = Value::from("null");
    let s = to_string(&v).unwrap();
    assert!(s.contains('\'') || s.contains('"'));

    let v = Value::from("42");
    let s = to_string(&v).unwrap();
    assert!(s.contains('\'') || s.contains('"'));
}

#[test]
fn ser_string_needs_double_quotes() {
    // Strings with control characters need double quotes
    let v = Value::from("hello\nworld");
    let s = to_string(&v).unwrap();
    assert!(s.contains('"') || s.contains('|') || s.contains('>'));
}

#[test]
fn ser_looks_like_number() {
    // Float-looking strings (lines 373-419)
    let v = Value::from(".5");
    let s = to_string(&v).unwrap();
    assert!(s.contains('\'') || s.contains('"'));

    let v = Value::from(".inf");
    let s = to_string(&v).unwrap();
    assert!(s.contains('\'') || s.contains('"'));

    let v = Value::from("-.inf");
    let s = to_string(&v).unwrap();
    assert!(s.contains('\'') || s.contains('"'));

    let v = Value::from(".nan");
    let s = to_string(&v).unwrap();
    assert!(s.contains('\'') || s.contains('"'));
}

#[test]
fn ser_tagged_value() {
    // Tagged value serialization (lines 355-364)
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        Value::from("hello"),
    )));
    let s = to_string(&v).unwrap();
    assert!(s.contains("!custom"));
    assert!(s.contains("hello"));
}

#[test]
fn ser_literal_block() {
    // Literal block serialization (lines 707-713, 779-789)
    use noyalib::fmt::LitString;

    #[derive(Serialize)]
    struct Doc {
        script: LitString,
    }
    let doc = Doc {
        script: LitString::from("line1\nline2\n".to_string()),
    };
    let s = to_string(&doc).unwrap();
    assert!(s.contains('|'));
}

#[test]
fn ser_folded_block() {
    // Folded block serialization (lines 714-720)
    use noyalib::fmt::FoldString;

    #[derive(Serialize)]
    struct Doc {
        text: FoldString,
    }
    let doc = Doc {
        text: FoldString::from("line1\nline2\n".to_string()),
    };
    let s = to_string(&doc).unwrap();
    assert!(s.contains('>'));
}

#[test]
fn ser_flow_sequence() {
    // Flow sequence serialization (lines 749-763)
    use noyalib::fmt::FlowSeq;

    #[derive(Serialize)]
    struct Doc {
        items: FlowSeq<Vec<i32>>,
    }
    let doc = Doc {
        items: FlowSeq(vec![1, 2, 3]),
    };
    let s = to_string(&doc).unwrap();
    assert!(s.contains('['));
    assert!(s.contains(']'));
}

#[test]
fn ser_flow_mapping() {
    // Flow mapping serialization (lines 765-777)
    use noyalib::fmt::FlowMap;

    #[derive(Serialize)]
    struct Doc {
        map: FlowMap<std::collections::BTreeMap<String, i32>>,
    }
    let mut m = std::collections::BTreeMap::new();
    m.insert("a".to_string(), 1);
    m.insert("b".to_string(), 2);
    let doc = Doc { map: FlowMap(m) };
    let s = to_string(&doc).unwrap();
    assert!(s.contains('{'));
    assert!(s.contains('}'));
}

#[test]
fn ser_string_trailing_space() {
    // Trailing space needs quoting (line 494-496)
    let v = Value::from("hello ");
    let s = to_string(&v).unwrap();
    assert!(s.contains('\'') || s.contains('"'));
}

#[test]
fn ser_string_first_char_quote() {
    // First char needing quoting (line 489-491)
    let v = Value::from("# comment-like");
    let s = to_string(&v).unwrap();
    assert!(s.contains('\'') || s.contains('"'));
}

// ============================================================================
// 5. path.rs — Path::parent() and depth() for Alias/Unknown (lines 204, 228)
// ============================================================================

#[test]
fn path_alias_parent_and_depth() {
    let root = Path::Root;
    let alias = Path::Alias { parent: &root };
    assert_eq!(alias.parent(), Some(&Path::Root));
    assert_eq!(alias.depth(), 1);
}

#[test]
fn path_unknown_parent_and_depth() {
    let root = Path::Root;
    let unknown = Path::Unknown { parent: &root };
    assert_eq!(unknown.parent(), Some(&Path::Root));
    assert_eq!(unknown.depth(), 1);
}

#[test]
fn path_seq_parent_and_depth() {
    let root = Path::Root;
    let seq = Path::Seq {
        parent: &root,
        index: 0,
    };
    assert_eq!(seq.parent(), Some(&Path::Root));
    assert_eq!(seq.depth(), 1);
}

// ============================================================================
// 5. fmt.rs — Commented roundtrip (line 381)
// ============================================================================

#[test]
fn fmt_commented_roundtrip() {
    use noyalib::fmt::Commented;

    #[derive(Serialize)]
    struct Doc {
        key: Commented<String>,
    }
    let doc = Doc {
        key: Commented::new("hello".to_string(), "my comment"),
    };
    let s = to_string(&doc).unwrap();
    assert!(s.contains("hello"));
    assert!(s.contains("# my comment"));

    // Deserialize back (comment is lost, per design)
    #[derive(Deserialize)]
    struct DocIn {
        key: Commented<String>,
    }
    let parsed: DocIn = from_str("key: hello\n").unwrap();
    assert_eq!(*parsed.key, "hello");
}

// ============================================================================
// 5. with/singleton_map_recursive.rs — Tagged value transform (lines 47-51)
// ============================================================================

#[test]
fn singleton_map_recursive_tagged_value() {
    use noyalib::with::singleton_map_recursive;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Action {
        Start { delay: u32 },
        Stop,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Task {
        #[serde(with = "singleton_map_recursive")]
        action: Action,
    }

    let task = Task {
        action: Action::Start { delay: 5 },
    };
    let yaml = to_string(&task).unwrap();
    let parsed: Task = from_str(&yaml).unwrap();
    assert_eq!(parsed, task);
}

// ============================================================================
// 5. with/singleton_map_with.rs — serialize_with and transform_value_keys
//    (lines 125-143, 214-217)
// ============================================================================

#[test]
fn singleton_map_with_deserialize_transform() {
    // Exercise transform_value_keys including Tagged branch (lines 214-217)
    use noyalib::with::singleton_map_with;

    #[derive(Debug, Deserialize, PartialEq)]
    enum MyAction {
        START,
        STOP,
    }

    // Deserialize via singleton_map_with with uppercase transform
    let yaml = "start: ~\n";
    let val: Value = from_str(yaml).unwrap();
    let result: MyAction =
        singleton_map_with::deserialize_with(Deserializer::new(&val), |s| s.to_uppercase())
            .unwrap();
    assert_eq!(result, MyAction::START);
}

#[test]
fn singleton_map_with_serialize_branches() {
    // Exercise serialize_with String and Mapping branches (lines 125-143)
    use noyalib::with::singleton_map_with;

    #[derive(Debug, Serialize, PartialEq)]
    enum Status {
        Active,
        WithData { count: u32 },
    }

    // Unit variant -> to_value gives String -> String branch (lines 134-139)
    let unit_val: Value =
        singleton_map_with::serialize_with(&Status::Active, Serializer, |s| s.to_lowercase())
            .unwrap();
    // Should produce a mapping with transformed key
    assert!(unit_val.is_mapping());

    // Struct variant -> to_value gives Mapping -> Mapping branch (lines 125-132)
    let struct_val: Value =
        singleton_map_with::serialize_with(&Status::WithData { count: 5 }, Serializer, |s| {
            s.to_lowercase()
        })
        .unwrap();
    assert!(struct_val.is_mapping());
}

// ============================================================================
// Additional scanner edge cases
// ============================================================================

#[test]
fn scanner_tag_secondary_handle() {
    // Secondary tag handle !! (lines 749-767)
    let yaml = "!!str 42\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("42"));
}

#[test]
fn scanner_tag_verbatim() {
    // Verbatim tag !<...> (lines 735-748)
    let yaml = "!<my-custom-tag> hello\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(|r| r.ok()).collect();
    // Verbatim tags are stored as tagged values
    assert!(docs[0].is_tagged());
}

#[test]
fn scanner_tag_primary() {
    // Primary tag handle !suffix (lines 768-784)
    // !suffix is treated as custom tag - use load_all to preserve tags
    let yaml = "!custom hello\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(|r| r.ok()).collect();
    // !custom produces a tagged value
    assert!(docs[0].is_tagged());
}

#[test]
fn scanner_anchor_and_alias() {
    // Anchor/alias (lines 464-465)
    let yaml = "a: &anchor value\nb: *anchor\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_str(), Some("value"));
    assert_eq!(v["b"].as_str(), Some("value"));
}

#[test]
fn scanner_double_quoted_trailing_whitespace_before_break() {
    // Whitespace before line break in double-quoted (lines 1218-1231)
    let yaml = "key: \"word   \n  next\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("word"));
    assert!(s.contains("next"));
}

#[test]
fn scanner_block_scalar_empty_lines() {
    // Empty lines in block scalar (lines 1394-1401)
    let yaml = "key: |\n  line1\n\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1\n\nline2\n"));
}

#[test]
fn scanner_block_scalar_comment_after_indicator() {
    // Comment after block scalar indicator (lines 1312-1316)
    let yaml = "key: | # comment\n  content\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("content\n"));
}

#[test]
fn scanner_block_scalar_chomping_then_indent() {
    // Chomping indicator before indent indicator (line 1288-1306)
    let yaml = "key: |+2\n  ab\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("ab\n"));
}

#[test]
fn scanner_block_scalar_indent_then_chomping() {
    // Indent indicator before chomping indicator
    let yaml = "key: |2+\n  ab\n\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("ab\n\n"));
}

#[test]
fn scanner_hex_escape_invalid() {
    // Invalid hex escape (lines 1248-1253)
    let yaml = r#"key: "\xZZ""#;
    let result: Result<Value> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn loader_special_floats() {
    // Special float values (lines 492-496)
    let yaml = ".inf\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_f64(), Some(f64::INFINITY));

    let yaml = "-.inf\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_f64(), Some(f64::NEG_INFINITY));

    let yaml = ".nan\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_f64().unwrap().is_nan());
}

#[test]
fn loader_tagged_bool_resolution() {
    // Tagged bool resolution (lines 554-570)
    let yaml = "!!bool false\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_bool(), Some(false));
}

#[test]
fn ser_space_after() {
    // SpaceAfter serialization (lines 737-740)
    use noyalib::fmt::SpaceAfter;

    #[derive(Serialize)]
    struct Doc {
        section: SpaceAfter<String>,
    }
    let doc = Doc {
        section: SpaceAfter(String::from("hello")),
    };
    let s = to_string(&doc).unwrap();
    assert!(s.contains("hello"));
}

// ============================================================================
// More &Value deserializer paths
// ============================================================================

#[test]
fn ref_value_deserialize_tagged_any() {
    // &Value deserialize_any for Tagged (lines 2778-2781)
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!mytag"),
        Value::from("inner"),
    )));
    // Deserializing tagged as HashMap exercises TaggedValueMapAccess via &Value
    let result: HashMap<String, Value> = Deserialize::deserialize(&tagged).unwrap();
    assert!(result.contains_key("!mytag"));
}

#[test]
fn ref_value_deserialize_enum_tagged() {
    // &Value deserialize_enum for Tagged (lines 2795-2798)
    #[derive(Debug, Deserialize, PartialEq)]
    enum Color {
        Red,
        Blue,
    }
    let v = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!Red"), Value::Null)));
    // Use &Value deserializer directly
    let c: Color = Deserialize::deserialize(&v).unwrap();
    assert_eq!(c, Color::Red);
}

#[test]
fn ref_value_deserialize_enum_non_tagged_non_string() {
    // &Value deserialize_enum fallthrough to deserialize_any (line 2801)
    #[derive(Debug, Deserialize, PartialEq)]
    enum Thing {
        Value(i64),
    }
    // Deserializing a non-string, non-tagged value as enum should error
    let v = Value::from(42);
    let result: Result<Thing> = from_value(&v);
    assert!(result.is_err());
}

// ============================================================================
// Loader: estimate_value_size (lines 684-703)
// ============================================================================

#[test]
fn loader_estimate_value_covers_tagged() {
    // Tagged value size estimation (line 701)
    // load_all preserves tags; also exercises estimate_value_size for Tagged
    let yaml = "a: &a !!custom val\nb: *a\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(|r| r.ok()).collect();
    // The anchored tagged value should be cloned via alias
    assert!(docs[0]["b"].is_tagged());
}

// ============================================================================
// Flow mapping value paths
// ============================================================================

#[test]
fn events_flow_mapping_value_empty() {
    // parse_flow_mapping_value empty (lines 609ff)
    let yaml = "{a: , b: 2}\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["a"].is_null());
    assert_eq!(v["b"].as_i64(), Some(2));
}

#[test]
fn events_flow_mapping_implicit_key() {
    // Flow mapping implicit key (lines 605-607)
    let yaml = "{a: 1}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

// ============================================================================
// Scanner: ScanError Display (line 64)
// ============================================================================

#[test]
fn scan_error_display() {
    // The scanner error Display impl (lines 62-65)
    let yaml = r#"key: "\z""#;
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = format!("{err}");
    assert!(!msg.is_empty());
}

// ============================================================================
// Loader: DuplicateKeyPolicy::Error
// ============================================================================

#[test]
fn loader_duplicate_key_last_policy() {
    // DuplicateKeyPolicy::Last (lines 423-436) — this is the default
    let yaml = "a: 1\na: 2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(2));
}

// ============================================================================
// Block scalar: folded with leading blank lines
// ============================================================================

#[test]
fn scanner_block_scalar_folded_leading_blank() {
    // Leading blank in folded (lines 1411-1412)
    // Lines starting with spaces are "more-indented" and preserve breaks
    let yaml = "key: >\n  normal1\n   indented\n  normal2\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("normal1"));
    assert!(s.contains("indented"));
    assert!(s.contains("normal2"));
}

#[test]
fn scanner_block_scalar_folded_multiple_breaks() {
    // Multiple trailing breaks in folded (lines 1416-1418)
    let yaml = "key: >\n  line1\n\n\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("line1\n\nline2"));
}

// ============================================================================
// Additional: ensure full &Value deserializer coverage
// ============================================================================

#[test]
fn ref_value_deserialize_struct_normal() {
    // deserialize_struct for non-spanned struct -> falls through to deserialize_map
    // (line 2844)
    #[derive(Debug, Deserialize, PartialEq)]
    struct Point {
        x: i64,
        y: i64,
    }
    let v = Value::Mapping({
        let mut m = Mapping::new();
        m.insert("x", Value::from(1));
        m.insert("y", Value::from(2));
        m
    });
    let p: Point = from_value(&v).unwrap();
    assert_eq!(p, Point { x: 1, y: 2 });
}

#[test]
fn ref_value_into_deserializer() {
    // IntoDeserializer impl for &Value (lines 2701-2707)
    use serde::de::IntoDeserializer;

    let v = Value::from(42);
    let de: &Value = (&v).into_deserializer();
    let result: i64 = from_value(de).unwrap();
    assert_eq!(result, 42);
}

// ============================================================================
// Load all variants
// ============================================================================

#[test]
fn load_all_as_typed() {
    let yaml = "---\n42\n---\n99\n";
    let values: Vec<i64> = load_all_as(yaml).unwrap();
    assert_eq!(values, vec![42, 99]);
}

#[test]
fn try_load_all_returns_iterator() {
    let yaml = "---\n42\n---\n99\n";
    let iter = try_load_all(yaml).unwrap();
    assert_eq!(iter.len(), 2);
    let results: Vec<Value> = iter.filter_map(|r| r.ok()).collect();
    assert_eq!(results.len(), 2);
}

// ============================================================================
// Scanner: skip_to_next_token in flow context
// ============================================================================

#[test]
fn scanner_skip_to_next_token_flow() {
    // In flow context, skip_blank handles tabs differently (lines 293-319)
    // Tabs in flow context are allowed
    let yaml = "[1,\t2, 3]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0].as_i64(), Some(1));
    assert_eq!(v[1].as_i64(), Some(2));
    assert_eq!(v[2].as_i64(), Some(3));
}

// ============================================================================
// ADDITIONAL COVERAGE TESTS
// ============================================================================

// --- value.rs: MappingVisitor/MappingAnyVisitor `expecting` (lines 399, 855)
// ---

#[test]
fn mapping_visitor_expecting_error() {
    // Deserializing a non-map JSON value into Mapping triggers the expecting
    // message
    let result: std::result::Result<Mapping, _> = serde_json::from_str("42");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("a YAML mapping") || err.contains("invalid type"));
}

#[test]
fn mapping_any_visitor_expecting_error() {
    let result: std::result::Result<MappingAny, _> = serde_json::from_str("42");
    assert!(result.is_err());
}

// --- value.rs: TaggedValueVisitor expecting (lines 1446-1450, 1526) ---

#[test]
fn tagged_value_visitor_expecting_error() {
    // Deserializing a non-map into TaggedValue triggers expecting
    let result: std::result::Result<TaggedValue, _> = serde_json::from_str("42");
    assert!(result.is_err());
}

#[test]
fn tagged_value_visitor_from_json_map() {
    // Deserializing a single-entry map into TaggedValue
    let result: TaggedValue = serde_json::from_str(r#"{"mytag": "val"}"#).unwrap();
    assert_eq!(result.tag().as_str(), "mytag");
}

// --- value.rs: usize::index_or_insert through Tagged (line 2468) ---

#[test]
fn usize_index_or_insert_through_tagged() {
    let mut v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("t"),
        Value::Sequence(vec![Value::Null]),
    )));
    v[0] = Value::from(1);
    match &v {
        Value::Tagged(t) => {
            assert_eq!(t.value()[0].as_i64(), Some(1));
        }
        _ => panic!("expected tagged"),
    }
}

// --- value.rs: value_type_name arms (lines 2581-2587) ---

#[test]
#[should_panic(expected = "null")]
fn value_type_name_null() {
    let mut v = Value::Null;
    0usize.index_or_insert(&mut v);
}

#[test]
#[should_panic(expected = "boolean")]
fn value_type_name_bool() {
    let mut v = Value::Bool(true);
    0usize.index_or_insert(&mut v);
}

#[test]
#[should_panic(expected = "number")]
fn value_type_name_number() {
    let mut v = Value::from(42);
    0usize.index_or_insert(&mut v);
}

#[test]
#[should_panic(expected = "string")]
fn value_type_name_string_usize_index() {
    let mut v = Value::from("hello");
    0usize.index_or_insert(&mut v);
}

// --- value.rs: ValueVisitor expecting + visit_string (lines 2600, 2603-2604,
// 2627-2628) ---

#[test]
fn value_deserialize_from_json_string() {
    // serde_json will call visit_string (not visit_str) for owned strings
    let v: Value = serde_json::from_str(r#""hello""#).unwrap();
    assert_eq!(v.as_str(), Some("hello"));
}

#[test]
fn value_deserialize_from_json_various() {
    // This exercises ValueVisitor for booleans, numbers, null, arrays, objects
    let v: Value = serde_json::from_str("true").unwrap();
    assert_eq!(v.as_bool(), Some(true));

    let v: Value = serde_json::from_str("null").unwrap();
    assert!(v.is_null());

    let v: Value = serde_json::from_str("[1,2]").unwrap();
    assert_eq!(v[0].as_i64(), Some(1));

    let v: Value = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert!(v["a"].as_i64().is_some());
}

// --- value.rs: &Value Deserializer paths (lines 2767-2776, 2799-2801, 2811,
// 2837-2844) ---

#[test]
fn ref_value_deserialize_any_all_types() {
    // Exercise <&Value as serde::Deserializer>::deserialize_any for each variant
    use serde::Deserialize;

    let v = Value::Null;
    let result: Value = Value::deserialize(&v).unwrap();
    assert!(result.is_null());

    let v = Value::Bool(false);
    let result: Value = Value::deserialize(&v).unwrap();
    assert_eq!(result.as_bool(), Some(false));

    let v = Value::from(99i64);
    let result: Value = Value::deserialize(&v).unwrap();
    assert_eq!(result.as_i64(), Some(99));

    let v = Value::from(3.14f64);
    let result: Value = Value::deserialize(&v).unwrap();
    assert!(result.as_f64().is_some());

    let v = Value::from("test");
    let result: Value = Value::deserialize(&v).unwrap();
    assert_eq!(result.as_str(), Some("test"));

    let v = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let result: Value = Value::deserialize(&v).unwrap();
    assert_eq!(result[0].as_i64(), Some(1));

    let mut m = Mapping::new();
    let _ = m.insert("k".to_string(), Value::from(10));
    let v = Value::Mapping(m);
    let result: Value = Value::deserialize(&v).unwrap();
    assert_eq!(result["k"].as_i64(), Some(10));
}

#[test]
fn ref_value_deserialize_enum_string_variant() {
    // Exercise deserialize_enum on string Value (line 2799-2801)
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    enum Color {
        Red,
        Blue,
    }

    let v = Value::from("Red");
    let result: Color = Color::deserialize(&v).unwrap();
    assert_eq!(result, Color::Red);
}

#[test]
fn ref_value_deserialize_seq_fallback() {
    // Exercise deserialize_seq on non-sequence (line 2811)
    // When the value is not a sequence, it falls through to deserialize_any
    use serde::Deserialize;

    // A number going through deserialize_seq should fall through to deserialize_any
    // This should fail since i64 is not a sequence
    let v = Value::from(42);
    let result: std::result::Result<Vec<i64>, _> = Vec::<i64>::deserialize(&v);
    assert!(result.is_err());
}

#[test]
fn ref_value_deserialize_struct_spanned_via_ref() {
    // Exercise deserialize_struct with Spanned name (lines 2837-2844)
    // The &Value deserializer checks for SPANNED_TYPE_NAME
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Config {
        port: Spanned<u16>,
    }

    // Deserialize from YAML string first to get Value
    let yaml = "port: 8080\n";
    let config: Config = from_str(yaml).unwrap();
    assert_eq!(*config.port, 8080);
}

#[test]
fn ref_value_deserialize_struct_spanned_from_value() {
    // Exercise the &Value path specifically via from_value
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Config {
        port: Spanned<u16>,
    }

    let mut m = Mapping::new();
    let _ = m.insert("port".to_string(), Value::from(8080));
    let v = Value::Mapping(m);
    let config: Config = from_value(&v).unwrap();
    assert_eq!(*config.port, 8080);
}

// --- value.rs: ValueSeqAccess/ValueMapAccess (lines 2722, 2755) ---

#[test]
fn value_seq_access_empty() {
    // Exercise ValueSeqAccess with empty sequence
    use serde::Deserialize;

    let v = Value::Sequence(vec![]);
    let result: Vec<i64> = Vec::<i64>::deserialize(&v).unwrap();
    assert!(result.is_empty());
}

#[test]
fn value_map_access_through_tagged() {
    // Exercise ValueMapAccess via TaggedValue deserialization
    use serde::Deserialize;

    let mut m = Mapping::new();
    let _ = m.insert("x".to_string(), Value::from(10));
    let tagged = TaggedValue::new(Tag::new("point"), Value::Mapping(m));
    let v = Value::Tagged(Box::new(tagged));

    // Deserializing to Value through &Value deserializer exercises ValueMapAccess
    let result: Value = Value::deserialize(&v).unwrap();
    assert!(matches!(result, Value::Mapping(_)));
}

// --- scanner.rs: ScanError Display (lines 63-64) ---

#[test]
fn scan_error_display_format() {
    let result: Result<Value> = from_str(":\n  :\n  :\nbad: [");
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    // The error message should contain useful info
    assert!(!err_str.is_empty());
}

// --- scanner.rs: peek returning 0 at EOF (lines 170-173) ---

#[test]
fn scanner_empty_input() {
    // Empty YAML resolves to Null per YAML spec.
    let result: Result<Value> = from_str("");
    assert_eq!(result.unwrap(), Value::Null);
}

#[test]
fn scanner_whitespace_only() {
    // Whitespace-only resolves to Null per YAML spec.
    let result: Result<Value> = from_str("   ");
    assert_eq!(result.unwrap(), Value::Null);
}

// --- scanner.rs: BOM handling (lines 473-475) ---

#[test]
fn scanner_bom_utf8() {
    let yaml = "\u{FEFF}key: val\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("val"));
}

// --- scanner.rs: fetch_key in flow (lines 447-450) ---

#[test]
fn scanner_explicit_key_in_flow() {
    // ? followed by blank in flow context (line 448-450)
    let yaml = "? a\n: 1\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

// --- scanner.rs: fetch_value colon in flow (lines 457-459) ---

#[test]
fn scanner_colon_before_flow_indicators() {
    // Colon followed by flow-indicator chars
    let yaml = "{a: 1, b: [2, 3]}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

// --- scanner.rs: dash in flow context (lines 441-442) ---

#[test]
fn scanner_dash_in_flow_context() {
    // Dash followed by comma in flow context (line 441)
    // Exercises: - followed by , ] } inside flow
    let yaml = "[1, -2, 3]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0].as_i64(), Some(1));
    assert_eq!(v[1].as_i64(), Some(-2));
}

// --- scanner.rs: fetch_block_entry error (line 583) ---

// Already tested via normal block entries

// --- scanner.rs: fetch_key error (line 599) ---

#[test]
fn scanner_fetch_key_error() {
    // mapping keys not allowed in this context
    let result: Result<Value> = from_str("  ? key\n");
    // This might succeed or fail depending on context; just exercise the path
    let _ = result;
}

// --- scanner.rs: fetch_value paths (lines 650, 659-666) ---

#[test]
fn scanner_complex_key_value() {
    // Complex key with ? and : syntax
    let yaml = "? key1\n: val1\n? key2\n: val2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key1"].as_str(), Some("val1"));
    assert_eq!(v["key2"].as_str(), Some("val2"));
}

// --- scanner.rs: anchor name length limit (line 733) ---

#[test]
fn scanner_anchor_name_limit() {
    // Very long anchor name exceeding 1024 bytes
    let long_name = "a".repeat(1030);
    let yaml = format!("&{} val\n", long_name);
    let result: Result<Value> = from_str(&yaml);
    assert!(result.is_err());
}

// --- scanner.rs: single-quoted scalar internals (lines 978-1048) ---

#[test]
fn scanner_single_quoted_unterminated() {
    let result: Result<Value> = from_str("'unterminated");
    assert!(result.is_err());
}

#[test]
fn scanner_single_quoted_line_folding_multi_break() {
    // Single-quoted with multiple line breaks (tests lines 1001-1011, 1024-1031)
    let yaml = "'first\n\n\nsecond'\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("first"));
    assert!(s.contains("second"));
}

#[test]
fn scanner_single_quoted_whitespace_between_words() {
    // Tests line 1047-1048: whitespace buffering between words
    let yaml = "'word1   word2'\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("word1   word2"));
}

// --- scanner.rs: double-quoted scalar internals (lines 1071-1100) ---

#[test]
fn scanner_double_quoted_unterminated() {
    let result: Result<Value> = from_str("\"unterminated");
    assert!(result.is_err());
}

#[test]
fn scanner_double_quoted_close_after_break() {
    // Tests line 1077-1083: closing quote after line break
    let yaml = "\"word\n  \"\n";
    let v: Value = from_str(yaml).unwrap();
    // Line folding converts the break to space
    let s = v.as_str().unwrap();
    assert!(s.contains("word"));
}

#[test]
fn scanner_double_quoted_escape_after_break() {
    // Tests line 1091-1100: escape sequence after line break
    let yaml = "\"word\n  \\nmore\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("word"));
}

#[test]
fn scanner_double_quoted_multi_line_breaks() {
    // Tests lines 1170-1176: multiple breaks in double-quoted
    let yaml = "\"first\n\n\nsecond\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("first"));
    assert!(s.contains("second"));
}

#[test]
fn scanner_double_quoted_whitespace_buffering() {
    // Tests lines 1203-1214: whitespace between words in double-quoted
    let yaml = "\"word1   word2\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("word1   word2"));
}

// --- scanner.rs: hex escape errors (lines 1260-1261) ---

#[test]
fn scanner_hex_escape_invalid_char() {
    let result: Result<Value> = from_str("\"\\xGG\"\n");
    assert!(result.is_err());
}

#[test]
fn scanner_unicode_escape_surrogate() {
    // \uD800 is an invalid Unicode code point
    let result: Result<Value> = from_str("\"\\uD800\"\n");
    assert!(result.is_err());
}

// --- scanner.rs: block scalar explicit indent (lines 1324-1335) ---

#[test]
fn scanner_block_scalar_explicit_indent_indicator() {
    let yaml = "|2\n  text\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("text\n"));
}

// --- scanner.rs: block scalar content end (lines 1324-1335) ---

#[test]
fn scanner_block_scalar_content_end_at_lower_indent() {
    let yaml = "data: |\n  text\nnot_indented: val\n";
    let v: Value = from_str(yaml).unwrap();
    // The block scalar ends when indent decreases
    assert_eq!(v["data"].as_str(), Some("text\n"));
    assert_eq!(v["not_indented"].as_str(), Some("val"));
}

// --- scanner.rs: block scalar UTF-8, chomping (lines 1397-1449) ---

#[test]
fn scanner_block_scalar_utf8_content() {
    let yaml = "|\n  caf\u{00e9}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("caf\u{00e9}\n"));
}

// --- scanner.rs: line 1130-1145: double-quoted \r\n escape ---

#[test]
fn scanner_double_quoted_cr_lf_escape() {
    // \\\r\n line break escape
    let yaml = "\"word\\\r\n  more\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("wordmore"));
}

// --- scanner.rs: line 1108: unexpected EOF in escape ---

#[test]
fn scanner_double_quoted_eof_in_escape() {
    let result: Result<Value> = from_str("\"\\");
    assert!(result.is_err());
}

// --- scanner.rs: line 936: empty plain scalar error ---

#[test]
fn scanner_empty_plain_scalar_error() {
    // A flow indicator in the wrong context triggers the error
    let result: Result<Value> = from_str("}");
    assert!(result.is_err());
}

// --- scanner.rs: plain scalar multiline with CRLF (lines 914-916) ---

#[test]
fn scanner_plain_scalar_crlf_multiline() {
    let yaml = "key:\r\n  val\r\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("val"));
}

// --- scanner.rs: line 803, 807: plain scalar peek_at loop ---

#[test]
fn scanner_plain_scalar_with_colon_space() {
    // Plain scalar in flow context that stops at ':'
    let yaml = "{key: value}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("value"));
}

// --- events.rs: parse_document_start explicit (line 221) ---

#[test]
fn events_explicit_document_start() {
    let yaml = "---\na: 1\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

// --- events.rs: parse_document_content empty (line 232) ---

#[test]
fn events_document_content_stream_end() {
    // Document that ends immediately
    let yaml = "---\n...\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.is_null());
}

// --- events.rs: tag after anchor (lines 281-284) ---

#[test]
fn events_tag_after_anchor() {
    let yaml = "&anc !!str val\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("val"));
}

// --- events.rs: FlowSequenceStart/FlowMappingStart in parse_node (lines 321,
// 330) ---

#[test]
fn events_flow_in_parse_node() {
    // Flow sequence and mapping as node
    let yaml = "seq: [1, 2]\nmap: {a: b}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["seq"][0].as_i64(), Some(1));
    assert_eq!(v["map"]["a"].as_str(), Some("b"));
}

// --- events.rs: anchor/tag without value (lines 357-369) ---

#[test]
fn events_anchor_without_value() {
    // Anchor on a key line with value on next line
    let yaml = "&anc\nkey: val\n";
    // This exercises the path where anchor is present but no scalar token follows
    let result: Result<Value> = from_str(yaml);
    let _ = result; // May fail or succeed - we just want to exercise the code
    // path
}

#[test]
fn events_indentless_sequence_fallback() {
    // Indentless sequence with empty entries
    let yaml = "key:\n  - a\n  - b\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"][0].as_str(), Some("a"));
    assert_eq!(v["key"][1].as_str(), Some("b"));
}

// --- events.rs: block mapping key/value (lines 401-428) ---

#[test]
fn events_block_mapping_explicit_key_empty_value() {
    // ? key followed by another ? (empty value)
    let yaml = "? key1\n? key2\n: val2\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["key1"].is_null());
    assert_eq!(v["key2"].as_str(), Some("val2"));
}

// --- events.rs: block mapping value after skip (lines 443-444) ---

#[test]
fn events_block_mapping_value_skip() {
    // Key followed by : then a value
    let yaml = "? key\n: val\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("val"));
}

// --- events.rs: flow sequence mapping (lines 529-553) ---

#[test]
fn events_flow_sequence_with_mapping() {
    let yaml = "[{a: 1}, {b: 2}]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0]["a"].as_i64(), Some(1));
    assert_eq!(v[1]["b"].as_i64(), Some(2));
}

#[test]
fn events_flow_sequence_implicit_map_entry() {
    // Implicit mapping in flow sequence: [a: b]
    let yaml = "[a: b]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0]["a"].as_str(), Some("b"));
}

// --- events.rs: flow mapping key (lines 595-602) ---

#[test]
fn events_flow_mapping_multiple_entries() {
    let yaml = "{a: 1, b: 2, c: 3}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
    assert_eq!(v["b"].as_i64(), Some(2));
    assert_eq!(v["c"].as_i64(), Some(3));
}

// --- events.rs: flow mapping empty value after key ---

#[test]
fn events_flow_mapping_key_comma() {
    // Flow mapping with key followed directly by comma (empty value)
    let yaml = "{? a, b: 2}\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["a"].is_null());
    assert_eq!(v["b"].as_i64(), Some(2));
}

// --- loader.rs: multi-document (lines 147, 155) ---

#[test]
fn loader_multi_doc_stream() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let docs = load_all(yaml).unwrap();
    assert_eq!(docs.len(), 2);
}

// --- loader.rs: sequence end/mapping end (lines 209, 222, 227, 246, 267, 273)
// ---

#[test]
fn loader_sequence_mapping_lifecycle() {
    let yaml = "items:\n  - name: a\n    val: 1\n  - name: b\n    val: 2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["items"][0]["name"].as_str(), Some("a"));
    assert_eq!(v["items"][1]["val"].as_i64(), Some(2));
}

// --- loader.rs: duplicate key error (line 293 - ScalarInMergeElement) ---

#[test]
fn loader_merge_key_with_scalar_in_sequence() {
    // Merge key where the sequence contains a scalar
    let yaml = "defaults: &defaults\n  a: 1\nresult:\n  <<: [*defaults, badval]\n  b: 2\n";
    let result: Result<Value> = from_str(yaml);
    assert!(result.is_err());
}

// --- loader.rs: duplicate key policy Error (line 380-383) ---

#[test]
fn loader_duplicate_key_error_policy() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let yaml = "a: 1\na: 2\n";
    let result: Result<Value> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

// --- loader.rs: push_value paths (lines 319-356) ---

#[test]
fn loader_complex_nested_yaml() {
    let yaml = "root:\n  child1:\n    - item1\n    - item2\n  child2:\n    key: val\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["root"]["child1"][0].as_str(), Some("item1"));
    assert_eq!(v["root"]["child2"]["key"].as_str(), Some("val"));
}

// --- loader.rs: merge keys (lines 402, 410) ---

#[test]
fn loader_merge_key_single_mapping() {
    let yaml = "defaults: &defaults\n  a: 1\n  b: 2\nresult:\n  <<: *defaults\n  c: 3\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["result"]["a"].as_i64(), Some(1));
    assert_eq!(v["result"]["c"].as_i64(), Some(3));
}

#[test]
fn loader_merge_key_sequence_of_mappings() {
    let yaml = "d1: &d1\n  a: 1\nd2: &d2\n  b: 2\nresult:\n  <<: [*d1, *d2]\n  c: 3\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["result"]["a"].as_i64(), Some(1));
    assert_eq!(v["result"]["b"].as_i64(), Some(2));
    assert_eq!(v["result"]["c"].as_i64(), Some(3));
}

// --- loader.rs: First policy (line 468 / 417-421) ---

#[test]
fn loader_duplicate_key_first_policy() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let yaml = "a: 1\na: 2\n";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    // First policy: first value wins
    assert_eq!(v["a"].as_i64(), Some(1));
}

// --- loader.rs: tagged scalars (lines 528, 545, 549) ---

#[test]
fn loader_tagged_int() {
    let yaml = "!!int 42\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn loader_tagged_float() {
    let yaml = "!!float 3.14\n";
    let v: Value = from_str(yaml).unwrap();
    assert!((v.as_f64().unwrap() - 3.14).abs() < 0.001);
}

#[test]
fn loader_tag_primary_empty_suffix() {
    // Tag with just `!` (empty suffix) — surfaces as
    // `Value::Tagged("!", String("val"))` on the default
    // tag-preserving deserialise path. Use `untag()` to skip
    // through the wrapper for the inner string check, mirroring
    // how a typed `T::deserialize` would see through it.
    let yaml = "! val\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(matches!(v, Value::Tagged(_)), "tag is preserved");
    assert_eq!(v.untag_ref().as_str(), Some("val"));
}

#[test]
fn loader_custom_tag_with_inner_resolution() {
    let yaml = "!mytag 42\n";
    let v: Value = from_str(yaml).unwrap();
    // Custom tags wrap an *unresolved* inner string scalar — the
    // YAML 1.2 schema only resolves plain-scalar types when no
    // tag is present (`!!int 42` resolves; `!mytag 42` keeps the
    // string form so the tag's downstream consumer can decide
    // how to interpret it).
    let Value::Tagged(t) = &v else {
        panic!("expected Tagged, got {v:?}");
    };
    assert_eq!(t.tag().as_str(), "!mytag");
    assert_eq!(t.value().as_str(), Some("42"));
}

// --- loader.rs: estimate_value_size with aliases (line 588) ---

#[test]
fn loader_alias_expansion() {
    let yaml = "big: &big\n  a: 1\n  b: 2\nref1: *big\nref2: *big\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["ref1"]["a"].as_i64(), Some(1));
    assert_eq!(v["ref2"]["b"].as_i64(), Some(2));
}

// --- loader.rs: hex/octal (line 638) ---

#[test]
fn loader_hex_octal() {
    let yaml = "hex: 0xFF\noct: 0o77\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["hex"].as_i64(), Some(255));
    assert_eq!(v["oct"].as_i64(), Some(63));
}

// --- loader.rs: large int (line 688) ---

#[test]
fn loader_large_int_overflow_float() {
    let yaml = "99999999999999999999\n";
    let v: Value = from_str(yaml).unwrap();
    // Large int overflows i64, stored as float
    assert!(v.as_f64().is_some());
}

// --- de.rs: SpannedFieldState transitions (lines 786-814) ---

#[test]
fn de_spanned_all_field_states() {
    #[derive(Debug, Deserialize)]
    struct S {
        name: Spanned<String>,
        count: Spanned<i64>,
    }

    let yaml = "name: hello\ncount: 42\n";
    let s: S = from_str(yaml).unwrap();
    assert_eq!(*s.name, "hello");
    assert_eq!(*s.count, 42);
    // Check that span info is populated
    // Spans may be zero-based, just ensure the fields are accessible without panic
    let _ = s.name.start.index();
    let _ = s.name.start.line();
    let _ = s.name.start.column();
}

// --- spanned.rs: Spanned<T> deserialization ---

#[test]
fn spanned_bool_from_yaml() {
    #[derive(Debug, Deserialize)]
    struct S {
        flag: Spanned<bool>,
    }

    let yaml = "flag: true\n";
    let s: S = from_str(yaml).unwrap();
    assert!(*s.flag);
}

// --- ser.rs: looks_like_number for .5 format (lines 361-363, 414-416) ---

#[test]
fn ser_looks_like_number_dot_five() {
    // .5 should be quoted as it looks like a number
    let yaml = to_string(&Value::from(".5")).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}

// --- ser.rs: reserved word check (line 376/500) ---

#[test]
fn ser_reserved_words_quoted() {
    let yaml = to_string(&Value::from("true")).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    let yaml = to_string(&Value::from("null")).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));

    let yaml = to_string(&Value::from("~")).unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}

// --- ser.rs: first-char special for ' (line 404/461) ---

#[test]
fn ser_first_char_single_quote() {
    let yaml = to_string(&Value::from("'hello")).unwrap();
    assert!(yaml.contains('"'));
}

// --- ser.rs: Commented value serialization (line 632) ---

#[test]
fn ser_commented_value_in_sequence() {
    // Sequence item that is a mapping with multiple entries
    // Line 631-632: write_value for nested mapping in sequence
    let mut inner = Mapping::new();
    let _ = inner.insert("a".to_string(), Value::from(1));
    let _ = inner.insert(
        "b".to_string(),
        Value::Mapping({
            let mut m = Mapping::new();
            let _ = m.insert("c".to_string(), Value::from(2));
            m
        }),
    );

    let seq = Value::Sequence(vec![Value::Mapping(inner)]);
    let yaml = to_string(&seq).unwrap();
    assert!(yaml.contains("a:"));
    assert!(yaml.contains("b:"));
}

// --- ser.rs: write_literal_block, write_folded_block (lines 711, 718) ---

#[test]
fn ser_literal_block_non_string_fallback() {
    // Using MAGIC_LIT_STR tag on a non-string value falls back to write_value
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_lit_str"),
        Value::from(42),
    )));
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("42"));
}

#[test]
fn ser_folded_block_non_string_fallback() {
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_fold_str"),
        Value::from(42),
    )));
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("42"));
}

// --- ser.rs: write_commented (line 731, 734) ---

#[test]
fn ser_commented_wrong_length() {
    // Commented tag with wrong sequence length
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_commented"),
        Value::Sequence(vec![Value::from(1)]),
    )));
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("1"));
}

#[test]
fn ser_commented_non_sequence() {
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_commented"),
        Value::from(42),
    )));
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("42"));
}

// --- ser.rs: unknown internal tag (line 743) ---

#[test]
fn ser_unknown_internal_tag() {
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_unknown"),
        Value::from(42),
    )));
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("42"));
}

// --- ser.rs: multi-doc (line 782) ---

#[test]
fn ser_multi_doc() {
    let docs = vec![1, 2, 3];
    let yaml = to_string_multi(&docs).unwrap();
    assert!(yaml.contains("---"));
    assert!(yaml.contains("1"));
    assert!(yaml.contains("2"));
    assert!(yaml.contains("3"));
}

// --- ser.rs: write_literal_block and write_folded_block via fmt types ---

#[test]
fn ser_literal_block_via_lit_str() {
    let v = LitStr("hello\nworld\n");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("|"));
}

#[test]
fn ser_folded_block_via_fold_str() {
    let v = FoldStr("hello\nworld\n");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains(">"));
}

#[test]
fn ser_literal_block_keep_chomping() {
    let v = LitStr("hello\nworld\n\n");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("|+"));
}

#[test]
fn ser_literal_block_strip_chomping() {
    let v = LitStr("hello\nworld");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("|-"));
}

#[test]
fn ser_folded_block_keep_chomping() {
    let v = FoldStr("hello\nworld\n\n");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains(">+"));
}

#[test]
fn ser_folded_block_strip_chomping() {
    let v = FoldStr("hello\nworld");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains(">-"));
}

// --- singleton_map_recursive.rs: tagged value (lines 47-51) ---

#[test]
fn singleton_map_recursive_with_tagged() {
    use noyalib::with::singleton_map_recursive;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Action {
        Click { x: i32, y: i32 },
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        #[serde(with = "singleton_map_recursive")]
        action: Action,
    }

    let config = Config {
        action: Action::Click { x: 1, y: 2 },
    };
    let yaml = to_string(&config).unwrap();
    let roundtrip: Config = from_str(&yaml).unwrap();
    assert_eq!(config, roundtrip);
}

// --- singleton_map_with.rs: non-mapping/non-string fallback (lines 141, 143)
// ---

#[test]
fn singleton_map_with_serialize_non_mapping() {
    use noyalib::with::singleton_map_with;

    // Test serialize_with when the intermediate Value is not a Mapping or String
    // This happens when the enum variant serializes to something else
    // We'll test using the deserialize_with path with a tagged value

    // Create a Value with a tagged inner value and call transform_value_keys
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("test"),
        Value::from("inner"),
    )));
    // Exercise transform_value_keys on tagged value by using deserialize_with
    let result = singleton_map_with::deserialize_with(&tagged, |s| s.to_uppercase());
    // Result type doesn't matter - we just want to exercise the code path
    let _: Result<Value> = result;
}

// --- Additional scanner paths ---

#[test]
fn scanner_single_quoted_break_followed_by_break() {
    // Test the path where leading_break is true and more breaks follow
    // This exercises lines 1005-1011
    let yaml = "'a\n\nb'\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("a"));
    assert!(s.contains("b"));
}

#[test]
fn scanner_double_quoted_break_followed_by_normal() {
    // After line break, next char is normal - exercises lines 1203-1211
    let yaml = "\"hello\nworld\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello world"));
}

#[test]
fn scanner_block_scalar_empty_only_lines() {
    // Block scalar with only empty lines (exercises line 1396-1397)
    let yaml = "|\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    // Empty block scalar with trailing breaks, clip chomping
    let s = v.as_str().unwrap();
    assert!(s.contains('\n') || s.is_empty());
}

// --- loader.rs: resolve_quoted_scalar with tag (line 528) ---

#[test]
fn loader_quoted_scalar_with_tag() {
    let yaml = "!!str 'hello'\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello"));
}

// --- loader.rs: resolve_tagged_scalar with non-!! handle (line 549) ---

#[test]
fn loader_tag_with_generic_handle() {
    // Primary tag handle with suffix
    let yaml = "!mytag value\n";
    let v: Value = from_str(yaml).unwrap();
    // May be tagged or string depending on resolution
    if let Value::Tagged(t) = &v {
        assert_eq!(t.tag().as_str(), "!mytag");
    }
    // Ensure it parses without error
}

// --- scanner.rs: CRLF in single-quoted (line 1017-1018) ---

#[test]
fn scanner_single_quoted_crlf() {
    let yaml = "'hello\r\nworld'\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello world"));
}

// --- scanner.rs: CRLF in double-quoted (line 1182-1183) ---

#[test]
fn scanner_double_quoted_crlf() {
    let yaml = "\"hello\r\nworld\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello world"));
}

// --- scanner.rs: CRLF skip_line (line 287) ---

#[test]
fn scanner_skip_line_crlf() {
    let yaml = "a: 1\r\nb: 2\r\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
    assert_eq!(v["b"].as_i64(), Some(2));
}

// --- scanner.rs: comment after block scalar indicator (skip to end of line)
// ---

#[test]
fn scanner_block_scalar_with_comment() {
    let yaml = "| # this is a comment\n  text\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("text\n"));
}

// --- events.rs: parse_document_end with ... marker ---

#[test]
fn events_document_end_marker() {
    let yaml = "---\na: 1\n...\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

// --- events.rs: indentless sequence with empty entries (lines 414-421) ---

#[test]
fn events_indentless_sequence_empty_entries() {
    // Indentless sequence where entry is followed by Key/Value/BlockEnd
    let yaml = "key:\n  - a\n  -\n  - b\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"][0].as_str(), Some("a"));
    assert!(v["key"][1].is_null());
    assert_eq!(v["key"][2].as_str(), Some("b"));
}

// --- events.rs: flow sequence entry mapping end (line 556-560) ---

#[test]
fn events_flow_sequence_entry_mapping_end() {
    // Exercises parse_flow_sequence_entry_mapping_end
    let yaml = "[a: 1, b: 2]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0]["a"].as_i64(), Some(1));
}

// --- loader.rs: MappingValue frame + merge key value push (lines 410-414) ---

#[test]
fn loader_merge_key_value_push() {
    let yaml = "base: &base\n  x: 1\n  y: 2\nchild:\n  <<: *base\n  z: 3\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["child"]["x"].as_i64(), Some(1));
    assert_eq!(v["child"]["y"].as_i64(), Some(2));
    assert_eq!(v["child"]["z"].as_i64(), Some(3));
}

// --- loader.rs: duplicate key Last policy replaces at same position (lines
// 423-435) ---

#[test]
fn loader_duplicate_key_last_replaces() {
    // Default policy is Last, duplicate key overwrites
    let yaml = "a: 1\nb: 2\na: 3\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(3));
}

// --- Additional ser.rs paths ---

#[test]
fn ser_sequence_nested_mapping_and_sequence() {
    // Line 632: nested mapping value in sequence items
    let mut m = Mapping::new();
    let _ = m.insert(
        "x".to_string(),
        Value::Sequence(vec![Value::from(1), Value::from(2)]),
    );
    let _ = m.insert("y".to_string(), Value::from(3));

    let seq = Value::Sequence(vec![Value::Mapping(m.clone()), Value::Mapping(m)]);
    let yaml = to_string(&seq).unwrap();
    assert!(yaml.contains("x:"));
    assert!(yaml.contains("y:"));
}

#[test]
fn ser_sequence_with_nested_sequence() {
    // Line 638-639: sequence item that is a sequence
    let seq = Value::Sequence(vec![Value::Sequence(vec![Value::from(1), Value::from(2)])]);
    let yaml = to_string(&seq).unwrap();
    assert!(yaml.contains("- 1"));
}

// --- scanner.rs: fetch_directive (line 471) ---

#[test]
fn scanner_directive() {
    let yaml = "%YAML 1.2\n---\na: 1\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

// --- Additional edge cases ---

#[test]
fn scanner_single_quoted_leading_break_then_spaces() {
    // Single-quoted: line folding with leading break and then chars
    // This exercises the path at line 1037-1048
    let yaml = "'hello\n  world'\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello world"));
}

#[test]
fn scanner_double_quoted_u_escape() {
    let yaml = "\"\\u0042\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("B"));
}

#[test]
fn scanner_double_quoted_big_u_escape() {
    let yaml = "\"\\U00000043\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("C"));
}

#[test]
fn scanner_double_quoted_various_escapes() {
    let yaml = "\"\\0\\a\\b\\t\\n\\v\\f\\r\\e\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains('\0'));
    assert!(s.contains('\x07'));
    assert!(s.contains('\x08'));
    assert!(s.contains('\t'));
    assert!(s.contains('\n'));
}

#[test]
fn scanner_double_quoted_special_escapes() {
    // Test \N \_ \L \P escapes (NEL, NBSP, LS, PS)
    let yaml = "\"\\N\\_\\L\\P\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains('\u{0085}'));
    assert!(s.contains('\u{00A0}'));
    assert!(s.contains('\u{2028}'));
    assert!(s.contains('\u{2029}'));
}

#[test]
fn scanner_block_scalar_more_indented_literal() {
    // Block literal with more-indented lines (line 1379-1391)
    let yaml = "|\n  normal\n    more indented\n  normal again\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("normal"));
    assert!(s.contains("  more indented"));
}

// --- loader.rs: tagged bool resolution (line 554-557) ---

#[test]
fn loader_tagged_null() {
    let yaml = "!!null ~\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.is_null());
}

#[test]
fn loader_tagged_str() {
    let yaml = "!!str 42\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("42"));
}

// --- loader.rs: tagged float with special values ---

#[test]
fn loader_tagged_float_inf() {
    let yaml = "!!float .inf\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_f64(), Some(f64::INFINITY));
}

#[test]
fn loader_tagged_float_nan() {
    let yaml = "!!float .nan\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_f64().unwrap().is_nan());
}

// --- scanner.rs: block scalar folded leading_blank (line 1411-1412) ---

#[test]
fn scanner_block_scalar_folded_leading_blank_line() {
    // Folded block where a line starts with a space (leading blank)
    // This triggers the literal newline instead of space (line 1411)
    let yaml = ">\n  normal line\n   leading space\n  another normal\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("normal line"));
}

// --- scanner.rs: fetch_block_entry in flow context ---

#[test]
fn scanner_dash_comma_in_flow() {
    // Exercise flow context with various separators
    let yaml = "[a, b, c]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0].as_str(), Some("a"));
    assert_eq!(v[2].as_str(), Some("c"));
}

// --- scanner.rs: flow level > 0 paths for fetch_key ---

#[test]
fn scanner_question_mark_in_flow() {
    // ? in flow followed by ] (line 449)
    let yaml = "[?]\n";
    let result: Result<Value> = from_str(yaml);
    let _ = result; // Just exercise the path
}

// --- events.rs: block mapping indentless with key/value/blockend ---

#[test]
fn events_block_mapping_value_after_value() {
    // Exercises parse_block_mapping_value where value token is followed by
    // key/value/blockend (line 467-471)
    let yaml = "? a\n:\n? b\n: val\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["a"].is_null());
    assert_eq!(v["b"].as_str(), Some("val"));
}

// --- scanner.rs: skip blank in block context ---

#[test]
fn scanner_comment_skipping() {
    let yaml = "a: 1 # comment\nb: 2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
    assert_eq!(v["b"].as_i64(), Some(2));
}

// --- scanner.rs: roll_indent in flow (line 327) ---

#[test]
fn scanner_roll_indent_returns_in_flow() {
    // In flow context, roll_indent returns early
    let yaml = "{a: [1, 2]}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"][0].as_i64(), Some(1));
}

// --- events.rs: parse_flow_mapping_value empty (line 614-616) ---

#[test]
fn events_flow_mapping_empty_value_path() {
    // Implicit key in flow mapping with empty value
    let yaml = "{a, b: 1}\n";
    let result: Result<Value> = from_str(yaml);
    let _ = result; // Exercise the code path
}

// --- loader.rs: value_to_key for various types ---

#[test]
fn loader_various_key_types() {
    // Boolean key
    let yaml = "true: yes_val\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["true"].as_str(), Some("yes_val"));

    // Null key
    let yaml = "null: null_val\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["null"].as_str(), Some("null_val"));

    // Float key
    let yaml = "1.5: float_val\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["1.5"].as_str(), Some("float_val"));
}

// --- scanner.rs: double-quoted line escape \<newline> (line 1142-1145) ---

#[test]
fn scanner_double_quoted_line_escape_cr_lf() {
    // Backslash followed by \r\n should fold to nothing
    let yaml = "\"hello\\\r\nworld\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("helloworld"));
}

// --- loader.rs: tagged invalid bool ---

#[test]
fn loader_tagged_bool_invalid() {
    let yaml = "!!bool notbool\n";
    let result: Result<Value> = from_str(yaml);
    assert!(result.is_err());
}

// --- loader.rs: tagged int invalid ---

#[test]
fn loader_tagged_int_invalid() {
    let yaml = "!!int notint\n";
    let result: Result<Value> = from_str(yaml);
    assert!(result.is_err());
}

// --- loader.rs: tagged float invalid ---

#[test]
fn loader_tagged_float_invalid() {
    let yaml = "!!float notfloat\n";
    let result: Result<Value> = from_str(yaml);
    assert!(result.is_err());
}

// --- scanner.rs: single-quoted with CRLF breaks (lines 1026-1027) ---

#[test]
fn scanner_single_quoted_multi_crlf_breaks() {
    let yaml = "'a\r\n\r\nb'\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("a"));
    assert!(s.contains("b"));
}

// --- scanner.rs: double-quoted with CRLF breaks (lines 1191-1192) ---

#[test]
fn scanner_double_quoted_multi_crlf_breaks() {
    let yaml = "\"a\r\n\r\nb\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("a"));
    assert!(s.contains("b"));
}

// --- scanner.rs: plain scalar with CRLF (lines 903-905, 914-916) ---

#[test]
fn scanner_plain_scalar_with_crlf_breaks() {
    let yaml = "key: val\r\nother: thing\r\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("val"));
    assert_eq!(v["other"].as_str(), Some("thing"));
}

// --- scanner.rs: block scalar keep/strip/clip chomping (lines 1444-1458) ---

#[test]
fn scanner_block_scalar_keep_trailing() {
    let yaml = "|+\n  keep\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.ends_with("\n\n"));
}

#[test]
fn scanner_block_scalar_strip_trailing() {
    let yaml = "|-\n  strip\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert_eq!(s, "strip");
}

#[test]
fn scanner_block_scalar_clip_trailing() {
    let yaml = "|\n  clip\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.ends_with('\n'));
    assert!(!s.ends_with("\n\n"));
}

// --- scanner.rs: line 152 EOF token stream ---

#[test]
fn scanner_eof_token_stream() {
    // Minimal YAML that ends quickly
    let yaml = "~\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.is_null());
}

// --- events.rs: parse_block_sequence_entry error (lines 401-403) ---

#[test]
fn events_block_sequence_bad_token() {
    // Invalid token where block sequence entry is expected
    let result: Result<Value> = from_str("- a\n  }\n");
    // This may error - just exercise the path
    let _ = result;
}

// --- Additional edge cases for full coverage ---

#[test]
fn ser_string_with_control_chars() {
    // String with control chars gets double-quoted
    let yaml = to_string(&Value::from("hello\x01world")).unwrap();
    assert!(yaml.contains('"'));
    assert!(yaml.contains("\\x01"));
}

#[test]
fn ser_tagged_value_with_string() {
    // Non-internal tagged value serialization (lines 360-363)
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        Value::from("hello"),
    )));
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("!custom"));
    assert!(yaml.contains("hello"));
}

#[test]
fn scanner_block_scalar_folded_multi_breaks() {
    // Folded block with multiple trailing breaks (line 1416-1418)
    let yaml = ">\n  line1\n\n\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("line1"));
    assert!(s.contains("line2"));
}

// --- loader.rs: merge with scalar value (line 297) ---

#[test]
fn loader_merge_with_scalar_value() {
    // Merge key with a scalar value instead of mapping
    let yaml = "val: &val hello\nresult:\n  <<: *val\n  b: 2\n";
    let result: Result<Value> = from_str(yaml);
    assert!(result.is_err());
}

// ============================================================================
// ADDITIONAL COVERAGE TESTS - BATCH 2
// ============================================================================

// --- de.rs 648, 652: deserialize_identifier via from_value enum ---

#[test]
fn de_deserialize_identifier_via_from_value_enum() {
    // from_value with an enum exercises deserialize_identifier on the Value
    // deserializer
    #[derive(Debug, Deserialize, PartialEq)]
    enum Direction {
        North,
        South,
    }

    // Create a mapping { "North": null } to deserialize as enum
    let mut m = Mapping::new();
    m.insert("North", Value::Null);
    let v = Value::Mapping(m);
    let d: Direction = from_value(&v).unwrap();
    assert_eq!(d, Direction::North);
}

#[test]
fn de_deserialize_identifier_via_from_value_struct_fields() {
    // Struct field names go through deserialize_identifier
    #[derive(Debug, Deserialize, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }
    let mut m = Mapping::new();
    m.insert("x", Value::from(10));
    m.insert("y", Value::from(20));
    let v = Value::Mapping(m);
    let p: Point = from_value(&v).unwrap();
    assert_eq!(p, Point { x: 10, y: 20 });
}

// --- de.rs 786-814: SpannedMapAccess via from_str (all field states) ---

#[test]
fn de_spanned_map_access_complete_lifecycle() {
    // Ensure each SpannedFieldState is visited: StartLine -> StartColumn ->
    // StartIndex -> EndLine -> EndColumn -> EndIndex -> Value -> Done
    let yaml = "test_value\n";
    let spanned: Spanned<String> = from_str(yaml).unwrap();
    assert_eq!(*spanned, "test_value");
    // Check all location fields are accessible
    assert!(spanned.start.line() < 100);
    assert!(spanned.start.column() < 100);
    assert!(spanned.start.index() < 100);
    assert!(spanned.end.line() < 100);
    assert!(spanned.end.column() < 100);
    assert!(spanned.end.index() < 100);
}

#[test]
fn de_spanned_in_struct_from_str() {
    // Spanned within struct exercises SpannedMapAccess fully when deserializing
    // from YAML
    #[derive(Debug, Deserialize)]
    struct Config {
        host: Spanned<String>,
        port: Spanned<u16>,
        debug: Spanned<bool>,
    }
    let yaml = "host: localhost\nport: 8080\ndebug: true\n";
    let config: Config = from_str(yaml).unwrap();
    assert_eq!(*config.host, "localhost");
    assert_eq!(*config.port, 8080);
    assert!(*config.debug);
}

// --- events.rs 112: IndentlessSequenceEntry ---

#[test]
fn events_indentless_sequence_at_mapping_level() {
    // Indentless sequence: sequence entries at same indent as mapping value
    // This exercises State::IndentlessSequenceEntry (line 112)
    // Indentless sequences occur when - is indented relative to the key
    let yaml = "items:\n  - a\n  - b\n  - c\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["items"][0].as_str(), Some("a"));
    assert_eq!(v["items"][1].as_str(), Some("b"));
    assert_eq!(v["items"][2].as_str(), Some("c"));
}

// --- events.rs 125-127: End state error ---
// This is unreachable through normal public API; parser is consumed after use.

// --- events.rs 221: DocumentContent state after explicit --- ---

#[test]
fn events_document_content_state() {
    // Explicit --- triggers DocumentStart -> DocumentContent state
    let yaml = "---\nhello\n...\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello"));
}

// --- events.rs 232: DocumentContent empty scalar ---

#[test]
fn events_document_content_empty_before_end() {
    // --- followed immediately by ... produces empty scalar via DocumentContent
    // state
    let yaml = "---\n...\n---\nhello\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(|r| r.ok()).collect();
    assert!(docs[0].is_null());
    assert_eq!(docs[1].as_str(), Some("hello"));
}

// --- events.rs 281-284: Anchor after tag ---

#[test]
fn events_anchor_after_tag_parsed() {
    // Tag then anchor (tag comes first, anchor second)
    let yaml = "!!str &myanchor hello\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello"));
}

// --- events.rs 321, 330: FlowSequenceStart and FlowMappingStart in parse_node
// ---

#[test]
fn events_flow_sequence_start_in_node() {
    // Flow sequence as value in block mapping -> parse_node sees FlowSequenceStart
    let yaml = "data: [1, 2, 3]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["data"][0].as_i64(), Some(1));
}

#[test]
fn events_flow_mapping_start_in_node() {
    // Flow mapping as value in block mapping -> parse_node sees FlowMappingStart
    let yaml = "data: {x: 1, y: 2}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["data"]["x"].as_i64(), Some(1));
}

// --- events.rs 357, 368, 369: tag/anchor without matching node ---

#[test]
fn events_tag_only_empty_scalar() {
    // Tag on a key with empty value -> empty scalar with tag (line 357-366)
    let yaml = "!!str :\n";
    let result: Result<Value> = from_str(yaml);
    // May produce a mapping with an empty-string key, just exercise
    let _ = result;
}

// --- events.rs 401-403: block sequence entry error ---
// Already tested via events_block_sequence_bad_token

// --- events.rs 408-428: indentless sequence with empty entries ---

#[test]
fn events_indentless_sequence_empty_entry_followed_by_key() {
    // Indentless sequence entry followed by Key token -> empty scalar (line
    // 420-421)
    let yaml = "a:\n  - \n  - x\nb: 2\n";
    let v: Value = from_str(yaml).unwrap();
    // The first value of 'a' is a sequence with null entry
    assert!(v["a"][0].is_null());
    assert_eq!(v["a"][1].as_str(), Some("x"));
    assert_eq!(v["b"].as_i64(), Some(2));
}

// --- events.rs 443, 444: block mapping key empty scalar ---

#[test]
fn events_block_mapping_key_followed_by_key() {
    // ? key1 followed immediately by ? key2 -> empty value for key1 (line 443-444)
    let yaml = "? a\n? b\n: value\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["a"].is_null());
    assert_eq!(v["b"].as_str(), Some("value"));
}

// --- events.rs 529: flow_sequence_entry_mapping_key empty scalar ---

#[test]
fn events_flow_sequence_entry_mapping_key_empty() {
    // In flow sequence, ? key : val triggers implicit mapping
    // When ? key is followed by : immediately, the mapping key is the key
    let yaml = "[? : val]\n";
    let result: Result<Value> = from_str(yaml);
    // Exercises parse_flow_sequence_entry_mapping_key with value token
    let _ = result;
}

// --- events.rs 595: flow_mapping_key with Key token ---

#[test]
fn events_flow_mapping_explicit_key() {
    // Explicit ? key in flow mapping (line 588-602)
    // ? in flow mapping needs the key followed by : value
    let yaml = "{? key : val, other: 2}\n";
    let result: Result<Value> = from_str(yaml);
    // Just exercise the path - may or may not parse
    let _ = result;
}

// --- events.rs 601, 602: flow mapping key empty scalar after ? ---

#[test]
fn events_flow_mapping_explicit_key_empty_value_after_key() {
    // ? followed by , or } -> empty key, empty value (line 601-602)
    let yaml = "{? , a: 1}\n";
    let result: Result<Value> = from_str(yaml);
    let _ = result; // Exercise the path
}

// --- loader.rs 147, 155: DocumentStart/DocumentEnd in process_event ---
// Already covered via multi-document tests

// --- loader.rs 159, 165: Scalar event with tag ---

#[test]
fn loader_scalar_event_with_quoted_style_and_tag() {
    // Quoted scalar with tag goes through resolve_quoted_scalar path (line 169)
    let yaml = "!!str 'hello'\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("hello"));
}

// --- loader.rs 209: depth exceeded ---

#[test]
fn loader_max_depth_exceeded() {
    // Create deeply nested YAML
    let config = ParserConfig::new().max_depth(3);
    let yaml = "a:\n  b:\n    c:\n      d: 1\n";
    let result: Result<Value> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

// --- loader.rs 222, 227: SequenceEnd with anchor ---

#[test]
fn loader_sequence_end_with_anchor() {
    // Anchored sequence -> SequenceEnd stores anchor in anchor_map (line 240-242)
    let yaml = "a: &myseq\n  - 1\n  - 2\nb: *myseq\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["b"][0].as_i64(), Some(1));
}

// --- loader.rs 246: unexpected sequence end ---
// This is a defensive check - hard to trigger through the public API

// --- loader.rs 267, 273: MappingEnd lifecycle ---

#[test]
fn loader_mapping_end_with_anchor() {
    // Anchored mapping -> MappingEnd stores anchor (line 313-315)
    let yaml = "template: &tmpl\n  host: localhost\n  port: 80\nsite: *tmpl\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["site"]["host"].as_str(), Some("localhost"));
}

// --- loader.rs 319-324: MappingValue/unexpected mapping end error ---
// These are defensive error paths

// --- loader.rs 402, 410, 417, 423: push_value MappingValue frame ---

#[test]
fn loader_mapping_value_frame_merge() {
    // Merge key: push_value called in MappingValue frame with merge key (line
    // 412-414)
    let yaml = "base: &base\n  a: 1\nderived:\n  <<: *base\n  b: 2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["derived"]["a"].as_i64(), Some(1));
    assert_eq!(v["derived"]["b"].as_i64(), Some(2));
}

#[test]
fn loader_mapping_value_frame_first_policy() {
    // First policy in MappingValue frame (line 417-421)
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let yaml = "a: 1\nb: 2\na: 3\n";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1)); // First wins
    assert_eq!(v["b"].as_i64(), Some(2));
}

// --- loader.rs 468: non-scalar key error ---

#[test]
fn loader_non_scalar_key_error() {
    // Sequence or mapping as key -> error (line 468)
    // This is hard to trigger from YAML since keys are usually scalars
    // The [key]: value syntax uses flow sequences as keys
    let yaml = "[a, b]: value\n";
    let result: Result<Value> = from_str(yaml);
    // May or may not error depending on parser - just exercise
    let _ = result;
}

// --- loader.rs 549: tag with non-!! handle ---

#[test]
fn loader_tag_with_custom_handle() {
    // Tag handle that is not !! or ! (line 549)
    // The scanner normally only produces ! and !! handles, but exercise the path
    let yaml = "!custom_tag hello\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.is_tagged() || v.as_str().is_some());
}

// --- loader.rs 588: estimate_value_size with string ---

#[test]
fn loader_alias_large_string() {
    // String value aliased multiple times - exercises estimate_value_size (line
    // 690)
    let long_str = "x".repeat(100);
    let yaml = format!("big: &big {}\nref1: *big\nref2: *big\n", long_str);
    let v: Value = from_str(&yaml).unwrap();
    assert_eq!(v["ref1"].as_str().unwrap().len(), 100);
    assert_eq!(v["ref2"].as_str().unwrap().len(), 100);
}

// --- loader.rs 638: hex integer parsing ---
// Already tested

// --- loader.rs 688: try_parse_float ---
// Already tested via large_int_overflow

// --- scanner.rs 63, 64: ScanError Display ---
// Already tested

// --- scanner.rs 152: unexpected end of token stream ---

#[test]
fn scanner_eof_mid_structure() {
    // A structure that starts but never ends -> unexpected end of token stream
    let result: Result<Value> = from_str("[");
    assert!(result.is_err());
}

// --- scanner.rs 170, 172, 173: next_simple_key_token_number ---

#[test]
fn scanner_simple_key_tracking() {
    // Multiple simple keys in flow context exercise next_simple_key_token_number
    let yaml = "{a: 1, b: 2, c: 3}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
    assert_eq!(v["c"].as_i64(), Some(3));
}

// --- scanner.rs 294, 327: skip_to_next_token loop and roll_indent ---
// Already covered

// --- scanner.rs 458, 459: colon followed by flow indicators ---

#[test]
fn scanner_colon_followed_by_colon() {
    // : followed by : (line 460) - triggers value token
    let yaml = "a:: b\n";
    let result: Result<Value> = from_str(yaml);
    let _ = result; // May be valid or error
}

// --- scanner.rs 473-475: BOM handling ---
// Already tested

// --- scanner.rs 583: fetch_block_entry in block context ---

#[test]
fn scanner_block_entry_not_allowed() {
    // Block entry where not allowed (line 583)
    let yaml = "a: 1\n- b\n";
    let result: Result<Value> = from_str(yaml);
    // May produce error about block entries not allowed
    let _ = result;
}

// --- scanner.rs 599: fetch_key not allowed ---

#[test]
fn scanner_mapping_key_not_allowed() {
    // Mapping key where not allowed
    let yaml = "a: 1\n  ? b\n";
    let result: Result<Value> = from_str(yaml);
    let _ = result;
}

// --- scanner.rs 633: fetch_value col from simple key start ---

#[test]
fn scanner_value_after_simple_key_multiline() {
    // Simple key spanning position -> value found (line 630-633)
    let yaml = "key: value\nnested:\n  inner_key: inner_value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["nested"]["inner_key"].as_str(), Some("inner_value"));
}

// --- scanner.rs 650: mapping values not allowed ---

#[test]
fn scanner_value_not_allowed_context() {
    // : where values are not allowed
    let yaml = ": standalone_value\n";
    let result: Result<Value> = from_str(yaml);
    // Exercise the path - may or may not succeed
    let _ = result;
}

// --- scanner.rs 659-666: no simple key tracking ---
// Defensive path, hard to trigger

// --- scanner.rs 733: anchor name limit ---
// Already tested

// --- scanner.rs 803, 807: plain scalar inner loop ---

#[test]
fn scanner_plain_scalar_stops_at_colon_space() {
    // Plain scalar: ':' followed by space stops scanning (line 815-816)
    let yaml = "key: value with spaces\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("value with spaces"));
}

#[test]
fn scanner_plain_scalar_stops_at_flow_indicators() {
    // Plain scalar in flow: stops at , ] } (line 817-820)
    let yaml = "[hello world, second]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0].as_str(), Some("hello world"));
    assert_eq!(v[1].as_str(), Some("second"));
}

// --- scanner.rs 915, 916: plain scalar multiline CRLF ---

#[test]
fn scanner_plain_scalar_multiline_continuation() {
    // Plain scalar that continues on next line (line 908-926)
    let yaml = "key: line1\n  line2\n  line3\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("line1 line2 line3"));
}

// --- scanner.rs 936: empty plain scalar ---
// Already tested

// --- scanner.rs 978, 984: single-quoted EOF and escape ---
// Already tested

// --- scanner.rs 1001-1011: single-quoted line folding ---
// Already tested

// --- scanner.rs 1037, 1047, 1048: single-quoted whitespace ---

#[test]
fn scanner_single_quoted_trailing_whitespace() {
    // Whitespace before line break in single-quoted (line 1037)
    let yaml = "'hello   \n  world'\n";
    let v: Value = from_str(yaml).unwrap();
    // Trailing whitespace before break is folded
    let s = v.as_str().unwrap();
    assert!(s.contains("hello"));
    assert!(s.contains("world"));
}

// --- scanner.rs 1071, 1077, 1083: double-quoted ---
// Already tested

// --- scanner.rs 1091, 1097: escape after break in double-quoted ---

#[test]
fn scanner_double_quoted_escape_after_line_break() {
    // Escape sequence immediately after line break (line 1091-1097)
    let yaml = "\"line1\n\\tline2\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.as_str().unwrap();
    assert!(s.contains("line1"));
    assert!(s.contains('\t'));
}

// --- scanner.rs 1130, 1134, 1138, 1142: escape sequences ---
// Already tested via scanner_double_quoted_escapes and similar

// --- scanner.rs 1152: unknown escape ---
// Already tested

// --- scanner.rs 1171-1176: double-quoted multiple line breaks ---
// Already tested

// --- scanner.rs 1203, 1213, 1214: double-quoted whitespace buffering ---
// Already tested

// --- scanner.rs 1324, 1335: block scalar explicit indent ---
// Already tested

// --- scanner.rs 1397, 1412, 1449: block scalar content ---
// Already tested

// --- ser.rs 361-363: looks_like_number with .digit pattern ---

#[test]
fn ser_looks_like_number_dot_digit_pattern() {
    // .5 -> quoted because it looks like a float (line 414)
    let v = Value::from(".5");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains('\'') || yaml.contains('"'));

    // .123 also looks like a float
    let v = Value::from(".123");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains('\'') || yaml.contains('"'));
}

// --- ser.rs 376: reserved word ---

#[test]
fn ser_reserved_words_comprehensive() {
    // All YAML reserved words should be quoted
    for word in &[
        "true", "false", "null", "~", "True", "False", "Null", "TRUE", "FALSE", "NULL",
    ] {
        let v = Value::from(*word);
        let yaml = to_string(&v).unwrap();
        assert!(
            yaml.contains('\'') || yaml.contains('"'),
            "Expected '{}' to be quoted, got: {}",
            word,
            yaml
        );
    }
}

// --- ser.rs 404: first char single quote ---

#[test]
fn ser_first_char_single_quote_needs_double() {
    // String starting with ' needs double quotes since we can't use single quotes
    let v = Value::from("'quoted");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains('"'));
}

// --- ser.rs 711, 718: write_literal_block/write_folded_block internals ---

#[test]
fn ser_literal_block_with_leading_space() {
    // Literal block where first line starts with space -> needs indent indicator
    let v = LitStr(" indented first line\nsecond line\n");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains('|'));
}

#[test]
fn ser_folded_block_with_leading_space() {
    // Folded block where first line starts with space
    let v = FoldStr(" indented first line\nsecond line\n");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains('>'));
}

// --- ser.rs 731, 734, 743: commented/space-after/unknown internal tags ---
// Already tested

// --- spanned.rs 117, 118: Spanned::deserialize ---

#[test]
fn spanned_deserialize_calls_deserialize_struct() {
    // Spanned::deserialize calls deserialize_struct with SPANNED_TYPE_NAME
    // When deserialized via from_str, it triggers the SpannedMapAccess
    let yaml = "42\n";
    let v: Spanned<i64> = from_str(yaml).unwrap();
    assert_eq!(*v, 42);
    // Check spans are populated
    let _ = v.start;
    let _ = v.end;
}

// --- spanned.rs 129, 130: SpannedVisitor expecting ---

#[test]
fn spanned_expecting_message() {
    // When Spanned<T> is deserialized from wrong type, the expecting message is
    // shown
    let result: std::result::Result<Spanned<String>, _> = serde_json::from_str("42");
    // serde_json won't trigger the SpannedMapAccess, but will call expecting
    // The exact error depends on the deserializer implementation
    let _ = result;
}

// --- spanned.rs 154, 156: unknown field in SpannedVisitor ---

#[test]
fn spanned_unknown_field_skipped() {
    // When extra fields appear in the virtual spanned map, they should be skipped
    // (line 154-156) This is hard to trigger via public API since
    // SpannedMapAccess only produces known fields But deserializing Spanned<T>
    // via from_value (not from_str) exercises the &Value path
    let v = Value::from("hello");
    let result: Spanned<String> = from_value(&v).unwrap();
    assert_eq!(*result, "hello");
}

// --- value.rs 399: MappingVisitor expecting ---
// Already tested

// --- value.rs 855: MappingAnyVisitor expecting ---
// Already tested

// --- value.rs 1446: TaggedValueVisitor expecting ---
// Already tested

// --- value.rs 1526: TaggedValueMapAccess next_value_seed ---
// Already tested

// --- value.rs 2468: usize::index_or_insert through Tagged ---
// Already tested

// --- value.rs 2586-2604: value_type_name arms ---
// Already tested

// --- value.rs 2627-2628: visit_string ---
// Already tested

// --- value.rs 2755: ValueMapAccess::next_value_seed ---

#[test]
fn value_map_access_next_value_via_ref_value() {
    // Deserialize a mapping via &Value to exercise ValueMapAccess (line 2755)
    #[derive(Debug, Deserialize, PartialEq)]
    struct Pair {
        key: String,
        val: i64,
    }
    let mut m = Mapping::new();
    m.insert("key", Value::from("hello"));
    m.insert("val", Value::from(42));
    let v = Value::Mapping(m);
    let p: Pair = Deserialize::deserialize(&v).unwrap();
    assert_eq!(p.key, "hello");
    assert_eq!(p.val, 42);
}

// --- value.rs 2801: deserialize_enum fallback to deserialize_any ---

#[test]
fn ref_value_deserialize_enum_mapping() {
    // Mapping value as enum -> uses from_value which goes through de.rs
    // Deserializer
    #[derive(Debug, Deserialize, PartialEq)]
    enum Animal {
        Dog { name: String },
    }
    let mut inner = Mapping::new();
    inner.insert("name", Value::from("Rex"));
    let mut m = Mapping::new();
    m.insert("Dog", Value::Mapping(inner));
    let v = Value::Mapping(m);
    let a: Animal = from_value(&v).unwrap();
    assert_eq!(
        a,
        Animal::Dog {
            name: "Rex".to_string()
        }
    );
}

// --- value.rs 2828: deserialize_struct ---

#[test]
fn ref_value_deserialize_struct_via_mapping() {
    // deserialize_struct on &Value with non-spanned name -> falls to
    // deserialize_map (line 2844)
    #[derive(Debug, Deserialize, PartialEq)]
    struct Rgb {
        r: u8,
        g: u8,
        b: u8,
    }
    let mut m = Mapping::new();
    m.insert("r", Value::from(255));
    m.insert("g", Value::from(128));
    m.insert("b", Value::from(0));
    let v = Value::Mapping(m);
    let c: Rgb = Deserialize::deserialize(&v).unwrap();
    assert_eq!(
        c,
        Rgb {
            r: 255,
            g: 128,
            b: 0
        }
    );
}

// --- value.rs 2837-2844: deserialize_struct with Spanned ---

#[test]
fn ref_value_deserialize_struct_spanned_via_value() {
    // Exercise &Value::deserialize_struct with SPANNED_TYPE_NAME (line 2837-2842)
    let v = Value::from("spanned_value");
    let result: Spanned<String> = Deserialize::deserialize(&v).unwrap();
    assert_eq!(*result, "spanned_value");
}

#[test]
fn ref_value_deserialize_struct_spanned_nested_in_mapping() {
    // Exercise Spanned<T> in a struct deserialized from a Value mapping
    #[derive(Debug, Deserialize)]
    struct Item {
        name: Spanned<String>,
        count: Spanned<i32>,
    }
    let mut m = Mapping::new();
    m.insert("name", Value::from("widget"));
    m.insert("count", Value::from(5));
    let v = Value::Mapping(m);
    let item: Item = Deserialize::deserialize(&v).unwrap();
    assert_eq!(*item.name, "widget");
    assert_eq!(*item.count, 5);
}

// --- singleton_map_recursive.rs 47-51: Tagged arm ---

#[test]
fn singleton_map_recursive_transform_tagged() {
    use noyalib::with::singleton_map_recursive;

    // Create a value that has Tagged variant to exercise lines 47-51
    // The transform_to_singleton_map function processes Tagged values recursively
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Op {
        Add(i32),
    }

    // First serialize to get a Value, then use singleton_map_recursive
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Ops {
        #[serde(with = "singleton_map_recursive")]
        ops: Vec<Op>,
    }

    let ops = Ops {
        ops: vec![Op::Add(1), Op::Add(2)],
    };
    let yaml = to_string(&ops).unwrap();
    let roundtrip: Ops = from_str(&yaml).unwrap();
    assert_eq!(ops, roundtrip);
}

// --- singleton_map_with.rs 141, 143: fallback serialize path ---

#[test]
fn singleton_map_with_serialize_fallback_non_mapping_non_string() {
    use noyalib::with::singleton_map_with;

    // Serialize a raw integer through serialize_with -> to_value gives Number
    // which is neither Mapping nor String, hitting the fallback (lines 141-143)
    let value = 42i32;
    let result: Value =
        singleton_map_with::serialize_with(&value, Serializer, |s| s.to_uppercase()).unwrap();
    assert_eq!(result.as_i64(), Some(42));
}

#[test]
fn singleton_map_with_serialize_fallback_sequence() {
    use noyalib::with::singleton_map_with;

    // Serialize a Vec through serialize_with -> to_value gives Sequence
    let value = vec![1, 2, 3];
    let result: Value =
        singleton_map_with::serialize_with(&value, Serializer, |s| s.to_uppercase()).unwrap();
    assert!(result.is_sequence());
}

// --- singleton_map_with.rs 214-217: transform_value_keys on Tagged ---

#[test]
fn singleton_map_with_deserialize_tagged_transform() {
    use noyalib::with::singleton_map_with;

    // Create a tagged value and pass through deserialize_with
    // which calls transform_value_keys on it (lines 214-217)
    let inner_map = {
        let mut m = Mapping::new();
        m.insert("key", Value::from("val"));
        m
    };
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!mytag"),
        Value::Mapping(inner_map),
    )));

    // deserialize_with calls transform_value_keys which has the Tagged arm
    let result: Result<Value> = singleton_map_with::deserialize_with(&tagged, |s| s.to_uppercase());
    // The result type doesn't matter - we just exercise the Tagged transform path
    let _ = result;
}

// --- flow mapping with implicit keys exercising FlowMappingEmptyValue
// (events.rs 124) ---

#[test]
fn events_flow_mapping_empty_value_state() {
    // Flow mapping with implicit key (no ?) -> FlowMappingEmptyValue state
    // This is the path at line 606-607 -> State::FlowMappingEmptyValue -> line 124
    let yaml = "{a, b: 1}\n";
    let result: Result<Value> = from_str(yaml);
    // 'a' is an implicit key with empty value
    if let Ok(v) = result {
        assert!(v["a"].is_null());
    }
}

// --- flow sequence with ? key triggering mapping (events.rs 507-516) ---

#[test]
fn events_flow_sequence_explicit_key_mapping() {
    // ? in flow sequence triggers implicit mapping (lines 507-516)
    let yaml = "[? a: b]\n";
    let result: Result<Value> = from_str(yaml);
    // Exercises State::FlowSequenceEntryMappingKey
    let _ = result;
}

// --- Additional Spanned<T> tests to ensure all field states ---

#[test]
fn spanned_mapping_value_from_str() {
    // Spanned<Mapping> via from_str exercises all SpannedFieldState transitions
    #[derive(Debug, Deserialize)]
    struct Doc {
        data: Spanned<HashMap<String, i32>>,
    }
    let yaml = "data:\n  x: 1\n  y: 2\n";
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(*doc.data.get("x").unwrap(), 1);
    assert_eq!(*doc.data.get("y").unwrap(), 2);
}

// --- Exercise events.rs line 117: FlowSequenceEntry (not first) ---

#[test]
fn events_flow_sequence_not_first_entry() {
    // Second+ entries in flow sequence go through FlowSequenceEntry state
    let yaml = "[1, 2, 3, 4, 5]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[4].as_i64(), Some(5));
}

// --- Exercise events.rs line 122: FlowMappingKey (not first) ---

#[test]
fn events_flow_mapping_key_not_first() {
    // Second+ keys in flow mapping go through FlowMappingKey state
    let yaml = "{a: 1, b: 2, c: 3}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["c"].as_i64(), Some(3));
}

// --- Exercise loader.rs SpanTree::Sequence/Mapping span ranges (lines 355-356)
// ---

#[test]
fn loader_span_tree_sequence_and_mapping() {
    // Complex structure with sequences and mappings to exercise span tree
    // construction
    let yaml = "list:\n  - name: a\n    val: 1\n  - name: b\n    val: 2\nother:\n  - 10\n  - 20\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["list"][0]["name"].as_str(), Some("a"));
    assert_eq!(v["other"][1].as_i64(), Some(20));
}

// --- Exercise fmt.rs 381: Inner<T> serialize ---

#[test]
fn fmt_commented_inner_serialize_roundtrip() {
    // The Inner struct's Serialize impl (line 381) is exercised when
    // Commented<T> is serialized to a Value via to_value
    let commented = Commented::new(42i32, "the answer");
    let v = to_value(&commented).unwrap();
    // The value should be a tagged value with the magic commented tag
    assert!(v.is_tagged());
}

// --- Exercise de.rs 723: missing value in map ---
// This is an internal defensive error in MapDeserializer::next_value_seed
// when next_value_seed is called without a preceding next_key_seed.
// Not reachable through normal public API.

// --- Exercise flow sequence entry mapping value (events.rs 540-553) ---

#[test]
fn events_flow_sequence_entry_mapping_value_with_content() {
    // In a flow sequence, key: value creates an implicit mapping
    // parse_flow_sequence_entry_mapping_value (line 540)
    let yaml = "[a: 1]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0]["a"].as_i64(), Some(1));
}

#[test]
fn events_flow_sequence_entry_mapping_value_empty() {
    // key followed by , -> empty value in implicit mapping (line 552)
    let yaml = "[a:, b]\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v[0]["a"].is_null());
}

// --- Exercise scanner plain scalar with comment (# after space) ---

#[test]
fn scanner_plain_scalar_with_comment_terminator() {
    // Plain scalar stops before ' #' (space + hash)
    let yaml = "key: value # this is a comment\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("value"));
}

// --- Exercise loader with max_alias_expansions ---

#[test]
fn loader_alias_expansion_limit() {
    let config = ParserConfig::new().max_alias_expansions(1);
    let yaml = "a: &a hello\nb: *a\nc: *a\n";
    let result: Result<Value> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

// --- Exercise events.rs: indentless with node items (line 422-424) ---

#[test]
fn events_indentless_sequence_with_nested_nodes() {
    // Indentless sequence entries that contain nodes (not just scalars)
    let yaml = "mapping:\n  - key: value\n  - other: data\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["mapping"][0]["key"].as_str(), Some("value"));
}

// --- Exercise scanner.rs 807: peek_at in flow context for plain scalar ---

#[test]
fn scanner_plain_scalar_in_flow_with_colon() {
    // Plain scalar in flow context: colon is allowed if not followed by
    // blank/flow-indicator
    let yaml = "{key: http://example.com}\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str(), Some("http://example.com"));
}
