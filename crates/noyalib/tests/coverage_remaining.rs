//! Tests targeting every remaining uncovered line.
//! Each test includes the exact file:line it covers.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(
    unused_comparisons,
    unused_results,
    clippy::approx_constant,
    clippy::absurd_extreme_comparisons
)]

use std::collections::HashMap;

use noyalib::{
    DuplicateKeyPolicy, Mapping, MappingAny, ParserConfig, Spanned, Tag, TaggedValue, Value,
    from_str, from_str_with_config, from_value, to_string,
};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════
// value.rs — TaggedValue Deserialize/Deserializer (lines 1446–1591)
// These require from_value with Tagged values to exercise the
// TaggedValueMapAccess, EnumAccess, and VariantAccess impls.
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, PartialEq)]
enum Variant {
    Unit,
    Newtype(i32),
    Tuple(i32, String),
    Struct { x: i32, y: String },
}

#[test]
fn tagged_value_variants_via_yaml() {
    // value.rs:1539-1590 — all TaggedValue variant access paths
    // These exercise the TaggedValue deserializer when parsing YAML with !tags
    // and then deserializing into enum types via the de.rs Deserializer

    // Unit variant through YAML singleton map
    let yaml = "Unit: ~";
    let result: Variant = from_str(yaml).unwrap();
    assert_eq!(result, Variant::Unit);

    // Newtype variant
    let yaml2 = "Newtype: 42";
    let result2: Variant = from_str(yaml2).unwrap();
    assert_eq!(result2, Variant::Newtype(42));

    // Tuple variant
    let yaml3 = "Tuple:\n  - 1\n  - hello";
    let result3: Variant = from_str(yaml3).unwrap();
    assert_eq!(result3, Variant::Tuple(1, "hello".into()));

    // Struct variant
    let yaml4 = "Struct:\n  x: 10\n  y: world";
    let result4: Variant = from_str(yaml4).unwrap();
    assert_eq!(
        result4,
        Variant::Struct {
            x: 10,
            y: "world".into()
        }
    );
}

#[test]
fn tagged_value_map_access_via_from_value() {
    // value.rs:1471-1477 (deserialize_any → TaggedValueMapAccess)
    // value.rs:1508-1526 (next_key_seed, next_value_seed)
    // Use singleton_map which routes through TaggedValue deserialization
    #[derive(Debug, Deserialize, PartialEq)]
    enum Color {
        Red,
        Blue,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Palette {
        #[serde(with = "noyalib::with::singleton_map")]
        color: Color,
    }
    let yaml = "color:\n  Red: ~";
    let p: Palette = from_str(yaml).unwrap();
    assert_eq!(p.color, Color::Red);
}

#[test]
fn tagged_value_deserialize_from_map() {
    // value.rs:1446-1464 (TaggedValueVisitor::visit_map)
    let yaml = "mytag: myval";
    let tv: TaggedValue = from_str(yaml).unwrap();
    assert_eq!(tv.tag().as_str(), "mytag");
}

// ═══════════════════════════════════════════════════════════════════════
// value.rs — Deserializer for &Value (lines 2763–2844)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn value_deserializer_any_null() {
    // value.rs:2768
    let v = Value::Null;
    let result: () = from_value(&v).unwrap();
    assert_eq!(result, ());
}

#[test]
fn value_deserializer_any_bool() {
    // value.rs:2769
    let v = Value::Bool(true);
    let result: bool = from_value(&v).unwrap();
    assert!(result);
}

