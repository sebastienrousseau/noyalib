//! Bare `nan` / `inf` spellings are strings, not floats.
//!
//! Rust's `f64::from_str` accepts `nan`, `inf` and `infinity` in any
//! case with an optional sign. YAML 1.2 spells the float specials
//! `.nan`, `.inf` and `-.inf`, with a leading dot. Letting the bare
//! forms resolve to floats destroyed the scalar's original text, so a
//! mapping key `nAn` came back as `nan` and `nAn: null` did not
//! round-trip. Found by the `roundtrip_value` proptest.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Value, from_str, to_string};

fn first_key(v: &Value) -> String {
    match v {
        Value::Mapping(m) => m.keys().next().expect("one key").clone(),
        other => panic!("expected a mapping, got {other:?}"),
    }
}

#[test]
fn bare_specials_keep_their_spelling_as_keys() {
    // The exact case proptest minimised to, plus the neighbours that
    // `f64::from_str` also accepts.
    for key in [
        "nAn", "nan", "NaN", "NAN", "iNf", "inf", "INF", "Infinity", "-inf", "+inf",
    ] {
        let parsed: Value = from_str(&format!("{key}: null\n")).expect(key);
        assert_eq!(first_key(&parsed), key, "key {key:?} lost its spelling");
    }
}

#[test]
fn bare_specials_are_strings_as_values() {
    for text in ["nan", "nAn", "inf", "iNf", "-inf", "infinity"] {
        let parsed: Value = from_str(&format!("v: {text}\n")).expect(text);
        let Value::Mapping(m) = &parsed else {
            panic!("expected mapping")
        };
        assert_eq!(
            m.get("v"),
            Some(&Value::String(text.to_owned())),
            "bare {text:?} should stay a string"
        );
    }
}

#[test]
fn dotted_specials_still_resolve_to_floats() {
    for (text, check) in [
        (".nan", "nan"),
        (".NaN", "nan"),
        (".inf", "inf"),
        (".INF", "inf"),
        ("-.inf", "-inf"),
    ] {
        let parsed: Value = from_str(&format!("v: {text}\n")).expect(text);
        let Value::Mapping(m) = &parsed else {
            panic!("expected mapping")
        };
        let Some(Value::Number(n)) = m.get("v") else {
            panic!("{text} should resolve to a float, got {:?}", m.get("v"));
        };
        let f = n.as_f64();
        match check {
            "nan" => assert!(f.is_nan(), "{text} should be NaN"),
            "inf" => assert!(
                f.is_infinite() && f.is_sign_positive(),
                "{text} should be +inf"
            ),
            _ => assert!(
                f.is_infinite() && f.is_sign_negative(),
                "{text} should be -inf"
            ),
        }
    }
}

#[test]
fn ordinary_floats_are_unaffected() {
    for text in ["1.5", "-2.25", "1e3", "0.0"] {
        let parsed: Value = from_str(&format!("v: {text}\n")).expect(text);
        let Value::Mapping(m) = &parsed else {
            panic!("expected mapping")
        };
        assert!(
            matches!(m.get("v"), Some(Value::Number(_))),
            "{text} should still parse as a number"
        );
    }
}

#[test]
fn the_proptest_case_round_trips() {
    let original: Value = from_str("nAn: null\n").expect("parse");
    let yaml = to_string(&original).expect("serialise");
    let reparsed: Value = from_str(&yaml).expect("reparse");
    assert_eq!(original, reparsed, "round-trip changed the value");
}
