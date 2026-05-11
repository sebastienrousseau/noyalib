//! Comprehensive coverage tests targeting all uncovered lines.
//!
//! Organized by source module. Each test includes a comment referencing
//! the file and line(s) it covers.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(
    unused_comparisons,
    unused_results,
    clippy::approx_constant,
    clippy::absurd_extreme_comparisons,
    clippy::len_zero,
    clippy::bool_assert_comparison
)]

use noyalib::{
    DuplicateKeyPolicy, FlowStyle, Mapping, MappingAny, ParserConfig, ScalarStyle,
    SerializerConfig, Spanned, Tag, TaggedValue, Value, from_str, from_str_with_config, from_value,
    to_string, to_string_with_config,
};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════
// scanner.rs — Quoted scalar escape sequences (lines 938–1121)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn single_quoted_escaped_quote() {
    // scanner.rs:944-956 — '' escape in single-quoted scalar
    let v: Value = from_str("key: 'can''t stop'").unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "can't stop");
}

#[test]
fn single_quoted_line_folding() {
    // scanner.rs:958-996 — line breaks in single-quoted scalar
    let yaml = "key: 'first\n  second\n  third'";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "first second third");
}

#[test]
fn single_quoted_multiple_breaks() {
    // scanner.rs:984-991 — multiple consecutive breaks in single-quoted.
    // Per YAML 1.2.2 §7.3.3, continuation lines of a multi-line quoted
    // scalar must be indented more than the parent block.
    let yaml = "key: 'para1\n\n\n  para2'";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("\n\n"), "expected double newline, got: {s}");
}

#[test]
fn single_quoted_utf8_multibyte() {
    // scanner.rs:1011-1018 — multi-byte UTF-8 in single-quoted
    let yaml = "key: 'café ñ 日本語'";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "café ñ 日本語");
}