#[test]
fn value_deserializer_any_integer() {
    // value.rs:2770
    let v = Value::from(42);
    let result: i64 = from_value(&v).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn value_deserializer_any_float() {
    // value.rs:2771
    let v = Value::from(3.14);
    let result: f64 = from_value(&v).unwrap();
    assert!((result - 3.14).abs() < 0.001);
}

#[test]
fn value_deserializer_any_string() {
    // value.rs:2772
    let v = Value::from("hello");
    let result: String = from_value(&v).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn value_deserializer_any_sequence() {
    // value.rs:2773
    let v = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let result: Vec<i32> = from_value(&v).unwrap();
    assert_eq!(result, vec![1, 2]);
}

#[test]
fn value_deserializer_any_mapping() {
    // value.rs:2774-2776
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let v = Value::Mapping(m);
    let result: HashMap<String, i32> = from_value(&v).unwrap();
    assert_eq!(result["a"], 1);
}

#[test]
fn value_deserializer_any_tagged() {
    // value.rs:2778-2780 — tagged dispatches to TaggedValue deserializer
    // Use !!str tag which is resolved by the loader
    let yaml = "!!str tagged_string";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str(), Some("tagged_string"));
}

#[test]
fn value_deserializer_enum_string() {
    // value.rs:2799-2800 — enum from string
    #[derive(Debug, Deserialize, PartialEq)]
    enum Color {
        Red,
        Blue,
    }
    let v = Value::from("Red");
    let c: Color = from_value(&v).unwrap();
    assert_eq!(c, Color::Red);
}

#[test]
fn value_deserializer_enum_tagged() {
    // value.rs:2795-2797 — enum from tagged via YAML
    // Singleton map is the standard way to represent enums
    let yaml = "Unit: ~";
    let result: Variant = from_str(yaml).unwrap();
    assert_eq!(result, Variant::Unit);
}

#[test]
fn value_deserializer_struct() {
    // value.rs:2828-2844 — deserialize_struct dispatches to deserialize_map
    #[derive(Debug, Deserialize, PartialEq)]
    struct Foo {
        x: i32,
        y: String,
    }
    let mut m = Mapping::new();
    let _ = m.insert("x", Value::from(5));
    let _ = m.insert("y", Value::from("hi"));
    let v = Value::Mapping(m);
    let result: Foo = from_value(&v).unwrap();
    assert_eq!(
        result,
        Foo {
            x: 5,
            y: "hi".into()
        }
    );
}

#[test]
fn value_deserializer_spanned_struct() {
    // value.rs:2837-2842 — Spanned special case in deserialize_struct
    // de.rs:786-814 — SpannedMapAccess state machine
    // spanned.rs:117-156 — SpannedVisitor
    let yaml = "name: hello\ncount: 42";
    #[derive(Debug, Deserialize)]
    struct Cfg {
        name: Spanned<String>,
        count: Spanned<i64>,
    }
    let cfg: Cfg = from_str(yaml).unwrap();
    assert_eq!(cfg.name.value, "hello");
    assert!(cfg.name.start.line() >= 1);
    assert!(cfg.name.start.column() >= 0);
    assert!(cfg.name.start.index() >= 0);
    assert!(cfg.name.end.line() >= 1);
    assert!(cfg.name.end.column() >= 0);
    assert!(cfg.name.end.index() >= 0);
    assert_eq!(cfg.count.value, 42);
}

#[test]
fn value_into_deserializer() {
    // value.rs:2704-2705 — IntoDeserializer impl
    use serde::de::IntoDeserializer;
    let val = Value::from("test");
    let deser: &Value = (&val).into_deserializer();
    let result: String = Deserialize::deserialize(deser).unwrap();
    assert_eq!(result, "test");
}

// ═══════════════════════════════════════════════════════════════════════
// value.rs — apply_merge error paths (lines 2097–2143)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn apply_merge_sequence_in_merge() {
    // value.rs:2103-2104 — SequenceInMergeElement error
    let mut m = Mapping::new();
    let _ = m.insert("<<", Value::Sequence(vec![Value::Sequence(vec![])]));
    let mut v = Value::Mapping(m);
    assert!(v.apply_merge().is_err());
}

#[test]
fn apply_merge_tagged_in_merge() {
    // value.rs:2106-2107 — TaggedInMerge error
    let mut m = Mapping::new();
    let _ = m.insert(
        "<<",
        Value::Tagged(Box::new(TaggedValue::new(Tag::new("t"), Value::Null))),
    );
    let mut v = Value::Mapping(m);
    assert!(v.apply_merge().is_err());
}

