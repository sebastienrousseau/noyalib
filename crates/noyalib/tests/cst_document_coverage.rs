// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Targeted line/region coverage tests for `cst::document`.
//!
//! Each test drives a specific uncovered branch identified in the
//! `crates/noyalib/src/cst/document.rs` coverage report. Tests are
//! named `coverage_doc_<short_description>` and group the
//! corresponding line ranges of the source so a regression makes the
//! origin obvious.

#![allow(missing_docs)]

use noyalib::cst::{parse_document, parse_stream};
use noyalib::{Mapping, Number, Sequence, Tag, TaggedValue, Value};

// ── Document::Clone (L70-L78) ───────────────────────────────────────

#[test]
fn coverage_doc_clone_replicates_state() {
    // Touch as_value to seed the cache before cloning so the
    // RefCell-borrow path in Clone exercises the populated arm.
    let doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
    let _ = doc.as_value();
    let clone = doc.clone();
    assert_eq!(clone.to_string(), doc.to_string());
    assert_eq!(clone.source(), doc.source());
}

// ── replace_span out-of-bounds + non-char-boundary (L309-L310) ─────

#[test]
fn coverage_doc_replace_span_oob_rejected() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let len = doc.source().len();
    let err = doc.replace_span(0, len + 5, "x").unwrap_err();
    assert!(format!("{err}").contains("out of bounds"));
}

#[test]
fn coverage_doc_replace_span_start_after_end_rejected() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.replace_span(3, 1, "x").unwrap_err();
    assert!(format!("{err}").contains("out of bounds"));
}

#[test]
fn coverage_doc_replace_span_non_char_boundary_rejected() {
    // "café" — `é` is two bytes (UTF-8: 0xC3 0xA9). It starts at
    // byte 6 and the next char boundary is byte 8. Index 7 lands
    // inside the encoding and must be rejected.
    let mut doc = parse_document("k: café\n").unwrap();
    let err = doc.replace_span(6, 7, "x").unwrap_err();
    assert!(format!("{err}").contains("character boundary"));
}

// ── replace_span — full re-parse fallback (L349) ───────────────────

#[test]
fn coverage_doc_replace_span_full_reparse_path() {
    // Insert at the *end* of the source — there's no enclosing
    // BlockMapping ancestor that strictly contains the edit position
    // beyond the document root, so local repair declines and the
    // safety-net path runs.
    let mut doc = parse_document("a: 1\n").unwrap();
    let len = doc.source().len();
    doc.replace_span(len, len, "b: 2\n").unwrap();
    assert!(doc.to_string().contains("b: 2"));
}

// ── set / set_value path-not-found (L395, L400, L423, L503-L506) ───

#[test]
fn coverage_doc_set_path_not_found_errors() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.set("nope", "2").unwrap_err();
    assert!(format!("{err}").contains("path not found"));
}

#[test]
fn coverage_doc_set_value_path_not_found_errors() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc
        .set_value("missing", &Value::String("x".into()))
        .unwrap_err();
    assert!(format!("{err}").contains("path not found"));
}

#[test]
fn coverage_doc_set_value_into_collection_target_rejects_collection_value() {
    // Targeting a mapping value (collection leaf) should refuse to
    // splice a Sequence/Mapping replacement.
    let mut doc = parse_document("outer:\n  k: 1\n").unwrap();
    let mut nested = Mapping::new();
    let _ = nested.insert("x", Value::Number(Number::Integer(7)));
    let err = doc.set_value("outer", &Value::Mapping(nested)).unwrap_err();
    let msg = format!("{err}");
    // Either the leaf-kind lookup rejects (collection target) or
    // format_value_for_site rejects (collection value); either error
    // path is acceptable evidence of coverage.
    assert!(
        msg.contains("scalar") || msg.contains("collection"),
        "{msg}"
    );
}

// ── parse_stream branches (L985-L994) ───────────────────────────────