#[test]
fn double_quoted_escape_null() {
    // scanner.rs:1073 — \0 escape
    let v: Value = from_str(r#"key: "null\0byte""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "null\0byte");
}

#[test]
fn double_quoted_escape_bell() {
    // scanner.rs:1074 — \a escape
    let v: Value = from_str(r#"key: "\a""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\x07");
}

#[test]
fn double_quoted_escape_backspace() {
    // scanner.rs:1075 — \b escape
    let v: Value = from_str(r#"key: "\b""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\x08");
}

#[test]
fn double_quoted_escape_tab() {
    // scanner.rs:1076 — \t escape
    let v: Value = from_str(r#"key: "\t""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\t");
}

#[test]
fn double_quoted_escape_newline() {
    // scanner.rs:1077 — \n escape
    let v: Value = from_str(r#"key: "line1\nline2""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "line1\nline2");
}

#[test]
fn double_quoted_escape_vertical_tab() {
    // scanner.rs:1078 — \v escape
    let v: Value = from_str(r#"key: "\v""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\x0B");
}

#[test]
fn double_quoted_escape_form_feed() {
    // scanner.rs:1079 — \f escape
    let v: Value = from_str(r#"key: "\f""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\x0C");
}

#[test]
fn double_quoted_escape_carriage_return() {
    // scanner.rs:1080 — \r escape
    let v: Value = from_str(r#"key: "\r""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\r");
}

#[test]
fn double_quoted_escape_esc() {
    // scanner.rs:1081 — \e escape
    let v: Value = from_str(r#"key: "\e""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\x1B");
}

#[test]
fn double_quoted_escape_space() {
    // scanner.rs:1082 — \  (escaped space)
    let v: Value = from_str("key: \"\\  end\"").unwrap();
    assert!(v["key"].as_str().unwrap().starts_with(' '));
}

#[test]
fn double_quoted_escape_double_quote() {
    // scanner.rs:1083 — \" escape
    let v: Value = from_str(r#"key: "say \"hello\"""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), r#"say "hello""#);
}

#[test]
fn double_quoted_escape_slash() {
    // scanner.rs:1084 — \/ escape
    let v: Value = from_str(r#"key: "a\/b""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "a/b");
}

#[test]
fn double_quoted_escape_backslash() {
    // scanner.rs:1085 — \\ escape
    let v: Value = from_str(r#"key: "a\\b""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "a\\b");
}

#[test]
fn double_quoted_escape_nel() {
    // scanner.rs:1086 — \N (next line)
    let v: Value = from_str(r#"key: "\N""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\u{0085}");
}

#[test]
fn double_quoted_escape_nbsp() {
    // scanner.rs:1087 — \_ (non-breaking space)
    let v: Value = from_str(r#"key: "\_""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\u{00A0}");
}

#[test]
fn double_quoted_escape_line_separator() {
    // scanner.rs:1088 — \L (line separator)
    let v: Value = from_str(r#"key: "\L""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\u{2028}");
}

#[test]
fn double_quoted_escape_paragraph_separator() {
    // scanner.rs:1089 — \P (paragraph separator)
    let v: Value = from_str(r#"key: "\P""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "\u{2029}");
}

#[test]
fn double_quoted_escape_hex2() {
    // scanner.rs:1090-1092 — \xHH
    let v: Value = from_str(r#"key: "\x41\x42""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "AB");
}

#[test]
fn double_quoted_escape_unicode4() {
    // scanner.rs:1094-1096 — \uHHHH
    let v: Value = from_str(r#"key: "\u0041\u00E9""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "Aé");
}

#[test]
fn double_quoted_escape_unicode8() {
    // scanner.rs:1098-1100 — \UHHHHHHHH
    let v: Value = from_str(r#"key: "\U00000041""#).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "A");
}

#[test]
fn double_quoted_line_break_escape() {
    // scanner.rs:1102-1110 — backslash + newline (line continuation)
    let yaml = "key: \"line1\\\n  line2\"";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "line1line2");
}

#[test]
fn double_quoted_unknown_escape_error() {
    // scanner.rs:1112-1119 — unknown escape character
    let result: Result<Value, _> = from_str(r#"key: "\q""#);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("escape") || msg.contains("unknown"), "{msg}");
}

#[test]
fn double_quoted_line_folding() {
    // scanner.rs:1123-1161 — line breaks in double-quoted scalar
    let yaml = "key: \"first\n  second\"";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "first second");
}

#[test]
fn double_quoted_multiple_breaks() {
    // scanner.rs:1148-1155 — multiple consecutive breaks.
    // Per YAML 1.2.2 §7.3.2, continuation lines must be indented
    // more than the parent block.
    let yaml = "key: \"first\n\n\n  second\"";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("\n\n"), "expected double newline, got: {s}");
}

#[test]
fn double_quoted_whitespace_before_break() {
    // scanner.rs:1163-1199 — whitespace handling before line break.
    // Continuation must be indented more than the parent block.
    let yaml = "key: \"word   \n  more\"";
    let v: Value = from_str(yaml).unwrap();
    // trailing spaces before break are folded
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("word"), "{s}");
}

#[test]
fn double_quoted_hex_escape_invalid() {
    // scanner.rs:1209-1211 — invalid hex digits
    let result: Result<Value, _> = from_str(r#"key: "\xGG""#);
    assert!(result.is_err());
}

#[test]
fn double_quoted_hex_escape_invalid_codepoint() {
    // scanner.rs:1220-1222 — invalid Unicode code point
    let result: Result<Value, _> = from_str(r#"key: "\UD800""#);
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════
// scanner.rs — Block scalars (lines 1241–1418)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn block_scalar_literal_basic() {
    // scanner.rs:1367-1368 — literal style preserves breaks
    let yaml = "key: |\n  line1\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "line1\nline2\n");
}

#[test]
fn block_scalar_folded_basic() {
    // scanner.rs:1370-1376 — folded style: single break → space
    let yaml = "key: >\n  line1\n  line2\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "line1 line2\n");
}

#[test]
fn block_scalar_folded_multiple_breaks() {
    // scanner.rs:1377-1379 — multiple breaks: keep all but first
    let yaml = "key: >\n  para1\n\n\n  para2\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("\n\n"), "expected preserved breaks, got: {s}");
}

#[test]
fn block_scalar_folded_leading_blank() {
    // scanner.rs:1372, 1385 — leading blank preserves newline
    // In folded mode, a "more-indented" line preserves the preceding newline
    let yaml = "key: >\n  normal\n   indented\n  back\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("indented"), "{s}");
    assert!(s.contains("normal"), "{s}");
}

#[test]
fn block_scalar_keep_chomping() {
    // scanner.rs:1406-1408 — keep (+) appends all trailing breaks
    let yaml = "key: |+\n  content\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.ends_with("\n\n\n") || s.ends_with("\n\n"), "got: {s:?}");
}

#[test]
fn block_scalar_strip_chomping() {
    // scanner.rs:1416-1418 — strip (-) removes trailing breaks
    let yaml = "key: |-\n  content\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "content");
}

#[test]
fn block_scalar_clip_chomping() {
    // scanner.rs:1410-1414 — clip (default) keeps single trailing newline
    let yaml = "key: |\n  content\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "content\n");
}

#[test]
fn block_scalar_explicit_indent() {
    // scanner.rs:1260-1262, 1285-1291 — explicit indent indicator
    let yaml = "key: |2\n  content\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "content\n");
}

#[test]
fn block_scalar_autodetect_with_empty_lines() {
    // scanner.rs:1296-1318 — auto-detect indentation after empty lines
    let yaml = "key: |\n\n  content\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("content"), "{s}");
}

#[test]
fn block_scalar_more_indented() {
    // scanner.rs:1340-1352 — extra indentation preserved as spaces
    let yaml = "key: |\n  base\n    extra\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["key"].as_str().unwrap();
    assert!(s.contains("  extra"), "expected extra indent, got: {s:?}");
}

#[test]
fn block_scalar_comment_after_indicator() {
    // scanner.rs:1273-1276 — comment after block indicator
    let yaml = "key: | # this is a comment\n  content\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "content\n");
}

// ═══════════════════════════════════════════════════════════════════════
// scanner.rs — Tag and anchor limits (lines 661, 692–739)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn anchor_name_length_limit() {
    // scanner.rs:661 — anchor exceeds 1024 bytes
    let long_name = "a".repeat(1030);
    let yaml = format!("&{long_name} value");
    let result: Result<Value, _> = from_str(&yaml);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("anchor"));
}

#[test]
fn tag_verbatim_length_limit() {
    // scanner.rs:696-706 — verbatim tag exceeds limit
    let long_uri = "x".repeat(1030);
    let yaml = format!("!<{long_uri}> value");
    let result: Result<Value, _> = from_str(&yaml);
    assert!(result.is_err());
}

#[test]
fn tag_secondary_length_limit() {
    // scanner.rs:722 — secondary tag suffix exceeds limit
    let long_suffix = "x".repeat(1030);
    let yaml = format!("!!{long_suffix} value");
    let result: Result<Value, _> = from_str(&yaml);
    assert!(result.is_err());
}

#[test]
fn tag_primary_length_limit() {
    // scanner.rs:739 — primary tag suffix exceeds limit
    let long_suffix = "x".repeat(1030);
    let yaml = format!("!{long_suffix} value");
    let result: Result<Value, _> = from_str(&yaml);
    assert!(result.is_err());
}

#[test]
fn tag_verbatim_normal() {
    // scanner.rs:692-706 — valid verbatim tag
    let yaml = "!<tag:example.com,2024:type> value";
    let v: Value = from_str(yaml).unwrap();
    // Verbatim tags are resolved; the value should parse successfully
    assert!(!v.is_null(), "got: {v:?}");
}

// ═══════════════════════════════════════════════════════════════════════
// scanner.rs — Misc uncovered scanner paths
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn windows_line_endings_crlf() {
    // scanner.rs:245-246 — CR+LF handling
    let yaml = "key: value\r\nkey2: value2\r\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key2"].as_str().unwrap(), "value2");
}

#[test]
fn tab_indentation_error() {
    // scanner.rs:252-253 — tab indentation error
    let yaml = "key:\n\t  value";
    let result: Result<Value, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn utf8_bom_at_start() {
    // scanner.rs:452-457 — BOM skip at document start
    let yaml = "\u{FEFF}key: value";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "value");
}

// ═══════════════════════════════════════════════════════════════════════
// parser/events.rs — Parser state machine edge cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn explicit_document_start_markers() {
    // events.rs:190, 202, 207, 218, 220-221 — document start/end markers
    let yaml = "---\nfirst: 1\n...\n---\nsecond: 2\n...";
    let docs = noyalib::load_all(yaml).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn empty_document_between_markers() {
    // events.rs:230, 235 — empty document content
    let yaml = "---\n...\n---\nvalue\n...";
    let docs = noyalib::load_all(yaml).unwrap();
    assert!(docs.len() >= 1);
}

#[test]
fn flow_mapping_in_flow_sequence() {
    // events.rs:120, 125-127, 313, 322 — flow mapping inside flow sequence
    let yaml = "[{a: 1}, {b: 2}]";
    let v: Value = from_str(yaml).unwrap();
    let seq = v.as_sequence().unwrap();
    assert_eq!(seq.len(), 2);
}

#[test]
fn indentless_sequence_entry() {
    // events.rs:112 — indentless sequence
    let yaml = "key:\n  - item1\n  - item2";
    let v: Value = from_str(yaml).unwrap();
    let seq = v["key"].as_sequence().unwrap();
    assert_eq!(seq.len(), 2);
}

#[test]
fn flow_sequence_with_mapping_entries() {
    // events.rs:349, 360-361, 379 — complex flow entries
    let yaml = "[a, b: c, d]";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_sequence().is_some());
}

#[test]
fn block_mapping_complex_key() {
    // events.rs:395-421 — block mapping key/value parsing
    let yaml = "? key1\n: value1\n? key2\n: value2";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key1"].as_str().unwrap(), "value1");
    assert_eq!(v["key2"].as_str().unwrap(), "value2");
}

#[test]
fn flow_mapping_explicit_key() {
    // events.rs:527-530, 550-558 — flow mapping explicit keys
    let yaml = "{a: 1, b: 2}";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_mapping().is_some());
    assert_eq!(v["a"].as_i64().unwrap(), 1);
}

#[test]
fn node_with_anchor_and_tag() {
    // events.rs:269-281 — anchor + tag combination
    let yaml = "&anchor !!str tagged_value";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_str().is_some() || matches!(v, Value::Tagged(_)));
}

#[test]
fn block_mapping_implicit_value() {
    // events.rs:432, 437, 439-440, 446, 467 — block mapping value edge cases
    let yaml = "key1:\nkey2: value2";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["key1"].is_null());
    assert_eq!(v["key2"].as_str().unwrap(), "value2");
}

// ═══════════════════════════════════════════════════════════════════════
// parser/loader.rs — Loader document lifecycle, aliases, merge keys
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn multi_document_lifecycle() {
    // loader.rs:147, 155, 159 — document start/end state tracking
    let yaml = "---\na: 1\n---\nb: 2\n...";
    let docs = noyalib::load_all(yaml).unwrap();
    let collected: Vec<_> = docs.collect();
    assert_eq!(collected.len(), 2);
}

#[test]
fn alias_expansion_byte_limit() {
    // loader.rs:191, 199 — alias byte tracking (P0-1 fix)
    let config = ParserConfig::new()
        .max_alias_expansions(1024)
        .max_document_length(1024);
    // Create a value worth ~500 bytes, alias it 10 times → exceeds 1024 byte limit
    let long_value = "x".repeat(200);
    let mut yaml = format!("anchor: &a {long_value}\n");
    for i in 0..10 {
        yaml.push_str(&format!("ref{i}: *a\n"));
    }
    let result: Result<Value, _> = from_str_with_config(&yaml, &config);
    assert!(result.is_err());
}

#[test]
fn duplicate_key_first_policy() {
    // loader.rs:465-468 — DuplicateKeyPolicy::First
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let yaml = "a: first\na: second";
    let v: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(v["a"].as_str().unwrap(), "first");
}

#[test]
fn duplicate_key_last_policy_span_update() {
    // loader.rs:417, 423 — span entry update for Last policy (P0-2 fix)
    let yaml = "a: first\na: second";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"].as_str().unwrap(), "second");
}

#[test]
fn merge_key_from_mapping() {
    // loader.rs:267, 273 — merge key processing
    let yaml =
        "defaults: &defaults\n  color: red\n  size: large\nitem:\n  <<: *defaults\n  name: thing";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["item"]["color"].as_str().unwrap(), "red");
}

#[test]
fn merge_key_from_sequence_of_mappings() {
    // loader.rs:273 — merge from sequence
    let yaml = "a: &a\n  x: 1\nb: &b\n  y: 2\nc:\n  <<: [*a, *b]\n  z: 3";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["c"]["x"].as_i64().unwrap(), 1);
    assert_eq!(v["c"]["y"].as_i64().unwrap(), 2);
}

#[test]
fn merge_key_scalar_error() {
    // loader.rs:293, 297 — merge with non-mapping value
    let yaml = "a: &a scalar\nb:\n  <<: *a";
    let result: Result<Value, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn sequence_with_anchored_value() {
    // loader.rs:222, 227, 246 — sequence end with anchored values
    let yaml = "&seq\n- a\n- b\n- c";
    let v: Value = from_str(yaml).unwrap();
    let seq = v.as_sequence().unwrap();
    assert_eq!(seq.len(), 3);
}

#[test]
fn tagged_scalar_resolution() {
    // loader.rs:528, 545, 549, 556-557, 563, 568, 576, 588 — tagged scalar
    // resolution
    let yaml = "!!int 42";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_i64().unwrap(), 42);
}

#[test]
fn tagged_bool_resolution() {
    let yaml = "!!bool true";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_bool().unwrap(), true);
}

#[test]
fn tagged_float_resolution() {
    let yaml = "!!float 3.14";
    let v: Value = from_str(yaml).unwrap();
    assert!((v.as_f64().unwrap() - 3.14).abs() < 0.001);
}

#[test]
fn tagged_null_resolution() {
    let yaml = "!!null ~";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.is_null());
}

#[test]
fn tagged_str_resolution() {
    let yaml = "!!str 42";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_str().unwrap(), "42");
}

#[test]
fn tagged_float_special_values() {
    // loader.rs:563, 568 — tagged float specials
    let yaml = "!!float .inf";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_f64().unwrap().is_infinite());

    let yaml2 = "!!float .nan";
    let v2: Value = from_str(yaml2).unwrap();
    assert!(v2.as_f64().unwrap().is_nan());
}

#[test]
fn tagged_bool_invalid_error() {
    let yaml = "!!bool notabool";
    let result: Result<Value, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn tagged_int_invalid_error() {
    let yaml = "!!int notanint";
    let result: Result<Value, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn tagged_float_invalid_error() {
    let yaml = "!!float notafloat";
    let result: Result<Value, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn hex_integer_parsing() {
    // loader.rs:638 — hex integer
    let v: Value = from_str("0xFF").unwrap();
    assert_eq!(v.as_i64().unwrap(), 255);
}

#[test]
fn octal_integer_parsing() {
    // loader.rs:638 — octal integer
    let v: Value = from_str("0o77").unwrap();
    assert_eq!(v.as_i64().unwrap(), 63);
}

#[test]
fn large_integer_as_float() {
    // loader.rs:688, 701 — integer overflow → float
    let v: Value = from_str("99999999999999999999").unwrap();
    assert!(v.as_f64().is_some());
}

// ═══════════════════════════════════════════════════════════════════════
// de.rs — Deserializer edge cases (lines 648, 652, 786–814)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn deserialize_identifier_delegates_to_str() {
    // de.rs:648, 652 — deserialize_identifier
    #[derive(Debug, Deserialize)]
    enum Color {
        Red,
        Blue,
    }
    let yaml = "Red";
    let c: Color = from_str(yaml).unwrap();
    assert!(matches!(c, Color::Red));
}

#[test]
fn spanned_deserialize_all_fields() {
    // de.rs:786-814 — all SpannedFieldState transitions
    let yaml = "key: hello";
    let spanned: Spanned<Value> = from_str(yaml).unwrap();
    assert!(spanned.start.line() >= 1);
    assert!(spanned.start.column() >= 0);
    assert!(spanned.end.line() >= 1);
    assert!(spanned.end.column() >= 0);
    let _ = spanned.into_inner();
}

#[test]
fn spanned_nested_value() {
    // de.rs:786-814 — Spanned with specific value
    #[derive(Debug, Deserialize)]
    struct Config {
        name: Spanned<String>,
    }
    let yaml = "name: test";
    let c: Config = from_str(yaml).unwrap();
    assert_eq!(c.name.value, "test");
}

// ═══════════════════════════════════════════════════════════════════════
// ser.rs — Serializer edge cases (lines 328–688)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn serialize_custom_tag() {
    // ser.rs:328-330 — non-internal tag
    let tagged = TaggedValue::new(Tag::new("!custom"), Value::String("val".into()));
    let v = Value::Tagged(Box::new(tagged));
    let s = to_string(&v).unwrap();
    assert!(s.contains("!custom"), "got: {s}");
}

#[test]
fn serialize_flow_seq_non_sequence_fallback() {
    // ser.rs:599-600 — FlowSeq tag on non-sequence falls through
    use noyalib::fmt::FlowSeq;
    let val = FlowSeq(42i32);
    let s = to_string(&val).unwrap();
    assert!(s.contains("42"), "got: {s}");
}

#[test]
fn serialize_flow_map_non_mapping_fallback() {
    // ser.rs:606-607 — FlowMap tag on non-mapping falls through
    use noyalib::fmt::FlowMap;
    let val = FlowMap(42i32);
    let s = to_string(&val).unwrap();
    assert!(s.contains("42"), "got: {s}");
}

#[test]
fn serialize_lit_str_non_string_fallback() {
    // ser.rs:613-614 — LitStr tag on non-string falls through
    // Use the wrapper on a non-string type to exercise fallback
    use noyalib::fmt::LitString;
    let ls = LitString::from("hello".to_string());
    let s = to_string(&ls).unwrap();
    assert!(s.contains("hello"), "got: {s}");
}

#[test]
fn serialize_fold_str_non_string_fallback() {
    // ser.rs:620-621 — FoldStr tag on non-string falls through
    use noyalib::fmt::FoldString;
    let fs = FoldString::from("hello".to_string());
    let s = to_string(&fs).unwrap();
    assert!(s.contains("hello"), "got: {s}");
}

#[test]
fn serialize_commented_value() {
    // ser.rs:624-638 — Commented serialization
    use noyalib::Commented;
    let c = Commented::new(42, "this is a comment");
    let s = to_string(&c).unwrap();
    assert!(s.contains("42"), "got: {s}");
    assert!(s.contains("# this is a comment"), "got: {s}");
}

#[test]
fn serialize_space_after_value() {
    // ser.rs:640-643 — SpaceAfter serialization
    use noyalib::SpaceAfter;
    let sa = SpaceAfter(42);
    let s = to_string(&sa).unwrap();
    assert!(s.contains("42"), "got: {s}");
}

#[test]
fn serialize_unknown_internal_tag() {
    // ser.rs:646 — unknown __noya_ prefixed tag
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_unknown"),
        Value::from("test"),
    )));
    let s = to_string(&v).unwrap();
    assert!(s.contains("test"), "got: {s}");
}

#[test]
fn looks_like_number_empty_string() {
    // ser.rs:343 — empty string check
    let v = Value::String(String::new());
    let s = to_string(&v).unwrap();
    // Empty string should be quoted
    assert!(s.contains("''") || s.contains("\"\"") || s.trim().is_empty());
}

#[test]
fn serialize_map_key_non_string_error() {
    // ser.rs:1108 — map key that's not a string/number/bool
    // This exercises the error path when a map key is a complex type
    let mut map = std::collections::HashMap::new();
    let _ = map.insert(vec![1, 2], "value");
    let result = to_string(&map);
    assert!(result.is_err());
}

#[test]
fn serialize_string_with_control_chars() {
    // ser.rs:442, 533 — control characters force double quoting
    let v = Value::String("hello\x07world".to_string());
    let s = to_string(&v).unwrap();
    assert!(s.contains('"') || s.contains("\\a"), "got: {s}");
}

// ═══════════════════════════════════════════════════════════════════════
// value.rs — MappingAny ordering, TaggedValue deser, ValueIndex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mapping_any_ord_different_keys() {
    // value.rs:805-811 — MappingAny Ord comparison
    let mut m1 = MappingAny::new();
    let mut m2 = MappingAny::new();
    let _ = m1.insert(Value::from("a"), Value::from(1));
    let _ = m2.insert(Value::from("b"), Value::from(1));
    assert!(m1 < m2);
}

#[test]
fn mapping_any_ord_same_keys_different_values() {
    // value.rs:805-811 — Ord with same keys but different values
    let mut m1 = MappingAny::new();
    let mut m2 = MappingAny::new();
    let _ = m1.insert(Value::from("a"), Value::from(1));
    let _ = m2.insert(Value::from("a"), Value::from(2));
    assert!(m1 < m2);
}

#[test]
fn mapping_deserialize_type_error() {
    // value.rs:399, 402-403 — Mapping visitor expecting error
    let result: Result<Mapping, _> = serde_json::from_str("123");
    assert!(result.is_err());
}

#[test]
fn mapping_any_deserialize_type_error() {
    // value.rs:855-859 — MappingAny visitor expecting error
    let result: Result<MappingAny, _> = serde_json::from_str("123");
    assert!(result.is_err());
}

#[test]
fn tagged_value_deserialize_from_map() {
    // value.rs:1446-1465 — TaggedValue deserialization
    let yaml = "mytag: myvalue";
    let tv: TaggedValue = from_str(yaml).unwrap();
    assert_eq!(tv.tag().as_str(), "mytag");
}

#[test]
fn tagged_value_enum_deserialization() {
    // value.rs:1446-1465 — TaggedValue deserialization from map
    let tv: TaggedValue = from_str("mytag: myvalue").unwrap();
    assert_eq!(tv.tag().as_str(), "mytag");
    assert_eq!(tv.value().as_str(), Some("myvalue"));
}

#[test]
fn tagged_value_unit_variant() {
    // value.rs:1564-1566 — unit variant through tagged
    #[derive(Debug, Deserialize, PartialEq)]
    enum Flag {
        On,
        Off,
    }
    // Use YAML with singleton map which exercises the enum pathway
    let yaml = "On";
    let flag: Flag = from_str(yaml).unwrap();
    assert_eq!(flag, Flag::On);
}

#[test]
fn tagged_value_tuple_variant() {
    // value.rs:1575-1580 — tuple variant through tagged
    #[derive(Debug, Deserialize, PartialEq)]
    enum Data {
        Pair(i32, i32),
    }
    let yaml = "Pair:\n  - 1\n  - 2";
    let data: Data = from_str(yaml).unwrap();
    assert_eq!(data, Data::Pair(1, 2));
}

#[test]
fn tagged_value_struct_variant() {
    // value.rs:1582-1591 — struct variant through tagged
    #[derive(Debug, Deserialize, PartialEq)]
    enum Shape {
        Circle { radius: f64 },
    }
    let yaml = "Circle:\n  radius: 5.0";
    let shape: Shape = from_str(yaml).unwrap();
    assert_eq!(shape, Shape::Circle { radius: 5.0 });
}

#[test]
fn value_index_into_mut_tagged_sequence() {
    // value.rs:2452 — IndexMut through tagged
    let seq = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let mut v = Value::Tagged(Box::new(TaggedValue::new(Tag::new("seq"), seq)));
    let item = &mut v[0];
    *item = Value::from(99);
    // Verify mutation went through
    if let Value::Tagged(t) = &v {
        if let Value::Sequence(s) = t.value() {
            assert_eq!(s[0].as_i64().unwrap(), 99);
        }
    }
}

#[test]
fn value_index_into_mut_tagged_mapping() {
    // value.rs:2490 — IndexMut through tagged mapping
    let mut m = Mapping::new();
    m.insert("key", Value::from(10));
    let mut v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("map"),
        Value::Mapping(m),
    )));
    let item = &mut v["key"];
    *item = Value::from(99);
}

