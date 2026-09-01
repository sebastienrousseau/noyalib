// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::remove` on a mapping that carries a `<<` merge key
//! (issue #334).
//!
//! The loaded mapping lists merge-provided keys after the explicit
//! ones, and the span tree holds only the explicit entries. Looking a
//! merge-provided key up by its position in the loaded mapping used to
//! index past the end of the span-entry list and panic. It must be a
//! refusal that leaves the document unchanged, and an explicit key
//! next to a `<<` entry must still be removable wherever the `<<` sits.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

const SRC: &str = "base: &b\n  a: 1\n  b: 2\nmerged:\n  <<: *b\n  c: 3\n";

#[test]
fn removing_a_merge_provided_key_is_refused_not_a_panic() {
    let mut doc = parse_document(SRC).unwrap();
    for key in ["merged.a", "merged.b"] {
        let err = doc.remove(key).unwrap_err().to_string();
        assert!(err.contains("merge"), "{key}: {err}");
        assert_eq!(doc.to_string(), SRC, "{key}: a refusal must not mutate");
    }
}

#[test]
fn removing_an_explicit_key_beside_a_merge_key_works() {
    let cases = [
        // `<<` first
        (
            "base: &b\n  a: 1\nmerged:\n  <<: *b\n  c: 3\n  d: 4\n",
            "merged.c",
            "base: &b\n  a: 1\nmerged:\n  <<: *b\n  d: 4\n",
        ),
        // `<<` in the middle
        (
            "base: &b\n  a: 1\nmerged:\n  c: 3\n  <<: *b\n  d: 4\n",
            "merged.d",
            "base: &b\n  a: 1\nmerged:\n  c: 3\n  <<: *b\n",
        ),
        // `<<` last
        (
            "base: &b\n  a: 1\nmerged:\n  c: 3\n  d: 4\n  <<: *b\n",
            "merged.c",
            "base: &b\n  a: 1\nmerged:\n  d: 4\n  <<: *b\n",
        ),
    ];
    for (src, path, want) in cases {
        let mut doc = parse_document(src).unwrap();
        doc.remove(path).unwrap();
        assert_eq!(doc.to_string(), want, "{src:?} - {path}");
        let merged = doc.as_value()["merged"].clone();
        assert_eq!(merged["a"].as_i64(), Some(1), "merged value survives");
        assert!(merged.get(path.trim_start_matches("merged.")).is_none());
    }
}

#[test]
fn the_last_explicit_key_beside_a_merge_key_leaves_the_merge_entry() {
    let mut doc = parse_document(SRC).unwrap();
    doc.remove("merged.c").unwrap();
    assert_eq!(
        doc.to_string(),
        "base: &b\n  a: 1\n  b: 2\nmerged:\n  <<: *b\n"
    );
    assert_eq!(doc.as_value()["merged"]["a"].as_i64(), Some(1));
}

#[test]
fn a_merge_only_mapping_is_not_treated_as_a_sole_entry() {
    // `merged` loads as `{a: 1}` — one key — but that key is not an entry
    // of its own. The sole-entry arm must not replace the `<<` line with
    // `{}`.
    let src = "base: &b\n  a: 1\nmerged:\n  <<: *b\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.remove("merged.a").unwrap_err().to_string();
    assert!(err.contains("merge"), "{err}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn descending_through_a_merge_provided_key_is_refused() {
    let src = "base: &b\n  a:\n    x: 1\nmerged:\n  <<: *b\n  c: 3\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.remove("merged.a.x").unwrap_err().to_string();
    assert!(err.contains("merge"), "{err}");
    assert_eq!(doc.to_string(), src);
}
