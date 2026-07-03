// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Targeted coverage harness for the `ser`, `cst::format`, and `cst::builder`
//! modules. Each test names the file under test and the specific branch /
//! uncovered region it exists to drive — see `ser_cst_coverage_extra.rs`
//! ranges in `cargo llvm-cov` reports for the lines we're closing in on.
//!
//! These tests intentionally use only the public crate API
//! (`noyalib::to_string`, `noyalib::to_string_with_config`,
//! `noyalib::to_writer_value`, `noyalib::cst::parse_document`, …). Internal
//! magic tags (`__noya_*`) are exercised indirectly via the
//! `noyalib::fmt::*` newtypes and the anchor-tracking serializer.

use std::collections::BTreeMap;
use std::io;

use noyalib::cst::{FormatConfig, format, format_with_config, parse_document, parse_stream};
use noyalib::fmt::{Commented, FlowMap, FlowSeq, FoldString, LitString, SpaceAfter};
use noyalib::{
    FlowStyle, Mapping, RcAnchor, ScalarStyle, SerializerConfig, Tag, TaggedValue, Value,
    to_fmt_writer, to_fmt_writer_with_config, to_string, to_string_multi,
    to_string_multi_with_config, to_string_tracking_shared, to_string_tracking_shared_with_config,
    to_string_value, to_string_value_with_config, to_string_with_config, to_value, to_writer,
    to_writer_multi, to_writer_multi_with_config, to_writer_tracking_shared,
    to_writer_tracking_shared_with_config, to_writer_value, to_writer_value_with_config,
    to_writer_with_config,
};
use serde::Serialize;

// ============================================================================
// ser.rs — `to_string_with_config` / `to_writer_*` thin wrappers
// ============================================================================

#[test]
fn ser_ser_to_string_with_config_default_round_trip() {
    // Drives line 325: `let v = to_value(value)?` + `value_to_string`.
    let cfg = SerializerConfig::new();
    let yaml = to_string_with_config(&"plain", &cfg).unwrap();
    assert!(yaml.contains("plain"));
}

