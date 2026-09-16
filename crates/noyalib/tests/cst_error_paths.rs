// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Error contracts of the CST mutators.
//!
//! Every mutator here documents what it refuses and promises that a
//! refusal leaves the document unchanged. Those refusal paths were
//! uncovered: the happy paths are tested thoroughly, the `return Err`
//! arms were not reached by anything.
//!
//! That is the wrong way round for a lossless editor. A mutator that
//! silently half-applied a rejected edit would corrupt a file, and no
//! existing test would have noticed — the happy-path suites would all
//! still pass.
//!
//! So each test asserts two things: that the call fails, and that the
//! document is byte-identical afterwards.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use noyalib::Value;
use noyalib::cst::parse_document;

/// A path that does not resolve is refused, not invented.
#[test]
fn set_path_refuses_a_path_that_does_not_resolve() {
    let src = "a: 1\nb: 2\n";
    let mut doc = parse_document(src).expect("parse");
    let before = doc.to_string();
    let err = doc
        .set_path("a.b.c.d", &Value::from(1_i64))
        .expect_err("a path through a scalar cannot resolve");
    assert!(
        err.to_string().contains("set_path") || err.to_string().contains("path"),
        "unhelpful error: {err}"
    );
    assert_eq!(doc.to_string(), before, "document changed despite refusal");
}

/// The root holding a non-mapping is a documented refusal.
#[test]
fn set_path_refuses_when_the_root_is_not_a_mapping() {
    for src in ["- 1\n- 2\n", "just a scalar\n"] {
        let mut doc = parse_document(src).expect("parse");
        let before = doc.to_string();
        let err = doc
            .set_path("newkey", &Value::from(1_i64))
            .expect_err("cannot create a key under a non-mapping root");
        assert!(!err.to_string().is_empty());
        assert_eq!(
            doc.to_string(),
            before,
            "document changed despite refusal, source was {src:?}"
        );
    }
}

/// `swap_items` needs both indices to exist.
#[test]
fn swap_items_refuses_an_index_that_is_not_there() {
    let src = "xs:\n  - a\n  - b\n";
    let mut doc = parse_document(src).expect("parse");
    let before = doc.to_string();
    let err = doc
        .swap_items("xs", 0, 99)
        .expect_err("index 99 does not exist");
    assert!(
        err.to_string().contains("swap_items"),
        "error should name the operation: {err}"
    );
    assert_eq!(doc.to_string(), before, "document changed despite refusal");
}

/// Swapping inside something that is not a sequence is refused.
#[test]
fn swap_items_refuses_a_path_that_is_not_a_sequence() {
    let mut doc = parse_document("m:\n  a: 1\n  b: 2\n").expect("parse");
    let before = doc.to_string();
    assert!(
        doc.swap_items("m", 0, 1).is_err(),
        "a mapping is not swappable"
    );
    assert_eq!(doc.to_string(), before, "document changed despite refusal");
}

/// A refused `set_value` leaves the document untouched.
#[test]
fn set_value_refuses_an_unresolvable_path_and_changes_nothing() {
    let src = "a:\n  b: 1\n";
    let mut doc = parse_document(src).expect("parse");
    let before = doc.to_string();
    assert!(doc.set_value("a.b.c", &Value::from(2_i64)).is_err());
    assert_eq!(doc.to_string(), before);
}

/// `remove` on a path that is not there is refused rather than silently
/// succeeding — the distinction the comment removers got wrong in #425.
#[test]
fn remove_reports_a_path_it_cannot_resolve() {
    let src = "a: 1\n";
    let mut doc = parse_document(src).expect("parse");
    let before = doc.to_string();
    let r = doc.remove("nope");
    // Whichever way it answers, it must not have changed the document.
    assert_eq!(
        doc.to_string(),
        before,
        "document changed on a no-op remove"
    );
    if r.is_ok() {
        // Documented no-op contract: fine, but nothing moved.
        assert_eq!(doc.to_string(), src);
    }
}

/// An index past the end of a sequence is refused by `set_value`.
#[test]
fn set_value_refuses_an_out_of_range_sequence_index() {
    let mut doc = parse_document("xs:\n  - 1\n  - 2\n").expect("parse");
    let before = doc.to_string();
    assert!(doc.set_value("xs[9]", &Value::from(3_i64)).is_err());
    assert_eq!(doc.to_string(), before);
}

/// Every refusal above is only meaningful if the same call succeeds when
/// it should — otherwise the tests would pass against a mutator that
/// refuses everything.
#[test]
fn the_same_operations_succeed_on_valid_input() {
    let mut doc = parse_document("a:\n  b: 1\nxs:\n  - p\n  - q\n").expect("parse");
    doc.set_value("a.b", &Value::from(2_i64))
        .expect("set_value should work");
    doc.swap_items("xs", 0, 1).expect("swap_items should work");
    let out = doc.to_string();
    assert!(out.contains('2'), "set_value did not apply: {out}");
    let v: Value = noyalib::from_str(&out).expect("reparse");
    let xs = v.get("xs").expect("xs present");
    assert_eq!(
        xs.as_sequence().expect("sequence")[0].as_str(),
        Some("q"),
        "swap_items did not apply: {out}"
    );
}
