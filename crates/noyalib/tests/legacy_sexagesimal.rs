// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `ParserConfig::legacy_sexagesimal` — opt-in YAML 1.1 base-60
//! number parsing for migrations from Ruby YAML / pyyaml configs
//! that use `60:00`-style time-of-day notation.

#![allow(missing_docs)]

use noyalib::{ParserConfig, Value, from_str, from_str_with_config};

// ── Off-by-default contract ─────────────────────────────────────────

#[test]
fn off_by_default() {
    assert!(!ParserConfig::new().legacy_sexagesimal);
    // Default: `60:00` is a string.
    let v: Value = from_str("duration: 60:00").unwrap();
    assert_eq!(v["duration"].as_str(), Some("60:00"));
}

// ── Integer sexagesimal ────────────────────────────────────────────

#[test]
fn two_component_minutes_seconds() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: 60:00", &cfg).unwrap();
    // 60 * 60 + 0 = 3 600
    assert_eq!(v["d"].as_i64(), Some(3600));
}

#[test]
fn three_component_hours_minutes_seconds() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: 1:30:00", &cfg).unwrap();
    // (1 * 60 + 30) * 60 + 0 = 5 400
    assert_eq!(v["d"].as_i64(), Some(5400));
}

#[test]
fn negative_sexagesimal() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: -1:30:00", &cfg).unwrap();
    assert_eq!(v["d"].as_i64(), Some(-5400));
}

#[test]
fn explicit_plus_sign() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: +1:30:00", &cfg).unwrap();
    assert_eq!(v["d"].as_i64(), Some(5400));
}

#[test]
fn first_component_unbounded() {
    // Hours can be any non-negative integer; only secondary
    // components are clamped to 0..60.
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: 100:00", &cfg).unwrap();
    assert_eq!(v["d"].as_i64(), Some(6000));
}

#[test]
fn rejects_secondary_component_over_60() {
    // `1:99` has a secondary component that exceeds 59 — not a
    // valid sexagesimal. Falls back to string.
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: 1:99", &cfg).unwrap();
    assert_eq!(v["d"].as_str(), Some("1:99"));
}

#[test]
fn rejects_non_digit_components() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: 1:ab", &cfg).unwrap();
    assert_eq!(v["d"].as_str(), Some("1:ab"));
}

#[test]
fn rejects_empty_components() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: 1::00", &cfg).unwrap();
    assert_eq!(v["d"].as_str(), Some("1::00"));
}

// ── Float sexagesimal (last component fractional) ─────────────────

#[test]
fn two_component_with_fraction() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: 1:30.5", &cfg).unwrap();
    // 1 * 60 + 30.5 = 90.5
    assert_eq!(v["d"].as_f64(), Some(90.5));
}

#[test]
fn three_component_with_fraction() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: 1:30:00.5", &cfg).unwrap();
    // (1 * 60 + 30) * 60 + 0.5 = 5400.5
    assert_eq!(v["d"].as_f64(), Some(5400.5));
}

// ── Interaction with other plain-scalar resolutions ───────────────

#[test]
fn does_not_disturb_normal_integers() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("a: 42\nb: -7\nc: 0\n", &cfg).unwrap();
    assert_eq!(v["a"].as_i64(), Some(42));
    assert_eq!(v["b"].as_i64(), Some(-7));
    assert_eq!(v["c"].as_i64(), Some(0));
}

#[test]
fn does_not_disturb_normal_floats() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("a: 1.5\nb: -2.5\nc: .inf\n", &cfg).unwrap();
    assert_eq!(v["a"].as_f64(), Some(1.5));
    assert_eq!(v["b"].as_f64(), Some(-2.5));
    assert!(v["c"].as_f64().unwrap().is_infinite());
}

#[test]
fn does_not_misinterpret_iso_timestamps_as_sexagesimal() {
    // `02:59:43` looks sexagesimal (would be 10 783) but the
    // leading `2001-12-15T` makes the whole scalar a non-numeric
    // string. Sanity-check that we don't accidentally pick it up.
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("ts: 2001-12-15T02:59:43Z", &cfg).unwrap();
    assert_eq!(v["ts"].as_str(), Some("2001-12-15T02:59:43Z"));
}

// ── Typed deserialization round-trip ──────────────────────────────

#[test]
fn typed_deserialize_minutes_seconds() {
    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    struct Cfg {
        timeout: u64,
    }
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let parsed: Cfg = from_str_with_config("timeout: 1:30:00", &cfg).unwrap();
    assert_eq!(parsed.timeout, 5400);
}

#[test]
fn quoted_string_unaffected_even_with_toggle_on() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let v: Value = from_str_with_config("d: \"60:00\"", &cfg).unwrap();
    // Quoted scalars are always strings; the toggle is a
    // *plain*-scalar resolver.
    assert_eq!(v["d"].as_str(), Some("60:00"));
}