#[test]
fn ser_ser_to_writer_with_config_writes_full_payload() {
    // Drives lines 361-363: `let s = to_string_with_config?; write_all(&s)`.
    let mut buf = Vec::<u8>::new();
    to_writer_with_config(&mut buf, &vec![1i32, 2, 3], &SerializerConfig::new()).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn ser_ser_to_writer_value_with_config_writes_payload() {
    // Drives 616-618: to_writer_value_with_config.
    let v = Value::Mapping({
        let mut m = Mapping::new();
        let _ = m.insert("k", Value::String("v".into()));
        m
    });
    let mut buf = Vec::<u8>::new();
    to_writer_value_with_config(&mut buf, &v, &SerializerConfig::new()).unwrap();
    assert!(buf.contains(&b'k'));
}

#[test]
fn ser_ser_to_writer_value_default_writes_payload() {
    let v: Value = to_value(&42i64).unwrap();
    let mut buf = Vec::<u8>::new();
    to_writer_value(&mut buf, &v).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn ser_ser_to_writer_default_round_trip() {
    let mut buf = Vec::<u8>::new();
    to_writer(&mut buf, &"x").unwrap();
    assert!(buf.contains(&b'x'));
}

#[test]
fn ser_ser_to_writer_multi_default() {
    let docs = vec![1i32, 2, 3];
    let mut buf = Vec::<u8>::new();
    to_writer_multi(&mut buf, &docs).unwrap();
    assert!(buf.contains(&b'-'));
}

#[test]
fn ser_ser_to_writer_multi_with_config_writes_full() {
    // Drives 1393-1395.
    let docs = vec![1i32, 2];
    let mut buf = Vec::<u8>::new();
    to_writer_multi_with_config(&mut buf, &docs, &SerializerConfig::new()).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn ser_ser_to_string_multi_pair() {
    // Drives 1364-1365: `let v = to_value(value)?; write_value(...)?`.
    let yaml = to_string_multi(&[1i32, 2, 3]).unwrap();
    let _ = to_string_multi_with_config(&[1i32, 2, 3], &SerializerConfig::new()).unwrap();
    assert!(yaml.contains("---"));
}

#[test]
fn ser_ser_to_writer_io_error_propagates() {
    // Drives the `?` propagation in `to_writer_with_config`. A
    // failing writer surfaces `Error::Io`.
    struct BadWriter;
    impl io::Write for BadWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("denied"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let res = to_writer(BadWriter, &"hello");
    assert!(res.is_err());
}

#[test]
fn ser_ser_to_fmt_writer_default_and_with_config() {
    let mut s = String::new();
    to_fmt_writer(&mut s, &"abc").unwrap();
    let mut s2 = String::new();
    to_fmt_writer_with_config(&mut s2, &"abc", &SerializerConfig::new()).unwrap();
    assert!(s.contains("abc") && s2.contains("abc"));
}

#[test]
fn ser_ser_to_fmt_writer_error_propagates_serialize() {
    // Drives lines 497-500: write_str returning fmt::Error becomes
    // Error::Serialize.
    struct BadFmt;
    impl core::fmt::Write for BadFmt {
        fn write_str(&mut self, _: &str) -> core::fmt::Result {
            Err(core::fmt::Error)
        }
    }
    let mut bad = BadFmt;
    let res = to_fmt_writer(&mut bad, &"abc");
    assert!(res.is_err());
}

#[test]
fn ser_ser_to_string_tracking_shared_default_and_with_config() {
    // Drives 458-460 (writer variants), 1542-1549 (newtype magic
    // anchor branch), 1206-1240 (anchor def/ref emission).
    let shared: RcAnchor<String> = RcAnchor::from("hello".to_string());
    let doc = vec![shared.clone(), shared.clone(), shared];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    assert!(yaml.contains("&id001"));
    assert!(yaml.contains("*id001"));
    let yaml2 =
        to_string_tracking_shared_with_config(&doc, &SerializerConfig::new().indent(4)).unwrap();
    assert!(yaml2.contains("*id001"));
}

#[test]
fn ser_ser_to_writer_tracking_shared_default_and_with_config() {
    let shared: RcAnchor<String> = RcAnchor::from("payload".to_string());
    let doc = vec![shared.clone(), shared];
    let mut buf = Vec::<u8>::new();
    to_writer_tracking_shared(&mut buf, &doc).unwrap();
    assert!(!buf.is_empty());
    let mut buf2 = Vec::<u8>::new();
    to_writer_tracking_shared_with_config(&mut buf2, &doc, &SerializerConfig::new()).unwrap();
    assert!(!buf2.is_empty());
}

// ============================================================================
// ser.rs — anchor tagged emission paths
// ============================================================================

#[test]
fn ser_ser_anchor_tagged_block_mapping_value() {
    // Drives 1117-1122 (mapping value with MAGIC_ANCHOR_DEF needing
    // space after `:`) and 1213-1219 (anchor wrapping a non-empty
    // mapping → newline + write_mapping with is_root=true).
    #[derive(Clone, Serialize)]
    struct Inner {
        a: i32,
        b: i32,
    }
    let shared: RcAnchor<Inner> = RcAnchor::from(Inner { a: 1, b: 2 });
    let doc = vec![shared.clone(), shared];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    assert!(yaml.contains("&id"));
    assert!(yaml.contains("*id"));
}

#[test]
fn ser_ser_anchor_tagged_block_sequence_value() {
    // Drives 1221-1224 (anchor wrapping non-empty sequence).
    let shared: RcAnchor<Vec<i32>> = RcAnchor::from(vec![1, 2, 3]);
    let doc = vec![shared.clone(), shared];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    assert!(yaml.contains("&id"));
}

#[test]
fn ser_ser_anchor_tagged_scalar_value() {
    // Drives 1226-1229 (anchor wrapping a scalar — push ' ' then
    // write_value).
    let shared: RcAnchor<i32> = RcAnchor::from(42);
    let doc = vec![shared.clone(), shared];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    assert!(yaml.contains("&id"));
    assert!(yaml.contains("42"));
}

// ============================================================================
// ser.rs — internal-tag fallback paths (non-matching value shape)
// ============================================================================

#[test]
fn ser_ser_unknown_internal_tag_falls_through() {
    // Drives 1242-1245: an unknown `__noya_*` tag falls through to
    // regular write_value.
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_completely_unknown"),
        Value::String("hello".into()),
    )));
    let yaml = to_string_value(&v).unwrap();
    assert!(yaml.contains("hello"));
}

#[test]
fn ser_ser_magic_flow_seq_with_non_sequence_falls_through() {
    // Drives 1158: MAGIC_FLOW_SEQ wrapping a non-sequence value
    // falls through to write_value(...).
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_flow_seq"),
        Value::String("scalar".into()),
    )));
    let yaml = to_string_value(&v).unwrap();
    assert!(yaml.contains("scalar"));
}

