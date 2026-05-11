// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 3.1 — JSON Schema codegen via `schemars`.
//!
//! End-to-end checks for the public surface: derive `JsonSchema`
//! on a real Rust type, generate the schema as `Value` and as
//! YAML, verify round-trip equality, and confirm that the
//! standard schemars / serde attributes (`#[doc]`, `#[serde]`)
//! propagate as documented.

#![cfg(feature = "schema")]
#![allow(missing_docs)]

use noyalib::{JsonSchema, Value, from_str, schema_for, schema_for_yaml};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ServerConfig {
    /// Port the server binds on.
    port: u16,
    /// Hostname or IP literal.
    host: String,
    #[serde(default)]
    tls: bool,
}

#[test]
fn schema_for_returns_object_shaped_value() {
    let schema = schema_for::<ServerConfig>().unwrap();
    assert_eq!(schema["type"].as_str(), Some("object"));
    assert_eq!(schema["title"].as_str(), Some("ServerConfig"));
}

#[test]
fn schema_yaml_round_trips_through_value() {
    let yaml = schema_for_yaml::<ServerConfig>().unwrap();
    let parsed: Value = from_str(&yaml).unwrap();
    let direct = schema_for::<ServerConfig>().unwrap();
    assert_eq!(parsed, direct);
}

#[test]
fn schema_yaml_is_valid_yaml() {
    // The emitted schema is itself a YAML document — it must
    // parse back successfully (no exotic constructs that need
    // schema-aware deserialization).
    let yaml = schema_for_yaml::<ServerConfig>().unwrap();
    let _: Value = from_str(&yaml).unwrap();
}

#[test]
fn doc_strings_become_descriptions() {
    let schema = schema_for::<ServerConfig>().unwrap();
    assert_eq!(
        schema["properties"]["port"]["description"].as_str(),
        Some("Port the server binds on."),
    );
    assert_eq!(
        schema["properties"]["host"]["description"].as_str(),
        Some("Hostname or IP literal."),
    );
}

#[test]
fn serde_default_drops_field_from_required() {
    let schema = schema_for::<ServerConfig>().unwrap();
    let required: Vec<&str> = match &schema["required"] {
        Value::Sequence(s) => s.iter().filter_map(Value::as_str).collect(),
        other => panic!("required must be a sequence, got {other:?}"),
    };
    assert!(required.contains(&"port"));
    assert!(required.contains(&"host"));
    assert!(!required.contains(&"tls"));
}

#[test]
fn integer_bounds_are_emitted_for_fixed_width_ints() {
    let schema = schema_for::<ServerConfig>().unwrap();
    let port = &schema["properties"]["port"];
    assert_eq!(port["minimum"].as_i64(), Some(0));
    assert_eq!(port["maximum"].as_i64(), Some(65_535));
}

// ── Nested types ────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct Database {
    host: String,
    port: u16,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct App {
    name: String,
    db: Database,
}

#[test]
fn nested_types_are_emitted_via_definitions() {
    let schema = schema_for::<App>().unwrap();
    // schemars 1.x uses `$defs` for the JSON Schema 2020-12
    // standard definitions slot.
    let defs = &schema["$defs"];
    let db = &defs["Database"];
    assert_eq!(db["type"].as_str(), Some("object"));
    assert!(matches!(db["properties"]["host"], Value::Mapping(_)));
}

// ── Enums ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
#[serde(rename_all = "lowercase")]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

// ── JsonSchema for noyalib::Value ───────────────────────────────────

#[test]
fn json_schema_for_value_field() {
    use noyalib::Value;

    #[derive(Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct Envelope {
        id: String,
        payload: Value,
    }

    let schema = schema_for::<Envelope>().unwrap();
    assert_eq!(schema["type"].as_str(), Some("object"));
    // `payload` is `Value` — schemars resolves it to a `$ref`
    // pointing at the `YamlValue` definition under `$defs`.
    assert!(matches!(schema["properties"]["payload"], Value::Mapping(_)));
    let defs = &schema["$defs"];
    let yaml_value = &defs["YamlValue"];
    // The Value schema is a oneOf union of every JSON-expressible
    // shape.
    assert!(matches!(yaml_value["oneOf"], Value::Sequence(_)));
}

#[test]
fn unit_only_enum_emits_string_enum() {
    let schema = schema_for::<LogLevel>().unwrap();
    let variants: Vec<&str> = match &schema["enum"] {
        Value::Sequence(s) => s.iter().filter_map(Value::as_str).collect(),
        other => panic!("expected enum array, got {other:?}"),
    };
    assert!(variants.contains(&"trace"));
    assert!(variants.contains(&"warn"));
    assert!(variants.contains(&"error"));
}