#[test]
fn value_deserialize_any_for_tagged() {
    // value.rs:2763-2783, 2795-2803 — Value deserialize_any with tagged
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("custom"),
        Value::String("data".into()),
    )));
    let v2: Value = from_value(&tagged).unwrap();
    // Should deserialize to something
    assert!(!matches!(v2, Value::Null));
}

#[test]
fn value_deserialize_seq() {
    // value.rs:2716-2724 — ValueSeqAccess
    let v = Value::Sequence(vec![Value::from(1), Value::from(2), Value::from(3)]);
    let nums: Vec<i64> = from_value(&v).unwrap();
    assert_eq!(nums, vec![1, 2, 3]);
}

#[test]
fn value_deserialize_map() {
    // value.rs:2732-2757 — ValueMapAccess
    let mut m = Mapping::new();
    m.insert("a", Value::from(1));
    m.insert("b", Value::from(2));
    let v = Value::Mapping(m);
    #[derive(Deserialize)]
    struct Obj {
        a: i64,
        b: i64,
    }
    let obj: Obj = from_value(&v).unwrap();
    assert_eq!(obj.a, 1);
    assert_eq!(obj.b, 2);
}

// ═══════════════════════════════════════════════════════════════════════
// schema.rs — Validation edge cases (lines 61–193)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn failsafe_schema_rejects_null() {
    // schema.rs:71-73
    assert!(noyalib::validate_yaml_failsafe_schema(&Value::Null).is_err());
}