#[test]
fn ser_ser_magic_flow_map_with_non_mapping_falls_through() {
    // Drives 1165: MAGIC_FLOW_MAP wrapping a non-mapping value.
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_flow_map"),
        Value::String("scalar".into()),
    )));
    let yaml = to_string_value(&v).unwrap();
    assert!(yaml.contains("scalar"));
}

#[test]
fn ser_ser_magic_lit_str_with_non_string_falls_through() {
    // Drives 1172: MAGIC_LIT_STR wrapping a non-string.
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_lit_str"),
        Value::Number(noyalib::Number::Integer(7)),
    )));
    let yaml = to_string_value(&v).unwrap();
    assert!(yaml.contains('7'));
}

#[test]
fn ser_ser_magic_fold_str_with_non_string_falls_through() {
    // Drives 1179: MAGIC_FOLD_STR wrapping a non-string.
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_fold_str"),
        Value::Number(noyalib::Number::Integer(9)),
    )));
    let yaml = to_string_value(&v).unwrap();
    assert!(yaml.contains('9'));
}

#[test]
fn ser_ser_magic_commented_with_seq_len_other_falls_through() {
    // Drives 1191-1196: MAGIC_COMMENTED with a non-2-element seq
    // takes the fallthrough path.
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_commented"),
        Value::Sequence(vec![
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("c".into()),
        ]),
    )));
    let yaml = to_string_value(&v).unwrap();
    assert!(yaml.contains('a') && yaml.contains('c'));
}

#[test]
fn ser_ser_magic_commented_non_sequence_value_falls_through() {
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_commented"),
        Value::String("plain".into()),
    )));
    let yaml = to_string_value(&v).unwrap();
    assert!(yaml.contains("plain"));
}

// ============================================================================
// ser.rs — fmt newtype magic
// ============================================================================

#[test]
fn ser_ser_flow_seq_newtype() {
    // Drives 1523-1532: serialize_newtype_struct with MAGIC_FLOW_SEQ.
    #[derive(Serialize)]
    struct Doc {
        items: FlowSeq<Vec<i32>>,
    }
    let yaml = to_string(&Doc {
        items: FlowSeq(vec![1, 2, 3]),
    })
    .unwrap();
    assert!(yaml.contains('['));
}

#[test]
fn ser_ser_flow_map_newtype() {
    let mut m = BTreeMap::new();
    let _ = m.insert("a".to_string(), 1i32);
    let _ = m.insert("b".to_string(), 2);
    #[derive(Serialize)]
    struct Doc {
        flow: FlowMap<BTreeMap<String, i32>>,
    }
    let yaml = to_string(&Doc { flow: FlowMap(m) }).unwrap();
    assert!(yaml.contains('{'));
}

#[test]
fn ser_ser_lit_string_newtype() {
    #[derive(Serialize)]
    struct Doc {
        body: LitString,
    }
    let yaml = to_string(&Doc {
        body: LitString::from("line1\nline2\n".to_string()),
    })
    .unwrap();
    assert!(yaml.contains('|'));
}

#[test]
fn ser_ser_fold_string_newtype() {
    #[derive(Serialize)]
    struct Doc {
        body: FoldString,
    }
    let yaml = to_string(&Doc {
        body: FoldString::from("a paragraph\nthat folds\n".to_string()),
    })
    .unwrap();
    assert!(yaml.contains('>'));
}

#[test]
fn ser_ser_commented_newtype() {
    // Drives 1534-1540 newtype_struct branch + 1182-1190 (write).
    let v = Commented::new("value", "explanatory note");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("explanatory note"));
}

#[test]
fn ser_ser_space_after_newtype() {
    #[derive(Serialize)]
    struct Doc {
        a: SpaceAfter<i32>,
        b: i32,
    }
    let yaml = to_string(&Doc {
        a: SpaceAfter(1),
        b: 2,
    })
    .unwrap();
    assert!(yaml.contains('a') && yaml.contains('b'));
}

