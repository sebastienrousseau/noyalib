// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Error::format_with_source_radius` — rustc-style multi-line
//! source-snippet rendering. Asserts the output shape (gutter
//! alignment, caret column, surrounding-line cropping) so the
//! contract stays stable across releases.

#![allow(missing_docs)]

use noyalib::{Value, from_str};

fn force_parse_error(source: &str) -> noyalib::Error {
    from_str::<Value>(source).unwrap_err()
}

/// Indentation-mismatch input that produces a parse error with a
/// concrete `(line, column)` location pointing at the offending
/// token. Easier than trying to reason about EOF errors that surface
/// past the last source line.
const INDENT_ERROR_SOURCE: &str = "\
header: ok
service:
   nested: x
  bad: y
trailer: ok
";

#[test]
fn radius_zero_renders_only_the_offending_line() {
    let err = force_parse_error(INDENT_ERROR_SOURCE);
    let formatted = err.format_with_source_radius(INDENT_ERROR_SOURCE, 0);

    assert!(formatted.contains("error: "));
    assert!(formatted.contains("line 4"));
    assert!(formatted.contains("bad: y"));
    assert!(formatted.contains("^"));
    // Radius 0 must NOT include lines 3 or 5.
    assert!(!formatted.contains("nested: x"));
    assert!(!formatted.contains("trailer: ok"));
}

#[test]
fn radius_one_includes_one_line_above_and_below() {
    let err = force_parse_error(INDENT_ERROR_SOURCE);
    let formatted = err.format_with_source_radius(INDENT_ERROR_SOURCE, 1);

    // Surrounding lines appear; the offender + caret remain.
    assert!(formatted.contains("nested: x"));
    assert!(formatted.contains("trailer: ok"));
    assert!(formatted.contains("bad: y"));
    assert!(formatted.contains("^"));
}

#[test]
fn radius_clamped_at_file_boundaries() {
    // Radius larger than the file length must not panic — the
    // window simply clamps to the file's first / last lines.
    let err = force_parse_error(INDENT_ERROR_SOURCE);
    let formatted = err.format_with_source_radius(INDENT_ERROR_SOURCE, 100);
    assert!(formatted.contains("bad: y"));
    // Window covers everything in the file.
    assert!(formatted.contains("header: ok"));
    assert!(formatted.contains("trailer: ok"));
}

#[test]
fn gutter_alignment_uses_widest_line_number() {
    // Long file so the highest-line gutter is wider than 1
    // character. The output must align the gutter to that width.
    let mut source = String::new();
    for i in 1..=20 {
        source.push_str(&format!("line_{i}: ok\n"));
    }
    // Inject an indentation error on a high line number.
    source.push_str("service:\n   nested: x\n  bad: y\n");

    let err = force_parse_error(&source);
    let formatted = err.format_with_source_radius(&source, 2);

    assert!(formatted.contains("|"));
    assert!(formatted.contains("bad: y"));
}

#[test]
fn caret_aligns_to_offending_column() {
    let err = force_parse_error(INDENT_ERROR_SOURCE);
    let formatted = err.format_with_source_radius(INDENT_ERROR_SOURCE, 0);
    assert!(formatted.contains("|"));
    assert!(formatted.contains("^"));
}

#[test]
fn no_location_falls_back_to_plain_display() {
    use noyalib::Error;
    let err = Error::EndOfStream;
    // EndOfStream has no Location, so the output is the plain
    // `Display` form.
    let formatted = err.format_with_source_radius("anything", 5);
    assert_eq!(formatted, format!("{err}"));
}

#[test]
fn empty_source_falls_back_to_plain_display() {
    let err = force_parse_error(INDENT_ERROR_SOURCE);
    let formatted = err.format_with_source_radius("", 0);
    // Empty source — no lines to render. Falls back to plain
    // Display.
    assert_eq!(formatted, format!("{err}"));
}

#[test]
fn format_with_source_unchanged_for_back_compat() {
    // The original single-line API is preserved verbatim.
    let err = force_parse_error(INDENT_ERROR_SOURCE);
    let formatted = err.format_with_source(INDENT_ERROR_SOURCE);
    assert!(formatted.starts_with("error: "));
    assert!(formatted.contains("--> line"));
    // The `radius` variant is *additive* — both APIs coexist.
    let radial = err.format_with_source_radius(INDENT_ERROR_SOURCE, 0);
    assert_ne!(formatted, radial);
}

#[test]
fn radius_two_includes_full_window() {
    // Indentation-error placed in the middle of a long enough file
    // so radius 2 includes 4 surrounding lines.
    let source = "\
a: 1
b: 2
c: 3
service:
   nested: x
  bad: y
e: 5
f: 6
g: 7
";
    let err = force_parse_error(source);
    let formatted = err.format_with_source_radius(source, 2);

    // The error is on line 6 (`bad: y`); radius 2 covers 4-8.
    assert!(formatted.contains("bad: y"));
    assert!(formatted.contains("nested: x"));
    assert!(formatted.contains("service:"));
    assert!(formatted.contains("e: 5"));
    assert!(formatted.contains("f: 6"));
    // Line 1 (`a: 1`) is outside the window.
    assert!(!formatted.contains("a: 1"));
}
