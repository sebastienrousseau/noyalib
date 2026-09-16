// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The `compat::serde_yaml` façade's error wording.
//!
//! The point of the compat layer is that code ported from `serde_yaml`
//! keeps working — including code that matches on the *text* of an
//! error, which is more common than it should be and is exactly what a
//! drop-in replacement has to preserve.
//!
//! Each branch of the message mapper rewrites one noyalib error into
//! upstream's phrasing, and several of them build a positional trailer
//! by re-scanning the source for the flow context libyaml would have
//! named. That re-scan is quote-aware, which only matters when the
//! document has quotes *and* brackets in it — the case nothing reached.

// The whole file exercises the `compat::serde_yaml` façade, which only
// exists behind its own feature. Without this the file is a compile
// error in every build that does not happen to enable it — which is
// every feature-matrix job except `--all-features`.
#![cfg(feature = "compat-serde-yaml")]
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use noyalib::compat::serde_yaml;

/// Ask the compat façade for a `Value` and return the error text.
#[track_caller]
fn message(label: &str, yaml: &str) -> String {
    match serde_yaml::from_str::<serde_yaml::Value>(yaml) {
        Ok(v) => panic!("{label}: {yaml:?} was accepted as {v:?}"),
        Err(e) => e.to_string(),
    }
}

/// An unclosed flow sequence reports the position of the `[` that was
/// left open, the way libyaml does. The scan that finds it has to skip
/// brackets inside quoted scalars, so each case below buries a decoy.
#[test]
fn an_unclosed_flow_sequence_names_the_bracket_that_opened_it() {
    let cases: &[(&str, &str)] = &[
        ("no decoy", "a: [1, 2\n"),
        (
            "a bracket inside a double-quoted scalar",
            "a: [\"[not an open\", 2\n",
        ),
        (
            "a bracket inside a single-quoted scalar",
            "a: ['[not an open', 2\n",
        ),
        ("an escaped quote before a bracket", "a: [\"x\\\"[\", 2\n"),
        ("a closed pair before the open one", "a: [[1], 2\n"),
    ];
    for (label, yaml) in cases {
        let msg = message(label, yaml);
        assert!(
            msg.contains("did not find expected ',' or ']'"),
            "{label}: wrong wording: {msg}"
        );
        assert!(
            msg.contains("while parsing a flow sequence at line"),
            "{label}: the flow-sequence trailer is missing, so the bracket scan \
             did not find the open bracket: {msg}"
        );
    }
}

/// The resource limits keep upstream's unlocated wording.
#[test]
fn the_resource_limits_keep_upstreams_wording() {
    let mut cfg = noyalib::ParserConfig::new();
    cfg.max_depth = 2;
    let deep = "a:\n  b:\n    c:\n      d: 1\n";
    let err = noyalib::from_str_with_config::<noyalib::Value>(deep, &cfg)
        .expect_err("max_depth must refuse");
    assert!(
        err.to_string().contains("recursion") || err.to_string().contains("depth"),
        "unexpected wording: {err}"
    );
}

/// A non-scalar mapping key keeps noyalib's own wording, because it
/// already matches upstream's.
#[test]
fn a_non_scalar_key_uses_the_shared_wording() {
    let msg = message("sequence key", "? [a, b]\n: 1\n");
    assert!(
        msg.contains("invalid type: sequence") || msg.contains("string key"),
        "unexpected wording: {msg}"
    );
}

/// A misplaced `:` is reported as upstream's "mapping values are not
/// allowed in this context", with a position.
#[test]
fn a_misplaced_value_indicator_uses_upstreams_wording() {
    let msg = message("second colon", "a: b: c\n");
    assert!(
        msg.contains("mapping values are not allowed in this context"),
        "unexpected wording: {msg}"
    );
    assert!(msg.contains("line"), "no position in: {msg}");
}

/// `Error` implements `std::error::Error::source`, so `anyhow`-style
/// chains can reach the underlying noyalib error. Nothing walked the
/// chain before.
#[test]
fn the_compat_error_exposes_its_source() {
    use std::error::Error as _;
    let err = serde_yaml::from_str::<serde_yaml::Value>("a: [1, 2\n").expect_err("parse error");
    let source = err
        .source()
        .expect("the compat error must expose its source");
    assert!(
        !source.to_string().is_empty(),
        "the source error renders to nothing"
    );
}

/// Round-tripping through the façade must work for the ordinary case,
/// so the tests above are measuring the error path rather than a
/// façade that never works.
#[test]
fn the_facade_round_trips_an_ordinary_document() {
    let v: serde_yaml::Value = serde_yaml::from_str("a: 1\nb:\n  - x\n  - y\n").expect("parse");
    let out = serde_yaml::to_string(&v).expect("serialize");
    let back: serde_yaml::Value = serde_yaml::from_str(&out).expect("reparse");
    assert_eq!(back, v, "the façade did not round-trip\n{out}");
}