// ============================================================================
// ser.rs — newtype_variant / tuple_variant / struct_variant branches
// ============================================================================

#[test]
fn ser_ser_newtype_variant() {
    // Drives 1565-1567 (serialize_newtype_variant).
    #[derive(Serialize)]
    enum E {
        N(i32),
    }
    let yaml = to_string(&E::N(42)).unwrap();
    assert!(yaml.contains("42"));
    assert!(yaml.contains('N'));
}

#[test]
fn ser_ser_tuple_variant() {
    // Drives 1696 (SerializeTupleVariant::serialize_field).
    #[derive(Serialize)]
    enum E {
        T(i32, i32, i32),
    }
    let yaml = to_string(&E::T(1, 2, 3)).unwrap();
    assert!(yaml.contains('T'));
}

#[test]
fn ser_ser_struct_variant() {
    // Drives 1786 (SerializeStructVariant::serialize_field).
    #[derive(Serialize)]
    enum E {
        S { x: i32, y: i32 },
    }
    let yaml = to_string(&E::S { x: 1, y: 2 }).unwrap();
    assert!(yaml.contains('S'));
}

// ============================================================================
// ser.rs — SerializeMap key-coercion paths (line 1722-1728)
// ============================================================================

#[test]
fn ser_ser_map_key_integer_coerced() {
    // Drives 1725: integer key path.
    let mut m = BTreeMap::<i32, &str>::new();
    let _ = m.insert(7, "seven");
    let _ = m.insert(11, "eleven");
    let yaml = to_string(&m).unwrap();
    assert!(yaml.contains("seven"));
    assert!(yaml.contains("11"));
}

#[test]
fn ser_ser_map_key_bool_coerced() {
    // Drives 1726: bool key path.
    let mut m = BTreeMap::<bool, &str>::new();
    let _ = m.insert(true, "yes");
    let yaml = to_string(&m).unwrap();
    assert!(yaml.contains("yes"));
}

#[test]
fn ser_ser_map_key_non_string_rejected() {
    // Drives 1727 (`return Err(... "map key must be a string")`).
    let mut m = BTreeMap::<Vec<i32>, &str>::new();
    let _ = m.insert(vec![1, 2], "x");
    let res = to_string(&m);
    assert!(res.is_err());
}

// ============================================================================
// ser.rs — flow style / scalar style / config exercises
// ============================================================================

#[test]
fn ser_ser_flow_style_auto_threshold_below_uses_flow() {
    // Drives FlowStyle::Auto branch.
    let v = vec![1, 2, 3];
    let cfg = SerializerConfig::new()
        .flow_style(FlowStyle::Auto)
        .flow_threshold(10);
    let yaml = to_string_with_config(&v, &cfg).unwrap();
    assert!(!yaml.is_empty());
}

#[test]
fn ser_ser_compact_list_indent_branch() {
    // Drives 1130-1135: compact_list_indent toggling sequence next_indent.
    #[derive(Serialize)]
    struct Doc {
        items: Vec<i32>,
    }
    let cfg = SerializerConfig::new().compact_list_indent(true);
    let yaml = to_string_with_config(
        &Doc {
            items: vec![1, 2, 3],
        },
        &cfg,
    )
    .unwrap();
    assert!(yaml.contains('-'));
}

#[test]
fn ser_ser_document_start_and_end_markers() {
    let cfg = SerializerConfig::new()
        .document_start(true)
        .document_end(true);
    let yaml = to_string_with_config(&"x", &cfg).unwrap();
    assert!(yaml.starts_with("---\n"));
    assert!(yaml.contains("..."));
}

#[test]
fn ser_ser_quote_all_force_quotes() {
    let cfg = SerializerConfig::new().quote_all(true);
    let yaml = to_string_with_config(&"plain", &cfg).unwrap();
    assert!(yaml.contains('\''));
}

#[test]
fn ser_ser_block_scalars_disabled() {
    // Force the block-scalar fast-path off.
    let cfg = SerializerConfig::new()
        .block_scalars(false)
        .block_scalar_threshold(0);
    let yaml = to_string_with_config(&"line1\nline2\n", &cfg).unwrap();
    assert!(!yaml.is_empty());
}

