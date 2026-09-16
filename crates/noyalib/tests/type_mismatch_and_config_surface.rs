// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! What happens when the YAML and the Rust type disagree.
//!
//! A deserializer's happy path is one branch per type; its unhappy path
//! is one branch per *pair* of types, and those are what a caller
//! actually hits — a port number written `"8080"`, a list where a
//! string was expected. The message each pair produces is the whole
//! user experience of a misconfigured file, and none of them was
//! pinned.
//!
//! Also covered here: the `Spanned` deserialization entry point, the
//! float spellings YAML needs (`.inf`, `-.inf`, `.nan`), and the
//! borrowed value graph's non-string mapping keys — each a small branch
//! reached only from a shape the common tests do not produce.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use noyalib::{Spanned, from_str, to_string};
use serde::Deserialize;
use serde_bytes::ByteBuf;

// The fields exist to give serde a type to ask for; nothing reads them
// back, because every case here is expected to fail before that point.
#[derive(Deserialize, Debug)]
struct HasString {
    #[allow(dead_code)]
    s: String,
}

#[derive(Deserialize, Debug)]
struct HasBytes {
    #[allow(dead_code)]
    b: ByteBuf,
}

/// A wrong type must be an error, and the message must name both sides
/// so the reader can tell which end to fix.
#[track_caller]
fn rejects<T>(label: &str, yaml: &str) -> String
where
    T: for<'de> Deserialize<'de> + std::fmt::Debug + 'static,
{
    match from_str::<T>(yaml) {
        Ok(v) => panic!("{label}: {yaml:?} was accepted as {v:?}"),
        Err(e) => {
            let msg = e.to_string();
            assert!(!msg.is_empty(), "{label}: empty message");
            msg
        }
    }
}

#[test]
fn a_string_field_refuses_every_non_string_shape() {
    for (label, yaml) in [("a sequence", "s: [1]\n"), ("a mapping", "s: {a: 1}\n")] {
        let msg = rejects::<HasString>(label, yaml);
        assert!(
            msg.contains("string"),
            "{label}: message does not mention the expected type: {msg}"
        );
    }
}

#[test]
fn a_bytes_field_refuses_every_non_string_shape() {
    for (label, yaml) in [
        ("an integer", "b: 123\n"),
        ("a float", "b: 1.5\n"),
        ("a boolean", "b: true\n"),
        ("a sequence", "b: [1]\n"),
    ] {
        let msg = rejects::<HasBytes>(label, yaml);
        assert!(
            msg.contains("bytes") || msg.contains("string") || msg.contains("binary"),
            "{label}: message does not say what was wanted: {msg}"
        );
    }
}

#[test]
fn numeric_and_scalar_fields_refuse_text_that_is_not_that_type() {
    let cases: &[(&str, &str)] = &[
        ("i64 from text", "a: hello\n"),
        ("i64 from a sequence", "a: [1]\n"),
    ];
    for (label, yaml) in cases {
        let _ = rejects::<BTreeMap<String, i64>>(label, yaml);
    }
    let _ = rejects::<BTreeMap<String, f64>>("f64 from text", "a: hello\n");
    let _ = rejects::<BTreeMap<String, bool>>("bool from text", "a: hello\n");
    let _ = rejects::<BTreeMap<String, char>>("char from a word", "a: hello\n");
    let _ = rejects::<BTreeMap<String, u64>>("u64 from a negative", "a: -1\n");
}

// ── Spanned ─────────────────────────────────────────────────────