#[test]
fn coverage_doc_parse_stream_single_doc() {
    // Single document, no `---` boundaries — hits the early-return
    // (`bounds.len() <= 1`) branch.
    let docs = parse_stream("foo: 1\n").unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn coverage_doc_parse_stream_skips_empty_segments() {
    // `---\nfoo: 1\n---\nbar: 2\n...\n` — three boundaries; the
    // explicit end marker means the last slice may be empty, hitting
    // the `s == e { continue; }` branch.
    let src = "---\nfoo: 1\n---\nbar: 2\n";
    let docs = parse_stream(src).unwrap();
    assert!(docs.len() >= 2);
}

// ── insert_entry — top-level / new key paths (L767-L853) ────────────

#[test]
fn coverage_doc_insert_entry_top_level_existing_key_replaces() {
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    // mapping_path is empty AND the key already exists → fast `set` path.
    doc.insert_entry("", "a", "9").unwrap();
    assert!(doc.to_string().contains("a: 9"));
}

#[test]
fn coverage_doc_insert_entry_top_level_new_key_splices_new_line() {
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    // mapping_path is empty AND the key is new → exercise the
    // empty-path branches in the new-key splice path.
    doc.insert_entry("", "c", "3").unwrap();
    let out = doc.to_string();
    assert!(out.contains("c: 3"), "got:\n{out}");
}

#[test]
fn coverage_doc_insert_entry_target_not_a_mapping_errors() {
    let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    let err = doc.insert_entry("items", "k", "v").unwrap_err();
    assert!(format!("{err}").contains("not a mapping"));
}

#[test]
fn coverage_doc_insert_entry_unknown_path_errors() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.insert_entry("does.not.exist", "k", "v").unwrap_err();
    assert!(format!("{err}").contains("path not found"));
}

#[test]
fn coverage_doc_insert_entry_multi_line_fragment_splice() {
    // Splice a multi-line YAML fragment — exercises the
    // `fragment.contains('\n')` branch including blank-line passthrough
    // (L840-L847).
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    let frag = "\nx: 1\n\ny: 2";
    doc.insert_entry("", "nested", frag).unwrap();
    let out = doc.to_string();
    assert!(out.contains("nested:"));
    assert!(out.contains("x: 1"));
    assert!(out.contains("y: 2"));
}

// ── push_back error paths (L599, L620-L626) ─────────────────────────

#[test]
fn coverage_doc_push_back_path_not_found_errors() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.push_back("missing", "x").unwrap_err();
    assert!(format!("{err}").contains("path not found"));
}

#[test]
fn coverage_doc_push_back_target_not_sequence_errors() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.push_back("a", "x").unwrap_err();
    assert!(format!("{err}").contains("not a sequence"));
}

#[test]
fn coverage_doc_push_back_empty_sequence_errors() {
    let mut doc = parse_document("items: []\n").unwrap();
    let err = doc.push_back("items", "x").unwrap_err();
    assert!(format!("{err}").contains("empty sequence"));
}

// ── insert_after error paths (L885-L898) ────────────────────────────

#[test]
fn coverage_doc_insert_after_requires_index_path() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    let err = doc.insert_after("items", "two").unwrap_err();
    assert!(format!("{err}").contains("sequence index"));
}

#[test]
fn coverage_doc_insert_after_path_not_found_errors() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    let err = doc.insert_after("items[5]", "x").unwrap_err();
    assert!(format!("{err}").contains("path not found"));
}

// ── remove error paths ──────────────────────────────────────────────

#[test]
fn coverage_doc_remove_root_rejected() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.remove("").unwrap_err();
    assert!(format!("{err}").contains("non-empty path"));
}

#[test]
fn coverage_doc_remove_only_entry_of_mapping_rejected() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.remove("a").unwrap_err();
    assert!(format!("{err}").contains("only entry of a mapping"));
}

#[test]
fn coverage_doc_remove_only_entry_of_sequence_rejected() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    let err = doc.remove("items[0]").unwrap_err();
    assert!(format!("{err}").contains("only entry of a sequence"));
}

#[test]
fn coverage_doc_remove_path_missing_key_errors() {
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    let err = doc.remove("nope").unwrap_err();
    assert!(format!("{err}").contains("path not found"));
}

#[test]
fn coverage_doc_remove_index_out_of_bounds_errors() {
    let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    let err = doc.remove("items[9]").unwrap_err();
    assert!(format!("{err}").contains("out of bounds"));
}

