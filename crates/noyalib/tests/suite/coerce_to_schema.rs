// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Schema-driven type coercion — `noyalib::coerce_to_schema`.
//!
//! Verifies the surgical-fix surface that maps JSON Schema
//! type-mismatch errors back into in-place `Value` mutations.
//! Pairs with `validate_against_schema`: run coercion first, then
//! validate, and the residue (if any) is the set of violations
//! that *cannot* be auto-fixed.

#![cfg(feature = "validate-schema")]
#![allow(missing_docs)]

use noyalib::{Value, coerce_to_schema, from_str, validate_against_schema};

fn parse(s: &str) -> Value {
    from_str(s).unwrap()
}

#[test]
fn string_coerced_to_integer_when_schema_requires_it() {
    let schema = parse("type: object\nproperties:\n  port:\n    type: integer\n");
    let mut data = parse("port: \"8080\"\n");
    let n = coerce_to_schema(&mut data, &schema).unwrap();
    assert_eq!(n, 1, "exactly one fix expected");
    validate_against_schema(&data, &schema).unwrap();
}

#[test]
fn string_coerced_to_number() {
    let schema = parse("type: object\nproperties:\n  ratio:\n    type: number\n");
    let mut data = parse("ratio: \"3.14\"\n");
    let n = coerce_to_schema(&mut data, &schema).unwrap();
    assert_eq!(n, 1);
    validate_against_schema(&data, &schema).unwrap();
}

#[test]
fn string_coerced_to_boolean() {
    let schema = parse("type: object\nproperties:\n  enabled:\n    type: boolean\n");
    let mut data = parse("enabled: \"true\"\n");
    let n = coerce_to_schema(&mut data, &schema).unwrap();
    assert_eq!(n, 1);
    validate_against_schema(&data, &schema).unwrap();
}

#[test]
fn unparseable_string_is_left_in_place() {
    let schema = parse("type: object\nproperties:\n  port:\n    type: integer\n");
    let mut data = parse("port: \"not-a-number\"\n");
    let n = coerce_to_schema(&mut data, &schema).unwrap();
    assert_eq!(n, 0, "no fix when the string can't be parsed");
    // The original violation persists.
    assert!(validate_against_schema(&data, &schema).is_err());
}

#[test]
fn nested_object_coercion() {
    let schema = parse(
        "type: object
properties:
  db:
    type: object
    properties:
      port:
        type: integer
",
    );
    let mut data = parse("db:\n  port: \"5432\"\n");
    let n = coerce_to_schema(&mut data, &schema).unwrap();
    assert_eq!(n, 1);
    validate_against_schema(&data, &schema).unwrap();
}

#[test]
fn sequence_item_coercion() {
    let schema = parse(
        "type: object
properties:
  ports:
    type: array
    items:
      type: integer
",
    );
    let mut data = parse("ports:\n  - \"80\"\n  - \"443\"\n  - \"8080\"\n");
    let n = coerce_to_schema(&mut data, &schema).unwrap();
    assert_eq!(n, 3, "all three sequence items should coerce");
    validate_against_schema(&data, &schema).unwrap();
}

#[test]
fn coerce_returns_zero_when_already_valid() {
    let schema = parse("type: object\nproperties:\n  port:\n    type: integer\n");
    let mut data = parse("port: 8080\n");
    let n = coerce_to_schema(&mut data, &schema).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn mixed_valid_and_invalid_coercions() {
    let schema = parse(
        "type: object
properties:
  port:
    type: integer
  host:
    type: string
",
    );
    // `port` is fixable; `host` is already a string and stays as-is.
    let mut data = parse("port: \"80\"\nhost: localhost\n");
    let n = coerce_to_schema(&mut data, &schema).unwrap();
    assert_eq!(n, 1);
    validate_against_schema(&data, &schema).unwrap();
}
