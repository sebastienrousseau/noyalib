// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::key_span` — read-only byte span of a mapping entry's key
//! token (the companion to `span_at`, which returns the value span).
//!
//! Each test parses a document and checks that `key_span` returns the
//! exact bytes of the key as written (quotes included), or `None` for
//! sites that own no simple scalar key of their own.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

fn slice(src: &str, span: Option<(usize, usize)>) -> Option<&str> {
    span.map(|(s, e)| &src[s..e])
}

// ── Happy paths ─────────────────────────────────────────────────────

#[test]
fn simple_block_mapping_key() {
    let src = "name: foo\nversion: 1\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(slice(src, doc.key_span("name")), Some("name"));
    assert_eq!(slice(src, doc.key_span("version")), Some("version"));
}

#[test]
fn key_span_is_the_key_not_the_value() {
    let src = "name: foo\n";
    let doc = parse_document(src).unwrap();
    // key_span addresses the key; span_at addresses the value.
    assert_eq!(slice(src, doc.key_span("name")), Some("name"));
    assert_eq!(slice(src, doc.span_at("name")), Some("foo"));
}

#[test]
fn double_quoted_key_span_includes_quotes() {
    let src = "\"quoted key\": 1\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(
        slice(src, doc.key_span("quoted key")),
        Some("\"quoted key\"")
    );
}

#[test]
fn single_quoted_key_span_includes_quotes() {
    let src = "'a: b': 1\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(slice(src, doc.key_span("a: b")), Some("'a: b'"));
}

#[test]
fn nested_mapping_key() {
    let src = "server:\n  host: 0.0.0.0\n  port: 8080\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(slice(src, doc.key_span("server")), Some("server"));
    assert_eq!(slice(src, doc.key_span("server.host")), Some("host"));
    assert_eq!(slice(src, doc.key_span("server.port")), Some("port"));
}

#[test]
fn key_of_a_mapping_inside_a_sequence() {
    let src = "items:\n  - name: a\n  - name: b\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(slice(src, doc.key_span("items[0].name")), Some("name"));
    assert_eq!(slice(src, doc.key_span("items[1].name")), Some("name"));
}

#[test]
fn duplicate_keys_can_be_positioned() {
    // The key token resolves even when the same key appears twice —
    // exactly the diagnostic case this accessor exists for.
    let src = "k: one\nk: two\n";
    let doc = parse_document(src).unwrap();
    let (s, e) = doc.key_span("k").unwrap();
    assert_eq!(&src[s..e], "k");
}

// ── None cases ──────────────────────────────────────────────────────

#[test]
fn missing_path_is_none() {
    let doc = parse_document("a: 1\n").unwrap();
    assert_eq!(doc.key_span("nope"), None);
    assert_eq!(doc.key_span("a.b.c"), None);
}

#[test]
fn empty_path_is_none() {
    let doc = parse_document("a: 1\n").unwrap();
    assert_eq!(doc.key_span(""), None);
}

#[test]
fn sequence_index_is_none() {
    let doc = parse_document("items:\n  - a\n  - b\n").unwrap();
    // A sequence item has no key of its own.
    assert_eq!(doc.key_span("items[0]"), None);
}

#[test]
fn alias_site_owns_no_key_bytes() {
    let src = "base: &b\n  k: 1\nuse: *b\n";
    let doc = parse_document(src).unwrap();
    // The `*b` site reflects the anchor's entries but owns no key
    // bytes of its own, so a key addressed through it is None.
    assert_eq!(doc.key_span("use.k"), None);
    // The anchor's own key still resolves.
    assert_eq!(slice(src, doc.key_span("base.k")), Some("k"));
}

#[test]
fn merge_key_provided_key_is_none() {
    let src = "base: &b\n  k: 1\nchild:\n  <<: *b\n";
    let doc = parse_document(src).unwrap();
    // `k` is visible in `child` via the `<<` merge but has no source
    // entry of its own there.
    assert_eq!(doc.key_span("child.k"), None);
}

// ── Non-mutating ────────────────────────────────────────────────────

#[test]
fn key_span_does_not_modify_the_document() {
    let src = "name: foo  # comment\nversion: 1\n";
    let doc = parse_document(src).unwrap();
    let _ = doc.key_span("name");
    let _ = doc.key_span("missing");
    assert_eq!(doc.source(), src);
}
