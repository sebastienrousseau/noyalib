// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! YAML 1.2.2 §9.1.2: `l-document-suffix ::= c-document-end s-l-comments`,
//! and `s-separate-in-line` admits tabs, so a tab may follow `...`.
//! Found by fuzz_serde_yaml_compat on `.\r...\t`.

use noyalib::Value;

#[test]
fn tab_separation_after_document_end_marker_is_accepted() {
    for (source, expected) in [
        (".\r...\t", "."),
        ("a\n...\t\n", "a"),
        ("a\n...\t# note\n", "a"),
        ("a\n... \t# note\n", "a"),
    ] {
        let value: Value = noyalib::from_str(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
        assert_eq!(value.as_str(), Some(expected), "{source:?}");
        assert!(
            noyalib::cst::parse_document(source).is_ok(),
            "CST {source:?}"
        );
    }
}

#[test]
fn content_after_document_end_marker_is_still_refused() {
    for source in ["a\n...\tb\n", "a\n... b\n"] {
        assert!(noyalib::from_str::<Value>(source).is_err(), "{source:?}");
    }
}