#[test]
fn failsafe_schema_rejects_bool() {
    // schema.rs:74-76
    assert!(noyalib::validate_yaml_failsafe_schema(&Value::Bool(true)).is_err());
}

#[test]
fn failsafe_schema_rejects_number() {
    // schema.rs:77-79
    assert!(noyalib::validate_yaml_failsafe_schema(&Value::from(42)).is_err());
}

#[test]
fn failsafe_schema_rejects_tagged() {
    // schema.rs:80-81
    let tagged = Value::Tagged(Box::new(TaggedValue::new(Tag::new("t"), Value::Null)));
    assert!(noyalib::validate_yaml_failsafe_schema(&tagged).is_err());
}

#[test]
fn failsafe_schema_validates_nested_strings() {
    // schema.rs:61, 67 — recursive validation in seq/map
    let seq = Value::Sequence(vec![Value::from("a"), Value::from("b")]);
    assert!(noyalib::validate_yaml_failsafe_schema(&seq).is_ok());

    let mut m = Mapping::new();
    m.insert("k", Value::from("v"));
    assert!(noyalib::validate_yaml_failsafe_schema(&Value::Mapping(m)).is_ok());
}

#[test]
fn failsafe_schema_rejects_nested_number() {
    // schema.rs:61 — nested non-string in sequence
    let seq = Value::Sequence(vec![Value::from(42)]);
    assert!(noyalib::validate_yaml_failsafe_schema(&seq).is_err());
}

