// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `ParserConfig::version(YamlVersion::V1_1)` — bundle toggle for
//! the three YAML 1.1 plain-scalar resolution differences. Setting
//! the version flips `legacy_booleans`, `legacy_octal_numbers`, and
//! `legacy_sexagesimal` together. The fine-grained flags remain
//! available for callers who want to mix and match (e.g. "1.1
//! booleans but reject octal").

#![allow(missing_docs)]

use noyalib::{ParserConfig, Value, YamlVersion, from_str, from_str_with_config};

// ── Default version is 1.2 ──────────────────────────────────────────

#[test]
fn default_is_v1_2() {
    let cfg = ParserConfig::new();
    assert_eq!(cfg.yaml_version, YamlVersion::V1_2);
    assert!(!cfg.legacy_booleans);
    assert!(!cfg.legacy_octal_numbers);
    assert!(!cfg.legacy_sexagesimal);
}

// ── 1.1 mode flips the three legacy flags ──────────────────────────

#[test]
fn v1_1_sets_all_legacy_flags() {
    let cfg = ParserConfig::new().version(YamlVersion::V1_1);
    assert_eq!(cfg.yaml_version, YamlVersion::V1_1);
    assert!(cfg.legacy_booleans);
    assert!(cfg.legacy_octal_numbers);
    assert!(cfg.legacy_sexagesimal);
}

// ── 1.2 explicit reset clears them ─────────────────────────────────

#[test]
fn v1_2_resets_legacy_flags() {
    // Start from 1.1, then revert to 1.2.
    let cfg = ParserConfig::new()
        .version(YamlVersion::V1_1)
        .version(YamlVersion::V1_2);
    assert_eq!(cfg.yaml_version, YamlVersion::V1_2);
    assert!(!cfg.legacy_booleans);
    assert!(!cfg.legacy_octal_numbers);
    assert!(!cfg.legacy_sexagesimal);
}

// ── End-to-end: 1.1 booleans ───────────────────────────────────────

#[test]
fn v1_1_resolves_yes_no_on_off_as_bool() {
    let cfg = ParserConfig::new().version(YamlVersion::V1_1);
    for (yaml, expected) in [
        ("yes", true),
        ("no", false),
        ("on", true),
        ("off", false),
        ("Yes", true),
        ("No", false),
        ("ON", true),
        ("OFF", false),
    ] {
        let v: Value = from_str_with_config(yaml, &cfg).unwrap();
        assert_eq!(
            v,
            Value::Bool(expected),
            "expected {yaml:?} → Bool({expected})"
        );
    }
}

#[test]
fn v1_2_keeps_yes_no_on_off_as_strings() {
    for yaml in ["yes", "no", "on", "off"] {
        let v: Value = from_str(yaml).unwrap();
        assert_eq!(v.as_str(), Some(yaml), "1.2 must keep {yaml:?} as string");
    }
}

// ── End-to-end: 1.1 octal ──────────────────────────────────────────

#[test]
fn v1_1_resolves_bare_zero_prefix_as_octal() {
    let cfg = ParserConfig::new().version(YamlVersion::V1_1);
    let v: Value = from_str_with_config("0644", &cfg).unwrap();
    assert_eq!(v, Value::from(420_i64), "0644 octal must equal 420");
}

#[test]
fn v1_2_keeps_bare_zero_prefix_as_decimal() {
    let v: Value = from_str("0644").unwrap();
    assert_eq!(v, Value::from(644_i64), "1.2 reads 0644 as decimal 644");
}

// ── End-to-end: 1.1 sexagesimal ────────────────────────────────────

#[test]
fn v1_1_resolves_colon_separated_as_base60() {
    let cfg = ParserConfig::new().version(YamlVersion::V1_1);
    for (yaml, expected) in [
        ("1:30", 90_i64),       // 1*60 + 30
        ("1:30:00", 5_400_i64), // 1*3600 + 30*60
        ("60:00", 3_600_i64),   // 60*60 + 0
    ] {
        let v: Value = from_str_with_config(yaml, &cfg).unwrap();
        assert_eq!(v, Value::from(expected), "{yaml} must equal {expected}");
    }
}

#[test]
fn v1_2_keeps_colon_separated_as_strings() {
    let v: Value = from_str("1:30").unwrap();
    assert_eq!(v.as_str(), Some("1:30"));
}

// ── Fine-grained override after version() ──────────────────────────

#[test]
fn version_then_individual_override_is_honoured() {
    // Caller wants 1.1 booleans/octal but explicitly rejects
    // sexagesimal (a common shape — sexagesimal often surprises).
    let cfg = ParserConfig::new()
        .version(YamlVersion::V1_1)
        .legacy_sexagesimal(false);

    let v: Value = from_str_with_config("on", &cfg).unwrap();
    assert_eq!(v, Value::Bool(true), "1.1 booleans still on");

    let v: Value = from_str_with_config("0644", &cfg).unwrap();
    assert_eq!(v, Value::from(420_i64), "1.1 octal still on");

    let v: Value = from_str_with_config("1:30", &cfg).unwrap();
    assert_eq!(
        v.as_str(),
        Some("1:30"),
        "individual override beat the version preset"
    );
}

// ── Mixed-document round-trip: a Kubernetes-flavoured file with
// 1.1-isms commonly seen in older configs ─────────────────────────

#[test]
fn realistic_v1_1_document_round_trips() {
    let yaml = "\
production: yes
debug: off
permissions: 0755
timeout: 1:30
";
    let cfg = ParserConfig::new().version(YamlVersion::V1_1);
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(v["production"], Value::Bool(true));
    assert_eq!(v["debug"], Value::Bool(false));
    assert_eq!(v["permissions"], Value::from(493_i64)); // 0755 octal
    assert_eq!(v["timeout"], Value::from(90_i64)); // 1:30 sexagesimal
}