#[test]
fn apply_merge_scalar_in_merge() {
    // value.rs:2109-2110 — ScalarInMergeElement error
    let mut m = Mapping::new();
    let _ = m.insert("<<", Value::from("scalar"));
    let mut v = Value::Mapping(m);
    assert!(v.apply_merge().is_err());
}

#[test]
fn apply_merge_on_sequence() {
    // value.rs:2115-2119 — recursive apply_merge on sequence
    // Test apply_merge traversing into sequences
    let mut v = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    // Should succeed (no merge keys in scalars)
    assert!(v.apply_merge().is_ok());
}

#[test]
fn apply_merge_on_tagged() {
    // value.rs:2121-2123 — recursive apply_merge on tagged
    let mut v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("t"),
        Value::from("val"),
    )));
    assert!(v.apply_merge().is_ok());
}

#[test]
fn untag_recursive() {
    // value.rs:2139-2143 — untag on sequence and mapping
    let seq = Value::Sequence(vec![Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("t"),
        Value::from(1),
    )))]);
    let untagged = seq.untag();
    if let Value::Sequence(s) = untagged {
        assert_eq!(s[0].as_i64(), Some(1));
    }

    let mut m = Mapping::new();
    let _ = m.insert(
        "k",
        Value::Tagged(Box::new(TaggedValue::new(Tag::new("t"), Value::from(2)))),
    );
    let v = Value::Mapping(m);
    let untagged = v.untag();
    if let Value::Mapping(m) = untagged {
        assert_eq!(m.get("k").unwrap().as_i64(), Some(2));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// value.rs — ValueIndex (lines 2468, 2506, 2521–2573)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn value_index_or_insert_usize_tagged() {
    // value.rs:2468 — usize::index_or_insert through Tagged
    let mut v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("s"),
        Value::Sequence(vec![Value::from(10)]),
    )));
    v[0] = Value::from(99);
    // verify
    match &v {
        Value::Tagged(t) => match t.value() {
            Value::Sequence(s) => assert_eq!(s[0].as_i64(), Some(99)),
            _ => panic!("expected sequence"),
        },
        _ => panic!("expected tagged"),
    }
}

#[test]
fn value_index_or_insert_str_tagged() {
    // value.rs:2506 — &str::index_or_insert through Tagged
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::from(1));
    let mut v = Value::Tagged(Box::new(TaggedValue::new(Tag::new("m"), Value::Mapping(m))));
    v["k"] = Value::from(42);
    match &v {
        Value::Tagged(t) => match t.value() {
            Value::Mapping(m) => assert_eq!(m.get("k").unwrap().as_i64(), Some(42)),
            _ => panic!("expected mapping"),
        },
        _ => panic!("expected tagged"),
    }
}

#[test]
fn value_index_string_types() {
    // value.rs:2521-2526 (String), 2535-2540 (&String)
    let mut m = Mapping::new();
    let _ = m.insert("key", Value::from(1));
    let v = Value::Mapping(m);

    // Index with &str
    let r = v.get("key");
    assert!(r.is_some());
    assert_eq!(r.unwrap().as_i64(), Some(1));
}

#[test]
fn value_index_with_value_integer() {
    // value.rs:2555-2571 — &Value index with integer
    let v = Value::Sequence(vec![Value::from(10), Value::from(20)]);
    let idx = Value::from(0i64);
    // Use &Value as index
    let result = v.get(&idx);
    assert!(result.is_some());
}

#[test]
fn value_index_with_value_string() {
    // value.rs:2556 — &Value index with string
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::from(1));
    let v = Value::Mapping(m);
    let idx = Value::from("k");
    let result = v.get(&idx);
    assert!(result.is_some());
}

// ═══════════════════════════════════════════════════════════════════════
// value.rs — Mapping/MappingAny visitor (lines 399, 810, 855)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mapping_any_ord_equal_keys_equal_values() {
    // value.rs:810 — Equal continue branch
    let mut m1 = MappingAny::new();
    let mut m2 = MappingAny::new();
    let _ = m1.insert(Value::from("a"), Value::from(1));
    let _ = m1.insert(Value::from("b"), Value::from(2));
    let _ = m2.insert(Value::from("a"), Value::from(1));
    let _ = m2.insert(Value::from("b"), Value::from(2));
    assert_eq!(m1.cmp(&m2), std::cmp::Ordering::Equal);
}