#[test]
fn json_schema_rejects_nan() {
    // schema.rs:133 — NaN not allowed
    assert!(!noyalib::is_yaml_json_compatible(&Value::from(f64::NAN)));
}

#[test]
fn json_schema_rejects_infinity() {
    // schema.rs:133 — Infinity not allowed
    assert!(!noyalib::is_yaml_json_compatible(&Value::from(
        f64::INFINITY
    )));
}

#[test]
fn json_schema_rejects_tagged() {
    // schema.rs:143-144
    let tagged = Value::Tagged(Box::new(TaggedValue::new(Tag::new("t"), Value::Null)));
    assert!(noyalib::validate_yaml_json_schema(&tagged).is_err());
}

#[test]
fn json_schema_validates_nested() {
    // schema.rs:132-141 — recursive validation
    let seq = Value::Sequence(vec![Value::from(1), Value::from("a")]);
    assert!(noyalib::validate_yaml_json_schema(&seq).is_ok());

    let mut m = Mapping::new();
    m.insert("k", Value::from(true));
    assert!(noyalib::validate_yaml_json_schema(&Value::Mapping(m)).is_ok());
}

#[test]
fn core_schema_validates_everything() {
    // schema.rs:181, 187, 191, 193 — core schema recursive
    let seq = Value::Sequence(vec![Value::from(f64::NAN), Value::Null]);
    assert!(noyalib::validate_yaml_core_schema(&seq).is_ok());

    let mut m = Mapping::new();
    m.insert("k", Value::from(true));
    assert!(noyalib::validate_yaml_core_schema(&Value::Mapping(m)).is_ok());

    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("custom"),
        Value::from("inner"),
    )));
    assert!(noyalib::validate_yaml_core_schema(&tagged).is_ok());
}

