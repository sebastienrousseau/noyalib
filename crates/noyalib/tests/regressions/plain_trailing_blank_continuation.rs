// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A plain scalar continues on the next line even when its line ends
//! with spaces or tabs: trailing white space before a break is
//! stripped by folding (YAML 1.2.2 §6.5, §7.3.3), it does not end the
//! scalar. Found by the serde_yaml compat fuzzer (input `m \n\n`\n`):
//! the single-line fast path read the trailing blank as the end of the
//! scalar, so `m \nx` failed with "stray content after document".

use noyalib::{Value, from_str};

const CASES: &[(&str, &str)] = &[
    ("m \nx\n", "m x"),
    ("m \t\nx\n", "m x"),
    ("m \n\nx\n", "m\nx"),
    ("m \n\n`\n", "m\n`"),
    ("m  \n  x  \n  y\n", "m x y"),
];

#[test]
fn trailing_blanks_do_not_end_a_multi_line_plain_scalar() {
    for &(doc, want) in CASES {
        let v: Value = from_str(doc).unwrap_or_else(|e| panic!("{doc:?}: {e}"));
        assert_eq!(v.as_str(), Some(want), "{doc:?}");
    }
    let v: Value = from_str("a: m \n\n  x\nb: 1\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("m\nx"));
    let v: Value = from_str("- m \n  x\n- y\n").unwrap();
    assert_eq!(v[0].as_str(), Some("m x"));
}

#[test]
fn single_line_scalars_are_unchanged() {
    let v: Value = from_str("a: m \nb: n\t\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("m"));
    assert_eq!(v["b"].as_str(), Some("n"));
    let v: Value = from_str("a: m  # note\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("m"));
    let v: Value = from_str("m ").unwrap();
    assert_eq!(v.as_str(), Some("m"));
}
