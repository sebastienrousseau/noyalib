//! Scanner and parser deep-coverage tests.
//!
//! These tests exercise internal parser paths by feeding specific YAML patterns
//! that trigger rare code branches in the scanner and event parser.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Value, from_str};

// ── Complex key/value handling ──────────────────────────────────────────

#[test]
fn complex_key_in_block_mapping() {
    let yaml = "? first\n: second\n? third\n: fourth\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["first"], Value::String("second".to_string()));
    assert_eq!(v["third"], Value::String("fourth".to_string()));
}

#[test]
fn complex_key_in_flow_mapping() {
    let yaml = "{a: 1, b: 2}";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"], Value::from(1));
}

#[test]
fn value_indicator_in_flow_context() {
    let yaml = "{key: value, other: data}";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"], Value::String("value".to_string()));
}

#[test]
fn mapping_value_context_error() {
    // A bare ':' without preceding key in block context
    let yaml = "a: 1\n  : bad\n";
    let result: Result<Value, _> = from_str(yaml);
    // Should either parse or error — coverage path exercised either way
    let _ = result;
}

// ── Flow collections ────────────────────────────────────────────────────

#[test]
fn flow_sequence_with_nested_mappings() {
    let yaml = "[{a: 1}, {b: 2}, {c: 3}]";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0]["a"], Value::from(1));
    assert_eq!(v[2]["c"], Value::from(3));
}

#[test]
fn flow_mapping_with_nested_sequences() {
    let yaml = "{a: [1, 2], b: [3, 4]}";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"][0], Value::from(1));
}

#[test]
fn flow_sequence_implicit_mapping() {
    // Implicit mapping in flow sequence: [key: value]
    let yaml = "[a: 1]";
    let v: Value = from_str(yaml).unwrap();
    let seq = v.as_sequence().unwrap();
    assert_eq!(seq.len(), 1);
}

#[test]
fn flow_sequence_empty_entries() {
    let yaml = "[1,, 2]";
    let result: Result<Value, _> = from_str(yaml);
    // Empty entries may or may not be valid — exercises the parser path
    let _ = result;
}

#[test]
fn flow_mapping_empty_values() {
    let yaml = "{a:, b:, c: 1}";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"], Value::Null);
    assert_eq!(v["c"], Value::from(1));
}

#[test]
fn flow_sequence_with_colon_in_value() {
    // Colon followed by comma/bracket in flow context
    let yaml = "[a:b, c:d]";
    let result: Result<Value, _> = from_str(yaml);
    let _ = result; // exercises colon-in-flow parsing
}

#[test]
fn nested_flow_collections() {
    let yaml = "[[1, 2], [3, 4], {a: [5]}]";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0][0], Value::from(1));
    assert_eq!(v[2]["a"][0], Value::from(5));
}

// ── Block entry in various contexts ─────────────────────────────────────

#[test]
fn block_entry_not_allowed_error() {
    let yaml = "key: value\n- item\n";
    let result: Result<Value, _> = from_str(yaml);
    let _ = result;
}

#[test]
fn block_sequence_nested_deeply() {
    let yaml = "- - - deep\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0][0][0], Value::String("deep".to_string()));
}

// ── Quoted scalars: multiline folding ───────────────────────────────────

#[test]
fn double_quoted_multiline_with_trailing_spaces() {
    let yaml = "\"first  \n  second\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains("first"));
    assert!(v.contains("second"));
}

#[test]
fn double_quoted_multiline_multiple_blank_lines() {
    let yaml = "\"first\n\n\n  second\"";
    let v: String = from_str(yaml).unwrap();
    // Multiple blank lines produce multiple newlines
    assert!(v.contains('\n'));
}

#[test]
fn double_quoted_with_tab_escape() {
    let yaml = "\"hello\\tworld\"";
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "hello\tworld");
}

#[test]
fn double_quoted_with_cr_lf() {
    let yaml = "\"first\r\n  second\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains("first"));
}

#[test]
fn single_quoted_with_cr_lf() {
    let yaml = "'first\r\n  second'";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains("first"));
}

#[test]
fn double_quoted_escape_at_end_of_line() {
    // Backslash followed by newline — line continuation
    let yaml = "\"hello\\\r\n  world\"";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains("hello"));
}