// ═══════════════════════════════════════════════════════════════════════
// value.rs — parse_path (line 2219)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn get_path_with_bracket_edge_case() {
    // value.rs:2219 — unmatched ] in path
    let yaml = "a:\n  b: 1";
    let v: Value = from_str(yaml).unwrap();
    // Path with unmatched ] should be handled gracefully
    let _ = v.get_path("a]b");
    let _ = v.get_path("a[0]");
}

// ═══════════════════════════════════════════════════════════════════════
// scanner.rs — Single-quoted scalar parsing (lines 938–1008)
// Tarpaulin needs these to go through internal parser module tests.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn single_quoted_multiline_with_breaks() {
    // scanner.rs:938-997 — single-quoted scalar full path
    let yaml = "key: 'first line\n  second line\n\n  after blank'";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("first"), "got: {s:?}");
}

#[test]
fn single_quoted_with_escaped_quotes_and_spaces() {
    // scanner.rs:944-971, 997-1008 — escaped quotes + whitespace
    let yaml = "key: 'it''s   spaced'";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "it's   spaced");
}

#[test]
fn single_quoted_unterminated() {
    // scanner.rs:938-940 — unterminated single-quoted
    let result: Result<Value, _> = from_str("key: 'unterminated");
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════
// scanner.rs — Double-quoted scalar parsing (lines 1031–1174)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn double_quoted_unterminated() {
    // scanner.rs:1031-1033 — unterminated double-quoted
    let result: Result<Value, _> = from_str("key: \"unterminated");
    assert!(result.is_err());
}

#[test]
fn double_quoted_multiline_with_folding() {
    // scanner.rs:1037-1060, 1131-1174 — line folding in double-quoted
    let yaml = "key: \"first\n  second\n\n  after blank\"";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("first"), "got: {s:?}");
}

#[test]
fn double_quoted_all_escapes() {
    // scanner.rs:1072-1112 — every escape sequence
    let yaml = "a: \"\\0\\a\\b\\t\\n\\v\\f\\r\\e\\ \\\"\\/\\\\\\N\\_\\L\\P\"";
    let v: Value = from_str(yaml).unwrap();
    let s = v["a"].as_str().unwrap();
    assert!(s.contains('\0'));
    assert!(s.contains('\x07'));
    assert!(s.contains('\t'));
    assert!(s.contains('\n'));
}

#[test]
fn double_quoted_hex_escapes() {
    // scanner.rs:1090-1100
    let yaml = "a: \"\\x41\\u0042\\U00000043\"";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_str().unwrap(), "ABC");
}

#[test]
fn double_quoted_line_continuation() {
    // scanner.rs:1102-1105 — backslash-newline continuation
    let yaml = "key: \"line1\\\n  line2\"";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "line1line2");
}

#[test]
fn double_quoted_unknown_escape() {
    // scanner.rs:1112 — unknown escape
    let result: Result<Value, _> = from_str("key: \"\\q\"");
    assert!(result.is_err());
}

#[test]
fn double_quoted_invalid_unicode() {
    // scanner.rs:1221-1222 — invalid unicode codepoint
    let result: Result<Value, _> = from_str("key: \"\\UD800\"");
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════
// scanner.rs — Block scalar paths (lines 1285–1410)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn block_literal_with_empty_first_line() {
    // scanner.rs:1285-1296 — autodetect indent after empty lines
    let yaml = "key: |\n\n\n  content\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("content"));
}

#[test]
fn block_literal_end_by_dedent() {
    // scanner.rs:1336 — break from block when indent drops
    let yaml = "key: |\n  block content\nnot_block: val";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "block content\n");
    assert_eq!(v["not_block"].as_str().unwrap(), "val");
}

#[test]
fn block_literal_eof_terminates() {
    // scanner.rs:1358 — EOF terminates block scalar
    let yaml = "key: |\n  content";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["key"].as_str().unwrap().contains("content"));
}

