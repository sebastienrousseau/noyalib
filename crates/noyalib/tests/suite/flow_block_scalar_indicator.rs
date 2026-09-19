// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A block scalar indicator inside a flow collection is an error, not
//! the start of a plain scalar (issue #331).
//!
//! `|` and `>` are indicators, so a plain scalar cannot begin with
//! either, and block scalars exist only in block context. The scanner
//! used to fall through to the plain-scalar fetcher inside `[…]` /
//! `{…}`, so `{a: |-\n  x\n  y}` loaded as `{a: "|- x y"}` -- the
//! indicator and the folded lines glued into one string -- where every
//! other YAML 1.2 implementation reports an error.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

const REJECTED: &[&str] = &[
    "m: {a: |-\n  x\n  y}\n",
    "s: [|-\n  x\n  y, b]\n",
    "m: {a: |\n  x\n}\n",
    "m: {a: >}\n",
    "s: [>]\n",
    "s: [a, |\n  x]\n",
    "{a: {b: |-\n  x}}\n",
];

#[test]
fn a_block_scalar_indicator_inside_a_flow_collection_is_rejected() {
    for src in REJECTED {
        let err = noyalib::from_str::<Value>(src)
            .expect_err(&format!("{src:?} must not load"))
            .to_string();
        assert!(
            err.contains("flow collection"),
            "{src:?}: diagnosis should name the flow context, got: {err}"
        );
    }
}

#[test]
fn the_cst_parser_rejects_the_same_inputs() {
    for src in REJECTED {
        assert!(
            parse_document(src).is_err(),
            "{src:?} must not build a document"
        );
    }
}

#[test]
fn indicator_bytes_inside_a_plain_scalar_are_still_content() {
    // Only the *first* character of a plain scalar is restricted; `|`
    // and `>` are ordinary characters after it, in flow context too.
    let cases: &[(&str, &str, &str)] = &[
        ("s: [a |b]\n", "s", "a |b"),
        ("s: [x|y]\n", "s", "x|y"),
        ("s: [a > b]\n", "s", "a > b"),
    ];
    for (src, key, want) in cases {
        let v: Value = noyalib::from_str(src).unwrap();
        assert_eq!(v[*key][0].as_str(), Some(*want), "{src:?}");
    }
    let v: Value = noyalib::from_str("m: {k: a>b, l: c|d}\n").unwrap();
    assert_eq!(v["m"]["k"].as_str(), Some("a>b"));
    assert_eq!(v["m"]["l"].as_str(), Some("c|d"));
}

#[test]
fn block_scalars_in_block_context_are_unaffected() {
    let v: Value =
        noyalib::from_str("a: |\n  x\n  y\nb: >-\n  p\n  q\nc:\n  - |-\n    z\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("x\ny\n"));
    assert_eq!(v["b"].as_str(), Some("p q"));
    assert_eq!(v["c"][0].as_str(), Some("z"));
    // A block scalar *containing* a flow collection is still a block
    // scalar: the indicator is read in block context.
    let v: Value = noyalib::from_str("a: |\n  [not, flow]\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("[not, flow]\n"));
}

#[test]
fn a_quoted_indicator_inside_a_flow_collection_is_a_string() {
    let v: Value = noyalib::from_str("s: ['|', \">-\", '|-']\n").unwrap();
    assert_eq!(v["s"][0].as_str(), Some("|"));
    assert_eq!(v["s"][1].as_str(), Some(">-"));
    assert_eq!(v["s"][2].as_str(), Some("|-"));
}
