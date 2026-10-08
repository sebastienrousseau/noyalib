//! Schema-validation hardening guarantees.
//!
//! The 2026-07-28 Model Context Protocol specification lifts tool
//! `inputSchema` / `outputSchema` to full JSON Schema 2020-12 and
//! requires implementations to refuse auto-dereferencing external
//! `$ref` URIs and to bound schema depth and validation time.
//!
//! External resolution is refused by an explicit retriever installed on
//! every validator. The test build enables `jsonschema`'s
//! `resolve-file` feature on purpose, mirroring a consumer whose build
//! unifies it in, so the refusal is proven under the hostile
//! configuration rather than the default one. The depth bound belongs
//! to the parser and to `jsonschema`, and could be undone by a
//! dependency bump without anything noticing.
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

/// A `file:` URL for `path`, written with `/` so it is valid in JSON and
/// in a double-quoted YAML string on Windows too (`C:\Users` would read
/// as a `\U` escape there).
fn file_url(path: &std::path::Path) -> String {
    let p = path.display().to_string().replace('\\', "/");
    if p.starts_with('/') {
        format!("file://{p}")
    } else {
        format!("file:///{p}")
    }
}

#[test]
fn file_ref_is_refused_and_contents_never_leak() {
    // The dev-dependency on `jsonschema` with `resolve-file` mirrors a
    // consumer whose build unifies that feature in. The refusal must
    // hold anyway, and the referenced file's contents must not reach
    // the error text.
    let dir = std::env::temp_dir().join(format!("noyalib-schema-ref-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("leak.json");
    std::fs::write(&target, r#"{"const":"TOPSECRET-4242"}"#).unwrap();
    let schema = format!("{{\"$ref\": \"{}\"}}", file_url(&target));
    let result = validate_against_schema_str("a: 1", &schema);
    let _ = std::fs::remove_dir_all(&dir);
    let msg = result
        .expect_err("a file:// $ref must be refused")
        .to_string();
    assert!(
        !msg.contains("TOPSECRET-4242"),
        "file contents leaked through the error: {msg}"
    );
    assert!(
        msg.contains("external") || msg.contains("not present in a registry"),
        "expected a refusal to resolve externally, got: {msg}"
    );
}

#[test]
fn file_ref_via_id_base_is_refused() {
    let dir = std::env::temp_dir().join(format!("noyalib-schema-id-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.json"), r#"{"const":"TOPSECRET-77"}"#).unwrap();
    let schema = format!("{{\"$id\": \"{}/\", \"$ref\": \"t.json\"}}", file_url(&dir));
    let result = validate_against_schema_str("a: 1", &schema);
    let _ = std::fs::remove_dir_all(&dir);
    let msg = result
        .expect_err("a relative file $ref must be refused")
        .to_string();
    assert!(!msg.contains("TOPSECRET-77"), "leaked: {msg}");
}

#[test]
fn coerce_to_schema_refuses_file_refs_too() {
    let dir = std::env::temp_dir().join(format!("noyalib-schema-coerce-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("t.json");
    std::fs::write(
        &target,
        r#"{"type":"object","properties":{"p":{"type":"integer"}}}"#,
    )
    .unwrap();
    let schema: noyalib::Value =
        noyalib::from_str(&format!("$ref: \"{}\"\n", file_url(&target))).unwrap();
    let mut data: noyalib::Value = noyalib::from_str("p: \"1\"\n").unwrap();
    let result = noyalib::coerce_to_schema(&mut data, &schema);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        result.is_err(),
        "a file:// $ref must be refused: {result:?}"
    );
}

#[test]
fn backtracking_pattern_is_bounded() {
    // A lookaround pattern over many failing items is the classic
    // catastrophic-backtracking shape. It must either be refused at
    // compile time or finish quickly.
    let schema = "type: array\nitems:\n  type: string\n  pattern: '^(\\w+\\s?)*$(?<=x)'\n";
    let item = format!("{}!", "a".repeat(40));
    let items: String = (0..200).map(|_| format!("- \"{item}\"\n")).collect();
    let start = std::time::Instant::now();
    let _ = validate_against_schema_str(&items, schema);
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "200 items took {elapsed:?}"
    );
}

#[test]
fn error_report_is_bounded() {
    // Many failing subschemas against one large instance must not
    // multiply into an error string the size of both.
    use noyalib::{CompiledSchema, Value};
    let mut schema = String::from("allOf:\n");
    for _ in 0..1_000 {
        schema.push_str("  - type: integer\n");
    }
    let schema: Value = noyalib::from_str(&schema).unwrap();
    let instance = Value::String("x".repeat(1024 * 1024));
    let compiled = CompiledSchema::compile(&schema).unwrap();
    let msg = compiled.validate(&instance).unwrap_err().to_string();
    assert!(
        msg.len() <= 128 * 1024,
        "error report is {} bytes",
        msg.len()
    );
    assert!(msg.contains("1000 total"), "the total is still reported");
    assert!(msg.contains("more not shown"), "the cut is visible");
    let violations = compiled.iter_errors(&instance).unwrap();
    let total: usize = violations.iter().map(|v| v.message.len()).sum();
    assert!(total <= 128 * 1024, "violations carry {total} bytes");
}

#[test]
fn error_caps_are_configurable() {
    use noyalib::{CompiledSchema, Value};
    let schema: Value = noyalib::from_str("type: array\nitems: {type: integer}\n").unwrap();
    let instance: Value = noyalib::from_str("[a, b, c, d, e]").unwrap();
    let compiled = CompiledSchema::builder(&schema)
        .max_errors(2)
        .build()
        .unwrap();
    assert_eq!(compiled.iter_errors(&instance).unwrap().len(), 2);
    let msg = compiled.validate(&instance).unwrap_err().to_string();
    assert!(
        msg.contains("5 total") && msg.contains("3 more not shown"),
        "{msg}"
    );
    let compiled = CompiledSchema::builder(&schema)
        .max_error_bytes(0)
        .build()
        .unwrap();
    assert_eq!(
        compiled.iter_errors(&instance).unwrap(),
        Vec::<noyalib::SchemaViolation>::new()
    );
}

#[test]
fn oversized_schemas_are_refused() {
    use noyalib::{CompiledSchema, Value};
    let schema: Value = noyalib::from_str("allOf: [{type: integer}, {type: integer}]\n").unwrap();
    let err = CompiledSchema::builder(&schema)
        .max_schema_nodes(3)
        .build()
        .unwrap_err();
    assert!(err.to_string().contains("nodes"), "{err}");
    let err = CompiledSchema::builder(&schema)
        .max_schema_depth(1)
        .build()
        .unwrap_err();
    assert!(err.to_string().contains("recursion depth limit"), "{err}");
    assert!(CompiledSchema::compile(&schema).is_ok());
}

#[test]
fn lookaround_patterns_need_an_explicit_opt_in() {
    use noyalib::{CompiledSchema, Value};
    let schema: Value = noyalib::from_str("type: string\npattern: '^(?!tmp)'\n").unwrap();
    let err = CompiledSchema::compile(&schema).unwrap_err().to_string();
    assert!(err.contains("not a valid JSON Schema"), "{err}");
    let compiled = CompiledSchema::builder(&schema)
        .backtracking_patterns(10_000)
        .build()
        .unwrap();
    assert!(compiled.validate(&Value::from("ok")).is_ok());
    assert!(compiled.validate(&Value::from("tmpfile")).is_err());
    // Linear-time patterns keep working with no opt-in.
    let schema: Value = noyalib::from_str("type: string\npattern: '^[a-z]+$'\n").unwrap();
    let compiled = CompiledSchema::compile(&schema).unwrap();
    assert!(compiled.validate(&Value::from("abc")).is_ok());
    assert!(compiled.validate(&Value::from("ABC")).is_err());
}

#[test]
fn backtracking_opt_in_is_bounded_by_its_limit() {
    // With the opt-in, the step limit is what bounds the work: each
    // item stops after at most 10,000 steps.
    use noyalib::{CompiledSchema, Value};
    let schema: Value = noyalib::from_str(
        "type: array\nitems:\n  type: string\n  pattern: '^(\\w+\\s?)*$(?<=x)'\n",
    )
    .unwrap();
    let compiled = CompiledSchema::builder(&schema)
        .backtracking_patterns(10_000)
        .build()
        .unwrap();
    let item = Value::from(format!("{}!", "a".repeat(40)));
    let instance = Value::Sequence((0..200).map(|_| item.clone()).collect());
    let msg = compiled.validate(&instance).unwrap_err().to_string();
    assert!(msg.contains("200 total"), "{msg}");
}

#[test]
fn cst_coerce_to_schema_refuses_file_refs_too() {
    let dir = std::env::temp_dir().join(format!("noyalib-schema-cst-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("t.json");
    std::fs::write(
        &target,
        r#"{"type":"object","properties":{"p":{"type":"integer"}}}"#,
    )
    .unwrap();
    let schema: noyalib::Value =
        noyalib::from_str(&format!("$ref: \"{}\"\n", file_url(&target))).unwrap();
    let mut doc = noyalib::cst::parse_document("p: \"1\"\n").unwrap();
    let result = noyalib::cst::coerce_to_schema(&mut doc, &schema);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        result.is_err(),
        "a file:// $ref must be refused: {result:?}"
    );
}

#[test]
fn coerce_paths_refuse_oversized_schemas() {
    let mut value: noyalib::Value = noyalib::from_str("a: 1\n").unwrap();
    let mut nested = String::from("x: ");
    for _ in 0..200 {
        nested.push_str("{x: ");
    }
    nested.push('1');
    for _ in 0..200 {
        nested.push('}');
    }
    let deep: noyalib::Value =
        noyalib::from_str_with_config(&nested, &noyalib::ParserConfig::new().max_depth(512))
            .unwrap();
    let err = noyalib::coerce_to_schema(&mut value, &deep).unwrap_err();
    assert!(err.to_string().contains("recursion depth limit"), "{err}");
    let mut doc = noyalib::cst::parse_document("a: 1\n").unwrap();
    let err = noyalib::cst::coerce_to_schema(&mut doc, &deep).unwrap_err();
    assert!(err.to_string().contains("recursion depth limit"), "{err}");
}
