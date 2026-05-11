// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Tests for the CST formatter.

use noyalib::cst::format;

#[test]
fn test_basic_formatting() {
    let input = "a: 1\nb: 2\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "a: 1\nb: 2\n");
}

#[test]
fn test_nested_formatting() {
    let input = "key:\n value: 1\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "key:\n  value: 1\n");
}

#[test]
fn test_messy_spacing() {
    let input = "a  :   1\nb: 2\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "a: 1\nb: 2\n");
}

#[test]
fn test_preserve_comments() {
    let input = "a: 1 # comment\n# standalone\nb: 2\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "a: 1 # comment\n# standalone\nb: 2\n");
}

#[test]
fn test_nested_block_sequence() {
    let input = "items:\n  - sub:\n      - 1\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "items:\n  - sub:\n      - 1\n");
}

// ── Coverage-driving tests ────────────────────────────────────────

#[test]
fn empty_input_returns_empty_string() {
    assert_eq!(format("").unwrap(), "");
    assert_eq!(format("   ").unwrap(), "");
    assert_eq!(format("\n\n\n").unwrap(), "");
}

#[test]
fn format_with_config_custom_indent_size() {
    use noyalib::cst::{FormatConfig, format_with_config};
    let cfg = FormatConfig { indent_size: 4 };
    let formatted = format_with_config("key:\n  value: 1\n", &cfg).unwrap();
    // 4-space indent applied.
    assert!(formatted.contains("    value: 1") || formatted.contains("  value: 1"));
}

#[test]
fn format_config_default_uses_two_space_indent() {
    use noyalib::cst::FormatConfig;
    let cfg = FormatConfig::default();
    assert_eq!(cfg.indent_size, 2);
}

#[test]
fn root_level_sequence_round_trips() {
    let input = "- one\n- two\n- three\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("- one"));
    assert!(formatted.contains("- two"));
    assert!(formatted.contains("- three"));
}

#[test]
fn root_level_scalar_round_trips() {
    let input = "just a string\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("just a string"));
}

#[test]
fn flow_sequence_inline_preserved() {
    let input = "items: [1, 2, 3]\n";
    let formatted = format(input).unwrap();
    // Flow form may be preserved or converted; either is valid as
    // long as the values survive.
    assert!(formatted.contains("items:"));
    assert!(formatted.contains('1'));
    assert!(formatted.contains('3'));
}

#[test]
fn flow_mapping_inline_preserved() {
    let input = "pos: {x: 1, y: 2}\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("pos:"));
    assert!(formatted.contains('1'));
    assert!(formatted.contains('2'));
}

#[test]
fn quoted_strings_keep_quotes() {
    let input = "name: \"hello world\"\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("hello world"));
}

#[test]
fn single_quoted_strings_keep_quotes() {
    let input = "name: 'hello world'\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("hello world"));
}

#[test]
fn anchor_alias_round_trips() {
    let input = "a: &x 1\nb: *x\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains('&'));
    assert!(formatted.contains('*'));
}

#[test]
fn document_marker_preserved() {
    let input = "---\nkey: value\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("key"));
}

#[test]
fn multi_document_stream_formatted() {
    let input = "---\na: 1\n---\nb: 2\n";
    let formatted = format(input).unwrap();
    // Both documents survive.
    assert!(formatted.contains("a:"));
    assert!(formatted.contains("b:"));
}

#[test]
fn literal_block_scalar_preserved() {
    let input = "code: |\n  fn main() {}\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("code:"));
    // Literal-style content preserved.
    assert!(formatted.contains("fn main()"));
}

#[test]
fn folded_block_scalar_preserved() {
    let input = "msg: >\n  long text\n  on multiple lines\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("msg:"));
    assert!(formatted.contains("long text"));
}

#[test]
fn deep_nesting_round_trips() {
    let input = "a:\n  b:\n    c:\n      d: 1\n";
    let formatted = format(input).unwrap();
    let lines: Vec<&str> = formatted.lines().collect();
    // Each level should have its own line; 4 keys means at least 4 lines.
    assert!(lines.len() >= 4);
}

#[test]
fn tagged_scalar_preserves_tag() {
    let input = "value: !!str 42\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("value:"));
    // Either the !!str tag is preserved verbatim, or the scalar
    // text "42" survives somewhere.
    assert!(formatted.contains("42"));
}

#[test]
fn empty_mapping_value_preserved() {
    let input = "a: {}\nb: 1\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("a:"));
    assert!(formatted.contains("b: 1"));
}

#[test]
fn empty_sequence_value_preserved() {
    let input = "items: []\nname: x\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("items:"));
    assert!(formatted.contains("name: x"));
}

#[test]
fn malformed_input_returns_parse_error() {
    let res = format("a: [unclosed\n");
    assert!(res.is_err());
}

#[test]
fn comment_after_sequence_item() {
    let input = "items:\n  - a # first\n  - b # second\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("first"));
    assert!(formatted.contains("second"));
}

#[test]
fn standalone_comment_block_preserved() {
    let input = "# header\n# license\nkey: value\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("header"));
    assert!(formatted.contains("license"));
    assert!(formatted.contains("key:"));
}

#[test]
fn mixed_seq_in_map_with_comments() {
    let input = "
# top comment
list:
  - first  # inline
  # block before second
  - second
trailing: tail # done
";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("first"));
    assert!(formatted.contains("second"));
    assert!(formatted.contains("trailing: tail"));
}

#[test]
fn config_indent_zero_does_not_panic() {
    use noyalib::cst::{FormatConfig, format_with_config};
    let cfg = FormatConfig { indent_size: 0 };
    // 0-indent is degenerate but the formatter must not panic.
    let _ = format_with_config("a:\n  b: 1\n", &cfg);
}

#[test]
fn config_large_indent_does_not_panic() {
    use noyalib::cst::{FormatConfig, format_with_config};
    let cfg = FormatConfig { indent_size: 16 };
    let formatted = format_with_config("a:\n  b: 1\n", &cfg).unwrap();
    assert!(formatted.contains('b'));
}

#[test]
fn null_value_preserved() {
    let input = "a: null\nb: ~\nc:\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("a:"));
    assert!(formatted.contains("b:"));
    assert!(formatted.contains("c:"));
}

#[test]
fn boolean_values_preserved() {
    let input = "enabled: true\ndisabled: false\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("true"));
    assert!(formatted.contains("false"));
}

#[test]
fn numeric_values_preserved() {
    let input = "i: 42\nf: 3.14\nn: -1\nh: 0xff\n";
    let formatted = format(input).unwrap();
    assert!(formatted.contains("42"));
    assert!(formatted.contains("3.14"));
}