#[test]
fn ser_ser_config_builder_all_methods_chainable() {
    // Touch every builder method so any unused helper folds into
    // the coverage map.
    let cfg = SerializerConfig::new()
        .indent(4)
        .document_start(false)
        .document_end(false)
        .block_scalars(true)
        .block_scalar_threshold(2)
        .flow_style(FlowStyle::Block)
        .scalar_style(ScalarStyle::Auto)
        .flow_threshold(8)
        .quote_all(false)
        .compact_list_indent(false)
        .folded_wrap_chars(120)
        .min_fold_chars(40)
        .max_depth(64);
    let yaml = to_string_with_config(&vec![1, 2, 3], &cfg).unwrap();
    assert!(yaml.contains('-'));
}

// ============================================================================
// ser.rs — special-scalar emission (NaN / inf / -inf, looks_like_number)
// ============================================================================

#[test]
fn ser_ser_float_nan_emits_dot_nan() {
    let yaml = to_string(&f64::NAN).unwrap();
    assert!(yaml.contains(".nan"));
}

#[test]
fn ser_ser_float_inf_emits_dot_inf() {
    let yaml = to_string(&f64::INFINITY).unwrap();
    assert!(yaml.contains(".inf"));
}

#[test]
fn ser_ser_float_neg_inf_emits_neg_dot_inf() {
    let yaml = to_string(&f64::NEG_INFINITY).unwrap();
    assert!(yaml.contains("-.inf"));
}

#[test]
fn ser_ser_string_looking_like_special_floats_quoted() {
    // Drives looks_like_number's special-float branch.
    let yaml = to_string(&".inf").unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}

#[test]
fn ser_ser_string_with_leading_dash_round_trip() {
    // Drives FIRST_CHAR_QUOTE entry for `-`. Whether the
    // serializer quotes or not, the value must round-trip back.
    let yaml = to_string(&"-leading").unwrap();
    let back: String = noyalib::from_str(&yaml).unwrap();
    assert_eq!(back, "-leading");
}

#[test]
fn ser_ser_string_with_colon_inside_quoted() {
    let yaml = to_string(&"key:value").unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}

#[test]
fn ser_ser_string_with_hash_inside_quoted() {
    let yaml = to_string(&"foo#bar").unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}

#[test]
fn ser_ser_string_with_control_char_uses_hex_escape() {
    let yaml = to_string(&"a\x01b").unwrap();
    assert!(yaml.contains("\\x01"));
}

#[test]
fn ser_ser_string_reserved_words_quoted() {
    for word in ["true", "false", "null", "Null", "NULL", "TRUE", "FALSE"] {
        let yaml = to_string(&word).unwrap();
        assert!(
            yaml.contains('"') || yaml.contains('\''),
            "expected quoted: {word} → {yaml}"
        );
    }
}

#[test]
fn ser_ser_string_trailing_space_quoted() {
    let yaml = to_string(&"hi ").unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}

#[test]
fn ser_ser_string_leading_signs_then_digit_quoted() {
    // Drives looks_like_number leading-sign loop.
    let yaml = to_string(&"++1").unwrap();
    assert!(yaml.contains('"') || yaml.contains('\''));
}

#[test]
fn ser_ser_empty_string_double_quoted() {
    let yaml = to_string(&"").unwrap();
    assert!(yaml.contains("\"\""));
}

#[test]
fn ser_ser_u64_overflow_returns_error() {
    let big: u64 = u64::MAX;
    let res = to_string(&big);
    #[cfg(not(feature = "lossless-u64"))]
    assert!(res.is_err());
    #[cfg(feature = "lossless-u64")]
    assert_eq!(res.unwrap().trim(), u64::MAX.to_string());
}

#[test]
fn ser_ser_bytes_emit_binary_tagged() {
    // Drives serialize_bytes binary-tagged branch.
    use serde_bytes::ByteBuf;
    let bb = ByteBuf::from(b"\x00\x01\x02".to_vec());
    let yaml = to_string(&bb).unwrap();
    assert!(yaml.contains("!!binary"));
}

#[test]
fn ser_ser_recursion_limit_triggers_error() {
    // Drives 672-674: recursion guard.
    let mut root = Value::Sequence(vec![Value::Null]);
    for _ in 0..50 {
        root = Value::Sequence(vec![root]);
    }
    let cfg = SerializerConfig::new().max_depth(10);
    let res = to_string_value_with_config(&root, &cfg);
    assert!(res.is_err());
}