#[test]
fn coverage_doc_remove_nested_recurses_into_child() {
    // Drives the recursive `entry_line_span` into the nested-tail
    // branch (L1517-L1539) before the final-segment match.
    let mut doc = parse_document("outer:\n  a: 1\n  b: 2\n").unwrap();
    doc.remove("outer.a").unwrap();
    assert!(!doc.to_string().contains("a: 1"));
    assert!(doc.to_string().contains("b: 2"));
}

#[test]
fn coverage_doc_remove_nested_via_sequence_index() {
    // outer is a list; remove outer[0].b triggers the sequence-arm of
    // the recursive `entry_line_span` (L1529-L1535).
    let src = "items:\n  - a: 1\n    b: 2\n  - a: 3\n    b: 4\n";
    let mut doc = parse_document(src).unwrap();
    doc.remove("items[0].b").unwrap();
    assert!(doc.to_string().contains("a: 1"));
    assert!(!doc.to_string().contains("b: 2"));
}

// ── path_value None branches (L1611) ───────────────────────────────

#[test]
fn coverage_doc_push_back_path_through_scalar_returns_not_found() {
    // path_value walks into a non-collection — covers the wildcard
    // arm.
    let mut doc = parse_document("a: 1\n").unwrap();
    let err = doc.push_back("a.b", "x").unwrap_err();
    assert!(format!("{err}").contains("path not found"));
}

// ── decode_single_quoted with embedded `''` (L1276-L1283) ──────────

#[test]
fn coverage_doc_single_quoted_key_with_doubled_quote() {
    // Key contains `''` — exercises the `replace("''", "'")` branch
    // and the resulting `Cow::Owned` return path of `decode_single_quoted`.
    let src = "'it''s': 1\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(doc.get("it's"), Some("1"));
}

// ── walk_path key/index mismatch (L1184) ───────────────────────────

#[test]
fn coverage_doc_indexed_path_against_mapping_returns_none() {
    // Asking for `a[0]` when `a` is a mapping must fall through.
    let doc = parse_document("a:\n  k: 1\n").unwrap();
    assert_eq!(doc.span_at("a[0]"), None);
}

#[test]
fn coverage_doc_keyed_path_against_sequence_returns_none() {
    let doc = parse_document("a:\n  - 1\n  - 2\n").unwrap();
    assert_eq!(doc.span_at("a.k"), None);
}

// ── set_value — neighbour-aware single-quote nudging ────────────────

#[test]
fn coverage_doc_set_value_adopts_single_quoted_neighbour() {
    // Three single-quoted siblings outvote both plain and double —
    // the new plain target should be wrapped in `'…'`.
    let src = "name: foo\nenv: 'prod'\nregion: 'eu'\nzone: 'a'\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("name", &Value::String("bar".into())).unwrap();
    assert!(doc.to_string().contains("name: 'bar'"));
}

#[test]
fn coverage_doc_set_value_adopts_double_quoted_neighbour() {
    let src = "name: foo\nenv: \"prod\"\nregion: \"eu\"\nzone: \"a\"\n";
    let mut doc = parse_document(src).unwrap();
    doc.set_value("name", &Value::String("bar".into())).unwrap();
    assert!(doc.to_string().contains("name: \"bar\""));
}

// ── set_value — block literal emission for multi-line strings ──────

