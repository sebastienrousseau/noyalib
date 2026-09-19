//! Regression coverage for the block scalar indentation indicator
//! (Refs #346).
//!
//! A literal/folded block scalar whose first content line itself starts
//! with a space or tab defeats a parser's indentation auto-detection
//! (YAML 1.2.2 §8.1.1.1): the leading whitespace on that first line gets
//! folded into the detected indentation, which then rejects any later,
//! less-indented line as "inconsistent indentation". The fix is an
//! explicit indentation indicator digit between the block style character
//! (`|`/`>`) and the chomping indicator.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::fmt::FoldStr;
use noyalib::{Mapping, Value, from_str, to_string};

#[test]
fn space_leading_first_line_gets_indentation_indicator() {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::String("  Indented\nsecond".to_owned()));
    let out = to_string(&Value::Mapping(m)).unwrap();
    assert_eq!(out, "k: |2-\n    Indented\n  second");

    let back: Value = from_str(&out).unwrap();
    assert_eq!(
        back.get_path("k").and_then(Value::as_str),
        Some("  Indented\nsecond")
    );
}

#[test]
fn folded_block_also_gets_indentation_indicator() {
    let out = to_string(&FoldStr("  Indented\nsecond")).unwrap();
    assert_eq!(out, ">2-\n    Indented\n  second");

    let back: Value = from_str(&out).unwrap();
    assert_eq!(back.as_str(), Some("  Indented\nsecond"));
}

#[test]
fn non_space_leading_first_line_keeps_plain_clip_indicator() {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::String("first\nsecond".to_owned()));
    let out = to_string(&Value::Mapping(m)).unwrap();
    assert_eq!(out, "k: |-\n  first\n  second");

    let back: Value = from_str(&out).unwrap();
    assert_eq!(
        back.get_path("k").and_then(Value::as_str),
        Some("first\nsecond")
    );
}

#[test]
fn nested_key_indentation_indicator_is_still_the_relative_indent() {
    // The indicator states the indentation *relative to the parent node*,
    // so it stays `2` (== `config.indent`) regardless of how deep the key
    // nests -- only the physical column the content lands on changes.
    let mut inner = Mapping::new();
    let _ = inner.insert("k", Value::String("  Indented\nsecond".to_owned()));
    let mut outer = Mapping::new();
    let _ = outer.insert("outer", Value::Mapping(inner));
    let out = to_string(&Value::Mapping(outer)).unwrap();
    assert_eq!(out, "outer:\n  k: |2-\n      Indented\n    second");

    let back: Value = from_str(&out).unwrap();
    assert_eq!(
        back.get_path("outer.k").and_then(Value::as_str),
        Some("  Indented\nsecond")
    );
}
