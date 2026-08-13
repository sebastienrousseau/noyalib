//! Regression: `remove` silently deleted more than it was asked to.
//!
//! `remove` had an unguarded fast path for single-line entries — the
//! typed oracle only ran for multi-line ones. A flow collection puts an
//! entry, its siblings and its parent on the same line, so "delete the
//! line" removed the whole parent while returning `Ok`:
//!
//!     a: {x: 1, y: 2}   remove("a.x")   ->   ""      (whole document)
//!
//! The oracle now runs on every path, so these are refused with the
//! source left intact rather than silently destroying data.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::cst::parse_document;

#[track_caller]
fn refuses_and_preserves(src: &str, path: &str) {
    let mut doc = parse_document(src).expect("parse");
    let err = doc.remove(path).unwrap_err();
    assert_eq!(
        doc.source(),
        src,
        "source must be untouched after a refused remove ({err})"
    );
}

#[test]
fn flow_mapping_entry_does_not_delete_the_document() {
    refuses_and_preserves("a: {x: 1, y: 2}\n", "a.x");
}

#[test]
fn flow_mapping_entry_does_not_delete_its_parent() {
    refuses_and_preserves("keep: 0\na: {x: 1, y: 2}\n", "a.x");
    refuses_and_preserves("a: {x: 1, y: 2}\nkeep: 9\n", "a.y");
}

#[test]
fn flow_sequence_item_is_refused() {
    refuses_and_preserves("a: [1, 2, 3]\n", "a[1]");
}

#[test]
fn sole_entry_is_still_refused() {
    refuses_and_preserves("only: 1\n", "only");
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
fn a_refused_remove_leaves_the_value_intact() {
    let src = "a: {x: 1, y: 2}\n";
    let mut doc = parse_document(src).expect("parse");
    let before: noyalib::Value = noyalib::from_str(src).expect("before");
    let _ = doc.remove("a.x");
    let after: noyalib::Value = noyalib::from_str(doc.source()).expect("after");
    assert_eq!(before, after, "a refused remove must not change the value");
}
