// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `#[serde(flatten)]` interaction with `noyalib::Value` and
//! `noyalib::Spanned<Value>`. Tests the Phase 1.3 contract:
//!
//! - **Bare `Value` flatten target — supported.** Unmatched keys are
//!   collected into a `Value::Mapping` exactly as `serde_yaml` /
//!   `serde_json` users expect.
//! - **`Spanned<Value>` flatten target — not supported, errors loudly.**
//!   serde's `FlatStructAccess` filters residue entries by the FIELDS
//!   list passed to `deserialize_struct`, and `Spanned` advertises
//!   internal magic field names that never appear in real input.
//!   The error message points users at the working alternative
//!   (bare `Value` + `Document::span_at`).
//! - **Bare `Spanned<Value>` (non-flatten) — fully supported.** Source
//!   locations are populated from the underlying YAML.

#![allow(missing_docs)]

use noyalib::{Spanned, Value, from_str};
use serde::{Deserialize, Serialize};

// ── Supported: flatten Value ─────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Config {
    name: String,
    version: String,
    #[serde(flatten)]
    extra: Value,
}

#[test]
fn flatten_value_collects_residue_into_mapping() {
    let yaml = "\
name: noyalib
version: 0.0.1
custom: data
port: 8080
";
    let cfg: Config = from_str(yaml).unwrap();
    assert_eq!(cfg.name, "noyalib");
    assert_eq!(cfg.version, "0.0.1");

    let extra = match cfg.extra {
        Value::Mapping(m) => m,
        other => panic!("expected residue Mapping, got {other:?}"),
    };
    assert_eq!(extra.len(), 2);
    assert_eq!(extra.get("custom"), Some(&Value::String("data".into())));
    assert_eq!(
        extra.get("port"),
        Some(&Value::Number(noyalib::Number::Integer(8080))),
    );
}

#[test]
fn flatten_value_round_trips() {
    let yaml = "\
name: noyalib
version: 0.0.1
custom: data
port: 8080
";
    let cfg: Config = from_str(yaml).unwrap();
    let back = noyalib::to_string(&cfg).unwrap();
    let again: Config = from_str(&back).unwrap();
    assert_eq!(cfg, again);
}

#[test]
fn flatten_value_with_no_residue() {
    let yaml = "name: noyalib\nversion: 0.0.1\n";
    let cfg: Config = from_str(yaml).unwrap();
    // Empty residue surfaces as an empty mapping.
    let extra = match cfg.extra {
        Value::Mapping(m) => m,
        other => panic!("expected empty Mapping, got {other:?}"),
    };
    assert!(extra.is_empty());
}

// ── Documented limitation: flatten Spanned<Value> ────────────────────

#[derive(Debug, Deserialize)]
struct ConfigSpanned {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    #[serde(flatten)]
    extra: Spanned<Value>,
}

#[test]
fn flatten_spanned_errors_with_actionable_message() {
    let yaml = "\
name: noyalib
version: 0.0.1
custom: data
port: 8080
";
    let err = from_str::<ConfigSpanned>(yaml).unwrap_err();
    let msg = err.to_string();
    // The error must reference the supported workaround so users
    // know what to do — not the bare `missing field` gibberish that
    // would result without our explicit guard.
    assert!(
        msg.contains("flatten"),
        "error must mention `flatten`: {msg}"
    );
    assert!(
        msg.contains("Value") || msg.contains("Document"),
        "error must point at the workaround: {msg}"
    );
}

// ── Spanned<Value> in regular (non-flatten) position ─────────────────

#[derive(Debug, Deserialize)]
struct WithSpannedField {
    name: String,
    extra: Spanned<Value>,
}

#[test]
fn spanned_value_in_regular_position_carries_location() {
    // `extra` here is a *named* field whose value is a YAML mapping;
    // Spanned wraps it cleanly, with span info populated from the
    // underlying parser.
    let yaml = "\
name: noyalib
extra:
  custom: data
  port: 8080
";
    let cfg: WithSpannedField = from_str(yaml).unwrap();
    assert_eq!(cfg.name, "noyalib");

    // The inner value is the nested mapping.
    match &cfg.extra.value {
        Value::Mapping(m) => {
            assert_eq!(m.len(), 2);
            assert_eq!(m.get("custom"), Some(&Value::String("data".into())));
        }
        other => panic!("expected Mapping, got {other:?}"),
    }

    // Span info is populated — `extra:`'s value starts on a line > 1.
    assert!(
        cfg.extra.start.line() >= 2,
        "Spanned<Value> in regular position must carry source line info; \
         got line {}",
        cfg.extra.start.line()
    );
}

#[test]
fn spanned_value_for_scalar() {
    // `Spanned<Value>` works for scalars too — the inner Value is
    // whichever variant the YAML produced.
    let yaml = "name: noyalib\nport: 8080\n";

    #[derive(Debug, Deserialize)]
    struct Sc {
        #[allow(dead_code)]
        name: String,
        port: Spanned<Value>,
    }

    let cfg: Sc = from_str(yaml).unwrap();
    assert_eq!(
        cfg.port.value,
        Value::Number(noyalib::Number::Integer(8080))
    );
    assert!(cfg.port.start.line() >= 2);
}
