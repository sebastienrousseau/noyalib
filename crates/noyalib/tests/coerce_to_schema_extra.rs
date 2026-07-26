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

// ── Coverage: value_path_from_pointer / coerce_logical edge arms ──────

#[test]
fn coerce_nested_pointer_builds_dotted_path() {
    // A nested property coercion drives value_path_from_pointer's
    // multi-segment branch (the `i > 0` dot separator).
    let yaml = "server:\n  port: \"5432\"\n";
    let schema = r#"
type: object
properties:
  server:
    type: object
    properties:
      port: { type: integer }
"#;
    let (n, after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 1, "nested port should coerce");
    assert!(after.contains("port: 5432"), "{after}");
}

#[test]
fn coerce_root_type_error_is_skipped() {
    // A bare scalar document that the schema wants to be an object:
    // the type error's instance path is the root (empty JSON pointer),
    // so `parse_json_pointer` yields no segments and
    // `value_path_from_pointer` returns None — the target is skipped,
    // not coerced.
    let yaml = "just a scalar\n";
    let schema = "type: object\n";
    let (n, _after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 0, "root-level type error is not coercible");
}

#[test]
fn coerce_uncoercible_target_type_is_skipped() {
    // The addressed scalar exists, but the schema wants a *container*
    // (object) there — `coerce_logical` has no scalar-to-object rule,
    // so it returns None and the value is left in place.
    let yaml = "x: hello\n";
    let schema = "type: object\nproperties:\n  x:\n    type: object\n";
    let (n, after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 0, "string cannot be coerced to object");
    assert!(after.contains("x: hello"), "{after}");
}

#[test]
fn coerce_single_quoted_scalar_is_unwrapped() {
    // A single-quoted numeric string exercises strip_quotes' quote
    // branch before the integer parse.
    let yaml = "port: '8080'\n";
    let schema = "type: object\nproperties:\n  port: { type: integer }\n";
    let (n, after) = run_coerce(yaml, schema).unwrap();
    assert_eq!(n, 1);
    assert!(after.contains("port: 8080"), "{after}");
}
