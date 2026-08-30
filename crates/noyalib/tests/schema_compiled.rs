// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `CompiledSchema` — compile a JSON Schema once, validate many (#329).
//!
//! Pins the four behaviours the issue asked for:
//!
//! 1. one compile serves many validations (no per-call compile);
//! 2. `validate_formats(true)` asserts `format` keywords that
//!    Draft 2020-12 otherwise treats as annotations;
//! 3. a custom format registered through the builder is asserted;
//! 4. `iter_errors` returns every violation with its instance path
//!    and the keyword that raised it.
//!
//! Plus the hardening carry-over: the compiled path refuses an
//! external `$ref` exactly as `validate_against_schema` does,
//! because the latter is now a compile-then-validate through
//! `CompiledSchema`.

#![cfg(feature = "validate-schema")]

use noyalib::{CompiledSchema, Value, from_str, validate_against_schema};

fn parse(s: &str) -> Value {
    from_str(s).unwrap()
}

#[test]
fn compile_once_validates_many() {
    let schema = parse(
        "type: object
required: [port]
properties:
  port:
    type: integer
    minimum: 0
    maximum: 65535
",
    );
    let compiled = CompiledSchema::compile(&schema).unwrap();
    for port in 0..100 {
        let v = parse(&format!("port: {port}\n"));
        compiled.validate(&v).unwrap();
    }
    let bad = parse("port: 70000\n");
    assert!(compiled.validate(&bad).is_err());
}

#[test]
fn compiled_and_uncompiled_agree() {
    let schema = parse("type: object\nrequired: [host]\nproperties:\n  port: {type: integer}\n");
    let compiled = CompiledSchema::compile(&schema).unwrap();
    for doc in ["host: a\n", "port: x\n", "host: a\nport: 1\n", "{}\n"] {
        let v = parse(doc);
        assert_eq!(
            compiled.validate(&v).is_ok(),
            validate_against_schema(&v, &schema).is_ok(),
            "diverged on {doc:?}"
        );
    }
}

#[test]
fn format_is_annotation_only_by_default() {
    // Draft 2020-12: `format` does not assert unless asked to.
    let schema = parse("type: object\nproperties:\n  date: {type: string, format: date}\n");
    let bad_date = parse("date: 01/15/2024\n");
    let compiled = CompiledSchema::compile(&schema).unwrap();
    assert!(compiled.validate(&bad_date).is_ok());
    // The one-shot function keeps the same default.
    assert!(validate_against_schema(&bad_date, &schema).is_ok());
}

#[test]
fn validate_formats_asserts_the_date_format() {
    let schema = parse("type: object\nproperties:\n  date: {type: string, format: date}\n");
    let compiled = CompiledSchema::builder(&schema)
        .validate_formats(true)
        .build()
        .unwrap();
    assert!(compiled.validate(&parse("date: 2026-01-15\n")).is_ok());
    let err = compiled.validate(&parse("date: 01/15/2024\n")).unwrap_err();
    assert!(err.to_string().contains("/date"), "got: {err}");
}

#[test]
fn custom_format_registered_through_the_builder_is_asserted() {
    let schema = parse("type: object\nproperties:\n  slug: {type: string, format: kebab-slug}\n");
    let compiled = CompiledSchema::builder(&schema)
        .validate_formats(true)
        .with_format("kebab-slug", |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
        .build()
        .unwrap();
    assert!(compiled.validate(&parse("slug: getting-started\n")).is_ok());
    assert!(
        compiled
            .validate(&parse("slug: Getting Started\n"))
            .is_err()
    );
}

#[test]
fn iter_errors_returns_every_violation_with_paths_and_keywords() {
    let schema = parse(
        "type: object
required: [host]
properties:
  port: {type: integer}
  tags:
    type: array
    items: {type: string}
",
    );
    let compiled = CompiledSchema::compile(&schema).unwrap();
    let bad = parse("port: x\ntags: [ok, 3]\n");
    let violations = compiled.iter_errors(&bad).unwrap();
    assert_eq!(violations.len(), 3, "got: {violations:?}");
    assert!(
        violations
            .iter()
            .any(|v| v.instance_path == "/port" && v.keyword == "type")
    );
    assert!(
        violations
            .iter()
            .any(|v| v.instance_path == "/tags/1" && v.keyword == "type")
    );
    assert!(violations.iter().any(|v| v.keyword == "required"));
    for v in &violations {
        assert!(!v.message.is_empty());
    }
}

#[test]
fn iter_errors_empty_on_conforming_instance() {
    let schema = parse("type: object\n");
    let compiled = CompiledSchema::compile(&schema).unwrap();
    assert!(compiled.iter_errors(&parse("a: 1\n")).unwrap().is_empty());
}

#[test]
fn invalid_schema_is_a_compile_error_not_a_validate_error() {
    let schema = parse("type: 42\n");
    let err = CompiledSchema::compile(&schema).unwrap_err();
    assert!(
        err.to_string().contains("not a valid JSON Schema"),
        "got: {err}"
    );
}

#[test]
fn external_ref_is_refused_on_the_compiled_path() {
    // Hardening carry-over: same refusal `tests/schema_hardening.rs`
    // pins for `validate_against_schema`.
    let schema = parse("$ref: \"https://example.com/schema.json\"\n");
    let compiled = CompiledSchema::compile(&schema);
    match compiled {
        Err(_) => {}
        Ok(c) => {
            // Some jsonschema versions defer resolution to validate
            // time; either way the reference must not be fetched and
            // the validation must fail.
            assert!(c.validate(&parse("a: 1\n")).is_err());
        }
    }
}

#[test]
fn builder_debug_lists_custom_format_names_only() {
    let schema = parse("type: object\n");
    let b = CompiledSchema::builder(&schema)
        .validate_formats(true)
        .with_format("kebab-slug", |_| true);
    let dbg = format!("{b:?}");
    assert!(dbg.contains("kebab-slug"), "got: {dbg}");
}
