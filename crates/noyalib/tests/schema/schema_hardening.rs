//! Schema-validation hardening guarantees.
//!
//! The 2026-07-28 Model Context Protocol specification lifts tool
//! `inputSchema` / `outputSchema` to full JSON Schema 2020-12 and
//! requires implementations to refuse auto-dereferencing external
//! `$ref` URIs and to bound schema depth and validation time.
//!
//! noyalib satisfies both today, but by construction rather than by
//! contract: external resolution is off because `jsonschema` is
//! declared `default-features = false`, and the depth bound belongs to
//! that crate. Either could be undone by a dependency bump or a stray
//! feature without anything noticing.
//!
//! These tests turn the accidents into guarantees. If one fails, the
//! property it protects has been lost — do not relax the test.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![cfg(feature = "validate-schema")]

use noyalib::validate_against_schema_str;

/// Build a schema nested `depth` levels deep.
fn nested_schema(depth: usize) -> String {
    let mut s = String::from(r#"{"type":"object","properties":{"a":"#);
    for _ in 0..depth {
        s.push_str(r#"{"type":"object","properties":{"a":"#);
    }
    s.push_str(r#"{"type":"integer"}"#);
    for _ in 0..depth {
        s.push_str("}}");
    }
    s.push_str("}}");
    s
}

#[test]
fn external_http_ref_is_refused_not_fetched() {
    // A remote `$ref` must never cause a network fetch. The address is
    // deliberately unroutable-by-intent: if this ever starts *hanging*
    // rather than erroring, resolution has been switched on.
    let schema = r#"{"$ref": "https://example.com/schema.json"}"#;
    let err = validate_against_schema_str("x: 1", schema)
        .expect_err("an external $ref must not be resolved");
    let msg = err.to_string();
    assert!(
        msg.contains("resolve-http") || msg.contains("not present in a registry"),
        "expected a refusal to resolve externally, got: {msg}"
    );
}

#[test]
fn external_ref_refusal_is_fast() {
    // A network attempt would take orders of magnitude longer than a
    // local refusal. This is the canary for accidental resolution.
    // The load-bearing guard is external_refs_are_not_resolved above,
    // which asserts the refusal *message*: a fetch to example.com can
    // complete quickly, so time alone cannot prove no request left.
    // This budget only catches hangs, and is wide enough that a
    // stalled shared CI runner cannot false-positive it (observed:
    // a 2s budget tripped on a loaded runner with no network involved).
    let schema = r#"{"$ref": "https://example.com/schema.json"}"#;
    let start = std::time::Instant::now();
    let _ = validate_against_schema_str("x: 1", schema);
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(15),
        "refusing an external $ref took {elapsed:?}; that suggests a hang on a network attempt"
    );
}

#[test]
fn deeply_nested_schemas_are_bounded_not_stack_overflowing() {
    // Unbounded recursion here is a denial-of-service vector and, worse,
    // a stack overflow aborts the process rather than returning an
    // error a caller can handle.
    for depth in [500usize, 2_000, 10_000] {
        let start = std::time::Instant::now();
        let result = validate_against_schema_str("a: 1", &nested_schema(depth));
        let elapsed = start.elapsed();

        assert!(
            result.is_err(),
            "depth {depth} must be refused, not accepted"
        );

        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "depth {depth} took {elapsed:?}; the depth bound is not holding"
        );
    }
}

#[test]
fn the_depth_bound_reports_itself() {
    // The error names the limit, so a caller hitting it can tell a
    // bounded refusal from a malformed schema.
    let err = validate_against_schema_str("a: 1", &nested_schema(2_000))
        .expect_err("a 2000-deep schema must be refused");
    let msg = err.to_string();
    assert!(
        msg.contains("recursion depth limit"),
        "expected the refusal to name the depth bound, got: {msg}"
    );
}

#[test]
fn ordinary_schemas_are_unaffected() {
    // The bounds must not cost normal use.
    let schema =
        r#"{"type":"object","properties":{"port":{"type":"integer"}},"required":["port"]}"#;
    validate_against_schema_str("port: 8080", schema).expect("a normal schema still validates");
    let _ = validate_against_schema_str("port: nope", schema)
        .expect_err("a violation is still reported");
}

#[test]
fn local_defs_refs_still_work() {
    // Only *external* refs are refused; `$defs` / local `$ref` are the
    // composition mechanism MCP tool schemas rely on.
    // `r##` because the JSON pointer contains `"#`, which would end an
    // `r#` literal early.
    let schema = r##"{
        "$defs": {"port": {"type": "integer"}},
        "type": "object",
        "properties": {"p": {"$ref": "#/$defs/port"}}
    }"##;
    validate_against_schema_str("p: 8080", schema).expect("local $ref must resolve");
    let _ = validate_against_schema_str("p: text", schema)
        .expect_err("local $ref must still enforce its type");
}
