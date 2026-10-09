// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A whitespace-only line whose spaces exceed a folded scalar's
//! indentation is a spaced line (YAML 1.2.2 §8.1.3): its extra spaces
//! are content and the breaks around it are not folded, exactly as a
//! literal scalar keeps it (yaml-test-suite DWX9). Found by fuzz_diff
//! (input `>\n\n\n  #\n    `): folded style dropped the line.

use noyalib::{Value, from_str};

const CASES: &[(&str, &str)] = &[
    (">\n  a\n    \n  b\n", "a\n  \nb\n"),
    (">\n  a\n    \n", "a\n  \n"),
    (">\n  a\n\n    \n", "a\n\n  \n"),
    (">\n  a\n    \n\n  b\n", "a\n  \n\nb\n"),
    (">-\n  a\n    \n  b\n", "a\n  \nb"),
    (">\n\n\n  #\n    ", "\n\n#\n  \n"),
    // A line of no more than the indentation stays an empty line.
    (">\n  a\n  \n  b\n", "a\nb\n"),
];

#[test]
fn folded_spaced_whitespace_lines_are_content() {
    for &(doc, want) in CASES {
        let v: Value = from_str(doc).unwrap_or_else(|e| panic!("{doc:?}: {e}"));
        assert_eq!(v.as_str(), Some(want), "{doc:?}");
    }
}

#[test]
fn literal_style_is_unchanged() {
    let v: Value = from_str("|\n  a\n    \n  b\n").unwrap();
    assert_eq!(v.as_str(), Some("a\n  \nb\n"));
}