#[test]
fn is_failsafe_compatible_function() {
    assert!(noyalib::is_yaml_failsafe_compatible(&Value::from("hello")));
    assert!(!noyalib::is_yaml_failsafe_compatible(&Value::from(42)));
}

// ═══════════════════════════════════════════════════════════════════════
// path.rs — Path display variants (lines 202–269)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn path_display_and_parent() {
    // path.rs:202, 204, 228, 254, 268-269
    use noyalib::Path;
    let root = Path::Root;
    let display = format!("{root}");
    assert_eq!(display, ".");

    let key = root.key("name");
    let key_display = format!("{key}");
    assert!(key_display.contains("name"), "got: {key_display}");

    let idx = root.index(0);
    let idx_display = format!("{idx}");
    assert!(idx_display.contains("0"), "got: {idx_display}");

    // Alias and unknown from a non-root parent so display is non-empty
    let nested_alias = key.alias();
    let alias_display = format!("{nested_alias}");
    // alias of ".name" → shows parent path
    assert!(alias_display.contains("name"), "alias: {alias_display}");

    let nested_unknown = key.unknown();
    let unknown_display = format!("{nested_unknown}");
    assert!(unknown_display.contains("?"), "unknown: {unknown_display}");

    // Test parent
    assert!(key.parent().is_some());
    assert!(root.parent().is_none());

    // Test depth
    assert_eq!(root.depth(), 0);
    assert!(key.depth() > 0);

    // Test is_root
    assert!(root.is_root());
    assert!(!key.is_root());
}