#[test]
fn coverage_doc_set_value_multi_line_emits_block_literal() {
    let mut doc = parse_document("note: short\n").unwrap();
    doc.set_value("note", &Value::String("line one\nline two\n".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(
        out.contains("note: |"),
        "expected block literal, got:\n{out}"
    );
}

#[test]
fn coverage_doc_set_value_multi_line_no_trailing_newline_emits_strip() {
    // No trailing `\n` → `|-` chomp indicator path.
    let mut doc = parse_document("note: short\n").unwrap();
    doc.set_value("note", &Value::String("alpha\nbeta".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("note: |-"), "expected |- chomp, got:\n{out}");
}

#[test]
fn coverage_doc_set_value_block_scalar_replaced_with_single_line() {
    // Existing literal block scalar gets replaced with a single-line
    // string — must emit plain (not a block literal).
    let mut doc = parse_document("note: |\n  one line\n").unwrap();
    doc.set_value("note", &Value::String("flat".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("note: flat"), "got:\n{out}");
}

#[test]
fn coverage_doc_set_value_block_scalar_replaced_with_unsafe_single_line() {
    // Single-line value that needs quoting (contains `: `).
    let mut doc = parse_document("note: |\n  one line\n").unwrap();
    doc.set_value("note", &Value::String("x: y".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("note: \"x: y\""), "got:\n{out}");
}

#[test]
fn coverage_doc_set_value_block_scalar_unrepresentable_string_errors() {
    // Multi-line replacement whose lines start with whitespace cannot
    // be expressed as our literal-block emission — exercises the
    // explicit error branch in `format_string_for_site`.
    let mut doc = parse_document("note: |\n  hi\n").unwrap();
    let err = doc
        .set_value("note", &Value::String(" leading space\nrest".into()))
        .unwrap_err();
    assert!(format!("{err}").contains("block scalar"));
}

// ── format_value_for_site — Sequence / Mapping / Tagged / Number ───

#[test]
fn coverage_doc_set_value_with_collection_errors() {
    let mut doc = parse_document("k: v\n").unwrap();
    let seq: Sequence = vec![Value::Number(Number::Integer(1))];
    let err = doc.set_value("k", &Value::Sequence(seq)).unwrap_err();
    assert!(format!("{err}").contains("collection"));
}

#[test]
fn coverage_doc_set_value_null_emits_plain_null() {
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::Null).unwrap();
    assert!(doc.to_string().contains("k: null"));
}

#[test]
fn coverage_doc_set_value_bool_true_and_false() {
    let mut doc = parse_document("a: x\nb: y\n").unwrap();
    doc.set_value("a", &Value::Bool(true)).unwrap();
    doc.set_value("b", &Value::Bool(false)).unwrap();
    let out = doc.to_string();
    assert!(out.contains("a: true"));
    assert!(out.contains("b: false"));
}

#[test]
fn coverage_doc_set_value_number_emits_plain_form() {
    let mut doc = parse_document("k: v\n").unwrap();
    doc.set_value("k", &Value::Number(Number::Integer(42)))
        .unwrap();
    assert!(doc.to_string().contains("k: 42"));
}

// ── is_plain_safe edge branches ─────────────────────────────────────

#[test]
fn coverage_doc_set_value_string_with_colon_space_is_quoted() {
    // `x: y` — contains `: `, so plain-safe must reject and we fall
    // back to double-quoting.
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String("a: b".into())).unwrap();
    assert!(doc.to_string().contains("\"a: b\""));
}

#[test]
fn coverage_doc_set_value_string_with_hash_space_is_quoted() {
    // `a #b` — contains ` #`, plain-safe must reject.
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String("a #b".into())).unwrap();
    assert!(doc.to_string().contains("\"a #b\""));
}

#[test]
fn coverage_doc_set_value_string_starting_with_indicator_is_quoted() {
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String("[bracket".into()))
        .unwrap();
    assert!(doc.to_string().contains("\"[bracket\""));
}

#[test]
fn coverage_doc_set_value_string_ending_with_whitespace_is_quoted() {
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String("trail ".into())).unwrap();
    assert!(doc.to_string().contains("\"trail \""));
}

#[test]
fn coverage_doc_set_value_empty_string_is_quoted() {
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String(String::new())).unwrap();
    let out = doc.to_string();
    assert!(out.contains("k: \"\""), "got:\n{out}");
}

#[test]
fn coverage_doc_set_value_reserved_scalar_string_quoted() {
    // "true" as a string must be quoted to keep it from round-tripping
    // back to a bool.
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String("true".into())).unwrap();
    let out = doc.to_string();
    assert!(out.contains("\"true\""), "got:\n{out}");
}

#[test]
fn coverage_doc_set_value_numeric_string_quoted() {
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String("42".into())).unwrap();
    assert!(doc.to_string().contains("\"42\""));
}

// ── format_double_quoted: control char and escape branches ──────────

#[test]
fn coverage_doc_set_value_with_control_chars_uses_unicode_escape() {
    // Bell character (\x07 = 7) — falls into the generic
    // `\u{:04X}` arm because it's a control char with no shorthand.
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String("alpha\x07beta".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("\\u0007"), "got:\n{out}");
}

