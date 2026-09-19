//! Regression coverage for `|+` (keep-chomping) trailing-newline
//! over-counting (Refs #347).
//!
//! `write_block_scalar`'s trailing-newline supplement counted *every*
//! trailing newline in the source string and re-emitted that many
//! newlines after the body loop, but the body loop (via `str::lines`)
//! already emits all but one of them itself. The supplement only needed
//! to make up the one newline `str::lines` always drops for the
//! string's final terminator -- emitting the full count double-counted
//! every trailing newline after the first, so each serialize/parse
//! round trip grew the string by one more `\n`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Mapping, SerializerConfig, Value, from_str, to_string_with_config};

fn cfg() -> SerializerConfig {
    SerializerConfig::new()
        .block_scalars(true)
        .block_scalar_threshold(1)
}

fn to_yaml(s: &str) -> String {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::String(s.to_owned()));
    to_string_with_config(&Value::Mapping(m), &cfg()).unwrap()
}

fn from_yaml(yaml: &str) -> String {
    let v: Value = from_str(yaml).unwrap();
    v.get_path("k").and_then(Value::as_str).unwrap().to_owned()
}

#[test]
fn keep_chomping_emits_exactly_the_source_trailing_newlines() {
    let out = to_yaml("text\n\n");
    assert_eq!(out, "k: |+\n  text\n\n");
    assert_eq!(from_yaml(&out), "text\n\n");
}

#[test]
fn keep_chomping_round_trips_stably_across_four_cycles() {
    let mut current = "text\n\n".to_owned();
    for _ in 0..4 {
        let out = to_yaml(&current);
        let back = from_yaml(&out);
        assert_eq!(
            back, "text\n\n",
            "value drifted across a serialize/parse cycle"
        );
        current = back;
    }
}

#[test]
fn clip_chomping_single_trailing_newline_is_unchanged() {
    let out = to_yaml("text\n");
    assert_eq!(out, "k: |\n  text\n");
    assert_eq!(from_yaml(&out), "text\n");
}

#[test]
fn strip_chomping_no_trailing_newline_is_unchanged() {
    let out = to_yaml("line1\nline2");
    assert_eq!(out, "k: |-\n  line1\n  line2");
    assert_eq!(from_yaml(&out), "line1\nline2");
}
