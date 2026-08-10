// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage harness for `cst::format` (formatter) and `ser` (serializer).
//!
//! Exercises comment normalisation, mixed indentation, blank lines, flow
//! collections and deep nesting through the public formatter, and drives the
//! serializer across edge numbers, empty/nested collections, quote-forcing,
//! block scalars, formatting-hint wrappers and error paths.

#![allow(
    missing_docs,
    dead_code,
    unused_results,
    unused_must_use,
    non_snake_case,
    clippy::all
)]

use std::collections::BTreeMap;

use noyalib::cst::{FormatConfig, format, format_with_config, parse_stream};
use noyalib::{
    Commented, FlowMap, FlowSeq, FlowStyle, FoldStr, LitStr, Number, SerializerConfig, SpaceAfter,
    Tag, TaggedValue, Value, to_string, to_string_multi, to_string_value,
    to_string_value_with_config, to_string_with_config, to_value,
};

// ─────────────────────────── formatter ────────────────────────────

#[test]
fn format_basic_mapping_idempotent() {
    assert_eq!(format("a: 1\nb: 2\n").unwrap(), "a: 1\nb: 2\n");
}

#[test]
fn format_normalises_under_indented_nesting() {
    // Messy key spacing and shallow indent both collapse to canonical form.
    assert_eq!(format("key:\n value: 1\n").unwrap(), "key:\n  value: 1\n");
    assert_eq!(format("a   :    1\n").unwrap(), "a: 1\n");
}

#[test]
fn format_inline_comment_preserved() {
    assert_eq!(format("a: 1 # note\n").unwrap(), "a: 1 # note\n");
}

#[test]
fn format_standalone_comment_preserved() {
    let out = format("# header\na: 1\n").unwrap();
    assert!(out.contains("# header"));
    assert!(out.contains("a: 1"));
}

#[test]
fn format_comment_inside_sequence() {
    let out = format("- 1 # one\n- 2\n").unwrap();
    assert!(out.contains("# one"));
    assert!(out.contains("- 1"));
    assert!(out.contains("- 2"));
}

#[test]
fn format_root_sequence_round_trips() {
    let out = format("- one\n- two\n- three\n").unwrap();
    assert_eq!(out, "- one\n- two\n- three\n");
}

#[test]
fn format_sequence_of_mappings() {
    let out = format("- a: 1\n  b: 2\n- c: 3\n").unwrap();
    assert!(out.contains("- a: 1"));
    assert!(out.contains("b: 2"));
    assert!(out.contains("- c: 3"));
}

#[test]
fn format_nested_block_sequence_under_mapping() {
    assert_eq!(
        format("items:\n  - sub:\n      - 1\n").unwrap(),
        "items:\n  - sub:\n      - 1\n"
    );
}

#[test]
fn format_deeply_nested_block_mapping() {
    let src = "a:\n  b:\n    c:\n      d: 1\n";
    assert_eq!(format(src).unwrap(), src);
}

#[test]
fn format_flow_sequence_written_verbatim() {
    let out = format("items: [1, 2, 3]\n").unwrap();
    assert!(out.contains("items:"));
    assert!(out.contains('[') && out.contains(']'));
    assert!(out.contains('1') && out.contains('3'));
}

#[test]
fn format_flow_mapping_written_verbatim() {
    let out = format("pos: {x: 1, y: 2}\n").unwrap();
    assert!(out.contains("pos:"));
    assert!(out.contains('{') && out.contains('}'));
}

#[test]
fn format_document_markers_preserved() {
    let out = format("---\na: 1\n...\n").unwrap();
    assert!(out.starts_with("---"));
    assert!(out.contains("a: 1"));
    assert!(out.contains("..."));
}

#[test]
fn format_empty_and_whitespace_input_is_empty() {
    assert_eq!(format("").unwrap(), "");
    assert_eq!(format("    ").unwrap(), "");
    assert_eq!(format("\n\n").unwrap(), "");
}

#[test]
fn format_with_config_four_space_indent() {
    let cfg = FormatConfig { indent_size: 4 };
    let out = format_with_config("key:\n  value: 1\n", &cfg).unwrap();
    assert!(out.contains("value: 1"));
    assert!(out.contains("key:"));
}

#[test]
fn format_appends_trailing_newline() {
    // Input without a trailing newline still ends in one.
    let out = format("a: 1").unwrap();
    assert!(out.ends_with('\n'));
}

#[test]
fn parse_stream_splits_multiple_documents() {
    let docs = parse_stream("a: 1\n---\nb: 2\n").unwrap();
    assert_eq!(docs.len(), 2);
    let single = parse_stream("a: 1\n").unwrap();
    assert_eq!(single.len(), 1);
}

// ─────────────────────────── serializer ───────────────────────────

#[test]
fn ser_scalar_integers_and_bool() {
    assert_eq!(to_string(&42_i32).unwrap(), "42");
    assert_eq!(to_string(&-7_i64).unwrap(), "-7");
    assert_eq!(to_string(&1_000_000_u64).unwrap(), "1000000");
    assert_eq!(to_string(&true).unwrap(), "true");
    assert_eq!(to_string(&false).unwrap(), "false");
}