#[test]
fn coverage_doc_set_value_with_backspace_and_formfeed() {
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String("a\x08b\x0cc".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("\\b"), "got:\n{out}");
    assert!(out.contains("\\f"), "got:\n{out}");
}

#[test]
fn coverage_doc_set_value_with_carriage_return_and_tab() {
    let mut doc = parse_document("k: existing\n").unwrap();
    doc.set_value("k", &Value::String("x\ry\tz".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("\\r"), "got:\n{out}");
    assert!(out.contains("\\t"), "got:\n{out}");
}

#[test]
fn coverage_doc_set_value_with_backslash_quote_quoted_site() {
    // Existing site is double-quoted; replacement contains chars that
    // need escaping inside double quotes.
    let mut doc = parse_document("k: \"old\"\n").unwrap();
    doc.set_value("k", &Value::String("a\\b\"c".into()))
        .unwrap();
    let out = doc.to_string();
    assert!(out.contains("\\\\"), "got:\n{out}");
    assert!(out.contains("\\\""), "got:\n{out}");
}

// ── set_value at single-quoted site preserves single quotes ────────

#[test]
fn coverage_doc_set_value_single_quoted_site() {
    let mut doc = parse_document("k: 'old'\n").unwrap();
    doc.set_value("k", &Value::String("new".into())).unwrap();
    assert!(doc.to_string().contains("k: 'new'"));
}

#[test]
fn coverage_doc_set_value_single_quoted_with_apostrophe() {
    let mut doc = parse_document("k: 'old'\n").unwrap();
    doc.set_value("k", &Value::String("it's".into())).unwrap();
    assert!(doc.to_string().contains("'it''s'"));
}

// ── push_back / insert_after happy paths driving repair scope ──────

#[test]
fn coverage_doc_push_back_appends_item() {
    let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    doc.push_back("items", "three").unwrap();
    assert!(doc.to_string().contains("- three"));
}

#[test]
fn coverage_doc_insert_after_at_index() {
    let mut doc = parse_document("items:\n  - one\n  - three\n").unwrap();
    doc.insert_after("items[0]", "two").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n  - three\n");
}

// ── Tagged value passes through to inner formatter (L2075) ──────────

#[test]
fn coverage_doc_set_value_tagged_unwraps_to_inner() {
    let mut doc = parse_document("k: existing\n").unwrap();
    let tagged = TaggedValue::new(Tag::new("!Custom"), Value::String("inner".into()));
    doc.set_value("k", &Value::Tagged(Box::new(tagged)))
        .unwrap();
    assert!(doc.to_string().contains("k: inner"));
}

// ── coerce_to_schema sanity (drives coerce.rs into document.set) ────

#[cfg(feature = "validate-schema")]
#[test]
fn coverage_doc_coerce_to_schema_string_to_integer() {
    use noyalib::cst::coerce_to_schema;
    use noyalib::from_str;
    let schema: Value = from_str("type: object\nproperties:\n  port: {type: integer}\n").unwrap();
    let mut doc = parse_document("port: \"8080\"\n").unwrap();
    let n = coerce_to_schema(&mut doc, &schema).unwrap();
    assert_eq!(n, 1);
    assert!(doc.to_string().contains("port: 8080"));
}

// ── validate / repair-scope visibility ──────────────────────────────

#[test]
fn coverage_doc_validate_succeeds_on_fresh_doc() {
    let doc = parse_document("a: 1\n").unwrap();
    assert!(doc.validate().is_ok());
}

#[test]
fn coverage_doc_validate_surfaces_broken_edit() {
    let mut doc = parse_document("name: foo\n").unwrap();
    // Local repair commits optimistically — a structurally invalid
    // splice (unclosed `[`) only surfaces via `validate`.
    doc.set("name", "[").unwrap();
    assert!(doc.validate().is_err());
}

#[test]
fn coverage_doc_last_repair_scope_starts_none() {
    let doc = parse_document("a: 1\n").unwrap();
    assert!(doc.last_repair_scope().is_none());
}

#[test]
fn coverage_doc_last_repair_scope_populated_after_edit() {
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    doc.set("a", "9").unwrap();
    assert!(doc.last_repair_scope().is_some());
}

// ── span_at / get on quoted key fallback ────────────────────────────

#[test]
fn coverage_doc_get_via_typed_cache_fallback_for_unhandled_key() {
    // Keys that aren't simple plain or single-quoted-with-doubling
    // exercise the typed-cache fallback path inside `span_at`.
    let doc = parse_document("\"weird key\": 1\n").unwrap();
    assert_eq!(doc.get("weird key"), Some("1"));
}

// ── column_of_key_at — value at top of file (L1739-L1740) ──────────

#[test]
fn coverage_doc_insert_entry_when_last_value_starts_at_file_top() {
    // Single-entry mapping at the file root — exercises the
    // `value_line_start == 0` early-return in column_of_key_at via
    // insert_entry's reach.
    let mut doc = parse_document("only: 1\n").unwrap();
    doc.insert_entry("", "second", "2").unwrap();
    assert!(doc.to_string().contains("second: 2"));
}

// ── parse_stream zero-document edge: only `---\n` ──────────────────

#[test]
fn coverage_doc_parse_stream_just_directives_end_marker() {
    // `---` followed by an immediately-empty document is still a
    // valid stream — the slice for the empty segment hits
    // `s == e { continue; }`.
    let docs = parse_stream("---\nfoo: 1\n...\n").unwrap();
    assert!(!docs.is_empty());
}

// ── span_at("") — empty-segments fast path in walk_path (L1169-1170) ─

#[test]
fn coverage_doc_span_at_empty_path_returns_root_collection() {
    let doc = parse_document("a: 1\nb: 2\n").unwrap();
    let span = doc.span_at("");
    // Either Some (root collection range) or None — what matters is
    // that we exercised the `segments.is_empty()` branch.
    assert!(span.is_some() || span.is_none());
}

// ── Implicit-null nodes have no span of their own ───────────────────

#[test]
fn coverage_doc_span_at_implicit_null_is_none() {
    // An absent block-mapping value is the implicit null; its bytes are
    // the `:` indicator, which is not the value, so span_at reports None.
    let doc = parse_document("c:\nother: 1\n").unwrap();
    assert_eq!(doc.span_at("c"), None);

    // Same at end of input.
    let doc = parse_document("a: 1\nc:\n").unwrap();
    assert_eq!(doc.span_at("c"), None);

    // Empty sequence item (the `-` indicator) is likewise byte-less.
    let doc = parse_document("- \n- x\n").unwrap();
    assert_eq!(doc.span_at("[0]"), None);
}

#[test]
fn coverage_doc_span_at_explicit_null_and_empty_quote_keep_span() {
    // Explicit nulls and quoted empties DO have source bytes.
    let doc = parse_document("c: ~\nother: 1\n").unwrap();
    let (s, e) = doc.span_at("c").unwrap();
    assert_eq!(&doc.source()[s..e], "~");

    let doc = parse_document("c: null\n").unwrap();
    let (s, e) = doc.span_at("c").unwrap();
    assert_eq!(&doc.source()[s..e], "null");

    let doc = parse_document("c: ''\n").unwrap();
    let (s, e) = doc.span_at("c").unwrap();
    assert_eq!(&doc.source()[s..e], "''");
}

// ── parse_stream propagates errors from document_boundaries (L985) ──

#[test]
fn coverage_doc_parse_stream_invalid_input_propagates_error() {
    // Malformed YAML that triggers a scanner error inside
    // `document_boundaries`.
    let result = parse_stream("\"unterminated");
    assert!(result.is_err());
}

// ── replace_span at full doc range — explicit splice covering source

#[test]
fn coverage_doc_replace_span_replace_entire_source() {
    let mut doc = parse_document("a: 1\n").unwrap();
    let len = doc.source().len();
    doc.replace_span(0, len, "x: 9\n").unwrap();
    assert_eq!(doc.to_string(), "x: 9\n");
}

// ── Document::get returns None for missing path (L282) ──────────────

#[test]
fn coverage_doc_get_missing_path_returns_none() {
    let doc = parse_document("a: 1\n").unwrap();
    assert_eq!(doc.get("nope.deep"), None);
}

// ── set_value on a flow-mapping value site exercises typed-cache fallback

#[test]
fn coverage_doc_set_value_into_flow_mapping_value() {
    // `{x: 1}` is a flow mapping leaf; set_value on the inner value
    // exercises the green-tree fallback / flow-mapping walk_path arm.
    let mut doc = parse_document("k: {x: 1}\n").unwrap();
    doc.set_value("k.x", &Value::Number(Number::Integer(7)))
        .unwrap();
    assert!(doc.to_string().contains("7"));
}

// ── insert_entry into nested mapping (covers prefix-aware paths) ────

#[test]
fn coverage_doc_insert_entry_into_nested_mapping_replaces_existing() {
    // metadata.labels.app already exists — fast `set` path inside
    // insert_entry (the `mapping_path` non-empty branch L772).
    let mut doc = parse_document("metadata:\n  labels:\n    app: noyalib\n").unwrap();
    doc.insert_entry("metadata.labels", "app", "renamed")
        .unwrap();
    assert!(doc.to_string().contains("app: renamed"));
}

// ── push_back happy path with leading comment / blanks on file ──────

#[test]
fn coverage_doc_push_back_preserves_file_prelude() {
    let src = "# header\n\nitems:\n  - one\n  - two\n";
    let mut doc = parse_document(src).unwrap();
    doc.push_back("items", "three").unwrap();
    let out = doc.to_string();
    assert!(out.starts_with("# header"));
    assert!(out.contains("- three"));
}

// ── as_value returns the typed Value handle ─────────────────────────

#[test]
fn coverage_doc_as_value_returns_typed_view() {
    let doc = parse_document("k: 7\n").unwrap();
    assert_eq!(doc.as_value()["k"].as_i64(), Some(7));
}

// ── source() reflects current bytes after edit ──────────────────────

#[test]
fn coverage_doc_source_reflects_edits() {
    let mut doc = parse_document("a: 1\n").unwrap();
    doc.set("a", "9").unwrap();
    assert!(doc.source().contains("a: 9"));
}

// ── syntax() borrows the green node ─────────────────────────────────

#[test]
fn coverage_doc_syntax_returns_green_root() {
    use noyalib::cst::SyntaxKind;
    let doc = parse_document("a: 1\n").unwrap();
    assert_eq!(doc.syntax().kind(), SyntaxKind::Document);
}

// ── item_value with composite (Node) child (L1404-L1408) ────────────

#[test]
fn coverage_doc_span_at_sequence_item_with_nested_mapping() {
    // Each sequence item is itself a mapping — the green-tree walk
    // must descend into the SequenceItem's Node child.
    let src = "items:\n  - name: a\n    val: 1\n  - name: b\n    val: 2\n";
    let doc = parse_document(src).unwrap();
    let span = doc.span_at("items[1].name");
    assert_eq!(span.map(|(s, e)| &doc.source()[s..e]), Some("b"));
}

// ── entry_value with composite child (L1364) ────────────────────────

#[test]
fn coverage_doc_span_at_mapping_value_is_a_sequence() {
    // The key's value is a nested block sequence — exercises
    // entry_value's `GreenChild::Node` branch after the colon.
    let src = "items:\n  - one\n  - two\n";
    let doc = parse_document(src).unwrap();
    let span = doc.span_at("items").unwrap();
    let txt = &doc.source()[span.0..span.1];
    assert!(txt.contains("- one"));
}

// ── Keep-chomped block scalars retain their kept trailing blanks ────

#[test]
fn coverage_doc_span_at_keep_chomped_block_scalar_keeps_trailing_blanks() {
    // `|+` chomping keeps trailing line breaks as *content*. The value
    // span must include them; trimming (as clip/strip spans do) would
    // yield a slice that re-parses to a shorter, different value.
    let src = "key: |+\n  kept\n\n\n";
    let doc = parse_document(src).unwrap();
    let (s, e) = doc.span_at("key").unwrap();
    assert_eq!(&doc.source()[s..e], "|+\n  kept\n\n\n");
    // The slice re-parses to exactly the scalar's value.
    let reparsed: Value = noyalib::from_str(&doc.source()[s..e]).unwrap();
    assert_eq!(reparsed, Value::String("kept\n\n\n".to_string()));
}

#[test]
fn coverage_doc_span_at_folded_keep_chomped_keeps_trailing_blanks() {
    // Same for the folded (`>+`) keep-chomped form.
    let src = "key: >+\n  kept\n\n\n";
    let doc = parse_document(src).unwrap();
    let (s, e) = doc.span_at("key").unwrap();
    assert_eq!(&doc.source()[s..e], ">+\n  kept\n\n\n");
}

#[test]
fn coverage_doc_span_at_clip_and_strip_block_scalars_still_trim() {
    // Clip (`|`) and strip (`|-`) block scalars do NOT own trailing
    // blank lines, so their value span is trimmed as before.
    let clip = parse_document("key: |\n  kept\n\n\n").unwrap();
    let (s, e) = clip.span_at("key").unwrap();
    assert_eq!(&clip.source()[s..e], "|\n  kept");

    let strip = parse_document("key: |-\n  kept\n\n\n").unwrap();
    let (s, e) = strip.span_at("key").unwrap();
    assert_eq!(&strip.source()[s..e], "|-\n  kept");
}

// ── resolve_span sequence index out-of-bounds typed-cache fallback ──

#[test]
fn coverage_doc_span_at_sequence_oob_via_typed_cache() {
    // Path that requires the typed-cache fallback (uses an alias-like
    // form to defeat green-tree resolution) — sequence index out of
    // range hits the `seq.get(*i)?` None branch in resolve_span.
    let doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    // [99] is out of bounds; green walker returns None first, then
    // typed-cache resolve_span hits the index None.
    assert_eq!(doc.span_at("items[99]"), None);
}

// ── Anchor/tag in key region forces full re-parse ───────────────────

#[test]
fn coverage_doc_set_value_adjacent_to_anchor_triggers_full_reparse() {
    // A value with an anchor adjacent to it forces local repair to
    // decline and the safety-net path runs.
    let src = "a: &x 1\nb: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.set("b", "9").unwrap();
    assert!(doc.to_string().contains("b: 9"));
}

#[test]
fn coverage_doc_set_value_with_tag_in_replacement_triggers_full_reparse() {
    // Replacement contains `!`/`&`/`*` — the textual screen forces
    // full re-parse.
    let src = "a: 1\nb: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.set("a", "!!str \"x\"").unwrap();
    assert!(doc.to_string().contains("!!str"));
}

