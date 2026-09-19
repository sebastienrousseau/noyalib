// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A colon or a hash inside a string forces quotes only where YAML gives
//! it meaning: `:` before a space, a tab, a flow indicator, or the end of
//! the text; `#` at the start or after whitespace. `word:count`, a
//! timestamp, a URL, and `a#b` are written plain, as libyaml writes them.

use noyalib::{Mapping, Value, from_str, to_string};

// A digit-leading timestamp such as `2026-12-31T10:00:00Z` is the digit
// rule's case (#339), not this one; `at 10:00:00Z` exercises the colons.
const PLAIN: &[&str] = &[
    "accent-contact:word-count",
    "at 10:00:00Z",
    "http://example.com/x",
    "key:value",
    "a#b",
    "c#",
];

const QUOTED: &[&str] = &["a: b", "a:", "a:\tb", "x:,y", "x:]y", "a #b", "#a", "a\t#b"];

fn emit(s: &str) -> String {
    let mut mapping = Mapping::new();
    let _ = mapping.insert("k", Value::String(s.to_string()));
    to_string(&Value::Mapping(mapping)).unwrap()
}

#[test]
fn colons_and_hashes_without_meaning_stay_plain() {
    for s in PLAIN {
        assert_eq!(emit(s), format!("k: {s}"), "{s} is plain-safe");
    }
}

#[test]
fn colons_and_hashes_with_meaning_are_quoted() {
    for s in QUOTED {
        let out = emit(s);
        assert!(
            out.starts_with("k: \"") || out.starts_with("k: '"),
            "{s} needs quotes, got {out}"
        );
    }
}

#[test]
fn every_case_reads_back_as_the_same_string() {
    for s in PLAIN.iter().chain(QUOTED) {
        let doc = emit(s);
        let value: Value = from_str(&doc).unwrap();
        assert_eq!(
            value.get("k"),
            Some(&Value::String((*s).to_string())),
            "round trip of {doc:?}"
        );
    }
}