#[test]
fn ser_float_whole_and_fractional_keep_floatness() {
    assert_eq!(to_string(&1.0_f64).unwrap(), "1.0");
    assert_eq!(to_string(&1.5_f64).unwrap(), "1.5");
}

#[test]
fn ser_float_special_values_via_value() {
    assert_eq!(
        to_string_value(&Value::Number(Number::Float(f64::NAN))).unwrap(),
        ".nan"
    );
    assert_eq!(
        to_string_value(&Value::Number(Number::Float(f64::INFINITY))).unwrap(),
        ".inf"
    );
    assert_eq!(
        to_string_value(&Value::Number(Number::Float(f64::NEG_INFINITY))).unwrap(),
        "-.inf"
    );
}

#[test]
fn ser_option_and_unit() {
    assert_eq!(to_string(&Option::<i32>::None).unwrap(), "null");
    assert_eq!(to_string(&Some(5_i32)).unwrap(), "5");
    assert_eq!(to_string(&()).unwrap(), "null");
}

#[test]
fn ser_char_plain_and_quoted() {
    assert_eq!(to_string(&'x').unwrap(), "x");
    // A colon char is structural and must be quoted.
    let colon = to_string(&':').unwrap();
    assert!(colon.contains('"') && colon.contains(':'));
}

#[test]
fn ser_string_quoting_rules() {
    // Reserved words and number-like strings get quoted.
    assert_eq!(to_string(&"true").unwrap(), "\"true\"");
    assert_eq!(to_string(&"123").unwrap(), "\"123\"");
    // Empty string quotes to a pair of double quotes.
    assert_eq!(to_string(&"").unwrap(), "\"\"");
    // Leading space forces quoting.
    assert!(to_string(&" leading").unwrap().starts_with('"'));
    // Plain-safe identifier stays unquoted.
    assert_eq!(to_string(&"plain_ok-1.2/x").unwrap(), "plain_ok-1.2/x");
}

#[test]
fn ser_string_with_control_char_double_quoted() {
    let out = to_string(&"tab\tok").unwrap();
    // Tab is allowed but a bell control char must be hex-escaped.
    let bell = to_string(&"a\u{07}b").unwrap();
    assert!(bell.contains("\\x07"));
    assert!(out.contains("tab"));
}

#[test]
fn ser_quote_all_uses_single_quotes() {
    let cfg = SerializerConfig::new().quote_all(true);
    assert_eq!(to_string_with_config(&"hello", &cfg).unwrap(), "'hello'");
    // Embedded single quote is doubled.
    assert_eq!(to_string_with_config(&"it's", &cfg).unwrap(), "'it''s'");
}

#[test]
fn ser_block_scalar_strip_chomping() {
    assert_eq!(to_string(&"a\nb").unwrap(), "|-\n  a\n  b");
}

#[test]
fn ser_block_scalar_keep_single_trailing_newline() {
    assert_eq!(to_string(&"a\nb\n").unwrap(), "|\n  a\n  b\n");
}

#[test]
fn ser_block_scalars_disabled_double_quotes_multiline() {
    let cfg = SerializerConfig::new().block_scalars(false);
    let out = to_string_with_config(&"a\nb", &cfg).unwrap();
    assert!(out.contains("\\n"));
    assert!(out.starts_with('"'));
}

#[test]
fn ser_empty_collections() {
    assert_eq!(to_string(&Vec::<i32>::new()).unwrap(), "[]");
    assert_eq!(to_string(&BTreeMap::<String, i32>::new()).unwrap(), "{}");
}

#[test]
fn ser_nested_sequence_block() {
    let v = vec![vec![1, 2], vec![3, 4]];
    let out = to_string(&v).unwrap();
    assert!(out.contains("- 1"));
    assert!(out.contains("- 4"));
}

#[test]
fn ser_struct_nested_mapping() {
    #[derive(serde::Serialize)]
    struct Inner {
        x: i32,
        y: i32,
    }
    #[derive(serde::Serialize)]
    struct Outer {
        name: String,
        inner: Inner,
    }
    let out = to_string(&Outer {
        name: "myapp".to_string(),
        inner: Inner { x: 1, y: 2 },
    })
    .unwrap();
    assert_eq!(out, "name: myapp\ninner:\n  x: 1\n  y: 2");
}

#[test]
fn ser_sequence_of_mappings_inline_first_key() {
    #[derive(serde::Serialize)]
    struct Kv {
        x: i32,
        y: i32,
    }
    let out = to_string(&vec![Kv { x: 1, y: 2 }]).unwrap();
    assert_eq!(out, "- x: 1\n  y: 2");
}

#[test]
fn ser_map_integer_and_bool_keys_coerced() {
    let mut ints: BTreeMap<i32, i32> = BTreeMap::new();
    ints.insert(1, 10);
    // Non-string map keys are coerced to quoted string keys.
    assert_eq!(to_string(&ints).unwrap(), "\"1\": 10");

    let mut bools: BTreeMap<bool, i32> = BTreeMap::new();
    bools.insert(true, 1);
    let out = to_string(&bools).unwrap();
    assert!(out.contains("true") && out.contains('1'), "got {out:?}");
}