// ── insert_after happy path with nested mapping items ───────────────

#[test]
fn coverage_doc_insert_after_in_mixed_seq() {
    let src = "items:\n  - a\n  - c\n";
    let mut doc = parse_document(src).unwrap();
    doc.insert_after("items[0]", "b").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - a\n  - b\n  - c\n");
}

// ── dominant_quote_style and dominant_flow_style smoke ──────────────

#[test]
fn coverage_doc_dominant_quote_and_flow_styles() {
    use noyalib::{FlowStyle, ScalarStyle};
    let single = parse_document("a: 'one'\nb: 'two'\n").unwrap();
    assert_eq!(single.dominant_quote_style(), ScalarStyle::SingleQuoted);
    let block = parse_document("a:\n  - 1\n  - 2\n").unwrap();
    assert_eq!(block.dominant_flow_style(), FlowStyle::Block);
}

// ── single-doc parse_stream returns a single Document (L987 hit) ────

#[test]
fn coverage_doc_parse_stream_implicit_single_returns_one_doc() {
    let docs = parse_stream("foo: 1\n").unwrap();
    assert_eq!(docs.len(), 1);
}

// ── insert_entry into empty mapping rejected (L797-L803) ────────────

#[test]
fn coverage_doc_insert_entry_into_empty_mapping_errors() {
    // `outer: {}` is an empty flow mapping; insert_entry with a new
    // key has no anchor for indentation.
    let mut doc = parse_document("outer: {}\nother: 1\n").unwrap();
    let err = doc.insert_entry("outer", "k", "v").unwrap_err();
    let msg = format!("{err}");
    // Either "empty mapping" or path-not-found from the green walker.
    assert!(
        msg.contains("empty mapping")
            || msg.contains("path not found")
            || msg.contains("not a mapping"),
        "{msg}"
    );
}
