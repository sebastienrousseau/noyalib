// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 2A mutation tests.
//!
//! Each test parses a YAML document, applies a path-targeted edit
//! via [`noyalib::cst::Document::set`] or `replace_span`, and
//! checks that the result is byte-identical to the expected output
//! — surrounding indentation, comments, and other entries are
//! preserved verbatim.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

#[test]
fn set_replaces_only_the_target_value() {
    let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
    doc.set("version", "0.0.2").unwrap();
    assert_eq!(doc.to_string(), "name: foo\nversion: 0.0.2\n");
}

#[test]
fn set_preserves_inline_comment() {
    let mut doc = parse_document("name: foo  # the project\nversion: 0.0.1\n").unwrap();
    doc.set("version", "0.0.2").unwrap();
    assert_eq!(
        doc.to_string(),
        "name: foo  # the project\nversion: 0.0.2\n"
    );
}

#[test]
fn set_preserves_blank_lines_between_entries() {
    let src = "name: foo\n\n\nversion: 0.0.1\n";
    let mut doc = parse_document(src).unwrap();
    doc.set("version", "0.0.2").unwrap();
    assert_eq!(doc.to_string(), "name: foo\n\n\nversion: 0.0.2\n");
}

#[test]
fn set_targets_nested_mapping_value() {
    let src = "package:\n  name: foo\n  version: 0.0.1\n";
    let mut doc = parse_document(src).unwrap();
    doc.set("package.version", "0.0.2").unwrap();
    assert_eq!(doc.to_string(), "package:\n  name: foo\n  version: 0.0.2\n");
}

#[test]
fn set_targets_sequence_index() {
    let src = "items:\n  - one\n  - two\n  - three\n";
    let mut doc = parse_document(src).unwrap();
    doc.set("items[1]", "TWO").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - TWO\n  - three\n");
}

#[test]
fn set_can_introduce_quotes() {
    let mut doc = parse_document("version: 1.2\n").unwrap();
    doc.set("version", "\"1.2\"").unwrap();
    assert_eq!(doc.to_string(), "version: \"1.2\"\n");
}

#[test]
fn set_returns_path_not_found_error() {
    let mut doc = parse_document("name: foo\n").unwrap();
    let err = doc.set("missing.key", "x").unwrap_err();
    assert!(err.to_string().contains("path not found"));
}

#[test]
fn set_with_invalid_replacement_surfaces_error_on_read() {
    // Phase A.2 lazy: the local-repair fast path commits
    // optimistically when the spliced fragment passes its own
    // (scanner-level) validation. Cross-document structural errors
    // — like an unclosed flow indicator at end-of-input — only
    // surface on the next typed-view read. Callers that need an
    // eager check call [`Document::validate`].
    let mut doc = parse_document("name: foo\n").unwrap();
    // The bare `[` opens a flow sequence that is never closed —
    // structurally broken at the document level.
    doc.set("name", "[").unwrap();
    // Source reflects the optimistic splice; round-trip via the
    // green tree still works.
    assert_eq!(doc.to_string(), "name: [\n");
    // Eager check: `validate()` surfaces the document-level parse
    // error as a regular `Result` — no panic.
    let err = doc
        .validate()
        .expect_err("validate must reject broken source");
    let msg = err.to_string();
    assert!(
        msg.contains("flow") || msg.contains("expected") || msg.contains("end"),
        "validate error should reference the parse failure; got {msg:?}",
    );
    // The typed view also refuses to materialise (it panics on
    // first read), but `validate` is the supported way to detect
    // this without unwinding.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = doc.as_value();
    }));
    assert!(result.is_err(), "as_value() must panic on invalid source");
}

#[test]
fn validate_succeeds_on_freshly_parsed_document() {
    let doc = parse_document("a: 1\nb: 2\n").unwrap();
    doc.validate()
        .expect("freshly-parsed document must validate");
}

#[test]
fn validate_succeeds_after_safe_edit() {
    let mut doc = parse_document("version: 0.0.1\n").unwrap();
    doc.set("version", "0.0.2").unwrap();
    doc.validate()
        .expect("safe edit must leave document in valid state");
}

