// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Cross-crate Value interop: noyalib::Value <-> serde_json::Value.
//!
//! Demonstrates direct transcoding between YAML and JSON value types
//! without going through intermediate strings or structs.
//!
//! Run: `cargo run --example transcode`

#[path = "support/mod.rs"]
mod support;

use noyalib::Value as YamlValue;
use serde_json::Value as JsonValue;

/// Convert noyalib::Value to serde_json::Value directly.
fn yaml_to_json(v: &YamlValue) -> JsonValue {
    match v {
        YamlValue::Null => JsonValue::Null,
        YamlValue::Bool(b) => JsonValue::Bool(*b),
        YamlValue::Number(n) => {
            let f = n.as_f64();
            if f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
                serde_json::json!(f as i64)
            } else {
                serde_json::json!(f)
            }
        }
        YamlValue::String(s) => JsonValue::String(s.clone()),
        YamlValue::Sequence(seq) => JsonValue::Array(seq.iter().map(yaml_to_json).collect()),
        YamlValue::Mapping(map) => {
            let obj: serde_json::Map<String, JsonValue> = map
                .iter()
                .map(|(k, v)| (k.clone(), yaml_to_json(v)))
                .collect();
            JsonValue::Object(obj)
        }
        YamlValue::Tagged(t) => yaml_to_json(t.value()),
    }
}

/// Convert serde_json::Value to noyalib::Value directly.
fn json_to_yaml(v: &JsonValue) -> YamlValue {
    match v {
        JsonValue::Null => YamlValue::Null,
        JsonValue::Bool(b) => YamlValue::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                YamlValue::from(i)
            } else {
                YamlValue::from(n.as_f64().unwrap_or(0.0))
            }
        }
        JsonValue::String(s) => YamlValue::String(s.clone()),
        JsonValue::Array(arr) => YamlValue::Sequence(arr.iter().map(json_to_yaml).collect()),
        JsonValue::Object(obj) => {
            let mut map = noyalib::Mapping::with_capacity(obj.len());
            for (k, v) in obj {
                let _ = map.insert(k.clone(), json_to_yaml(v));
            }
            YamlValue::Mapping(map)
        }
    }
}

fn main() {
    support::header("noyalib -- transcode");

    // ── YAML -> JSON (direct) ────────────────────────────────────────
    support::task_with_output("YAML Value -> JSON Value (direct)", || {
        let yaml: YamlValue =
            noyalib::from_str("name: noyalib\nversion: 1\ntags:\n  - yaml\n  - rust\n").unwrap();
        let json = yaml_to_json(&yaml);
        let formatted = serde_json::to_string_pretty(&json).unwrap();
        formatted.lines().map(|l| l.to_string()).collect()
    });

    // ── JSON -> YAML (direct) ────────────────────────────────────────
    support::task_with_output("JSON Value -> YAML Value (direct)", || {
        let json: JsonValue = serde_json::from_str(
            r#"{"host": "localhost", "port": 8080, "features": ["auth", "api"]}"#,
        )
        .unwrap();
        let yaml = json_to_yaml(&json);
        let formatted = noyalib::to_string(&yaml).unwrap();
        formatted.lines().map(|l| l.to_string()).collect()
    });

    // ── Roundtrip: YAML -> JSON -> YAML ──────────────────────────────
    support::task_with_output("Roundtrip: YAML -> JSON -> YAML", || {
        let original: YamlValue =
            noyalib::from_str("host: localhost\nport: 8080\ndebug: true\n").unwrap();
        let json = yaml_to_json(&original);
        let back = json_to_yaml(&json);
        vec![
            format!(
                "host  = {}",
                back.get("host").and_then(|v| v.as_str()).unwrap_or("?")
            ),
            format!(
                "port  = {}",
                back.get("port").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            format!("match = {}", original == back),
        ]
    });

    // ── Serde-based transcoding (alternative) ────────────────────────
    support::task_with_output("Serde-based transcoding (via serialize)", || {
        let yaml: YamlValue = noyalib::from_str("name: test\ncount: 42\n").unwrap();

        // YAML -> JSON via serde serialization (no custom function)
        let json_str = serde_json::to_string(&yaml).unwrap();
        let json: JsonValue = serde_json::from_str(&json_str).unwrap();

        // JSON -> YAML via serde serialization
        let yaml_str = noyalib::to_string(&json).unwrap();
        let roundtrip: YamlValue = noyalib::from_str(&yaml_str).unwrap();

        vec![
            format!("json   = {json_str}"),
            format!("yaml   = {}", yaml_str.trim()),
            format!("match  = {}", yaml == roundtrip),
        ]
    });

    support::summary(4);
}
