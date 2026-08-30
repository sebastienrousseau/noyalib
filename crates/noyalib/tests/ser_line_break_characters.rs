// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Serializing strings that carry CR, NEL (U+0085), LS (U+2028), or PS
//! (U+2029) -- the characters only double-quoted style represents
//! (issue #335).
//!
//! A literal block scalar normalises `\r` into its own line breaks, and
//! the three Unicode separators used to be written plain, reading back
//! as line breaks. All four now force double-quoted style with the
//! named escapes.

#![allow(missing_docs)]

use noyalib::{Mapping, SerializerConfig, Value};

fn one(s: &str) -> String {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::String(s.into()));
    noyalib::to_string(&Value::Mapping(m)).unwrap()
}

fn reads_back(out: &str, want: &str) {
    let v: Value = noyalib::from_str(out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
    assert_eq!(v["k"].as_str(), Some(want), "emitted {out:?}");
}

#[test]
fn the_four_characters_round_trip_through_double_quotes() {
    for (s, expect_escape) in [
        ("line\r\nwin", "\\r"),
        ("a\rb", "\\r"),
        ("trailing\r", "\\r"),
        ("x\u{0085}y", "\\N"),
        ("x\u{2028}y", "\\L"),
        ("x\u{2029}y", "\\P"),
        ("mix\r\u{2028}\nend", "\\L"),
    ] {
        let out = one(s);
        assert!(
            out.starts_with("k: \"") && out.contains(expect_escape),
            "{s:?} emitted {out:?}"
        );
        reads_back(&out, s);
    }
}

#[test]
fn quote_all_falls_back_to_double_quotes_for_them() {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::String("x\u{2028}y".into()));
    let cfg = SerializerConfig::default();
    let cfg = cfg.quote_all(true);
    let out = noyalib::to_string_with_config(&Value::Mapping(m), &cfg).unwrap();
    assert!(out.contains("\\L"), "{out:?}");
    reads_back(&out, "x\u{2028}y");
}

#[test]
fn plain_and_newline_only_strings_are_unaffected() {
    assert_eq!(one("plain"), "k: plain");
    assert_eq!(one("a b"), "k: a b");
    // Pure-\n multiline strings keep whatever style the config picks,
    // and still round-trip.
    let out = one("a\nb\nc");
    reads_back(&out, "a\nb\nc");
}
