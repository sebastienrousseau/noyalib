// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `ParserConfig::no_schema` and `ParserConfig::legacy_octal_numbers`
//! — opt-out / opt-in tweaks to the YAML 1.2 plain-scalar
//! resolution rules.

#![allow(missing_docs)]

use noyalib::{Number, ParserConfig, Value, from_str, from_str_with_config};

// ── no_schema ──────────────────────────────────────────────────────

#[test]
fn no_schema_keeps_yes_no_as_strings() {
    // With no_schema = true, even `true` / `false` / `null` stay as
    // strings — the application layer takes responsibility for
    // explicit typing.
    let cfg = ParserConfig::new().no_schema(true);
    let v: Value = from_str_with_config("a: true\nb: null\nc: 42\nd: 1.5\n", &cfg).unwrap();
    assert_eq!(v["a"].as_str(), Some("true"));
    assert_eq!(v["b"].as_str(), Some("null"));
    assert_eq!(v["c"].as_str(), Some("42"));
    assert_eq!(v["d"].as_str(), Some("1.5"));
}

#[test]
fn no_schema_does_not_affect_explicit_tags() {
    // `!!int` etc. still resolve under their own tag rules; the
    // no_schema toggle only governs *plain* (untagged) scalars.
    let cfg = ParserConfig::new().no_schema(true);
    let v: Value = from_str_with_config("a: !!int 42\nb: !!bool true\n", &cfg).unwrap();
    assert_eq!(v["a"].as_i64(), Some(42));
    assert_eq!(v["b"].as_bool(), Some(true));
}

#[test]
fn no_schema_default_is_off() {
    assert!(!ParserConfig::new().no_schema);
    // Default behaviour resolves `42` to an integer.
    let v: Value = from_str("port: 42").unwrap();
    assert_eq!(v["port"].as_i64(), Some(42));
}

#[test]
fn no_schema_keeps_quoted_strings_unchanged() {
    // Quoted scalars are always strings regardless; this test just
    // confirms no regression when the toggle is on.
    let cfg = ParserConfig::new().no_schema(true);
    let v: Value = from_str_with_config("a: \"42\"\nb: '42'\n", &cfg).unwrap();
    assert_eq!(v["a"].as_str(), Some("42"));
    assert_eq!(v["b"].as_str(), Some("42"));
}

// ── legacy_octal_numbers ───────────────────────────────────────────

#[test]
fn legacy_octal_off_treats_zero_prefix_as_decimal() {
    // Without the toggle, `0644` falls through to Rust's stdlib
    // integer parser which accepts leading zeros and decodes
    // decimal — so the value comes back as 644, not 420. This
    // documents the current behaviour; flip `legacy_octal_numbers`
    // on for the YAML-1.1-style octal interpretation.
    let v: Value = from_str("perm: 0644").unwrap();
    assert_eq!(v["perm"].as_i64(), Some(644));
}

#[test]
fn legacy_octal_on_parses_zero_prefix_as_octal() {
    let cfg = ParserConfig::new().legacy_octal_numbers(true);
    let v: Value = from_str_with_config("perm: 0644", &cfg).unwrap();
    // 0o644 = 420
    assert_eq!(v["perm"].as_i64(), Some(0o644));
}

#[test]
fn legacy_octal_still_supports_yaml_1_2_form() {
    let cfg = ParserConfig::new().legacy_octal_numbers(true);
    let v: Value = from_str_with_config("perm: 0o644", &cfg).unwrap();
    assert_eq!(v["perm"].as_i64(), Some(0o644));
}

#[test]
fn legacy_octal_does_not_misclassify_decimal_with_eight_or_nine() {
    // `08` and `09` are not valid octals — the toggle must not
    // silently misparse them as octal. They fall through to the
    // decimal integer path (Rust's stdlib accepts leading zeros).
    let cfg = ParserConfig::new().legacy_octal_numbers(true);
    let v: Value = from_str_with_config("a: 08\nb: 09\n", &cfg).unwrap();
    assert_eq!(v["a"].as_i64(), Some(8));
    assert_eq!(v["b"].as_i64(), Some(9));
}

#[test]
fn legacy_octal_default_is_off() {
    assert!(!ParserConfig::new().legacy_octal_numbers);
}

// ── combined ───────────────────────────────────────────────────────

#[test]
fn no_schema_overrides_legacy_octal() {
    // When no_schema is on, every plain scalar is a string —
    // legacy_octal has no effect.
    let cfg = ParserConfig::new()
        .no_schema(true)
        .legacy_octal_numbers(true);
    let v: Value = from_str_with_config("perm: 0644", &cfg).unwrap();
    assert_eq!(v["perm"].as_str(), Some("0644"));
}

#[test]
fn schema_strict_mode_helpful_for_norway_problem() {
    // The "Norway problem" — `NO` resolving to `false` — is the
    // canonical surprise no_schema fixes. With the toggle on, every
    // value passes through untouched.
    let cfg = ParserConfig::new().no_schema(true);
    let v: Value = from_str_with_config("country: NO\nflag: yes\n", &cfg).unwrap();
    assert_eq!(v["country"].as_str(), Some("NO"));
    assert_eq!(v["flag"].as_str(), Some("yes"));
}

#[test]
fn typed_deserialization_respects_no_schema() {
    // When no_schema is on, a `port: 8080` line surfaces as a
    // string. Asking for `u16` should error — the user must quote
    // intent or the schema breaks. This is the contract.
    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    struct Cfg {
        #[allow(dead_code)]
        port: u16,
    }
    let cfg = ParserConfig::new().no_schema(true);
    let res: Result<Cfg, _> = from_str_with_config("port: 8080", &cfg);
    assert!(
        res.is_err(),
        "no_schema must reject implicit numeric coercion"
    );
}

#[test]
fn typed_deserialization_with_legacy_octal_round_trip() {
    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    struct Perms {
        umask: i64,
    }
    let cfg = ParserConfig::new().legacy_octal_numbers(true);
    let p: Perms = from_str_with_config("umask: 0022", &cfg).unwrap();
    assert_eq!(p.umask, 0o022);
}

// ── numeric edge cases ─────────────────────────────────────────────

#[test]
fn legacy_octal_zero_alone_stays_decimal_zero() {
    let cfg = ParserConfig::new().legacy_octal_numbers(true);
    let v: Value = from_str_with_config("a: 0", &cfg).unwrap();
    // `0` alone is decimal zero either way.
    assert_eq!(v["a"], Value::Number(Number::Integer(0)));
}

#[test]
fn legacy_octal_larger_value() {
    let cfg = ParserConfig::new().legacy_octal_numbers(true);
    let v: Value = from_str_with_config("a: 0777", &cfg).unwrap();
    assert_eq!(v["a"].as_i64(), Some(0o777));
}
