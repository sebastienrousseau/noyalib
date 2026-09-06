// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Block scalars (literal | and folded >)

use noyalib::from_str;

#[test]
fn literal_block_scalar() {
    let v: String = from_str("|\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1\nline2\n");
}

#[test]
fn literal_block_strip() {
    let v: String = from_str("|-\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1\nline2");
}

#[test]
fn literal_block_keep() {
    let v: String = from_str("|+\n  line1\n  line2\n\n").unwrap();
    assert_eq!(v, "line1\nline2\n\n");
}

#[test]
fn folded_block_scalar() {
    let v: String = from_str(">\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1 line2\n");
}

#[test]
fn folded_block_strip() {
    let v: String = from_str(">-\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1 line2");
}

#[test]
fn folded_block_keep() {
    let v: String = from_str(">+\n  line1\n  line2\n\n").unwrap();
    assert_eq!(v, "line1 line2\n\n");
}

#[test]
fn literal_block_with_indent_indicator() {
    let v: String = from_str("|2\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1\nline2\n");
}

#[test]
fn literal_block_preserves_newlines() {
    let v: String = from_str("|\n  line1\n\n  line3\n").unwrap();
    assert_eq!(v, "line1\n\nline3\n");
}

#[test]
fn literal_block_in_mapping() {
    use std::collections::HashMap;
    let m: HashMap<String, String> = from_str("script: |\n  echo hello\n  echo world\n").unwrap();
    assert_eq!(m["script"], "echo hello\necho world\n");
}

#[test]
fn folded_block_in_mapping() {
    use std::collections::HashMap;
    let m: HashMap<String, String> =
        from_str("desc: >\n  This is a\n  long description\n").unwrap();
    assert_eq!(m["desc"], "This is a long description\n");
}

#[test]
fn literal_block_single_line() {
    let v: String = from_str("|\n  single line\n").unwrap();
    assert_eq!(v, "single line\n");
}

#[test]
fn folded_block_single_line() {
    let v: String = from_str(">\n  single line\n").unwrap();
    assert_eq!(v, "single line\n");
}

// yaml-test-suite FP8R — zero-indented folded block scalar at document root.
// Content lines at column 0 are valid because the parent (document) indent
// is -1; min content indent must be 0, not 1.
#[test]
fn folded_block_zero_indented_at_root() {
    let v: String = from_str("--- >\nline1\nline2\nline3\n").unwrap();
    assert_eq!(v, "line1 line2 line3\n");
}

// yaml-test-suite DK3J — zero-indented folded block scalar where one line
// looks like a comment. Inside a block scalar, `#` is content, not a comment.
#[test]
fn folded_block_zero_indented_with_hash_line() {
    let v: String = from_str("--- >\nline1\n# no comment\nline3\n").unwrap();
    assert_eq!(v, "line1 # no comment line3\n");
}

// Document end marker terminates a zero-indented block scalar (regression
// guard for the explicit `---` / `...` terminator added alongside the
// zero-indent fix above).
#[test]
fn folded_block_zero_indented_terminated_by_doc_end() {
    let docs: Vec<String> = noyalib::load_all_as("--- >\nline1\nline2\n...\n").unwrap();
    assert_eq!(docs, vec!["line1 line2\n".to_string()]);
}

// yaml-test-suite MJS9 (spec example 6.7) — folded scalar with empty line
// adjacent to a more-indented (tab-prefixed) line preserves both breaks.
#[test]
fn folded_block_more_indented_preserves_breaks() {
    let v: String = from_str(">\n  foo \n \n  \t bar\n\n  baz\n").unwrap();
    assert_eq!(v, "foo \n\n\t bar\n\nbaz\n");
}

// yaml-test-suite L24T — literal scalar preserves whitespace-only lines
// whose leading spaces exceed `block_indent` as content (the extra spaces
// plus the trailing newline).
#[test]
fn literal_block_preserves_overindented_blank_line() {
    use std::collections::HashMap;
    let m: HashMap<String, String> = from_str("foo: |\n  x\n   \n").unwrap();
    assert_eq!(m["foo"], "x\n \n");
}

// yaml-test-suite 8.13 — folded scalar with leading empty lines preserves
// each as a `\n` (the implicit header break does not consume them).
#[test]
fn folded_block_leading_empty_lines_preserved() {
    let v: String = from_str(">\n\n folded\n line\n\n next\n line\n").unwrap();
    assert_eq!(v, "\nfolded line\nnext line\n");
}

// A keep-chomped scalar whose only lines are blank, followed by another
// document. The blank lines are trailing breaks, not a leading-line
// indentation error: the `---` at column 0 is not the scalar's first
// content line (found by the suite-stream permutations, JEF9).
#[test]
fn all_blank_keep_scalar_before_a_document_marker() {
    use noyalib::{Value, load_all_as};
    for (src, expected) in [
        ("- |+\n   \n---\nb: 2\n", "\n"),
        ("- |+\n   \n...\n", "\n"),
        ("|+\n   \n---\nb: 2\n", "\n"),
        ("- |+\n\n\n---\nb: 2\n", "\n\n"),
    ] {
        let docs = load_all_as::<Value>(src).unwrap_or_else(|e| panic!("{src:?}: {e}"));
        let text = match &docs[0] {
            Value::Sequence(items) => items[0].as_str().map(str::to_owned),
            other => other.as_str().map(str::to_owned),
        };
        assert_eq!(text.as_deref(), Some(expected), "{src:?}");
        let cst = noyalib::cst::parse_stream(src).unwrap_or_else(|e| panic!("cst {src:?}: {e}"));
        assert_eq!(cst.iter().map(|d| d.source()).collect::<String>(), src);
    }
    // The standalone form is unchanged.
    let v: Vec<String> = from_str("- |+\n   \n").unwrap();
    assert_eq!(v, vec!["\n"]);
}
