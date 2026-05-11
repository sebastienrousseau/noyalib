// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Additional `cst::coerce_to_schema` tests targeting branches
//! not exercised by the headline `tests/coerce_to_schema.rs`
//! suite. Pushes per-method coverage on `cst/coerce.rs` from
//! 60 % function / 71 % region to the workspace 92 % gate.

#![cfg(feature = "validate-schema")]
#![allow(missing_docs)]

use noyalib::cst::{coerce_to_schema, parse_document};
use noyalib::{Value, from_str};

fn run_coerce(yaml: &str, schema_yaml: &str) -> Result<(usize, String), noyalib::Error> {
    let schema: Value = from_str(schema_yaml)?;
    let mut doc = parse_document(yaml)?;
    let n = coerce_to_schema(&mut doc, &schema)?;
    Ok((n, doc.to_string()))
}

#[test]
fn coerce_number_target_uses_ryu_format() {
    // Quoted scalar that the schema wants as a `number` (float).
    let yaml = "x: \"1.5\"\n";
    let schema = "type: object\nproperties:\n  x: { type: number }\n";
    let (n, after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 1);
    assert!(after.contains("1.5"));
    assert!(!after.contains("\"1.5\""));
}

#[test]
fn coerce_boolean_true() {
    let yaml = "active: \"true\"\n";
    let schema = "type: object\nproperties:\n  active: { type: boolean }\n";
    let (n, after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 1);
    assert!(after.contains("active: true"));
}

#[test]
fn coerce_boolean_false() {
    let yaml = "active: \"false\"\n";
    let schema = "type: object\nproperties:\n  active: { type: boolean }\n";
    let (n, after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 1);
    assert!(after.contains("active: false"));
}

#[test]
fn coerce_boolean_non_boolean_string_skipped() {
    let yaml = "active: \"yes\"\n";
    let schema = "type: object\nproperties:\n  active: { type: boolean }\n";
    let (n, _after) = run_coerce(yaml, schema).unwrap();
    // `yes` is not a JSON Schema boolean — skipped (None branch).
    assert_eq!(n, 0);
}

#[test]
fn coerce_string_target_is_no_op() {
    // String → string is not coercible (no transformation needed).
    let yaml = "name: \"alice\"\n";
    let schema = "type: object\nproperties:\n  name: { type: string }\n";
    let (n, _after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn coerce_invalid_integer_skipped() {
    // Quoted "abc" can't parse as integer — skipped.
    let yaml = "port: \"abc\"\n";
    let schema = "type: object\nproperties:\n  port: { type: integer }\n";
    let (n, _after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn coerce_inside_sequence_index() {
    let yaml = "items:\n  - \"1\"\n  - \"2\"\n";
    let schema =
        "type: object\nproperties:\n  items:\n    type: array\n    items: { type: integer }\n";
    let (n, after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 2);
    assert!(after.contains("- 1"));
    assert!(after.contains("- 2"));
}

#[test]
fn coerce_single_quoted_input() {
    let yaml = "x: '42'\n";
    let schema = "type: object\nproperties:\n  x: { type: integer }\n";
    let (n, after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 1);
    assert!(after.contains("x: 42"));
}

#[test]
fn coerce_unquoted_string_value_skipped() {
    // Already-typed scalar (unquoted `8080` → integer) — no
    // coercion needed.
    let yaml = "port: 8080\n";
    let schema = "type: object\nproperties:\n  port: { type: integer }\n";
    let (n, _after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn coerce_invalid_schema_returns_error() {
    let yaml = "x: 1\n";
    // Schema with a contradictory `type` keyword.
    let schema = "type: not-a-type\n";
    let res = run_coerce(yaml, schema);
    assert!(res.is_err());
}

#[test]
fn coerce_no_op_when_schema_matches() {
    let yaml = "port: 8080\nactive: true\n";
    let schema = r#"
type: object
properties:
  port: { type: integer }
  active: { type: boolean }
"#;
    let (n, after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 0);
    assert!(after.contains("port: 8080"));
}
