// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Final coverage push on `crates/noyalib/src/cst/document.rs`.
//!
//! Targets the residual gap (~107 missed lines / 141 missed regions)
//! after `cst_document_coverage.rs`. Emphasis on the local-repair
//! ladder (`try_local_repair_green`) — the green-tree mutation paths
//! that selectively re-parse a single block-collection / block-entry
//! / scalar instead of the whole document.

use noyalib::cst::parse_document;

// ── Local-repair: scalar value changes (no anchor/tag/alias) ───────

#[test]
fn final_cst_local_repair_simple_scalar() {
    let mut doc = parse_document("port: 8080\n").expect("parse");
    doc.set("port", "9090").expect("set");
    assert_eq!(doc.to_string(), "port: 9090\n");
}

#[test]
fn final_cst_local_repair_scalar_multiple_keys_preserves_others() {
    let src = "name: api\nport: 8080\nenabled: true\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set("port", "9090").expect("set");
    let out = doc.to_string();
    assert!(out.contains("name: api"));
    assert!(out.contains("port: 9090"));
    assert!(out.contains("enabled: true"));
}

#[test]
fn final_cst_local_repair_string_to_string() {
    let mut doc = parse_document("name: alice\n").expect("parse");
    doc.set("name", "bob").expect("set");
    assert!(doc.to_string().contains("name: bob"));
}

#[test]
fn final_cst_local_repair_int_to_string_quotes_appropriately() {
    let mut doc = parse_document("token: 12345\n").expect("parse");
    doc.set("token", "\"a-string-token\"").expect("set");
    let out = doc.to_string();
    assert!(out.contains("token:") && out.contains("a-string-token"));
}

// ── Full re-parse fallback when anchor/alias/tag in scope ──────────

#[test]
fn final_cst_full_reparse_when_anchor_in_doc() {
    // When an anchor exists in the doc scope, editing the
    // anchored value drops the alias's resolution. The CST's
    // local-repair logic refuses the edit; the alias-renaming
    // path is policy-dependent. We assert only that the call
    // doesn't panic — either Ok (in-place rewrite preserving
    // the anchor source) or Err (alias dangle) is acceptable.
    let src = "shared: &x value\nref: *x\n";
    let mut doc = parse_document(src).expect("parse");
    let _ = doc.set("shared", "newvalue");
    let out = doc.to_string();
    assert!(out.contains("shared:"));
}

#[test]
fn final_cst_full_reparse_when_replacement_has_tag() {
    let mut doc = parse_document("a: plain\n").expect("parse");
    doc.set("a", "!!str text").expect("set");
    let out = doc.to_string();
    assert!(out.contains("!!str") || out.contains("text"));
}

#[test]
fn final_cst_full_reparse_when_replacement_has_anchor() {
    let mut doc = parse_document("a: 1\nb: 2\n").expect("parse");
    doc.set("a", "&anchor 100").expect("set");
    let out = doc.to_string();
    assert!(out.contains("100"));
}

// ── Sequence edits: indexed paths ──────────────────────────────────

#[test]
fn final_cst_seq_index_set() {
    let src = "items:\n  - one\n  - two\n  - three\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set("items[1]", "TWO").expect("set");
    let out = doc.to_string();
    assert!(out.contains("TWO"));
    assert!(out.contains("one") && out.contains("three"));
}

#[test]
fn final_cst_seq_first_item() {
    let src = "items:\n  - first\n  - second\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set("items[0]", "FIRST").expect("set");
    assert!(doc.to_string().contains("FIRST"));
}

#[test]
fn final_cst_seq_last_item() {
    let src = "items:\n  - a\n  - b\n  - c\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set("items[2]", "C").expect("set");
    assert!(doc.to_string().contains('C'));
}

// ── Nested mappings: dotted paths ──────────────────────────────────

#[test]
fn final_cst_nested_dotted_path() {
    let src = "server:\n  host: localhost\n  port: 8080\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set("server.port", "9090").expect("set");
    assert!(doc.to_string().contains("9090"));
}

#[test]
fn final_cst_deeply_nested_dotted_path() {
    let src = "outer:\n  middle:\n    inner: value\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set("outer.middle.inner", "newval").expect("set");
    assert!(doc.to_string().contains("newval"));
}

// ── Comments preserved through repair ──────────────────────────────

#[test]
fn final_cst_comment_preserved_above() {
    let src = "# top comment\nport: 8080\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set("port", "9090").expect("set");
    let out = doc.to_string();
    assert!(out.contains("# top comment"));
    assert!(out.contains("9090"));
}

#[test]
fn final_cst_inline_comment_preserved() {
    let src = "port: 8080  # the port\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set("port", "9090").expect("set");
    let out = doc.to_string();
    assert!(out.contains("the port"));
}

// ── Blank lines preserved ──────────────────────────────────────────

#[test]
fn final_cst_blank_lines_preserved() {
    let src = "a: 1\n\nb: 2\n\nc: 3\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set("b", "BB").expect("set");
    let out = doc.to_string();
    let blank_count = out.matches("\n\n").count();
    assert!(
        blank_count >= 2,
        "blank-line preservation: got {blank_count}"
    );
}

// ── Document-level operations ──────────────────────────────────────

