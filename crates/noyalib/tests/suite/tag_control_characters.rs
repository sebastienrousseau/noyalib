//! Tags may not carry control characters.
//!
//! Found by `fuzz_roundtrip`: the scanner accepted a verbatim tag
//! whose bracketed content held DEL, NULs and a tab
//! (`!<\x7f\0\0\0\0\0\0\t>` …), and the emitter reproduced those
//! bytes raw — producing YAML that no longer parses ("stray content
//! after document"). The spec's `c-verbatim-tag` is
//! `"!" "<" ns-uri-char+ ">"`: URI characters only, one or more.
//! The scanner now rejects control characters in verbatim tags and
//! shorthand suffixes, and refuses the empty `!<>`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::Value;

#[track_caller]
fn refused(input: &str) {
    let r: Result<Value, _> = noyalib::from_str(input);
    assert!(r.is_err(), "{input:?} must be rejected, got {r:?}");
}

#[test]
fn verbatim_tag_with_control_characters_is_rejected() {
    // The fuzz_roundtrip crash input.
    refused("!<\u{7f}\0\0\0\0\0\0\t!");
    refused("!<\u{1}> v");
    // A line break inside the brackets is a control character too —
    // the old loop only stopped at `>`.
    refused("!<a\nb> v");
}

#[test]
fn empty_verbatim_tag_is_rejected() {
    // c-verbatim-tag requires ns-uri-char+ — one or more.
    refused("!<> v");
}

#[test]
fn shorthand_tag_with_control_characters_is_rejected() {
    refused("!\u{1}suffix v");
    refused("!!su\u{7f}ffix v");
    refused("!h!su\u{1}ffix v");
}

#[test]
fn ordinary_tags_still_parse() {
    let v: Value = noyalib::from_str("!<tag:example.com,2026:x> v").unwrap();
    assert!(matches!(v, Value::Tagged(_)));
    let v: Value = noyalib::from_str("!!str 42").unwrap();
    assert_eq!(v.as_str(), Some("42"));
    let v: Value = noyalib::from_str("!custom v").unwrap();
    assert!(matches!(v, Value::Tagged(_)));
}

#[test]
fn accepted_tags_round_trip_through_emit() {
    for src in ["!<tag:example.com,2026:x> v\n", "!custom v\n", "!!str 42\n"] {
        let v: Value = noyalib::from_str(src).unwrap();
        let emitted = noyalib::to_string(&v).unwrap();
        let back: Value = noyalib::from_str(&emitted)
            .unwrap_or_else(|e| panic!("emit of {src:?} must re-parse: {e} ({emitted:?})"));
        assert_eq!(v, back, "{src:?}");
    }
}

#[test]
fn unspellable_tag_bodies_emit_as_quoted_key_mappings() {
    // In the serde data model a tagged value and a single-entry
    // mapping keyed by its `!`-leading spelling are the same thing.
    // When the tag body holds a character no tag spelling can carry
    // (a tab; `>`), the emitter resolves the ambiguity toward the
    // mapping with a quoted key, and the output round-trips.
    let v: Value = noyalib::from_str("\"!\t\":").unwrap();
    let emitted = noyalib::to_string(&v).unwrap();
    let back: Value =
        noyalib::from_str(&emitted).unwrap_or_else(|e| panic!("must re-parse: {e} ({emitted:?})"));
    assert_eq!(v, back, "{emitted:?}");
}

#[test]
fn block_scalar_content_rejects_raw_control_characters() {
    // Found by the serde_yaml parity fuzzer (`>-\x07`): §5.1
    // c-printable governs block scalar content too.
    refused(">-\n \u{7}");
    refused("a: |\n  x\u{1}y\n");
    // Tabs inside content stay legal.
    let v: Value = noyalib::from_str("a: |\n  x\ty\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("x\ty\n"));
}

#[test]
fn double_quoted_rejects_raw_controls_but_keeps_escapes() {
    refused("a: \"x\u{7}y\"\n");
    refused("\"\u{1}\"");
    // The escape spellings remain the way to carry controls.
    let v: Value = noyalib::from_str("a: \"x\\x07y\\u0001\"\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("x\u{7}y\u{1}"));
}

#[test]
fn block_scalar_header_line_carries_no_content() {
    // §8.1.1: after `|`/`>` and its indicators, only blanks and a
    // comment may precede the line break — a literal `\n` (or any
    // text) on the header line is malformed, not content (found by
    // the serde_yaml parity fuzzer on `>-\n`).
    refused(">-\\n");
    refused("a: |x\n  y\n");
    refused("a: >2x\n  y\n");
    // Comments and the indicators themselves remain fine.
    let v: Value = noyalib::from_str("a: |- # note\n  x\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("x"));
}
