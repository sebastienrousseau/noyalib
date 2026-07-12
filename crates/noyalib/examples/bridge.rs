// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Serde interop: bridge between noyalib and serde_json.
//!
//! Run: `cargo run --example bridge`

#[path = "support.rs"]
mod support;

use noyalib::Value as YamlValue;
use serde_json::Value as JsonValue;

fn main() {
    support::header("noyalib -- bridge");

    // ── JSON -> YAML ─────────────────────────────────────────────────
    support::task_with_output("JSON to YAML conversion", || {
        let json: JsonValue = serde_json::from_str(
            r#"{"name": "noyalib", "version": 1, "features": ["serde", "safe"]}"#,
        )
        .unwrap();

        // Transcode: JSON Value -> serialize -> YAML string
        let yaml = noyalib::to_string(&json).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // ── YAML -> JSON ─────────────────────────────────────────────────
    support::task_with_output("YAML to JSON conversion", || {
        let yaml_str = "name: noyalib\nversion: 1\nfeatures:\n  - serde\n  - safe\n";
        let yaml_val: YamlValue = noyalib::from_str(yaml_str).unwrap();

        // Transcode: YAML Value -> serialize -> JSON string
        let json = serde_json::to_string_pretty(&yaml_val).unwrap();
        json.lines().map(|l| l.to_string()).collect()
    });

    // ── Roundtrip: YAML -> JSON -> YAML ──────────────────────────────
    support::task_with_output("YAML -> JSON -> YAML roundtrip", || {
        let original = "host: localhost\nport: 8080\ndebug: true\n";
        let yaml_val: YamlValue = noyalib::from_str(original).unwrap();
        let json_str = serde_json::to_string(&yaml_val).unwrap();
        let json_val: JsonValue = serde_json::from_str(&json_str).unwrap();
        let back_to_yaml = noyalib::to_string(&json_val).unwrap();
        let final_val: YamlValue = noyalib::from_str(&back_to_yaml).unwrap();

        vec![
            format!(
                "host  = {}",
                final_val
                    .get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "port  = {}",
                final_val.get("port").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            format!(
                "debug = {}",
                final_val
                    .get("debug")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            ),
            format!("match = {}", yaml_val == final_val),
        ]
    });

    // ── Shared struct between both formats ───────────────────────────
    support::task_with_output("Shared struct: serialize to both formats", || {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Config {
            name: String,
            port: u16,
        }

        let config = Config {
            name: "myapp".to_string(),
            port: 3000,
        };

        let yaml = noyalib::to_string(&config).unwrap();
        let json = serde_json::to_string(&config).unwrap();

        let from_yaml: Config = noyalib::from_str(&yaml).unwrap();
        let from_json: Config = serde_json::from_str(&json).unwrap();

        vec![
            format!("YAML: {}", yaml.trim()),
            format!("JSON: {json}"),
            format!("match = {}", from_yaml == from_json),
        ]
    });

    support::summary(4);
}