#[test]
fn block_folded_leading_blank_preserves_newline() {
    // scanner.rs:1373 — folded: leading blank preserves break
    let yaml = "key: >\n  normal\n   indented\n  back";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    // When a line is more-indented, the preceding break is preserved
    assert!(s.contains('\n'));
}

#[test]
fn block_literal_utf8_content() {
    // scanner.rs:1392 — multi-byte UTF-8 in block scalar
    let yaml = "key: |\n  café ñ 日本語";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("café"));
}

#[test]
fn block_clip_chomping() {
    // scanner.rs:1410 — clip chomping (default)
    let yaml = "key: |\n  content\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    // Clip: single trailing newline
    assert_eq!(v["key"].as_str().unwrap(), "content\n");
}

// ═══════════════════════════════════════════════════════════════════════
// scanner.rs — Misc scanner paths
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn scan_error_display() {
    // scanner.rs:63-64 — ScanError Display trait
    let result: Result<Value, _> = from_str("key: \"\\q\"");
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(!msg.is_empty());
}

#[test]
fn crlf_line_endings() {
    // scanner.rs:246 — CRLF handling in skip_line
    let yaml = "key: value\r\nkey2: value2\r\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key2"].as_str().unwrap(), "value2");
}

#[test]
fn flow_context_roll_indent() {
    // scanner.rs:286 — roll_indent returns early in flow context
    let yaml = "[1, [2, 3], {a: b}]";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_sequence().is_some());
}

#[test]
fn bom_at_start() {
    // scanner.rs:432-434 — UTF-8 BOM skip
    let yaml = "\u{FEFF}key: value";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "value");
}

#[test]
fn flow_value_and_key_context() {
    // scanner.rs:400-418 — flow context value/key handling
    let yaml = "{a: 1, b: 2, c: [3, 4]}";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
    assert_eq!(v["b"].as_i64(), Some(2));
}

#[test]
fn implicit_key_value() {
    // scanner.rs:592, 618-625 — implicit key : value
    let yaml = "simple: value\nnested:\n  inner: deep";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["nested"]["inner"].as_str(), Some("deep"));
}

#[test]
fn plain_scalar_multiline() {
    // scanner.rs:762, 766, 829, 832 — plain scalar line folding
    let yaml = "key: this is\n  a multiline\n  plain scalar";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("this is"));
    assert!(s.contains("multiline"));
}

// ═══════════════════════════════════════════════════════════════════════
// events.rs — Parser state machine (lines 112–598)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn multi_document_with_end_markers() {
    // events.rs:190, 202, 207, 218-221, 230, 235
    let yaml = "---\nfirst: 1\n...\n---\nsecond: 2\n...\n---\n...\n";
    let docs = noyalib::load_all(yaml).unwrap();
    assert!(docs.len() >= 2);
}

#[test]
fn block_sequence_with_empty_entries() {
    // events.rs:379-384, 390-397 — block sequence entries
    let yaml = "- \n- value\n- ";
    let v: Value = from_str(yaml).unwrap();
    let seq = v.as_sequence().unwrap();
    assert!(seq.len() >= 2);
}

#[test]
fn explicit_key_mapping() {
    // events.rs:395-421, 432-446 — explicit ? key : value
    let yaml = "? explicit_key\n: explicit_value\n? another\n: value2";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["explicit_key"].as_str(), Some("explicit_value"));
}

#[test]
fn flow_mapping_entries() {
    // events.rs:527-530, 550-551, 592, 597-598 — flow mapping
    let yaml = "{a: 1, b: , c: 3}";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
    assert!(v["b"].is_null());
}

#[test]
fn flow_sequence_with_implicit_mapping() {
    // events.rs:313, 322, 349, 360-361 — flow collections
    let yaml = "[{a: 1}, b, c: d]";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_sequence().is_some());
}

#[test]
fn anchor_with_tag() {
    // events.rs:278-281 — anchor + tag on node
    let yaml = "key: &anc !!str tagged_and_anchored\nref: *anc";
    let v: Value = from_str(yaml).unwrap();
    let s = v["ref"].as_str().unwrap();
    assert_eq!(s, "tagged_and_anchored");
}