#[test]
fn validate_is_idempotent() {
    // Once validation has populated the typed cache, repeated
    // calls are cheap and still succeed — they early-return.
    let doc = parse_document("k: v\n").unwrap();
    for _ in 0..3 {
        doc.validate()
            .expect("repeated validate calls must succeed");
    }
}

#[test]
fn span_at_returns_none_for_missing_path() {
    let doc = parse_document("name: foo\n").unwrap();
    assert!(doc.span_at("missing").is_none());
}

#[test]
fn get_returns_source_slice() {
    let doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    assert_eq!(doc.get("items[0]"), Some("one"));
    assert_eq!(doc.get("items[1]"), Some("two"));
}

#[test]
fn replace_span_round_trip_after_no_op() {
    let src = "key: value\nother: 1\n";
    let mut doc = parse_document(src).unwrap();
    // Replace the same bytes with themselves — should be a no-op.
    let (s, e) = doc.span_at("key").unwrap();
    let original = doc.source()[s..e].to_owned();
    doc.replace_span(s, e, &original).unwrap();
    assert_eq!(doc.to_string(), src);
}

#[test]
fn replace_span_rejects_non_char_boundary() {
    // Multi-byte UTF-8 in the source, then attempt to splice mid-character.
    let mut doc = parse_document("emoji: 🦀\n").unwrap();
    // The crab emoji is 4 bytes starting at index 7. Position 8 is mid-glyph.
    let err = doc.replace_span(8, 9, "x").unwrap_err();
    assert!(err.to_string().contains("character boundary"));
}

// ── set_value: typed mutation with style matching ────────────────

#[test]
fn set_value_string_into_plain_site_emits_plain() {
    let mut doc = parse_document("name: foo\n").unwrap();
    doc.set_value("name", &Value::String("bar".into())).unwrap();
    assert_eq!(doc.to_string(), "name: bar\n");
}

#[test]
fn set_value_string_into_double_quoted_site_quotes() {
    let mut doc = parse_document("name: \"foo\"\n").unwrap();
    doc.set_value("name", &Value::String("bar".into())).unwrap();
    assert_eq!(doc.to_string(), "name: \"bar\"\n");
}

#[test]
fn set_value_string_into_single_quoted_site_quotes() {
    let mut doc = parse_document("name: 'foo'\n").unwrap();
    doc.set_value("name", &Value::String("bar".into())).unwrap();
    assert_eq!(doc.to_string(), "name: 'bar'\n");
}

#[test]
fn set_value_string_with_embedded_quote_escapes_in_double_quoted_site() {
    let mut doc = parse_document("title: \"hello\"\n").unwrap();
    doc.set_value("title", &Value::String("she said \"hi\"".into()))
        .unwrap();
    assert_eq!(doc.to_string(), "title: \"she said \\\"hi\\\"\"\n");
}

#[test]
fn set_value_string_with_embedded_quote_doubles_in_single_quoted_site() {
    let mut doc = parse_document("title: 'a'\n").unwrap();
    doc.set_value("title", &Value::String("it's nice".into()))
        .unwrap();
    assert_eq!(doc.to_string(), "title: 'it''s nice'\n");
}

#[test]
fn set_value_string_falls_back_to_double_quoted_when_plain_is_unsafe() {
    // Existing site is a plain scalar but the new content cannot be
    // expressed plainly (`true` would resolve to a bool).
    let mut doc = parse_document("kind: foo\n").unwrap();
    doc.set_value("kind", &Value::String("true".into()))
        .unwrap();
    assert_eq!(doc.to_string(), "kind: \"true\"\n");
}

#[test]
fn set_value_number_emits_plain_regardless_of_existing_style() {
    let mut doc = parse_document("count: \"7\"\n").unwrap();
    doc.set_value("count", &Value::Number(42.into())).unwrap();
    assert_eq!(doc.to_string(), "count: 42\n");
    // Round-trip: the new value parses back as a number, not a string.
    assert!(doc.as_value()["count"].as_i64() == Some(42));
}

#[test]
fn set_value_bool_and_null() {
    let mut doc = parse_document("a: 1\nb: 1\n").unwrap();
    doc.set_value("a", &Value::Bool(false)).unwrap();
    doc.set_value("b", &Value::Null).unwrap();
    assert_eq!(doc.to_string(), "a: false\nb: null\n");
}

