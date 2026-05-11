// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 3.2 — schema validation library integration.
//!
//! End-to-end checks for the `validate_against_schema` /
//! `validate_against_schema_str` library surface across realistic
//! schema shapes: required fields, enum, integer bounds, nested
//! objects, array constraints, and the codegen → validation
//! round-trip with `schemars`-derived schemas.

#![cfg(feature = "validate-schema")]
#![allow(missing_docs)]

use noyalib::{Value, from_str, schema_for, validate_against_schema, validate_against_schema_str};

fn parse(s: &str) -> Value {
    from_str(s).unwrap()
}

#[test]
fn end_to_end_required_and_types() {
    let schema = parse(
        "type: object
required: [port, host]
properties:
  port:
    type: integer
  host:
    type: string
",
    );
    assert!(validate_against_schema(&parse("port: 8080\nhost: localhost\n"), &schema).is_ok());
    assert!(validate_against_schema(&parse("port: 8080\n"), &schema).is_err());
    assert!(validate_against_schema(&parse("port: x\nhost: y\n"), &schema).is_err());
}

#[test]
fn enum_constraint_with_string_values() {
    let schema = parse(
        "type: object
properties:
  level:
    enum: [trace, debug, info, warn, error]
",
    );
    assert!(validate_against_schema(&parse("level: warn\n"), &schema).is_ok());
    assert!(validate_against_schema(&parse("level: ULTRA\n"), &schema).is_err());
}

#[test]
fn integer_bounds_lower_and_upper() {
    let schema = parse(
        "type: object
properties:
  port:
    type: integer
    minimum: 0
    maximum: 65535
",
    );
    assert!(validate_against_schema(&parse("port: 8080\n"), &schema).is_ok());
    assert!(validate_against_schema(&parse("port: 0\n"), &schema).is_ok());
    assert!(validate_against_schema(&parse("port: 65535\n"), &schema).is_ok());
    assert!(validate_against_schema(&parse("port: -1\n"), &schema).is_err());
    assert!(validate_against_schema(&parse("port: 70000\n"), &schema).is_err());
}

#[test]
fn nested_objects_validated_recursively() {
    let schema = parse(
        "type: object
properties:
  db:
    type: object
    required: [host, port]
    properties:
      host: { type: string }
      port: { type: integer }
",
    );
    let good = parse("db:\n  host: localhost\n  port: 5432\n");
    let bad = parse("db:\n  host: localhost\n");
    assert!(validate_against_schema(&good, &schema).is_ok());
    assert!(validate_against_schema(&bad, &schema).is_err());
}

#[test]
fn array_min_items_constraint() {
    let schema = parse(
        "type: object
properties:
  tags:
    type: array
    minItems: 1
    items: { type: string }
",
    );
    assert!(validate_against_schema(&parse("tags: [yaml, serde]\n"), &schema).is_ok());
    assert!(validate_against_schema(&parse("tags: []\n"), &schema).is_err());
}

#[test]
fn validate_against_schema_str_handles_yaml_inputs() {
    let schema = "\
type: object
required: [port]
properties:
  port:
    type: integer
";
    assert!(validate_against_schema_str("port: 8080\n", schema).is_ok());
    assert!(validate_against_schema_str("port: hello\n", schema).is_err());
}

#[test]
fn aggregated_violations_list_all_paths() {
    let schema = parse(
        "type: object
required: [port, host, role]
properties:
  port: { type: integer }
  host: { type: string }
  role: { type: string }
",
    );
    let value = parse("port: not-int\n");
    let err = validate_against_schema(&value, &schema).unwrap_err();
    let msg = err.to_string();
    // Multiple violations expected: type mismatch on port, missing
    // host and role. The aggregated form must list each one.
    assert!(msg.contains("schema violations"), "got: {msg}");
    assert!(msg.contains("port"));
    assert!(msg.contains("host"));
    assert!(msg.contains("role"));
}

#[test]
fn malformed_schema_distinguished_from_data_error() {
    // `type` cannot be a number per JSON Schema 2020-12.
    let schema = parse("type: 99\n");
    let value = parse("any: 1\n");
    let err = validate_against_schema(&value, &schema).unwrap_err();
    assert!(
        err.to_string().contains("not a valid JSON Schema"),
        "got: {err}"
    );
}

// ── codegen ↔ validation round-trip ─────────────────────────────────

#[test]
fn validate_data_against_schemars_emitted_schema() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, noyalib::JsonSchema)]
    #[allow(dead_code)]
    struct ServerConfig {
        port: u16,
        host: String,
        #[serde(default)]
        tls: bool,
    }

    let schema = schema_for::<ServerConfig>().unwrap();

    // Valid: matches the contract.
    let good = parse("port: 8080\nhost: localhost\n");
    assert!(validate_against_schema(&good, &schema).is_ok());

    // Valid: tls is optional via #[serde(default)].
    let good_tls = parse("port: 8443\nhost: api.example.com\ntls: true\n");
    assert!(validate_against_schema(&good_tls, &schema).is_ok());

    // Invalid: missing required `port`.
    let bad_missing = parse("host: localhost\n");
    let err = validate_against_schema(&bad_missing, &schema).unwrap_err();
    assert!(err.to_string().contains("port"));

    // Invalid: port exceeds u16 range.
    let bad_range = parse("port: 70000\nhost: localhost\n");
    let err = validate_against_schema(&bad_range, &schema).unwrap_err();
    assert!(err.to_string().contains("port"));
}