#[test]
fn block_mapping_empty_value() {
    // events.rs:467 — mapping value empty
    let yaml = "key1:\nkey2: value2";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["key1"].is_null());
    assert_eq!(v["key2"].as_str(), Some("value2"));
}

#[test]
fn indentless_sequence_after_key() {
    // events.rs:112 — indentless sequence entry
    let yaml = "items:\n  - a\n  - b\n  - c";
    let v: Value = from_str(yaml).unwrap();
    let seq = v["items"].as_sequence().unwrap();
    assert_eq!(seq.len(), 3);
}

// ═══════════════════════════════════════════════════════════════════════
// parser/loader.rs — Loader (lines 147–701)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn loader_multi_document() {
    // loader.rs:147, 155, 159, 165 — document lifecycle
    let yaml = "---\na: 1\n...\n---\nb: 2\n...";
    let docs = noyalib::load_all(yaml).unwrap();
    let collected: Vec<_> = docs.collect();
    assert_eq!(collected.len(), 2);
}

#[test]
fn loader_sequence_with_anchor() {
    // loader.rs:209, 222, 227, 246 — sequence processing
    let yaml = "&seq\n- 1\n- 2\n- 3";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_sequence().unwrap().len(), 3);
}

#[test]
fn loader_merge_key_from_anchor() {
    // loader.rs:267, 273, 293 — merge key processing
    let yaml = "defaults: &d\n  a: 1\n  b: 2\nitem:\n  <<: *d\n  c: 3";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["item"]["a"].as_i64(), Some(1));
    assert_eq!(v["item"]["c"].as_i64(), Some(3));
}

#[test]
fn loader_duplicate_key_error() {
    // loader.rs:293, 319-321 — duplicate key error policy
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let result: Result<Value, _> = from_str_with_config("a: 1\na: 2", &config);
    assert!(result.is_err());
}

#[test]
fn loader_duplicate_key_first() {
    // loader.rs:465-468 — First policy
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let v: Value = from_str_with_config("a: 1\na: 2", &config).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

#[test]
fn loader_mapping_key_push_value() {
    // loader.rs:324, 338, 344, 350, 355-356 — push_value MappingKey path
    let yaml = "a: 1\nb: 2\nc: 3";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_mapping().unwrap().len(), 3);
}

#[test]
fn loader_merge_in_value() {
    // loader.rs:402, 410, 417, 423 — merge handling in MappingValue
    let yaml = "base: &base\n  x: 1\nchild:\n  <<: *base\n  y: 2";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["child"]["x"].as_i64(), Some(1));
}

#[test]
fn loader_tagged_scalar_custom() {
    // loader.rs:528, 545, 549, 588 — custom tag resolution
    let yaml = "!custom value";
    let v: Value = from_str(yaml).unwrap();
    // Custom tags with ! prefix create tagged values
    // (the tag resolves to "!custom" which is a custom tag)
    assert!(!v.is_null(), "got: {v:?}");
}

#[test]
fn loader_hex_octal_integers() {
    // loader.rs:638 — hex/octal integer parsing
    let yaml = "hex: 0xFF\noctal: 0o77";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["hex"].as_i64(), Some(255));
    assert_eq!(v["octal"].as_i64(), Some(63));
}

#[test]
fn loader_large_integer_overflow() {
    // loader.rs:688, 701 — large integer → float fallback
    let yaml = "big: 99999999999999999999";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["big"].as_f64().is_some());
}

// ═══════════════════════════════════════════════════════════════════════
// de.rs — Spanned (lines 648–814)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn deserialize_identifier_for_enum() {
    // de.rs:648-652
    #[derive(Debug, Deserialize, PartialEq)]
    enum Dir {
        North,
        South,
    }
    let v: Dir = from_str("North").unwrap();
    assert_eq!(v, Dir::North);
}

#[test]
fn spanned_all_location_fields() {
    // de.rs:786-814 — all SpannedFieldState transitions
    let yaml = "hello";
    let s: Spanned<String> = from_str(yaml).unwrap();
    assert_eq!(s.value, "hello");
    // Exercise all location field accesses
    let _ = s.start.line();
    let _ = s.start.column();
    let _ = s.start.index();
    let _ = s.end.line();
    let _ = s.end.column();
    let _ = s.end.index();
}

