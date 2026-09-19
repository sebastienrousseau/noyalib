// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A `:` at end of input is a value indicator (issue #312).
//!
//! `a:` and `a:\n` are the same document one byte apart, and the
//! trailing newline is not content. Before this was fixed the first
//! loaded as the scalar `"a:"` and the second as the mapping
//! `{a: null}`, because the plain-scalar scanner substituted a NUL for
//! the byte after the colon at end of input, and NUL is not blank or
//! break — so the scalar swallowed the colon instead of stopping at it.
//!
//! Three of the four faces were silent wrong values; `"a: 1\nb:"` was a
//! hard parse error on valid YAML. PyYAML and Psych accept all four.

use noyalib::{Mapping, Value, from_str};

fn load(src: &str) -> Value {
    from_str::<Value>(src).unwrap_or_else(|e| panic!("{src:?} failed to parse: {e}"))
}

fn map_with_null(key: &str) -> Value {
    let mut m = Mapping::new();
    let _ = m.insert(key, Value::Null);
    Value::Mapping(m)
}

#[test]
fn bare_key_at_eof_is_a_mapping_not_a_scalar() {
    assert_eq!(load("a:"), map_with_null("a"));
}

#[test]
fn trailing_bare_key_no_longer_fails_to_parse() {
    // This was `Err("simple key was required but not found")`.
    let mut expected = Mapping::new();
    let _ = expected.insert("a", Value::from(1));
    let _ = expected.insert("b", Value::Null);
    assert_eq!(load("a: 1\nb:"), Value::Mapping(expected));
}

#[test]
fn nested_bare_key_at_eof_nests() {
    // Was `{p: "a:"}` — a mapping whose value is a string, which a
    // consumer reading `p.a` cannot distinguish from a missing key.
    let mut outer = Mapping::new();
    let _ = outer.insert("p", map_with_null("a"));
    assert_eq!(load("p:\n  a:"), Value::Mapping(outer));
}

#[test]
fn bare_key_at_eof_inside_a_sequence() {
    assert_eq!(load("- a:"), Value::Sequence(vec![map_with_null("a")]));
}

#[test]
fn a_trailing_newline_changes_nothing() {
    // The pair that proved the library disagreed with itself.
    assert_eq!(load("a:"), load("a:\n"));
    assert_eq!(load("a: 1\nb:"), load("a: 1\nb:\n"));
}

#[test]
fn the_controls_are_unchanged() {
    assert_eq!(load("a: "), map_with_null("a"));
    assert_eq!(load("a:\n"), map_with_null("a"));
    assert_eq!(load("a: #c"), map_with_null("a"));
}

#[test]
fn a_colon_without_following_space_is_still_a_plain_scalar() {
    // `#` needs preceding whitespace to open a comment, so `a:#c` is
    // one scalar and must not become a mapping.
    assert_eq!(load("a:#c"), Value::from("a:#c"));
    // Likewise a colon inside a word: `12:30` is a scalar, not a key.
    assert_eq!(load("12:30"), Value::from("12:30"));
    assert_eq!(load("a:b"), Value::from("a:b"));
}

#[test]
fn deeper_nesting_at_eof() {
    let mut inner = Mapping::new();
    let _ = inner.insert("c", Value::Null);
    let mut mid = Mapping::new();
    let _ = mid.insert("b", Value::Mapping(inner));
    let mut outer = Mapping::new();
    let _ = outer.insert("a", Value::Mapping(mid));
    assert_eq!(load("a:\n  b:\n    c:"), Value::Mapping(outer));
}

#[test]
fn cst_and_serde_entry_points_agree() {
    for src in ["a:", "a: 1\nb:", "p:\n  a:", "- a:"] {
        let doc = noyalib::cst::parse_document(src)
            .unwrap_or_else(|e| panic!("{src:?} cst parse failed: {e}"));
        assert_eq!(
            *doc.as_value(),
            load(src),
            "entry points disagree on {src:?}"
        );
    }
}