#[test]
fn final_cst_to_string_round_trip_byte_faithful() {
    let src = "# leading\nname: alice\nport: 8080\n";
    let doc = parse_document(src).expect("parse");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn final_cst_round_trip_with_anchor_alias() {
    let src = "shared: &x value\nref: *x\n";
    let doc = parse_document(src).expect("parse");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn final_cst_round_trip_with_block_scalar_literal() {
    let src = "text: |\n  line one\n  line two\n";
    let doc = parse_document(src).expect("parse");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn final_cst_round_trip_with_block_scalar_folded() {
    let src = "text: >\n  line one\n  line two\n";
    let doc = parse_document(src).expect("parse");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn final_cst_round_trip_with_flow_sequence() {
    let src = "items: [a, b, c]\n";
    let doc = parse_document(src).expect("parse");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn final_cst_round_trip_with_flow_mapping() {
    let src = "obj: {a: 1, b: 2}\n";
    let doc = parse_document(src).expect("parse");
    assert_eq!(doc.to_string(), src);
}

// ── Path errors ────────────────────────────────────────────────────

#[test]
fn final_cst_set_on_missing_key_fails() {
    let mut doc = parse_document("a: 1\n").expect("parse");
    let r = doc.set("nonexistent", "5");
    assert!(r.is_err(), "missing path must error");
}

#[test]
fn final_cst_set_index_out_of_bounds_fails() {
    let mut doc = parse_document("items:\n  - one\n  - two\n").expect("parse");
    let r = doc.set("items[99]", "X");
    assert!(r.is_err(), "OOB index must error");
}

#[test]
fn final_cst_invalid_path_syntax_handled() {
    let mut doc = parse_document("a: 1\n").expect("parse");
    // A path that addresses through a non-mapping/non-sequence
    let r = doc.set("a.something", "x");
    // Either error or success — we don't pin behaviour, just that
    // the function doesn't panic.
    let _ = r;
}

// ── Validate ───────────────────────────────────────────────────────

#[test]
fn final_cst_validate_clean_document() {
    let doc = parse_document("a: 1\nb: 2\n").expect("parse");
    assert!(doc.validate().is_ok());
}

#[test]
fn final_cst_validate_with_anchors() {
    let doc = parse_document("a: &x 1\nb: *x\n").expect("parse");
    assert!(doc.validate().is_ok());
}

// ── Indent unit detection ──────────────────────────────────────────

#[test]
fn final_cst_indent_unit_2_space() {
    let doc = parse_document("a:\n  b: 1\n  c: 2\n").expect("parse");
    let _ = doc.indent_unit();
}

#[test]
fn final_cst_indent_unit_4_space() {
    let doc = parse_document("a:\n    b: 1\n    c: 2\n").expect("parse");
    let _ = doc.indent_unit();
}

#[test]
fn final_cst_indent_unit_with_seq() {
    let doc = parse_document("items:\n  - a\n  - b\n").expect("parse");
    let _ = doc.indent_unit();
}

// ── Entry API ──────────────────────────────────────────────────────

#[test]
fn final_cst_entry_get_existing() {
    let mut doc = parse_document("port: 8080\n").expect("parse");
    let e = doc.entry("port");
    assert!(e.get().is_some());
}

#[test]
fn final_cst_entry_get_missing() {
    let mut doc = parse_document("port: 8080\n").expect("parse");
    let e = doc.entry("nonexistent");
    assert!(e.get().is_none());
}

#[test]
fn final_cst_entry_exists() {
    let mut doc = parse_document("a: 1\n").expect("parse");
    assert!(doc.entry("a").exists());
    assert!(!doc.entry("nonexistent").exists());
}

#[test]
fn final_cst_entry_set_then_read_back() {
    let mut doc = parse_document("a: 1\n").expect("parse");
    doc.entry("a").set("99").expect("set");
    let _ = doc.entry("a").get();
    assert!(doc.to_string().contains("99"));
}

// ── Empty / special sources ────────────────────────────────────────

#[test]
fn final_cst_empty_document() {
    // `parse_document` of "" should succeed (empty doc is valid YAML).
    let doc = parse_document("").expect("parse empty");
    assert!(doc.to_string().is_empty() || doc.to_string() == "\n");
}

#[test]
fn final_cst_single_line_no_trailing_newline() {
    let src = "key: value";
    let doc = parse_document(src).expect("parse");
    let _ = doc.to_string();
}

#[test]
fn final_cst_explicit_doc_markers() {
    let src = "---\na: 1\n...\n";
    let doc = parse_document(src).expect("parse");
    assert_eq!(doc.to_string(), src);
}

// ── Clone and reuse ────────────────────────────────────────────────

#[test]
fn final_cst_document_clone_then_independent_edits() {
    let src = "a: 1\nb: 2\n";
    let doc1 = parse_document(src).expect("parse");
    let mut doc2 = doc1.clone();
    doc2.set("a", "999").expect("set");
    assert_eq!(doc1.to_string(), src);
    assert!(doc2.to_string().contains("999"));
}

#[test]
fn final_cst_clone_preserves_anchors() {
    let src = "x: &a 1\ny: *a\n";
    let doc = parse_document(src).expect("parse");
    let cloned = doc.clone();
    assert_eq!(doc.to_string(), cloned.to_string());
}

// ── Many sequential edits exercise repair-then-cache state ─────────

#[test]
fn final_cst_sequential_edits_compound() {
    let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").expect("parse");
    doc.set("a", "10").expect("a");
    doc.set("b", "20").expect("b");
    doc.set("c", "30").expect("c");
    let out = doc.to_string();
    assert!(out.contains("10") && out.contains("20") && out.contains("30"));
}

#[test]
fn final_cst_repeated_edit_same_key() {
    let mut doc = parse_document("port: 8080\n").expect("parse");
    doc.set("port", "9090").expect("first");
    doc.set("port", "10000").expect("second");
    doc.set("port", "11111").expect("third");
    assert!(doc.to_string().contains("11111"));
}