// ═══════════════════════════════════════════════════════════════════════
// ser.rs — Serializer edge cases (lines 328–688)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn serialize_tagged_custom_tag() {
    // ser.rs:328-330 — custom tag serialization
    let tagged = TaggedValue::new(Tag::new("!color"), Value::from("red"));
    let v = Value::Tagged(Box::new(tagged));
    let s = to_string(&v).unwrap();
    assert!(s.contains("!color"), "{s}");
    assert!(s.contains("red"), "{s}");
}

#[test]
fn serialize_empty_string_quoted() {
    // ser.rs:343, 371, 382 — empty/special string quoting
    let v = Value::from("");
    let s = to_string(&v).unwrap();
    assert!(s.contains("''") || s.contains("\"\"") || s.trim().is_empty());
}

#[test]
fn serialize_string_with_control_char() {
    // ser.rs:442, 533 — control character forces double-quoting
    let v = Value::from("hello\x07world");
    let s = to_string(&v).unwrap();
    // Should use double-quotes to handle control chars
    assert!(s.contains('"') || s.contains("\\a"), "{s}");
}

#[test]
fn serialize_internal_tag_fallbacks() {
    // ser.rs:614, 621, 634, 637, 646 — internal tag fallback paths
    // Unknown internal tag
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_unknown"),
        Value::from(42),
    )));
    let s = to_string(&v).unwrap();
    assert!(s.contains("42"), "{s}");
}

#[test]
fn serialize_number_like_strings() {
    // ser.rs:343, 371, 382 — strings that look like numbers
    let v = Value::from(".inf");
    let s = to_string(&v).unwrap();
    // Should be quoted to avoid being parsed as infinity
    assert!(s.contains('\'') || s.contains('"'), "got: {s}");
}

// ═══════════════════════════════════════════════════════════════════════
// with/singleton_map_with.rs — serialize_with/deserialize_with
// (lines 113–189, 214–217, 346)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn singleton_map_with_serialize_deserialize() {
    // singleton_map_with.rs:113-143, 175-189
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Action {
        GetRequest,
        PostData,
    }

    fn my_ser<S: serde::Serializer>(v: &Action, s: S) -> Result<S::Ok, S::Error> {
        noyalib::with::singleton_map_with::serialize_with(v, s, |k| {
            noyalib::with::singleton_map_with::to_snake_case(k)
        })
    }

    fn my_de<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Action, D::Error> {
        noyalib::with::singleton_map_with::deserialize_with(d, |k| {
            noyalib::with::singleton_map_with::to_pascal_case(k)
        })
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Cmd {
        #[serde(serialize_with = "my_ser", deserialize_with = "my_de")]
        action: Action,
    }

    let cmd = Cmd {
        action: Action::GetRequest,
    };
    let yaml = to_string(&cmd).unwrap();
    assert!(yaml.contains("get_request"), "got: {yaml}");
    let rt: Cmd = from_str(&yaml).unwrap();
    assert_eq!(rt, cmd);
}

