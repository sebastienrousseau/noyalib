// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Multi-document splitting for `parse_stream`.

use noyalib::cst::{Document, parse_stream};

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