#[test]
fn set_value_into_sequence_index_emits_matching_style() {
    let mut doc = parse_document("xs: ['one', 'two']\n").unwrap();
    doc.set_value("xs[1]", &Value::String("TWO".into()))
        .unwrap();
    assert_eq!(doc.to_string(), "xs: ['one', 'TWO']\n");
}

#[test]
fn set_value_rejects_collection_replacement() {
    let mut doc = parse_document("items: 1\n").unwrap();
    let err = doc
        .set_value("items", &Value::Sequence(vec![Value::Number(1.into())]))
        .unwrap_err();
    assert!(err.to_string().contains("collection"));
}

#[test]
fn set_value_at_block_scalar_target_collapses_to_plain_for_single_line() {
    // Phase 2: replacing a block scalar with a single-line string
    // emits a plain/quoted scalar rather than `|-\n  hello`.
    let mut doc = parse_document("text: |\n  line1\n  line2\n").unwrap();
    doc.set_value("text", &Value::String("hello".into()))
        .unwrap();
    assert!(
        doc.to_string().contains("text: hello"),
        "expected plain replacement, got: {}",
        doc
    );
}

#[test]
fn set_value_at_block_scalar_target_keeps_block_form_for_multiline() {
    // Multi-line replacement keeps the block-scalar form so the
    // result still looks like a block scalar.
    let mut doc = parse_document("text: |\n  line1\n  line2\n").unwrap();
    doc.set_value("text", &Value::String("alpha\nbeta\n".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("text: |\n  alpha\n  beta\n"), "got: {out}");
}

// ── remove: drop a mapping key or sequence index ─────────────────

#[test]
fn remove_middle_mapping_key() {
    let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    doc.remove("b").unwrap();
    assert_eq!(doc.to_string(), "a: 1\nc: 3\n");
}

#[test]
fn remove_first_mapping_key() {
    let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    doc.remove("a").unwrap();
    assert_eq!(doc.to_string(), "b: 2\nc: 3\n");
}

#[test]
fn remove_last_mapping_key() {
    let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    doc.remove("c").unwrap();
    assert_eq!(doc.to_string(), "a: 1\nb: 2\n");
}

#[test]
fn remove_nested_mapping_key() {
    let src = "package:\n  name: foo\n  version: 0.0.1\n  build: 7\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("package.version").unwrap();
    assert_eq!(doc.to_string(), "package:\n  name: foo\n  build: 7\n");
}

#[test]
fn remove_sequence_index() {
    let src = "items:\n  - one\n  - two\n  - three\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("items[1]").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - three\n");
}

#[test]
fn remove_first_sequence_item() {
    let src = "items:\n  - one\n  - two\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("items[0]").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - two\n");
}

#[test]
fn remove_returns_path_not_found_for_missing_key() {
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    let err = doc.remove("missing").unwrap_err();
    assert!(err.to_string().contains("path not found"));
}

#[test]
fn remove_rejects_only_entry_of_mapping() {
    let mut doc = parse_document("only: 1\n").unwrap();
    let err = doc.remove("only").unwrap_err();
    assert!(err.to_string().contains("only entry"));
}

#[test]
fn remove_rejects_only_entry_of_sequence() {
    let mut doc = parse_document("xs:\n  - a\n").unwrap();
    let err = doc.remove("xs[0]").unwrap_err();
    assert!(err.to_string().contains("only entry"));
}

#[test]
fn remove_rejects_multi_line_value() {
    // Removing a key whose value is a block scalar is deferred —
    // the entry's bytes span multiple lines.
    let mut doc = parse_document("a: 1\ntext: |\n  hello\n  world\nb: 2\n").unwrap();
    let err = doc.remove("text").unwrap_err();
    assert!(err.to_string().contains("multi-line"));
}

#[test]
fn remove_preserves_surrounding_comments() {
    let src = "# header\na: 1\nb: 2  # tail\nc: 3\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("b").unwrap();
    assert_eq!(doc.to_string(), "# header\na: 1\nc: 3\n");
}