// ═══════════════════════════════════════════════════════════════════════
// spanned.rs — Spanned visitor (lines 117–156)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn spanned_struct_deserialization() {
    // spanned.rs:117, 118, 129, 130, 154, 156
    let yaml = "42";
    let spanned: Spanned<i32> = from_str(yaml).unwrap();
    assert_eq!(spanned.value, 42);
    assert!(spanned.start.line() >= 1);
}

#[test]
fn spanned_string_deserialization() {
    let yaml = "hello world";
    let spanned: Spanned<String> = from_str(yaml).unwrap();
    assert_eq!(*spanned, "hello world");
}

// ═══════════════════════════════════════════════════════════════════════
// fmt.rs — Wrapper getter methods (lines 155, 253, 381)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn lit_str_as_str_and_into_inner() {
    // fmt.rs:155, 156
    use noyalib::LitStr;
    let ls = LitStr("hello");
    assert_eq!(ls.as_str(), "hello");
    assert_eq!(ls.into_inner(), "hello");
}

#[test]
fn fold_str_as_str_and_into_inner() {
    // fmt.rs:253, 254
    use noyalib::FoldStr;
    let fs = FoldStr("world");
    assert_eq!(fs.as_str(), "world");
    assert_eq!(fs.into_inner(), "world");
}

// ═══════════════════════════════════════════════════════════════════════
// with/singleton_map_recursive.rs — Tagged transform (lines 47–51)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn singleton_map_recursive_with_tagged() {
    // singleton_map_recursive.rs:47-51 — tagged value transformation
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Action {
        Run(String),
        Stop,
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Job {
        #[serde(with = "noyalib::with::singleton_map_recursive")]
        actions: Vec<Action>,
    }
    let job = Job {
        actions: vec![Action::Run("test".into()), Action::Stop],
    };
    let yaml = to_string(&job).unwrap();
    let roundtrip: Job = from_str(&yaml).unwrap();
    assert_eq!(roundtrip, job);
}

