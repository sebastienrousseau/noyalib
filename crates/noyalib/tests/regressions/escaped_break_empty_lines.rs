// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! In a double-quoted scalar, an escaped line break (`\` at the end of
//! a line) is followed by any number of empty lines, and each of them
//! is a line feed (YAML 1.2.2 §7.3.1, s-double-escaped: `\`
//! b-non-content l-empty*). Found by fuzz_diff: the empty lines were
//! folded as after an unescaped break, so `"a\` + empty line + `b"`
//! read "a b" where libyaml and the spec read "a\nb".

use noyalib::{Value, from_str};

const CASES: &[(&str, &str)] = &[
    ("\"a\\\n\nb\"", "a\nb"),
    ("\"a\\\n\n\nb\"", "a\n\nb"),
    ("\"a\\\r\rb\"", "a\nb"),
    ("\"a\\\n  \n  b\"", "a\nb"),
    ("\"\n\\\n\n}\"", " \n}"),
    ("\"a\\\n\n\"", "a\n"),
    // Unchanged: no empty line after the escape, and plain folding.
    ("\"a\\\nb\"", "ab"),
    ("\"a\\\n  b\"", "ab"),
    ("\"a\\\n\"", "a"),
    ("\"a\nb\"", "a b"),
    ("\"a\n\nb\"", "a\nb"),
    ("\"a \\\n b\"", "a b"),
];

#[test]
fn empty_lines_after_an_escaped_break_are_line_feeds() {
    for &(doc, want) in CASES {
        let v: Value = from_str(doc).unwrap_or_else(|e| panic!("{doc:?}: {e}"));
        assert_eq!(v.as_str(), Some(want), "{doc:?}");
    }
}
