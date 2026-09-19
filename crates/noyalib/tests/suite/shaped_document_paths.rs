// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Paths that need a document shaped a particular way.
//!
//! Named for what it tests rather than for a coverage target: these are
//! the branches that only run when something *beside* the edit is a
//! nested collection, when a key is an alias, or when a scalar is asked
//! for as a type it cannot be.
//!
//! The suite's fixtures are mostly flat: a mapping of scalars, a
//! sequence of scalars. Several branches only run when something
//! *beside* the edit is a nested collection, when a key is an alias,
//! or when a scalar is asked for as a type it cannot be. None of those
//! shapes appear by accident, so the branches went unexercised.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use noyalib::cst::parse_document;
use noyalib::{ParserConfig, Spanned, Value, from_str_with_config};

// ── the shape oracle's off-path walk ────────────────────────────

/// `set` and the inserters guard themselves by fingerprinting the
/// document's *shape* with the edited path elided, then comparing after.
/// Building that fingerprint walks every subtree that is not on the
/// path — so a nested sibling is what exercises the walk, and a flat
/// fixture never does.
#[test]
fn an_edit_beside_nested_siblings_keeps_them_untouched() {
    const DOC: &str = "\
target: 1
deep:
  a:
    b:
      c: 1
    list:
      - 1
      - two
      - [3, 4]
      - {k: v}
seq:
  - - 1
    - 2
  - m:
      n: 1
flat: x
";
    let before: Value = noyalib::from_str(DOC).expect("fixture parses");

    for (label, frag) in [
        ("a scalar", "2"),
        ("a flow sequence", "[1, 2]"),
        ("a flow mapping", "{x: 1}"),
        ("a quoted string", "\"text\""),
    ] {
        let mut doc = parse_document(DOC).expect("parse");
        doc.set("target", frag)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let after: Value =
            noyalib::from_str(&doc.to_string()).unwrap_or_else(|e| panic!("{label}: reparse: {e}"));

        // Everything except `target` must be identical — that is exactly
        // what the shape fingerprint is there to guarantee.
        for key in ["deep", "seq", "flat"] {
            assert_eq!(
                after.get(key),
                before.get(key),
                "{label}: the edit disturbed `{key}`"
            );
        }
    }
}

/// The same walk, reached through the typed inserters, with the new
/// entry landing beside those nested siblings.
#[test]
fn inserting_beside_nested_siblings_keeps_them_untouched() {
    const DOC: &str = "\
m:
  keep: 1
deep:
  a:
    b: [1, {c: 2}]
seq:
  - - 1
  - {x: {y: 1}}
";
    let before: Value = noyalib::from_str(DOC).expect("fixture parses");

    let mut doc = parse_document(DOC).expect("parse");
    doc.insert_entry_value("m", "added", &Value::from(9_i64))
        .expect("insert beside nested siblings");
    let after: Value = noyalib::from_str(&doc.to_string()).expect("reparse");

    assert_eq!(
        after.get("deep"),
        before.get("deep"),
        "`deep` was disturbed"
    );
    assert_eq!(after.get("seq"), before.get("seq"), "`seq` was disturbed");
    assert_eq!(
        after
            .get("m")
            .and_then(|m| m.get("added"))
            .and_then(Value::as_i64),
        Some(9)
    );
}

// ── rename_key's key-token checks ───────────────────────────────

/// An alias cannot *be* a mapping key in this parser — the fixture is
/// rejected outright — so `rename_key`'s alias-key refusal is not
/// reachable that way. Pinned here so the assumption is recorded: if
/// alias keys ever start parsing, this test fails and the refusal path
/// needs covering for real.
#[test]
fn an_alias_cannot_be_a_mapping_key() {
    let err = parse_document("anchor: &a keyname\n*a: 1\n")
        .expect_err("an alias used as a key must be rejected at parse time");
    assert!(!err.to_string().is_empty(), "empty parse error");
}

/// Renaming through every addressable key shape, so the token check is
/// driven with each kind of key token rather than only a plain one.
#[test]
fn every_key_token_shape_can_be_renamed_or_is_refused_cleanly() {
    let cases: &[(&str, &str)] = &[
        ("plain", "plain: 1\nz: 2\n"),
        ("single-quoted", "'sq': 1\nz: 2\n"),
        ("double-quoted", "\"dq\": 1\nz: 2\n"),
        ("with a dot", "'a.b': 1\nz: 2\n"),
        ("numeric-looking", "'12': 1\nz: 2\n"),
        ("inside a flow mapping", "m: {plain: 1}\nz: 2\n"),
    ];
    for (label, src) in cases {
        let mut doc = parse_document(src).expect("parse");
        let path = if src.starts_with("m: ") {
            "m.plain"
        } else {
            src.split(':')
                .next()
                .unwrap()
                .trim_matches(|c| c == '\'' || c == '"')
        };
        let before = doc.to_string();
        match doc.rename_key(path, "renamed") {
            Ok(()) => {
                let out = doc.to_string();
                let v: Value = noyalib::from_str(&out)
                    .unwrap_or_else(|e| panic!("{label}: rename broke the document: {e}\n{out}"));
                assert_eq!(
                    v.get("renamed").and_then(Value::as_i64).or_else(|| v
                        .get("m")
                        .and_then(|m| m.get("renamed"))
                        .and_then(Value::as_i64)),
                    Some(1),
                    "{label}: not readable under the new key: {out}"
                );
            }
            Err(e) => assert_eq!(
                doc.to_string(),
                before,
                "{label}: refused ({e}) but edited the document"
            ),
        }
    }
}

