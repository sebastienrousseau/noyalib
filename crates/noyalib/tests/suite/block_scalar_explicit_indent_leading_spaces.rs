// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A literal block scalar with an explicit indentation indicator whose
//! leading lines are spaces only (issue #384): the indicator fixes the
//! content indentation, so a leading line's surplus spaces are content,
//! not an over-indented empty line. The serializer writes exactly this
//! shape for a string that starts with a space-only line, and its own
//! parser refused it.

#![allow(missing_docs)]

use noyalib::{Mapping, Value, from_str, to_string};

fn k(text: &str) -> Value {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::String(text.into()));
    Value::Mapping(m)
}

#[test]
fn a_leading_space_only_line_under_an_indicator_is_content() {
    for (yaml, expected) in [
        ("k: |2\n   \n", " \n"),
        ("k: |2-\n   \n  x", " \nx"),
        ("k: |2+\n    \n\n", "  \n\n"),
        ("k: |2\n   \n  x\n", " \nx\n"),
        // No trailing break on the space-only line.
        ("k: |2\n   ", " \n"),
        // Exactly the indentation: an empty line, as before.
        ("k: |2\n  \n  x\n", "\nx\n"),
    ] {
        let value: Value = from_str(yaml).unwrap_or_else(|e| panic!("{yaml:?}: {e}"));
        assert_eq!(value["k"].as_str(), Some(expected), "{yaml:?}");
    }
}

#[test]
fn the_auto_detect_rule_still_refuses_an_over_indented_leading_empty_line() {
    let err = from_str::<Value>("k: |\n   \n  x\n").unwrap_err();
    assert!(
        err.to_string()
            .contains("a leading all-space line must not have too many spaces"),
        "{err}"
    );
}

#[test]
fn the_serializer_output_for_a_space_led_text_reads_back() {
    for text in [" \n", " \nx", "  \n\n", " \n x\n", " "] {
        let value = k(text);
        let out = to_string(&value).unwrap();
        let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{text:?} -> {out:?}: {e}"));
        assert_eq!(back, value, "{text:?} -> {out:?}");
    }
}