// ============================================================================
// ser.rs — write_flow_sequence / write_flow_mapping element loops
// ============================================================================

#[test]
fn ser_ser_flow_sequence_two_elements_emits_comma() {
    // Drives the `i > 0` branch in write_flow_sequence.
    #[derive(Serialize)]
    struct Doc {
        items: FlowSeq<Vec<i32>>,
    }
    let yaml = to_string(&Doc {
        items: FlowSeq(vec![1, 2]),
    })
    .unwrap();
    assert!(yaml.contains(','));
}

#[test]
fn ser_ser_flow_mapping_two_elements_emits_comma() {
    let mut m = BTreeMap::new();
    let _ = m.insert("a".to_string(), 1i32);
    let _ = m.insert("b".to_string(), 2);
    #[derive(Serialize)]
    struct Doc {
        flow: FlowMap<BTreeMap<String, i32>>,
    }
    let yaml = to_string(&Doc { flow: FlowMap(m) }).unwrap();
    assert!(yaml.contains(','));
}

// ============================================================================
// ser.rs — sequence-of-mapping with multi-key (drive line 1057 — value = Mapping)
// ============================================================================

#[test]
fn ser_ser_sequence_of_mapping_with_nested_mapping_value() {
    // Drives 1044-1048 + 1056-1060: mapping value within seq-item
    // dispatches to bumped indent for nested mappings.
    let mut nested = Mapping::new();
    let _ = nested.insert("x", Value::Number(noyalib::Number::Integer(1)));
    let _ = nested.insert("y", Value::Number(noyalib::Number::Integer(2)));

    let mut outer = Mapping::new();
    let _ = outer.insert("first", Value::Number(noyalib::Number::Integer(0)));
    let _ = outer.insert("inner", Value::Mapping(nested));

    let yaml = to_string_value(&Value::Sequence(vec![Value::Mapping(outer)])).unwrap();
    assert!(yaml.contains("first"));
    assert!(yaml.contains("inner"));
}

#[test]
fn ser_ser_sequence_of_sequence_at_root() {
    // Drives 1063-1064 (Sequence within sequence-item).
    let yaml = to_string_value(&Value::Sequence(vec![Value::Sequence(vec![
        Value::Number(noyalib::Number::Integer(1)),
        Value::Number(noyalib::Number::Integer(2)),
    ])]))
    .unwrap();
    assert!(yaml.contains('-'));
}

#[test]
fn ser_ser_empty_collections_emit_inline() {
    let yaml_seq = to_string_value(&Value::Sequence(vec![])).unwrap();
    let yaml_map = to_string_value(&Value::Mapping(Mapping::new())).unwrap();
    assert!(yaml_seq.contains("[]"));
    assert!(yaml_map.contains("{}"));
}

// ============================================================================
// cst/format.rs — branches by syntax kind
// ============================================================================

#[test]
fn ser_format_block_mapping_specific_branch() {
    // Drives `BlockMapping` branch (line 120-121).
    let input = "name: foo\nversion: 1\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("name:"));
}

#[test]
fn ser_format_block_sequence_specific_branch() {
    // Drives `BlockSequence` branch (line 123-124).
    let input = "- one\n- two\n- three\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("- one"));
}

#[test]
fn ser_format_mapping_entry_branch() {
    // Drives `MappingEntry` branch (126-127).
    let input = "k: v\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("k: v"));
}

#[test]
fn ser_format_sequence_item_branch() {
    let input = "- single\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("- single"));
}

#[test]
fn ser_format_flow_mapping_passthrough() {
    // Drives `FlowMapping | FlowSequence` branch (132-134) +
    // `write_verbatim`.
    let input = "{a: 1, b: 2}\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains('{'));
}

#[test]
fn ser_format_flow_sequence_passthrough() {
    let input = "[1, 2, 3]\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains('['));
}

#[test]
fn ser_format_doc_start_and_doc_end_markers() {
    // Drives 171-176 (DocStart / DocEnd branches).
    let input = "---\nfoo: bar\n...\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("---"));
    assert!(formatted.contains("..."));
}