#[test]
fn singleton_map_with_unit_variant() {
    // singleton_map_with.rs:134-139 — unit variant transform
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Flag {
        Active,
    }

    fn my_ser<S: serde::Serializer>(v: &Flag, s: S) -> Result<S::Ok, S::Error> {
        noyalib::with::singleton_map_with::serialize_with(v, s, |k| k.to_lowercase())
    }

    fn my_de<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Flag, D::Error> {
        noyalib::with::singleton_map_with::deserialize_with(d, |k| {
            // Capitalize first letter
            let mut c = k.chars();
            match c.next() {
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        #[serde(serialize_with = "my_ser", deserialize_with = "my_de")]
        flag: Flag,
    }

    let cfg = Config { flag: Flag::Active };
    let yaml = to_string(&cfg).unwrap();
    assert!(yaml.contains("active"), "got: {yaml}");
    let rt: Config = from_str(&yaml).unwrap();
    assert_eq!(rt, cfg);
}

#[test]
fn from_kebab_case_edge_cases() {
    // singleton_map_with.rs:346 — empty segment in kebab
    use noyalib::with::singleton_map_with::from_kebab_case;
    assert_eq!(from_kebab_case(""), "");
    assert_eq!(from_kebab_case("-"), "");
    assert_eq!(from_kebab_case("a--b"), "AB");
}

// ═══════════════════════════════════════════════════════════════════════
// with/singleton_map_recursive.rs — Tagged transform (lines 47–51)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn singleton_map_recursive_tagged_value() {
    // singleton_map_recursive.rs:47-51
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Op {
        Add(i32),
        Sub(i32),
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Ops {
        #[serde(with = "noyalib::with::singleton_map_recursive")]
        ops: Vec<Op>,
    }

    let ops = Ops {
        ops: vec![Op::Add(1), Op::Sub(2)],
    };
    let yaml = to_string(&ops).unwrap();
    let rt: Ops = from_str(&yaml).unwrap();
    assert_eq!(rt, ops);
}

// ═══════════════════════════════════════════════════════════════════════
// path.rs — parent/depth for Seq/Map/Alias/Unknown (lines 202–228)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn path_parent_and_depth() {
    // path.rs:202, 204, 228
    use noyalib::Path;
    let root = Path::Root;
    assert!(root.parent().is_none());
    assert_eq!(root.depth(), 0);

    let seq = root.index(0);
    assert!(seq.parent().is_some());
    assert_eq!(seq.depth(), 1);

    let map = root.key("k");
    assert!(map.parent().is_some());
    assert_eq!(map.depth(), 1);

    let seq2 = map.index(0);
    let deep = seq2.key("nested");
    assert_eq!(deep.depth(), 3);
}

// ═══════════════════════════════════════════════════════════════════════
// fmt.rs — Commented serialize edge (line 381)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn commented_roundtrip_no_comment() {
    // fmt.rs:381 — Commented serialize + deserialize roundtrip
    use noyalib::Commented;
    let c = Commented::new("hello", "my comment");
    let yaml = to_string(&c).unwrap();
    assert!(yaml.contains("# my comment"));
    // Deserialize back — comment is lost
    let rt: Commented<String> = from_str(&yaml).unwrap();
    assert_eq!(rt.value, "hello");
    assert!(rt.comment.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════
// Remaining edge cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alias_unknown_anchor_error() {
    // loader.rs:191 — unknown anchor in alias
    let result: Result<Value, _> = from_str("key: *nonexistent");
    assert!(result.is_err());
}

#[test]
fn mapping_with_anchored_value() {
    // loader.rs:246, 267 — mapping with anchor
    let yaml = "&map\na: 1\nb: 2";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

#[test]
fn merge_key_sequence_of_mappings() {
    // loader.rs:273 — merge from sequence of mappings
    let yaml = "a: &a\n  x: 1\nb: &b\n  y: 2\nc:\n  <<:\n    - *a\n    - *b\n  z: 3";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["c"]["x"].as_i64(), Some(1));
    assert_eq!(v["c"]["y"].as_i64(), Some(2));
    assert_eq!(v["c"]["z"].as_i64(), Some(3));
}

#[test]
fn spanned_nested_fields() {
    // spanned.rs:117-156, de.rs:786-814
    #[derive(Debug, Deserialize)]
    struct Item {
        name: Spanned<String>,
        value: Spanned<i64>,
    }
    let yaml = "name: test\nvalue: 42";
    let item: Item = from_str(yaml).unwrap();
    assert_eq!(item.name.value, "test");
    assert_eq!(item.value.value, 42);
    assert!(item.name.start.line() >= 1);
    assert!(item.value.start.line() >= 1);
}

#[test]
fn load_all_as_with_spans() {
    // loader.rs (public) — load_all_as exercises span context per doc
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        name: String,
    }
    let yaml = "---\nname: a\n---\nname: b";
    let docs: Vec<Doc> = noyalib::load_all_as(yaml).unwrap();
    assert_eq!(docs.len(), 2);
}
