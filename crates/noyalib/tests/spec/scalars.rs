// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Scalar types and quoting

use noyalib::{Value, from_str};

#[test]
fn plain_scalar_string() {
    let v: String = from_str("hello world").unwrap();
    assert_eq!(v, "hello world");
}

#[test]
fn double_quoted_scalar() {
    let v: String = from_str("\"hello world\"").unwrap();
    assert_eq!(v, "hello world");
}

#[test]
fn single_quoted_scalar() {
    let v: String = from_str("'hello world'").unwrap();
    assert_eq!(v, "hello world");
}

#[test]
fn double_quoted_escape_sequences() {
    let v: String = from_str(r#""a\nb\tc""#).unwrap();
    assert_eq!(v, "a\nb\tc");
}

#[test]
fn double_quoted_unicode_escape() {
    let v: String = from_str(r#""caf\u00e9""#).unwrap();
    assert_eq!(v, "caf\u{00e9}");
}

#[test]
fn single_quoted_no_escapes() {
    let v: String = from_str(r"'a\nb'").unwrap();
    assert_eq!(v, r"a\nb");
}

#[test]
fn single_quoted_doubled_single_quote() {
    let v: String = from_str("'it''s'").unwrap();
    assert_eq!(v, "it's");
}

#[test]
fn empty_string_double_quoted() {
    let v: String = from_str("\"\"").unwrap();
    assert_eq!(v, "");
}

#[test]
fn empty_string_single_quoted() {
    let v: String = from_str("''").unwrap();
    assert_eq!(v, "");
}

#[test]
fn multiline_plain_scalar() {
    let v: String = from_str("a\nb\nc").unwrap();
    assert_eq!(v, "a b c");
}

#[test]
fn multiline_double_quoted() {
    let v: String = from_str("\"a\nb\nc\"").unwrap();
    assert_eq!(v, "a b c");
}

// yaml-test-suite DE56 — trailing whitespace before a line break in a
// double-quoted scalar is stripped before folding (YAML 1.2.2 §7.3.2).
#[test]
fn double_quoted_strips_trailing_whitespace_before_break() {
    // Trailing two spaces after content + escaped tab before break.
    let v: String = from_str("\"a\\t  \n    b\"").unwrap();
    assert_eq!(v, "a\t b");
    // Trailing literal tab folds away leaving content + folded space.
    let v: String = from_str("\"a\t\n    b\"").unwrap();
    assert_eq!(v, "a b");
    // Trailing tab + spaces likewise fold to a single space.
    let v: String = from_str("\"a\t  \n    b\"").unwrap();
    assert_eq!(v, "a b");
}

// yaml-test-suite TL85 / 6WPF — a line containing only whitespace between
// content lines counts as an empty line, contributing one preserved `\n`.
#[test]
fn double_quoted_blank_line_with_spaces_preserves_newline() {
    let v: String = from_str("\"\n  foo \n \n  bar\n\n  baz\n\"").unwrap();
    assert_eq!(v, " foo\nbar\nbaz ");
}

// Regression guard: the original break-folding path collapsed three
// consecutive breaks into one space. The restructured iterator-style
// handler must still emit two preserved newlines for `\n\n\n`.
#[test]
fn double_quoted_three_breaks_yields_two_newlines() {
    let v: String = from_str("\"a\n\n\nb\"").unwrap();
    assert_eq!(v, "a\n\nb");
}

// yaml-test-suite NAT4 — single-quoted scalars containing only whitespace
// or only newlines must produce the canonical fold result (e.g. one space
// for a single break around whitespace; one `\n` per empty line in
// between).
#[test]
fn single_quoted_whitespace_only_folds_to_space() {
    use std::collections::HashMap;
    let m: HashMap<String, String> = from_str(
        "---\na: '\n  '\nb: '  \n  '\nc: \"\n  \"\nd: \"  \n  \"\n\
         e: '\n\n  '\nf: \"\n\n  \"\ng: '\n\n\n  '\nh: \"\n\n\n  \"\n",
    )
    .unwrap();
    assert_eq!(m["a"], " ");
    assert_eq!(m["b"], " ");
    assert_eq!(m["c"], " ");
    assert_eq!(m["d"], " ");
    assert_eq!(m["e"], "\n");
    assert_eq!(m["f"], "\n");
    assert_eq!(m["g"], "\n\n");
    assert_eq!(m["h"], "\n\n");
}

// yaml-test-suite NB6Z — a plain multi-line scalar with a tab on an
// otherwise-empty intermediate line treats that line as an empty line
// (preserved `\n`), not as inline whitespace folded to a space.
#[test]
fn plain_scalar_blank_line_with_tab_preserves_newline() {
    use std::collections::HashMap;
    let m: HashMap<String, String> = from_str("key:\n  value\n  with\n  \t\n  tabs\n").unwrap();
    assert_eq!(m["key"], "value with\ntabs");
}

// yaml-test-suite K858 — empty-content folded/literal scalars must not
// emit a phantom trailing `\n` under default (clip) chomping.
#[test]
fn empty_block_scalar_clip_yields_empty() {
    use std::collections::HashMap;
    let m: HashMap<String, String> = from_str("strip: >-\n\nclip: >\n\nkeep: |+\n\n").unwrap();
    assert_eq!(m["strip"], "");
    assert_eq!(m["clip"], "");
    assert_eq!(m["keep"], "\n");
}

#[test]
fn string_that_looks_like_integer() {
    let v: Value = from_str("'42'").unwrap();
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("42"));
}

#[test]
fn string_that_looks_like_bool() {
    let v: Value = from_str("'true'").unwrap();
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("true"));
}

#[test]
fn string_that_looks_like_null() {
    let v: Value = from_str("'null'").unwrap();
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("null"));
}

#[test]
fn string_that_looks_like_float() {
    let v: Value = from_str("'3.14'").unwrap();
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("3.14"));
}

#[test]
fn string_with_special_yaml_chars() {
    let v: String = from_str("\"key: value\"").unwrap();
    assert_eq!(v, "key: value");
}

#[test]
fn string_with_hash() {
    let v: String = from_str("\"no # comment\"").unwrap();
    assert_eq!(v, "no # comment");
}