#[test]
fn ser_format_dash_indicator_at_start_of_line() {
    // Drives 182-186: DashIndicator with at_line_start handling.
    let input = "items:\n  - 1\n  - 2\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("- 1"));
}

#[test]
fn ser_format_with_config_indent_size_four() {
    // Drives format_with_config + indent multiplier.
    let cfg = FormatConfig { indent_size: 4 };
    let formatted = format_with_config("k:\n  v: 1\n", &cfg).unwrap();
    assert!(formatted.contains("v: 1"));
}

#[test]
fn ser_format_block_mapping_with_block_value_drives_indent_step() {
    // Drives 263-272: block value under colon → newline + indent_level += 1.
    let input = "outer:\n  inner: value\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("inner: value"));
}

#[test]
fn ser_format_sequence_item_with_block_collection_inside() {
    // Drives 311-322 inside format_sequence_item: block mapping
    // following a dash.
    let input = "- key: 1\n  other: 2\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("key:"));
}

#[test]
fn ser_format_comment_is_preserved() {
    // Drives Comment branch (160-164).
    let input = "a: 1 # tail\n# header\nb: 2\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains('#'));
}

#[test]
fn ser_format_bom_and_whitespace_skipped() {
    // BOM + leading whitespace prefix.
    let input = "\u{FEFF}a: 1\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("a: 1"));
}

#[test]
fn ser_format_empty_input_returns_empty() {
    assert_eq!(format("").unwrap(), "");
    assert_eq!(format("   \n").unwrap(), "");
}

#[test]
fn ser_format_idempotent_on_canonical() {
    // Round-trip on already-canonical input.
    let canonical = "a: 1\nb:\n  - 2\n  - 3\n";
    let f1 = format(canonical).unwrap();
    let f2 = format(&f1).unwrap();
    assert_eq!(f1, f2);
}

// ============================================================================
// cst — indent_unit detection on varied inputs
// ============================================================================

#[test]
fn ser_cst_indent_unit_two_space() {
    let doc = parse_document("k:\n  v: 1\n").unwrap();
    assert_eq!(doc.indent_unit(), 2);
}

#[test]
fn ser_cst_indent_unit_four_space() {
    let doc = parse_document("k:\n    v: 1\n").unwrap();
    assert_eq!(doc.indent_unit(), 4);
}

#[test]
fn ser_cst_indent_unit_default_when_flat() {
    let doc = parse_document("a: 1\nb: 2\n").unwrap();
    assert_eq!(doc.indent_unit(), 2);
}

#[test]
fn ser_cst_dominant_quote_style_plain() {
    let doc = parse_document("a: one\nb: two\n").unwrap();
    assert_eq!(doc.dominant_quote_style(), ScalarStyle::Plain);
}

#[test]
fn ser_cst_dominant_quote_style_single() {
    let doc = parse_document("a: 'one'\nb: 'two'\n").unwrap();
    assert_eq!(doc.dominant_quote_style(), ScalarStyle::SingleQuoted);
}

#[test]
fn ser_cst_dominant_quote_style_double() {
    let doc = parse_document("a: \"one\"\nb: \"two\"\n").unwrap();
    assert_eq!(doc.dominant_quote_style(), ScalarStyle::DoubleQuoted);
}

#[test]
fn ser_cst_dominant_flow_style_block() {
    let doc = parse_document("a:\n  - 1\n  - 2\n").unwrap();
    assert_eq!(doc.dominant_flow_style(), FlowStyle::Block);
}

#[test]
fn ser_cst_dominant_flow_style_auto_for_flow_majority() {
    let doc = parse_document("a: [1, 2]\nb: [3, 4]\n").unwrap();
    assert_eq!(doc.dominant_flow_style(), FlowStyle::Auto);
}

// ============================================================================
// cst/builder.rs — drive parse_subtree / document_boundaries paths via API
// ============================================================================

#[test]
fn ser_cst_builder_replace_span_drives_local_repair() {
    // Drives parse_subtree → parse_block_collection /
    // parse_block_entry via `set`.
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    let (s, e) = doc.span_at("a").unwrap();
    doc.replace_span(s, e, "42").unwrap();
    assert_eq!(doc.to_string(), "a: 42\nb: 2\n");
}

