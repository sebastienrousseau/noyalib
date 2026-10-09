// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The CST treats only space, tab and the line breaks as white space
//! (YAML 1.2.2 §5.4, §5.5). NEL (U+0085), NBSP and U+3000 are content,
//! though Rust's `str::trim` strips them. Found by
//! fuzz_cst_format_roundtrip (input U+0085): `cst::format` turned the
//! scalar "\u{85}" into an empty document.

use noyalib::{Value, cst, from_str};

const DOCS: &[&str] = &[
    "\u{85}",
    "a: \u{85}\n",
    "a: x\u{a0}\n",
    "a: \u{3000}x\n",
    "- \u{85}x\n",
    "\u{85}: 1\n",
];

#[test]
fn format_keeps_unicode_space_content() {
    for doc in DOCS {
        let before: Value = from_str(doc).unwrap();
        let formatted = cst::format(doc).unwrap_or_else(|e| panic!("{doc:?}: {e}"));
        let after: Value = from_str(&formatted).unwrap();
        assert_eq!(after, before, "{doc:?} formatted as {formatted:?}");
    }
}

#[test]
fn format_keeps_unicode_space_in_comments() {
    for doc in ["# c\u{a0}\na: 1\n", "a: 1 # c\u{85}\n"] {
        let formatted = cst::format(doc).unwrap();
        assert_eq!(formatted, doc, "comment text changed");
    }
}
