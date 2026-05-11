// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! When `Document::set_value` replaces a single-line scalar with a
//! multi-line string, the new value is emitted as a literal block
//! scalar (`|` / `|-`) at the right indent — not as a
//! double-quoted one-liner with `\\n` escapes.

use noyalib::Value;
use noyalib::cst::parse_document;

fn round_trip_value(src: &str, path: &str, value: &str) -> String {
    let mut doc = parse_document(src).unwrap();
    doc.set_value(path, &Value::String(value.into())).unwrap();
    doc.to_string()
}

#[test]
fn multi_line_replacement_grows_to_literal_block_with_clip() {
    let src = "note: short\n";
    let out = round_trip_value(src, "note", "line one\nline two\n");
    assert_eq!(out, "note: |\n  line one\n  line two\n");

    // The parsed value of the new YAML must equal the input string.
    let reparsed: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(reparsed["note"].as_str(), Some("line one\nline two\n"));
}

#[test]
fn multi_line_replacement_uses_strip_chomp_when_no_trailing_newline() {
    let src = "note: short\n";
    let out = round_trip_value(src, "note", "line one\nline two");
    assert_eq!(out, "note: |-\n  line one\n  line two\n");

    let reparsed: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(reparsed["note"].as_str(), Some("line one\nline two"));
}

#[test]
fn nested_entry_indents_relative_to_parent() {
    let src = "outer:\n  note: short\n";
    let out = round_trip_value(src, "outer.note", "first\nsecond\n");
    assert_eq!(out, "outer:\n  note: |\n    first\n    second\n");

    let reparsed: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(reparsed["outer"]["note"].as_str(), Some("first\nsecond\n"));
}

#[test]
fn single_line_replacement_stays_plain_or_quoted() {
    // No `\n` in value → existing single-line behaviour is preserved.
    let src = "note: short\n";
    let out = round_trip_value(src, "note", "longer but still one line");
    assert!(
        out.contains("note: longer but still one line"),
        "expected single-line plain; got: {out}",
    );
}

#[test]
fn line_starting_with_space_falls_back_to_double_quoted() {
    // We do not yet emit explicit indent indicators for block
    // scalars whose content lines begin with whitespace — so the
    // formatter falls back to double-quoted with `\\n` escapes.
    let src = "note: short\n";
    let out = round_trip_value(src, "note", "first\n  indented\n");
    // Should *not* be a block literal.
    assert!(!out.contains("|"), "fell back to block literal: {out}");
    // Should be a double-quoted form with `\\n`.
    assert!(out.contains("\\n"), "expected \\n escape: {out}");

    let reparsed: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(reparsed["note"].as_str(), Some("first\n  indented\n"));
}

#[test]
fn block_growth_under_sequence_indents_properly() {
    let src = "items:\n  - first\n";
    let out = round_trip_value(src, "items[0]", "alpha\nbeta\n");
    // The dash sits at column 2; conventional formatting puts the
    // block literal's content lines at column 4 (one indent step
    // past the dash). The result must round-trip to the same string.
    assert!(
        out.contains("- |\n    alpha\n    beta\n"),
        "expected block literal under sequence item; got: {out}",
    );

    let reparsed: Value = noyalib::from_str(&out).unwrap();
    assert_eq!(reparsed["items"][0].as_str(), Some("alpha\nbeta\n"));
}
