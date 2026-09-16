// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Inserting a key must not steal a keep-chomped scalar's blank lines
//! (#429).
//!
//! A blank line is normally trivia and a new sibling may be spliced in
//! above it. Under `|+` or `>+` it is not: "keep" chomping makes the
//! trailing line breaks part of the value. Splicing above them adds the
//! key that was asked for *and* silently shortens a different key's
//! value.
//!
//! Nothing fails when that happens — the document still parses, every
//! byte is still present, and the raw-text diff looks like an ordinary
//! insertion. Only the value changed, which is why this needs a test
//! rather than review.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use noyalib::cst::parse_document;

/// The reported case: the scalar is nested, the insert is at the root.
#[test]
fn a_root_insert_keeps_a_nested_keep_chomped_value_intact() {
    let mut doc = parse_document("a:\n  b: |+\n    x\n\n").unwrap();
    doc.insert_entry("", "c", "2").unwrap();
    assert_eq!(format!("{doc}"), "a:\n  b: |+\n    x\n\nc: 2\n");
}

/// `>+` keeps its trailing breaks for the same reason `|+` does.
#[test]
fn folded_keep_chomping_is_treated_the_same() {
    let mut doc = parse_document("a:\n  b: >+\n    x\n\n").unwrap();
    doc.insert_entry("", "c", "2").unwrap();
    assert_eq!(format!("{doc}"), "a:\n  b: >+\n    x\n\nc: 2\n");
}

/// Several kept blank lines, not just one.
#[test]
fn every_kept_blank_line_stays_with_the_scalar() {
    let mut doc = parse_document("a:\n  b: |+\n    x\n\n\n\n").unwrap();
    doc.insert_entry("", "c", "2").unwrap();
    assert_eq!(format!("{doc}"), "a:\n  b: |+\n    x\n\n\n\nc: 2\n");
}

/// Clip chomping (the default) does *not* keep trailing breaks, so the
/// blank line there is trivia and the key goes above it as before.
///
/// Without this the fix could have been "always skip trailing blanks",
/// which would move keys for every document that happens to end in one.
#[test]
fn clip_chomping_still_places_the_key_above_the_blank_line() {
    let mut doc = parse_document("a:\n  b: |\n    x\n\n").unwrap();
    doc.insert_entry("", "c", "2").unwrap();
    let out = format!("{doc}");
    assert!(
        out.contains("c: 2"),
        "the key must still be inserted: {out:?}"
    );
    assert!(
        out.starts_with("a:\n  b: |\n    x\n"),
        "the scalar must be untouched: {out:?}"
    );
}

/// The value itself must survive a round trip, not merely the bytes.
#[test]
fn the_kept_value_round_trips_after_the_insert() {
    let mut doc = parse_document("a:\n  b: |+\n    x\n\n").unwrap();
    doc.insert_entry("", "c", "2").unwrap();
    let out = format!("{doc}");
    let reparsed = parse_document(&out).expect("reparse");
    assert_eq!(format!("{reparsed}"), out, "not stable on a second parse");
}