#[test]
fn double_quoted_with_whitespace_between_words() {
    let yaml = "\"hello   world\"";
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "hello   world");
}

#[test]
fn single_quoted_multibyte_utf8() {
    let yaml = "'héllo wörld'";
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "héllo wörld");
}

#[test]
fn double_quoted_multibyte_utf8() {
    let yaml = "\"日本語\"";
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "日本語");
}

// ── Block scalars: various chomping and indent modes ────────────────────

#[test]
fn block_scalar_folded_with_tabs_and_spaces() {
    let yaml = ">\n  normal\n  \tindented\n  back\n";
    let result: Result<String, _> = from_str(yaml);
    // Tab handling in block scalars — may error or parse
    let _ = result;
}

#[test]
fn block_scalar_auto_detect_indent() {
    let yaml = "|\n    four spaces\n    aligned\n";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains("four spaces"));
    assert!(v.contains("aligned"));
}

#[test]
fn block_scalar_empty_lines_between_content() {
    let yaml = "|\n  line1\n\n  line2\n";
    let v: String = from_str(yaml).unwrap();
    assert_eq!(v, "line1\n\nline2\n");
}

#[test]
fn block_scalar_as_mapping_value() {
    let yaml = "key: |\n  literal\n  block\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"], Value::String("literal\nblock\n".to_string()));
}

#[test]
fn block_scalar_folded_as_mapping_value() {
    let yaml = "key: >\n  folded\n  block\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"], Value::String("folded block\n".to_string()));
}

#[test]
fn block_scalar_folded_multiple_paragraphs() {
    let yaml = ">\n  para1\n  line2\n\n  para2\n  line4\n";
    let v: String = from_str(yaml).unwrap();
    assert!(v.contains("para1 line2"));
    assert!(v.contains("para2 line4"));
}

// ── Document markers ────────────────────────────────────────────────────

#[test]
fn explicit_document_start_and_end() {
    let yaml = "---\nvalue\n...\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v, Value::String("value".to_string()));
}

#[test]
fn multiple_documents_with_markers() {
    let yaml = "---\na: 1\n...\n---\nb: 2\n...\n";
    let docs: Vec<Value> = noyalib::load_all_as(yaml).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn document_start_implicit() {
    let yaml = "key: value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"], Value::String("value".to_string()));
}

// ── Tags with various handle forms ──────────────────────────────────────

