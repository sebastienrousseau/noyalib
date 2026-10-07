// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The CST stream, document and format entry points apply the same
//! document-count and length limits as the typed loaders.

use noyalib::cst::{
    FormatConfig, format, format_with_parser_config, parse_document, parse_document_with_config,
    parse_stream, parse_stream_with_config,
};
use noyalib::{BudgetBreach, Error, ParserConfig};

fn stream_of(n: usize) -> String {
    (0..n).map(|i| format!("--- {i}\n")).collect()
}

fn is_max_documents(r: &noyalib::Result<impl Sized>) -> bool {
    matches!(r, Err(Error::Budget(BudgetBreach::MaxDocuments { .. })))
}

#[test]
fn parse_stream_with_config_enforces_max_documents() {
    let cfg = ParserConfig::new().max_documents(2);
    assert_eq!(
        parse_stream_with_config(&stream_of(2), &cfg).unwrap().len(),
        2
    );
    assert!(is_max_documents(&parse_stream_with_config(
        &stream_of(3),
        &cfg
    )));
    // The typed loader agrees.
    assert!(noyalib::load_all_with_config(&stream_of(3), &cfg).is_err());
}

#[test]
fn parse_stream_enforces_the_default_max_documents() {
    let limit = ParserConfig::default().max_documents;
    assert!(parse_stream(&stream_of(limit)).is_ok());
    assert!(is_max_documents(&parse_stream(&stream_of(limit + 1))));
}

#[test]
fn strict_profile_caps_the_cst_stream() {
    let cfg = ParserConfig::strict();
    let over = stream_of(cfg.max_documents + 1);
    assert!(is_max_documents(&parse_stream_with_config(&over, &cfg)));
}

#[test]
fn parse_document_with_config_enforces_max_document_length() {
    let src = "key: a value longer than sixteen bytes\n";
    let cfg = ParserConfig::new().max_document_length(16);
    assert!(noyalib::from_str_with_config::<noyalib::Value>(src, &cfg).is_err());
    assert!(parse_document_with_config(src, &cfg).is_err());
    assert!(parse_stream_with_config(src, &cfg).is_err());
    assert!(parse_document(src).is_ok());
}

#[test]
fn an_edit_cannot_grow_a_document_past_max_document_length() {
    let cfg = ParserConfig::new().max_document_length(32);
    let mut doc = parse_document_with_config("a: 1\nb:\n  c: 2\n", &cfg).unwrap();
    let before = doc.source().to_owned();
    assert!(doc.set("b.c", &"x".repeat(64)).is_err());
    assert_eq!(doc.source(), before);
    let (s, e) = doc.span_at("a").unwrap();
    assert!(doc.replace_span(s, e, &"y".repeat(64)).is_err());
    assert_eq!(doc.source(), before);
}

#[test]
fn parse_stream_with_config_enforces_max_stream_bytes() {
    let src = stream_of(4);
    let cfg = ParserConfig::new().max_stream_bytes(src.len() - 1);
    assert!(parse_stream_with_config(&src, &cfg).is_err());
    let cfg = ParserConfig::new().max_stream_bytes(src.len());
    assert_eq!(parse_stream_with_config(&src, &cfg).unwrap().len(), 4);
}

#[test]
fn format_with_parser_config_applies_the_limits() {
    let fmt = FormatConfig::default();
    let cfg = ParserConfig::new().max_documents(2);
    assert!(is_max_documents(&format_with_parser_config(
        &stream_of(3),
        &fmt,
        &cfg
    )));
    assert_eq!(
        format_with_parser_config("a:   1\n", &fmt, &cfg).unwrap(),
        "a: 1\n"
    );
    let cfg = ParserConfig::new().max_document_length(4);
    assert!(format_with_parser_config("a: 12345\n", &fmt, &cfg).is_err());
}

#[test]
fn format_keeps_the_default_limits() {
    let limit = ParserConfig::default().max_documents;
    assert!(is_max_documents(&format(&stream_of(limit + 1))));
    assert_eq!(format("a:   1\n").unwrap(), "a: 1\n");
}
