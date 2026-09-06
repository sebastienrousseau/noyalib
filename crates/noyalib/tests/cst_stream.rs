// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Multi-document splitting for `parse_stream`.

use noyalib::cst::{Document, parse_stream, parse_stream_with_config};
use noyalib::{Error, ParserConfig, Value, load_all_as};

fn join_sources(docs: &[Document]) -> String {
    docs.iter().map(Document::source).collect()
}

#[test]
fn single_implicit_doc() {
    let src = "foo: 1\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].source(), src);
}

#[test]
fn single_explicit_doc() {
    let src = "---\nfoo: 1\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].source(), src);
}

#[test]
fn two_explicit_docs_no_end_marker() {
    let src = "---\nfoo: 1\n---\nbar: 2\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].source(), "---\nfoo: 1\n");
    assert_eq!(docs[1].source(), "---\nbar: 2\n");
    assert_eq!(join_sources(&docs), src);
    assert_eq!(docs[0].as_value()["foo"].as_i64(), Some(1));
    assert_eq!(docs[1].as_value()["bar"].as_i64(), Some(2));
}

#[test]
fn two_docs_with_explicit_end() {
    let src = "---\nfoo: 1\n...\n---\nbar: 2\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].source(), "---\nfoo: 1\n...\n");
    assert_eq!(docs[1].source(), "---\nbar: 2\n");
    assert_eq!(join_sources(&docs), src);
}

#[test]
fn bare_then_explicit() {
    let src = "foo: 1\n---\nbar: 2\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].source(), "foo: 1\n");
    assert_eq!(docs[1].source(), "---\nbar: 2\n");
    assert_eq!(join_sources(&docs), src);
}

#[test]
fn explicit_then_bare_via_end_marker() {
    let src = "---\nfoo: 1\n...\nbar: 2\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].source(), "---\nfoo: 1\n...\n");
    assert_eq!(docs[1].source(), "bar: 2\n");
    assert_eq!(join_sources(&docs), src);
}

#[test]
fn three_docs() {
    let src = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 3);
    assert_eq!(join_sources(&docs), src);
    assert_eq!(docs[0].as_value()["a"].as_i64(), Some(1));
    assert_eq!(docs[1].as_value()["b"].as_i64(), Some(2));
    assert_eq!(docs[2].as_value()["c"].as_i64(), Some(3));
}

#[test]
fn comment_between_docs_attaches_to_next() {
    let src = "---\nfoo: 1\n# between\n---\nbar: 2\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 2);
    // Comment is *before* the second `---`, so it stays with doc 0.
    assert_eq!(docs[0].source(), "---\nfoo: 1\n# between\n");
    assert_eq!(docs[1].source(), "---\nbar: 2\n");
    assert_eq!(join_sources(&docs), src);
}

#[test]
fn comment_after_end_marker_attaches_to_next() {
    let src = "---\nfoo: 1\n...\n# trailer\n---\nbar: 2\n";
    let docs = parse_stream(src).unwrap();
    assert_eq!(docs.len(), 2);
    // After `...`, trivia goes into the next document's prologue.
    assert_eq!(docs[0].source(), "---\nfoo: 1\n...\n");
    assert_eq!(docs[1].source(), "# trailer\n---\nbar: 2\n");
    assert_eq!(join_sources(&docs), src);
}

#[test]
fn each_doc_independently_editable() {
    let src = "---\nversion: 0.1.0\n---\nversion: 0.2.0\n";
    let mut docs = parse_stream(src).unwrap();
    docs[0].set("version", "0.1.1").unwrap();
    docs[1].set("version", "0.2.1").unwrap();
    assert_eq!(docs[0].source(), "---\nversion: 0.1.1\n");
    assert_eq!(docs[1].source(), "---\nversion: 0.2.1\n");
}

// Error locations count from the start of the stream (#407).

#[test]
fn error_in_a_later_document_is_located_in_the_stream() {
    let src = "a: 1\n---\nb: 2\n---\nc: *nope\n";
    let err = parse_stream(src).unwrap_err();
    let loc = err.location().expect("located");
    assert_eq!((loc.index(), loc.line(), loc.column()), (21, 5, 4));
    assert!(src[loc.index()..].starts_with("*nope"));
    // The typed loader reports the same position for the same bytes.
    let typed = load_all_as::<Value>(src).unwrap_err();
    assert_eq!(typed.location(), Some(loc));
    // Both CST entry points agree.
    let cfg = ParserConfig::new();
    let with_config = parse_stream_with_config(src, &cfg).unwrap_err();
    assert_eq!(with_config.location(), Some(loc));
    assert_eq!(err.to_string(), "unknown anchor: nope at line 5, column 4");
}

#[test]
fn error_in_the_first_document_is_unchanged() {
    let src = "a: *nope\n---\nb: 1\n";
    let loc = parse_stream(src).unwrap_err().location().expect("located");
    assert_eq!((loc.index(), loc.line(), loc.column()), (3, 1, 4));
}

#[test]
fn key_collision_in_a_later_document_is_located_in_the_stream() {
    let src = "a: 1\n---\nb: 2\n---\n1: x\n\"1\": y\n";
    let err = parse_stream(src).unwrap_err();
    assert!(matches!(err, Error::KeyCollisionAt { .. }), "{err:?}");
    let loc = err.location().expect("located");
    assert_eq!((loc.index(), loc.line(), loc.column()), (23, 6, 1));
    let typed = load_all_as::<Value>(src).unwrap_err();
    assert_eq!(typed.location(), Some(loc));
}

#[test]
fn similar_anchor_suggestion_is_located_in_the_stream() {
    let src = "a: 1\n---\nb: &nope 1\nc: *nop\n";
    let err = parse_stream(src).unwrap_err();
    let Error::UnknownAnchorAt {
        location,
        suggestion,
        ..
    } = err
    else {
        panic!("{err:?}");
    };
    assert_eq!((location.line(), location.column()), (4, 4));
    let (name, at) = suggestion.expect("a similar anchor is suggested");
    assert_eq!(name, "nope");
    assert_eq!((at.index(), at.line(), at.column()), (12, 3, 4));
    assert!(src[at.index()..].starts_with("&nope"));
}

#[test]
fn rendered_error_shows_the_failing_line_of_the_stream() {
    let src = "a: 1\n---\nb: 2\n---\nc: *nope\n";
    let rendered = parse_stream(src).unwrap_err().render(src);
    assert!(rendered.contains("5 | c: *nope"), "{rendered}");
}

#[test]
fn directive_after_an_unclosed_document_is_located_in_the_stream() {
    // YAML 1.2.2 §9.1.4: a directive needs the previous document
    // closed by `...`. The scanner rejects the stream before any
    // document is parsed; the error still carries the directive's
    // position, the same one the typed loader reports.
    let src = "a: 1\n%YAML 1.2\n---\nb: 2\n";
    let err = parse_stream(src).unwrap_err();
    let loc = err.location().expect("located");
    assert_eq!((loc.index(), loc.line(), loc.column()), (5, 2, 1));
    assert!(src[loc.index()..].starts_with("%YAML"));
    let typed = load_all_as::<Value>(src).unwrap_err();
    assert_eq!(typed.location(), Some(loc));
    assert!(
        err.to_string()
            .contains("directive must be preceded by '...'"),
        "{err}"
    );
}
