// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Transactional CST batch editing.

use noyalib::ParserConfig;
use noyalib::cst::{RepairScope, parse_document, parse_document_with_config};

#[test]
fn independent_spans_commit_as_one_atomic_batch() {
    let mut doc = parse_document("first: one\nsecond: two\n").unwrap();
    let first = doc.span_at("first").unwrap();
    let second = doc.span_at("second").unwrap();

    let mut edit = doc.edit();
    edit.replace_span(second.0, second.1, "2").unwrap();
    edit.replace_span(first.0, first.1, "1").unwrap();
    assert_eq!(edit.len(), 2);
    edit.commit().unwrap();

    assert_eq!(doc.source(), "first: 1\nsecond: 2\n");
    assert_eq!(doc.as_value()["first"].as_i64(), Some(1));
    assert_eq!(doc.as_value()["second"].as_i64(), Some(2));
    assert_eq!(doc.last_repair_scope(), Some(RepairScope::Document));
}

#[test]
fn failed_commit_preserves_every_document_view() {
    const SOURCE: &str = "first: one\nsecond: two\n";
    let mut doc = parse_document(SOURCE).unwrap();
    let before_value = doc.as_value().clone();
    let second = doc.span_at("second").unwrap();

    let mut edit = doc.edit();
    edit.replace_span(second.0, second.1, "[").unwrap();
    assert!(edit.commit().is_err());

    assert_eq!(doc.source(), SOURCE);
    assert_eq!(*doc.as_value(), before_value);
    doc.validate().unwrap();
}

#[test]
fn dropped_or_aborted_sessions_change_nothing() {
    const SOURCE: &str = "key: value\n";
    let mut doc = parse_document(SOURCE).unwrap();
    let span = doc.span_at("key").unwrap();

    {
        let mut edit = doc.edit();
        edit.replace_span(span.0, span.1, "dropped").unwrap();
    }
    assert_eq!(doc.source(), SOURCE);

    let mut edit = doc.edit();
    edit.replace_span(span.0, span.1, "aborted").unwrap();
    edit.abort();
    assert_eq!(doc.source(), SOURCE);
}

#[test]
fn overlaps_and_duplicate_offsets_are_refused_while_planning() {
    let mut doc = parse_document("abcdef\n").unwrap();
    let mut edit = doc.edit();
    edit.replace_span(1, 3, "BC").unwrap();

    assert!(edit.replace_span(2, 4, "CD").is_err());
    assert!(edit.replace_span(1, 1, "insert").is_err());
    assert_eq!(edit.len(), 1);
    edit.commit().unwrap();
    assert_eq!(doc.source(), "aBCdef\n");
}

#[test]
fn invalid_ranges_are_refused_before_commit() {
    let mut doc = parse_document("café: yes\n").unwrap();
    let source_len = doc.source().len();
    let mut edit = doc.edit();

    assert!(
        edit.replace_span(source_len + 1, source_len + 1, "x")
            .is_err()
    );
    assert!(edit.replace_span(4, 3, "x").is_err());
    assert!(edit.replace_span(4, 5, "x").is_err());
    assert!(edit.is_empty());
}

#[test]
fn a_batch_cannot_introduce_another_document() {
    const SOURCE: &str = "key: value\n";
    let mut doc = parse_document(SOURCE).unwrap();
    let end = doc.source().len();
    let mut edit = doc.edit();
    edit.replace_span(end, end, "---\nother: value\n").unwrap();

    assert!(edit.commit().is_err());
    assert_eq!(doc.source(), SOURCE);
}

#[test]
fn commit_uses_the_documents_parser_configuration() {
    const SOURCE: &str = "key: value\n";
    let config = ParserConfig::new().max_nodes(3);
    let mut doc = parse_document_with_config(SOURCE, &config).unwrap();
    let value = doc.span_at("key").unwrap();
    let mut edit = doc.edit();
    edit.replace_span(value.0, value.1, "[one, two]").unwrap();

    assert!(edit.commit().is_err());
    assert_eq!(doc.source(), SOURCE);
}

#[test]
fn empty_batch_is_a_noop() {
    let mut doc = parse_document("key: value\n").unwrap();
    assert_eq!(doc.last_repair_scope(), None);
    doc.edit().commit().unwrap();
    assert_eq!(doc.last_repair_scope(), None);
}