#[test]
fn ser_cst_builder_set_path_replaces_value() {
    let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
    doc.set("version", "0.0.2").unwrap();
    assert!(doc.to_string().contains("0.0.2"));
}

#[test]
fn ser_cst_builder_set_unknown_path_errors() {
    let mut doc = parse_document("a: 1\n").unwrap();
    assert!(doc.set("nope", "x").is_err());
}

#[test]
fn ser_cst_builder_replace_span_out_of_bounds_errors() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let len = doc.source().len();
    assert!(doc.replace_span(0, len + 5, "x").is_err());
    assert!(doc.replace_span(5, 1, "x").is_err());
}

#[test]
fn ser_cst_builder_replace_span_non_char_boundary_errors() {
    // Multi-byte char: 'é' is two bytes; cutting between them is
    // not a char boundary.
    let mut doc = parse_document("a: é\n").unwrap();
    // 'é' starts at byte index 3 in "a: é\n", spans bytes 3..5.
    // Splitting at 4 hits mid-char.
    assert!(doc.replace_span(4, 5, "x").is_err());
}

#[test]
fn ser_cst_builder_parse_stream_multiple_docs_via_doc_end() {
    // Drives document_boundaries with explicit DocEnd plus the
    // CR/LF tail handling (lines 224-235 + 244-249).
    let src = "a: 1\n...\nb: 2\n...\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn ser_cst_builder_parse_stream_doc_start_separated() {
    let src = "---\na: 1\n---\nb: 2\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn ser_cst_builder_parse_stream_single_doc() {
    let src = "x: 1\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn ser_cst_builder_parse_stream_empty() {
    let src = "";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn ser_cst_builder_parse_stream_with_crlf_doc_end() {
    let src = "a: 1\n...\r\nb: 2\n";
    let _ = parse_stream(src);
}

#[test]
fn ser_cst_builder_flow_collection_round_trip() {
    // Exercises FlowMapping/FlowSequence frame push/pop in
    // TreeBuilder::handle_token (lines 433-452).
    let src = "a: {x: 1, y: 2}\nb: [1, 2, 3]\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(doc.to_string(), src);
}

#[test]
fn ser_cst_builder_anchor_alias_tag_tokens_round_trip() {
    // Anchor / alias / tag mark tokens cycle through handle_token.
    let src = "a: &A 1\nb: *A\nc: !!str hello\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(doc.to_string(), src);
}

#[test]
fn ser_cst_builder_quoted_scalar_paths() {
    // Single, double, literal, folded scalar token branches.
    let src = "a: 'single'\nb: \"double\"\nc: |\n  literal\nd: >\n  folded\n";
    let doc = parse_document(src).unwrap();
    assert!(doc.to_string().contains('|'));
}

#[test]
fn ser_cst_builder_complex_mapping_with_question_indicator() {
    // Drives R::QuestionIndicator (419-425).
    let src = "? key\n: value\n";
    let _ = parse_document(src);
}

#[test]
fn ser_cst_builder_set_value_changes_scalar() {
    let mut doc = parse_document("name: noyalib\nversion: 0.0.1\n").unwrap();
    doc.set_value("version", &Value::String("0.0.2".into()))
        .unwrap();
    assert!(doc.to_string().contains("0.0.2"));
}

#[test]
fn ser_cst_builder_remove_path() {
    let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    doc.remove("b").unwrap();
    let out = doc.to_string();
    assert!(out.contains("a: 1"));
    assert!(out.contains("c: 3"));
    assert!(!out.contains("b: 2"));
}

#[test]
fn ser_cst_builder_push_back_extends_sequence() {
    // Drives parse_subtree for SequenceItem.
    let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    doc.push_back("items", "three").unwrap();
    assert!(doc.to_string().contains("three"));
}

#[test]
fn ser_cst_builder_insert_entry_extends_mapping() {
    // Drives parse_subtree for MappingEntry.
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    doc.insert_entry("", "c", "3").unwrap();
    assert!(doc.to_string().contains("c: 3"));
}

#[test]
fn ser_cst_builder_round_trip_byte_identical() {
    // Round-trip-from-source covers the assemble + handle_token
    // matrix.
    let src = "# leading comment\nname: noyalib\nversion: 0.0.1\nfeatures:\n  - parser\n  - cst\n# trailing\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(doc.to_string(), src);
}