/// `Spanned<T>` is recognised by a private type name rather than a
/// trait, so its entry point is reached only by naming the type.
#[test]
fn spanned_reports_the_position_of_the_value_it_wraps() {
    #[derive(Deserialize, Debug)]
    struct Cfg {
        first: Spanned<u16>,
        second: Spanned<String>,
    }
    let yaml = "first: 8080\nsecond: api\n";
    let cfg: Cfg = from_str(yaml).expect("spanned deserialize");

    assert_eq!(cfg.first.value, 8080);
    assert_eq!(cfg.second.value, "api");
    assert_eq!(cfg.first.start.line(), 1, "wrong line for `first`");
    assert_eq!(cfg.second.start.line(), 2, "wrong line for `second`");
    assert!(
        cfg.first.start.column() > 1,
        "the span points at the key, not the value"
    );
    assert!(
        cfg.first.end.index() > cfg.first.start.index(),
        "the span is empty"
    );
    // The bytes the span names are the value as written, plus the line
    // break that ends the token — the span runs to the start of the next
    // one. Pinned as-is: a caller slicing it must trim, and a change to
    // either end should be a deliberate one.
    let slice = &yaml[cfg.first.start.index()..cfg.first.end.index()];
    assert_eq!(slice, "8080\n", "the span's extent changed");
    assert_eq!(
        slice.trim_end(),
        "8080",
        "the span does not cover the value"
    );
}

/// A `Spanned` nested inside a sequence inside a mapping — the span
/// context has to be carried down, and a flat struct never tests that.
#[test]
fn spanned_survives_nesting() {
    #[derive(Deserialize, Debug)]
    struct Item {
        name: Spanned<String>,
    }
    let yaml = "items:\n  - name: one\n  - name: two\n";
    let doc: BTreeMap<String, Vec<Item>> = from_str(yaml).expect("nested spanned");
    let lines: Vec<_> = doc["items"].iter().map(|i| i.name.start.line()).collect();
    assert_eq!(lines, [2, 3], "nested spans lost their positions");
}

// ── float spellings ─────────────────────────────────────────────

/// YAML spells the non-finite floats `.inf`, `-.inf` and `.nan`. Each
/// has its own branch on the way out, and reading them back is the only
/// way to know the branch wrote the right one.
#[test]
fn the_non_finite_floats_round_trip_through_their_yaml_spellings() {
    let cases: &[(&str, f64)] = &[
        ("positive infinity", f64::INFINITY),
        ("negative infinity", f64::NEG_INFINITY),
    ];
    for (label, value) in cases {
        let out = to_string(&BTreeMap::from([("k".to_string(), *value)]))
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        assert!(
            out.contains(".inf"),
            "{label}: not written in YAML's spelling: {out}"
        );
        let back: BTreeMap<String, f64> =
            from_str(&out).unwrap_or_else(|e| panic!("{label}: reparse: {e}\n{out}"));
        assert_eq!(back["k"], *value, "{label}: did not round-trip\n{out}");
    }

    let out = to_string(&BTreeMap::from([("k".to_string(), f64::NAN)])).expect("nan");
    assert!(out.contains(".nan"), "NaN not written as `.nan`: {out}");
    let back: BTreeMap<String, f64> = from_str(&out).expect("nan reparse");
    assert!(back["k"].is_nan(), "NaN did not round-trip: {out}");
}

// ── non-string mapping keys ─────────────────────────────────────

/// YAML mapping keys need not be strings. Both value graphs coerce them
/// when a `BTreeMap<String, _>` is asked for, and each scalar kind takes
/// its own arm.
#[test]
fn non_string_mapping_keys_are_coerced_the_same_way_by_both_value_graphs() {
    let yaml = "1: one\ntrue: yes-key\n~: null-key\n2.5: float-key\n";

    let owned: BTreeMap<String, String> = from_str(yaml).expect("owned coercion");
    assert_eq!(owned.len(), 4, "owned graph lost a key: {owned:?}");

    let borrowed = noyalib::borrowed::from_str_borrowed(yaml).expect("borrowed parse");
    let map = borrowed.as_mapping().expect("borrowed mapping");
    assert_eq!(map.len(), 4, "borrowed graph lost a key");

    // Every key the owned graph produced must also be findable in the
    // borrowed one, or the two disagree about what the document says.
    for key in owned.keys() {
        assert!(
            map.get(key.as_str()).is_some(),
            "borrowed graph has no `{key}`; the two coercions disagree"
        );
    }
}
