//! Regression: `remove` silently deleted more than it was asked to.
//!
//! `remove` had an unguarded fast path for single-line entries — the
//! typed oracle only ran for multi-line ones. A flow collection puts an
//! entry, its siblings and its parent on the same line, so "delete the
//! line" removed the whole parent while returning `Ok`:
//!
//!     a: {x: 1, y: 2}   remove("a.x")   ->   ""      (whole document)
//!
//! v0.0.21 made the oracle run on every path, turning that silent
//! destruction into a refusal. #221 sub-ask 4 then implemented the edit
//! properly, so these now *succeed* — narrowly.
//!
//! The mechanism changed; the property under test did not. Each case
//! still asserts that the parent, the siblings and the rest of the
//! document survive. If the original bug ever returns, the document
//! collapses to `""` and every one of these fails just as loudly as
//! when they asserted a refusal.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::cst::parse_document;

/// Remove `path` and require the result to be exactly `want` — never the
/// empty document, and never a document missing the entry's parent.
#[track_caller]
fn removes_exactly(src: &str, path: &str, want: &str) {
    let mut doc = parse_document(src).expect("parse");
    doc.remove(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    assert_eq!(doc.source(), want, "removing {path} from {src:?}");
    assert!(
        !doc.source().is_empty(),
        "the original bug: removing {path} emptied the document"
    );
    let _reparsed = parse_document(doc.source()).expect("result re-parses");
}

#[test]
fn flow_mapping_entry_does_not_delete_the_document() {
    removes_exactly("a: {x: 1, y: 2}\n", "a.x", "a: {y: 2}\n");
}

#[test]
fn flow_mapping_entry_does_not_delete_its_parent() {
    removes_exactly("keep: 0\na: {x: 1, y: 2}\n", "a.x", "keep: 0\na: {y: 2}\n");
    removes_exactly("a: {x: 1, y: 2}\nkeep: 9\n", "a.y", "a: {x: 1}\nkeep: 9\n");
}

#[test]
fn flow_sequence_item_is_removed_not_the_sequence() {
    removes_exactly("a: [1, 2, 3]\n", "a[1]", "a: [1, 3]\n");
}

#[test]
fn sole_entry_becomes_an_empty_collection_rather_than_nothing() {
    // The one case where "the document is now empty" would be the
    // *correct* value — but it must be an empty **mapping**, not an
    // empty document that re-parses as null.
    removes_exactly("only: 1\n", "only", "{}\n");
}

#[test]
fn the_cases_that_worked_still_work() {
    for (src, path, want) in [
        ("a: 1\nb: 2\n", "a", "b: 2\n"),
        ("a: |\n  one\n  two\nb: 2\n", "a", "b: 2\n"),
        ("a:\n  x: 1\n  y: 2\nb: 2\n", "a", "b: 2\n"),
        ("a:\n  - 1\n  - 2\n", "a[0]", "a:\n  - 2\n"),
    ] {
        let mut doc = parse_document(src).expect("parse");
        doc.remove(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(doc.source(), want, "removing {path} from {src:?}");
    }
}

#[test]
fn a_flow_remove_changes_the_value_by_exactly_one_key() {
    // The typed-value counterpart: the oracle inside `remove` already
    // enforces this, but assert it from outside so a change to the
    // oracle cannot quietly widen what an edit is allowed to touch.
    let src = "a: {x: 1, y: 2}\nb: 3\n";
    let mut doc = parse_document(src).expect("parse");
    doc.remove("a.x").expect("remove");

    let after: noyalib::Value = noyalib::from_str(doc.source()).expect("after");
    let expected: noyalib::Value = noyalib::from_str("a: {y: 2}\nb: 3\n").expect("expected");
    assert_eq!(after, expected);
}

#[test]
fn a_genuinely_refused_remove_still_leaves_the_value_intact() {
    // Refusals have not gone away — they moved to paths that do not
    // resolve. The document must survive those untouched.
    let src = "a: {x: 1, y: 2}\n";
    let mut doc = parse_document(src).expect("parse");
    let before: noyalib::Value = noyalib::from_str(src).expect("before");
    assert!(doc.remove("a.nope").is_err());
    assert_eq!(doc.source(), src, "source untouched after a refusal");
    let after: noyalib::Value = noyalib::from_str(doc.source()).expect("after");
    assert_eq!(before, after, "a refused remove must not change the value");
}