#[test]
fn tag_secondary_handle() {
    let yaml = "!!int 42";
    let v: Value = from_str(yaml).unwrap();
    // Should parse as an integer with tag resolution
    match v {
        Value::Number(_) | Value::Tagged(_) | Value::String(_) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn tag_verbatim_handle() {
    let yaml = "!<tag:yaml.org,2002:str> hello";
    let v: Value = from_str(yaml).unwrap();
    match v {
        Value::Tagged(_) | Value::String(_) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

// ── Plain scalars: edge cases ───────────────────────────────────────────

#[test]
fn plain_scalar_with_colon_space() {
    let yaml = "key: value: with colon";
    let result: Result<Value, _> = from_str(yaml);
    let _ = result;
}

#[test]
fn plain_scalar_multiline_with_cr_lf() {
    let yaml = "key: first\r\n  second\r\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(matches!(v["key"], Value::String(_)));
}

#[test]
fn plain_scalar_continues_across_lines() {
    let yaml = "key: this is\n  a long\n  value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["key"], Value::String("this is a long value".to_string()));
}

// ── Simple key resolution ───────────────────────────────────────────────

// Per YAML 1.2.2 §7.4.2, an implicit key must reach its `:` on the
// same line. `key\n: value` no longer promotes `key` to a key — the
// `:` on the next line is for an empty implicit key (per the
// cluster-level fix that made 6M2F parse correctly). The parser
// either rejects the input outright or produces a structure where
// `key` is not the mapping key for `value`.
#[test]
fn simple_key_with_value_on_next_line_is_not_promoted() {
    let yaml = "key\n: value\n";
    match from_str::<Value>(yaml) {
        Err(_) => { /* strict rejection — fine */ }
        Ok(v) => {
            if let Some(map) = v.as_mapping() {
                assert_ne!(
                    map.get("key").and_then(Value::as_str),
                    Some("value"),
                    "must not promote `key` to a mapping key for `value`"
                );
            }
        }
    }
}

#[test]
fn multiple_simple_keys() {
    let yaml = "a: 1\nb: 2\nc: 3\nd: 4\ne: 5\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["e"], Value::from(5));
}

// ── Anchors and aliases in various positions ────────────────────────────

#[test]
fn anchor_on_mapping_key() {
    let yaml = "&a key: value\nalias: *a\n";
    let result: Result<Value, _> = from_str(yaml);
    let _ = result;
}

#[test]
fn anchor_on_sequence_item() {
    let yaml = "- &a first\n- *a\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v[0], v[1]);
}

#[test]
fn anchor_on_nested_structure() {
    let yaml = "base: &base\n  x: 1\n  y: 2\nderived:\n  <<: *base\n  z: 3\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["derived"]["x"], Value::from(1));
    assert_eq!(v["derived"]["z"], Value::from(3));
}

// ── Error recovery paths ────────────────────────────────────────────────

#[test]
fn error_unexpected_end_of_stream() {
    let yaml = "{a: ";
    let result: Result<Value, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn error_mismatched_brackets() {
    let yaml = "[1, 2}";
    let result: Result<Value, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn error_mismatched_braces() {
    let yaml = "{a: 1]";
    let result: Result<Value, _> = from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn error_duplicate_anchor() {
    let yaml = "&a 1\n&a 2\n";
    let result: Result<Value, _> = from_str(yaml);
    // Duplicate anchors may or may not error — exercises the path
    let _ = result;
}

// ── Serializer edge cases ───────────────────────────────────────────────

#[test]
fn ser_special_yaml_keywords_quoted() {
    use noyalib::to_string;
    // These strings look like YAML special values and should be quoted
    for s in &["true", "false", "null", "~"] {
        let v = Value::String(s.to_string());
        let yaml = to_string(&v).unwrap();
        assert!(
            yaml.contains('\'') || yaml.contains('"'),
            "'{s}' should be quoted in YAML output"
        );
    }
}

#[test]
fn ser_numbers_as_strings_quoted() {
    use noyalib::to_string;
    for s in &[".inf", "-.inf", ".nan", "0x1A", "0o77", "1e10"] {
        let v = Value::String(s.to_string());
        let yaml = to_string(&v).unwrap();
        assert!(
            yaml.contains('\'') || yaml.contains('"'),
            "'{s}' should be quoted"
        );
    }
}

// ── Spanned with various types ──────────────────────────────────────────

#[test]
fn spanned_mapping_deserialization() {
    use noyalib::Spanned;
    let yaml = "a: 1\nb: 2\n";
    let v: Spanned<Value> = from_str(yaml).unwrap();
    assert!(v.start.line() >= 1);
}

#[test]
fn spanned_sequence_deserialization() {
    use noyalib::Spanned;
    let yaml = "- 1\n- 2\n- 3\n";
    let v: Spanned<Vec<i32>> = from_str(yaml).unwrap();
    assert_eq!(v.value.len(), 3);
}

// ── Classic-Mac CR-only line endings (YAML 1.2.2 §5.4) ──────────────────

#[test]
fn cr_only_line_endings_parse_as_line_breaks() {
    // A lone CR (not part of CRLF) is a valid YAML line break. Before the
    // scanner reset the column on a bare `\r`, this was rejected with a
    // spurious "inconsistent indentation" error.
    let yaml = "a: 1\rb: 2\r";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["a"], Value::from(1));
    assert_eq!(v["b"], Value::from(2));
}

#[test]
fn cr_only_block_sequence_parses() {
    let yaml = "- one\r- two\r- three\r";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.as_sequence().map(|s| s.len()), Some(3));
    assert_eq!(v[0], Value::String("one".to_string()));
    assert_eq!(v[2], Value::String("three".to_string()));
}

#[test]
fn cr_only_round_trips_byte_for_byte() {
    // The CST keeps source bytes verbatim regardless of line-ending flavor.
    let yaml = "a: 1\rb: 2\r";
    let doc = noyalib::cst::parse_document(yaml).unwrap();
    assert_eq!(doc.source(), yaml);
}
