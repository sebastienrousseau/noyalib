//! A fragment passed to `set` must not reach outside its path.
//!
//! `set` splices verbatim by contract, and that is useful — a fragment
//! may legitimately turn a scalar into a mapping. What it must not do is
//! give the document entries the caller never asked for:
//!
//!     set("a", "v\nc: 3")  on  "a: 1\nb: 2\n"
//!
//! previously produced a document with a new key `c`, and returned
//! `Ok`. The re-parse guard cannot see it: the result is valid YAML,
//! just not the document anyone asked for — the same "valid but
//! misinterpreted" class that #221 sub-ask 5 is about.
//!
//! The oracle restores the original value at the edited path and
//! requires the result to equal the original document.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::Value;
use noyalib::cst::parse_document;

const SRC: &str = "a: 1\nb: 2\n";

#[track_caller]
fn refused(fragment: &str) {
    let mut doc = parse_document(SRC).expect("parse");
    let err = doc
        .set("a", fragment)
        .expect_err("a fragment escaping its path must be refused");
    assert_eq!(
        doc.source(),
        SRC,
        "the document must be untouched after a refusal ({err})"
    );
}

#[track_caller]
fn accepted(fragment: &str, expect_source: &str) {
    let mut doc = parse_document(SRC).expect("parse");
    doc.set("a", fragment)
        .unwrap_or_else(|e| panic!("{fragment:?} should splice: {e}"));
    assert_eq!(doc.source(), expect_source);
}

#[test]
fn a_newline_fragment_cannot_add_sibling_keys() {
    refused("v\nc: 3");
}

#[test]
fn a_multiline_fragment_cannot_smuggle_a_mapping() {
    refused("v\nc:\n  d: 4");
}

#[test]
fn restructuring_the_target_itself_is_still_allowed() {
    // The contract is verbatim splicing; changing what lives *at* the
    // path is the point of the method.
    accepted("{x: 1}", "a: {x: 1}\nb: 2\n");
    accepted("[1, 2]", "a: [1, 2]\nb: 2\n");
}

#[test]
fn ordinary_scalars_still_splice() {
    accepted("hello", "a: hello\nb: 2\n");
    accepted("true", "a: true\nb: 2\n");
    accepted("\"x: y\"", "a: \"x: y\"\nb: 2\n");
}

#[test]
fn siblings_survive_an_accepted_edit() {
    let mut doc = parse_document(SRC).expect("parse");
    doc.set("a", "hello").expect("set");
    let v: Value = noyalib::from_str(doc.source()).expect("reparse");
    let Value::Mapping(m) = &v else {
        panic!("expected mapping")
    };
    assert_eq!(
        m.get("b"),
        Some(&Value::Number(2.into())),
        "b must be untouched"
    );
    assert_eq!(m.len(), 2, "no entries added or lost");
}

#[test]
fn set_value_is_the_safe_route_for_values() {
    // Every fragment that `set` treats as YAML, `set_value` writes as
    // the string it is — this is what the docs now point callers at.
    for s in ["v\nc: 3", "true", "", "v # not a comment", "x: y", "- item"] {
        let mut doc = parse_document(SRC).expect("parse");
        doc.set_value("a", &Value::String(s.to_owned()))
            .unwrap_or_else(|e| panic!("set_value({s:?}): {e}"));
        let v: Value = noyalib::from_str(doc.source()).expect("reparse");
        let Value::Mapping(m) = &v else {
            panic!("expected mapping")
        };
        assert_eq!(
            m.get("a"),
            Some(&Value::String(s.to_owned())),
            "set_value must round-trip {s:?} exactly"
        );
        assert_eq!(m.len(), 2, "set_value({s:?}) changed the entry count");
    }
}