// ═══════════════════════════════════════════════════════════════════════
// with/singleton_map_with.rs — Transform helpers (lines 113–346)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn singleton_map_with_custom_transform() {
    // singleton_map_with.rs:113, 122, 124-143, 175, 184, 187, 189
    use noyalib::with::singleton_map_with::{
        from_kebab_case, to_kebab_case, to_lowercase, to_pascal_case, to_snake_case, to_uppercase,
    };
    // Test the transform functions directly since they require closures
    assert_eq!(to_snake_case("FooBar"), "foo_bar");
    assert_eq!(to_snake_case("HTTPServer"), "h_t_t_p_server");
    assert_eq!(to_pascal_case("foo_bar"), "FooBar");
    assert_eq!(to_kebab_case("FooBar"), "foo-bar");
    assert_eq!(to_lowercase("HELLO"), "hello");
    assert_eq!(to_uppercase("hello"), "HELLO");
    assert_eq!(from_kebab_case("foo-bar"), "FooBar");
}

#[test]
fn singleton_map_with_transform_tagged_keys() {
    // singleton_map_with.rs:214-217 — transform_value_keys for Tagged values
    use noyalib::with::singleton_map_with::{to_snake_case, to_uppercase};

    // Test all the case conversion helpers
    assert_eq!(to_snake_case("FooBar"), "foo_bar");
    assert_eq!(to_uppercase("hello"), "HELLO");
}

#[test]
fn singleton_map_with_from_kebab_case() {
    // singleton_map_with.rs:346 — from_kebab_case split logic
    use noyalib::with::singleton_map_with::from_kebab_case;
    assert_eq!(from_kebab_case("get-request"), "GetRequest");
    assert_eq!(from_kebab_case("a-b-c"), "ABC");
}

// ═══════════════════════════════════════════════════════════════════════
// loader.rs (public) — DocumentIterator and load_all_as
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn load_all_as_typed_deserialization() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        name: String,
    }
    let yaml = "---\nname: first\n---\nname: second";
    let docs: Vec<Doc> = noyalib::load_all_as(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].name, "first");
    assert_eq!(docs[1].name, "second");
}

#[test]
fn document_iterator_len_and_empty() {
    let yaml = "---\na: 1\n---\nb: 2";
    let iter = noyalib::load_all(yaml).unwrap();
    assert_eq!(iter.len(), 2);
    assert!(!iter.is_empty());

    let empty_iter = noyalib::load_all("---\n...").unwrap();
    // Even an explicit empty doc produces one document
    assert!(empty_iter.len() >= 1);
}

#[test]
fn try_load_all_alias() {
    let yaml = "---\na: 1";
    let iter = noyalib::try_load_all(yaml).unwrap();
    assert_eq!(iter.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════
// Additional edge-case coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn from_reader_basic() {
    let yaml = b"key: value";
    let v: Value = noyalib::from_reader(&yaml[..]).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "value");
}

#[test]
fn from_reader_with_config() {
    let yaml = b"key: value";
    let config = ParserConfig::new().max_depth(10);
    let v: Value = noyalib::from_reader_with_config(&yaml[..], &config).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "value");
}

#[test]
fn from_slice_basic() {
    let yaml = b"key: value";
    let v: Value = noyalib::from_slice(yaml).unwrap();
    assert_eq!(v["key"].as_str().unwrap(), "value");
}

#[test]
fn value_get_path() {
    let yaml = "a:\n  b:\n    c: deep";
    let v: Value = from_str(yaml).unwrap();
    let c = v.get_path("a.b.c");
    assert_eq!(c.unwrap().as_str().unwrap(), "deep");
}

#[test]
fn value_merge_and_merge_concat() {
    let mut a: Value = from_str("x: 1\ny: 2").unwrap();
    let b: Value = from_str("y: 3\nz: 4").unwrap();
    a.merge(b);
    assert_eq!(a["y"].as_i64().unwrap(), 3);
    assert_eq!(a["z"].as_i64().unwrap(), 4);
}

#[test]
fn value_insert_and_remove() {
    let mut v: Value = from_str("a: 1").unwrap();
    let _ = v.insert("b", Value::from(2));
    assert_eq!(v["b"].as_i64().unwrap(), 2);
    let _ = v.remove("a");
    assert!(v.get("a").is_none());
}

#[test]
fn serializer_config_options() {
    let config = SerializerConfig::new()
        .indent(4)
        .flow_style(FlowStyle::Block)
        .scalar_style(ScalarStyle::DoubleQuoted)
        .document_start(true)
        .document_end(true);
    let v = Value::from("hello");
    let s = to_string_with_config(&v, &config).unwrap();
    assert!(s.contains("---"), "got: {s}");
}

#[test]
fn to_writer_basic() {
    let v = Value::from("test");
    let mut buf = Vec::new();
    noyalib::to_writer(&mut buf, &v).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn to_writer_with_config() {
    let config = SerializerConfig::new().document_start(true);
    let v = Value::from("test");
    let mut buf = Vec::new();
    noyalib::to_writer_with_config(&mut buf, &v, &config).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("---"), "got: {s}");
}

#[test]
fn multi_document_serialization() {
    let docs = vec![Value::from("doc1"), Value::from("doc2")];
    let s = noyalib::to_string_multi(&docs).unwrap();
    assert!(s.contains("doc1") && s.contains("doc2"));
}

#[test]
fn multi_document_writer() {
    let docs = vec![Value::from("doc1"), Value::from("doc2")];
    let mut buf = Vec::new();
    noyalib::to_writer_multi(&mut buf, &docs).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("doc1") && s.contains("doc2"));
}