#[test]
fn ser_map_sequence_key_is_error() {
    // A tuple key serialises as a sequence, which is not a valid map key.
    let mut m: BTreeMap<(i32, i32), i32> = BTreeMap::new();
    m.insert((1, 2), 3);
    assert!(to_string(&m).is_err());
}

#[test]
fn ser_enum_variants() {
    #[derive(serde::Serialize)]
    enum E {
        Unit,
        Newtype(i32),
        Tuple(i32, i32),
        Struct { a: i32 },
    }
    assert_eq!(to_string(&E::Unit).unwrap(), "Unit");
    assert_eq!(to_string(&E::Newtype(7)).unwrap(), "Newtype: 7");
    let t = to_string(&E::Tuple(1, 2)).unwrap();
    assert!(t.contains("Tuple:"));
    assert!(t.contains("- 1") && t.contains("- 2"));
    let s = to_string(&E::Struct { a: 9 }).unwrap();
    assert!(s.contains("Struct:"));
    assert!(s.contains("a: 9"));
}

#[test]
fn ser_flow_style_flow_and_auto() {
    let seq = vec![1, 2, 3];
    let flow = SerializerConfig::new().flow_style(FlowStyle::Flow);
    assert_eq!(
        to_string_with_config(&seq, &flow).unwrap().trim_end(),
        "[1, 2, 3]"
    );
    let auto = SerializerConfig::new()
        .flow_style(FlowStyle::Auto)
        .flow_threshold(4);
    assert_eq!(
        to_string_with_config(&seq, &auto).unwrap().trim_end(),
        "[1, 2, 3]"
    );
}

#[test]
fn ser_document_markers_config() {
    let cfg = SerializerConfig::new()
        .document_start(true)
        .document_end(true);
    let out = to_string_with_config(&"x", &cfg).unwrap();
    assert!(out.starts_with("---\n"));
    assert!(out.ends_with("\n..."));
}

#[test]
fn ser_indent_config_widens_nesting() {
    let mut inner = noyalib::Mapping::new();
    let _ = inner.insert("b", Value::from(1));
    let mut outer = noyalib::Mapping::new();
    let _ = outer.insert("a", Value::Mapping(inner));
    let cfg = SerializerConfig::new().indent(4);
    let out = to_string_value_with_config(&Value::Mapping(outer), &cfg).unwrap();
    assert!(out.contains("a:\n    b: 1"));
}

#[test]
fn ser_multi_document() {
    let out = to_string_multi(&[1, 2, 3]).unwrap();
    assert!(out.contains("---"));
    assert!(out.contains('1') && out.contains('3'));
}

#[test]
fn ser_flow_hint_wrappers() {
    // Force flow via the FlowSeq / FlowMap formatting hints.
    assert_eq!(to_string(&FlowSeq(vec![1, 2, 3])).unwrap(), "[1, 2, 3]");
    let mut m: BTreeMap<String, i32> = BTreeMap::new();
    m.insert("a".to_string(), 1);
    let out = to_string(&FlowMap(m)).unwrap();
    assert!(out.starts_with('{') && out.ends_with('}'));
    assert!(out.contains("a: 1"));
}

#[test]
fn ser_literal_and_folded_block_hints() {
    assert_eq!(to_string(&LitStr("a\nb")).unwrap(), "|-\n  a\n  b");
    assert_eq!(to_string(&FoldStr("a\nb")).unwrap(), ">-\n  a\n  b");
}

#[test]
fn ser_commented_and_space_after_hints() {
    let out = to_string(&Commented::new(42, "note")).unwrap();
    assert_eq!(out, "42 # note");
    let sa = to_string(&SpaceAfter(7_i32)).unwrap();
    assert_eq!(sa, "7\n");
}

#[test]
fn ser_tagged_value_round_trip_shape() {
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!Color"),
        Value::from("#ff8800"),
    )));
    let out = to_string_value(&tagged).unwrap();
    assert!(out.contains("!Color"));
    assert!(out.contains("ff8800"));
}

#[test]
fn ser_bytes_emit_binary_tag() {
    // to_value routes &[u8] through serialize_bytes -> !!binary base64.
    let v = to_value(serde_bytes::Bytes::new(b"hi")).unwrap();
    let out = to_string_value(&v).unwrap();
    assert!(out.contains("!!binary"));
}

#[test]
fn ser_depth_limit_error() {
    let mut v = Value::Null;
    for _ in 0..10 {
        v = Value::Sequence(vec![v]);
    }
    let cfg = SerializerConfig::new().max_depth(2);
    assert!(to_string_value_with_config(&v, &cfg).is_err());
}

#[cfg(not(feature = "lossless-u64"))]
#[test]
fn ser_u64_above_i64_max_is_error() {
    assert!(to_string(&u64::MAX).is_err());
}
