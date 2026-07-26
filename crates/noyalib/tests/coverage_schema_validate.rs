// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for `schema_validate.rs`'s `coerce_to_schema` path — the
//! invalid-schema guard, the JSON-pointer / navigate / try_coerce
//! helpers, and each coercion target arm. The in-crate unit tests only
//! exercise `validate_against_schema`, leaving the coercion machinery
//! (which drives the `coerce_to_schema` public API) untested.

// `coerce_to_schema` / `validate_against_schema` live behind the
// `schema` feature and need `jsonschema`; the `validate-schema` feature
// enables both. Without this gate the file fails to compile under the
// default feature set (the `cargo test --tests (defaults)` CI leg).
#![cfg(feature = "validate-schema")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::{Value, coerce_to_schema, from_str};

fn parse(s: &str) -> Value {
    from_str(s).unwrap()
}

#[test]
fn coerce_rejects_invalid_schema() {
    // `type: 42` is representable as JSON but is not a valid JSON
    // Schema, so `validator_for` fails — the coerce-side schema guard.
    let schema = parse("type: 42\n");
    let mut data = parse("x: 1\n");
    let err = coerce_to_schema(&mut data, &schema).unwrap_err();
    assert!(
        err.to_string().contains("not a valid JSON Schema"),
        "expected schema-side error, got: {err}"
    );
}

#[test]
fn coerce_string_to_boolean_true_and_false() {
    let schema = parse("type: object\nproperties:\n  flag:\n    type: boolean\n");

    let mut t = parse("flag: \"true\"\n");
    assert_eq!(coerce_to_schema(&mut t, &schema).unwrap(), 1);
    assert_eq!(t["flag"].as_bool(), Some(true));

    let mut f = parse("flag: \"false\"\n");
    assert_eq!(coerce_to_schema(&mut f, &schema).unwrap(), 1);
    assert_eq!(f["flag"].as_bool(), Some(false));
}

#[test]
fn coerce_string_to_number() {
    let schema = parse("type: object\nproperties:\n  ratio:\n    type: number\n");
    let mut data = parse("ratio: \"3.5\"\n");
    assert_eq!(coerce_to_schema(&mut data, &schema).unwrap(), 1);
    assert_eq!(data["ratio"].as_f64(), Some(3.5));
}

#[test]
fn coerce_declines_non_boolean_string() {
    // A string that is neither "true" nor "false" is left untouched
    // (the boolean coercion arm returns `None`).
    let schema = parse("type: object\nproperties:\n  flag:\n    type: boolean\n");
    let mut data = parse("flag: \"maybe\"\n");
    assert_eq!(coerce_to_schema(&mut data, &schema).unwrap(), 0);
    assert_eq!(data["flag"].as_str(), Some("maybe"));
}

#[test]
fn coerce_declines_non_string_node() {
    // A non-string node (integer) where a string is expected: the
    // coercion only fires string -> scalar, so the mismatch is left
    // for validation to report rather than coerced.
    let schema = parse("type: object\nproperties:\n  name:\n    type: string\n");
    let mut data = parse("name: 42\n");
    assert_eq!(coerce_to_schema(&mut data, &schema).unwrap(), 0);
    assert!(data["name"].as_i64() == Some(42));
}

#[test]
fn coerce_declines_uncoercible_target_at_root() {
    // Root-level type error: schema expects an object but the document
    // is a bare scalar. The error's instance path is empty (root), and
    // the target type (object) is not one of the coercible scalar
    // targets — so nothing is applied.
    let schema = parse("type: object\n");
    let mut data = parse("just a scalar\n");
    assert_eq!(coerce_to_schema(&mut data, &schema).unwrap(), 0);
    assert_eq!(data.as_str(), Some("just a scalar"));
}

#[test]
fn coerce_multiple_fields_in_one_pass() {
    // Several string-typed values coerced to their declared scalar
    // types, exercising the multi-target fix loop.
    let schema = parse(
        "type: object
properties:
  port:
    type: integer
  ratio:
    type: number
  on:
    type: boolean
",
    );
    let mut data = parse("port: \"8080\"\nratio: \"0.5\"\non: \"true\"\n");
    let fixes = coerce_to_schema(&mut data, &schema).unwrap();
    assert_eq!(fixes, 3, "all three should coerce");
    assert_eq!(data["port"].as_i64(), Some(8080));
    assert_eq!(data["ratio"].as_f64(), Some(0.5));
    assert_eq!(data["on"].as_bool(), Some(true));
    // Re-validation now succeeds.
    noyalib::validate_against_schema(&data, &schema).unwrap();
}

#[test]
fn coerce_nested_pointer_navigates_into_object() {
    // A nested property coercion exercises the JSON-pointer navigation
    // through a mapping level.
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
    assert_eq!(coerce_to_schema(&mut data, &schema).unwrap(), 1);
    assert_eq!(data["db"]["port"].as_i64(), Some(5432));
}