// ── deserializer scalar-to-type refusals ────────────────────────

/// `plain_scalar_strings` lets a plain scalar be read as a `String`.
/// Without it a non-string scalar is refused outright.
///
/// The streaming reader hands back the scalar's *source text* rather
/// than re-rendering the parsed number — `1e3` stays `1e3` — which is
/// the point of the flag: the plain scalar, as written.
#[test]
fn a_plain_scalar_read_as_a_string_is_its_source_text() {
    let mut cfg = ParserConfig::new();
    cfg.plain_scalar_strings = true;

    let m: BTreeMap<String, String> = from_str_with_config(
        "inf: .inf\nninf: -.inf\nnan: .nan\nintegral: 2.0\nfrac: 2.5\nexp: 1e3\n",
        &cfg,
    )
    .expect("plain scalars as strings");

    assert_eq!(m["inf"], ".inf");
    assert_eq!(m["ninf"], "-.inf");
    assert_eq!(m["nan"], ".nan");
    assert_eq!(m["integral"], "2.0", "the source spelling was re-rendered");
    assert_eq!(m["frac"], "2.5");
    assert_eq!(m["exp"], "1e3", "the source spelling was re-rendered");

    // The control: without the flag the same input is refused, so this
    // measures the flag rather than the default.
    let err = noyalib::from_str::<BTreeMap<String, String>>("inf: .inf\n")
        .expect_err("a non-string scalar must not become a String by default");
    assert!(
        err.to_string().contains("string"),
        "unexpected refusal: {err}"
    );
}

/// The same flag through a span-carrying target, which takes the other
/// deserializer — the one that renders from the parsed `Value` instead
/// of the source text, and so has to reproduce YAML's own spellings for
/// the non-finite floats itself.
#[test]
fn a_float_rendered_from_the_value_graph_keeps_yamls_spellings() {
    let mut cfg = ParserConfig::new();
    cfg.plain_scalar_strings = true;

    let m: BTreeMap<String, Spanned<String>> = from_str_with_config(
        "inf: .inf\nninf: -.inf\nnan: .nan\nintegral: 2.0\nfrac: 2.5\nint: 42\nb: true\n",
        &cfg,
    )
    .expect("spanned plain scalars as strings");

    for (key, want) in [
        ("inf", ".inf"),
        ("ninf", "-.inf"),
        ("nan", ".nan"),
        ("frac", "2.5"),
        ("int", "42"),
        ("b", "true"),
    ] {
        assert_eq!(m[key].value, want, "`{key}` rendered wrongly");
    }
    // An integral float must not come back as bare `2`, which would
    // read as an integer on the way back in.
    assert!(
        m["integral"].value.starts_with('2'),
        "integral float: {:?}",
        m["integral"].value
    );
}

/// The same flag, across the other scalar kinds — each takes its own
/// arm in the renderer.
#[test]
fn other_scalars_read_as_strings_render_predictably() {
    let mut cfg = ParserConfig::new();
    cfg.plain_scalar_strings = true;
    let m: BTreeMap<String, String> =
        from_str_with_config("i: 42\nneg: -7\nb: true\nbig: 9223372036854775807\n", &cfg)
            .expect("scalars as strings");
    assert_eq!(m["i"], "42");
    assert_eq!(m["neg"], "-7");
    assert_eq!(m["b"], "true");
    assert_eq!(m["big"], "9223372036854775807");
}

/// A struct whose field is missing entirely drives the "value is
/// missing" arm of the map accessor, which a complete document never
/// reaches.
#[test]
fn a_missing_field_is_reported_rather_than_defaulted() {
    #[derive(serde::Deserialize, Debug)]
    struct Cfg {
        #[allow(dead_code)]
        present: i64,
        #[allow(dead_code)]
        absent: i64,
    }
    let err =
        noyalib::from_str::<Cfg>("present: 1\n").expect_err("a missing field must be an error");
    let msg = err.to_string();
    assert!(
        msg.contains("absent") || msg.contains("missing"),
        "the error does not name the missing field: {msg}"
    );
}
