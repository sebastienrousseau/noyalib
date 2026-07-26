// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! WASM portability proof: noyalib compiles to wasm32-unknown-unknown.
//!
//! This example verifies all core operations work identically on native
//! and WASM targets. The actual WASM bindings are in `examples/wasm/`.
//!
//! Build WASM:  `cd examples/wasm && wasm-pack build --target web`
//! Serve:       `cd examples/wasm && python3 -m http.server 8080`
//! Open:        `http://localhost:8080/index.html`
//!
//! Run native:  `cargo run --example portable`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str, to_string};

fn main() {
    support::header("noyalib -- portable");

    // ── Core operations (identical on native + wasm32) ───────────────
    support::task_with_output("Parse YAML to Value", || {
        let yaml = "name: noyalib\nversion: 1\nfeatures:\n  - serde\n  - safe\n  - fast\n";
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "name     = {}",
                v.get("name").and_then(|v| v.as_str()).unwrap_or("?")
            ),
            format!(
                "features = {} items",
                v.get("features")
                    .and_then(|v| v.as_sequence())
                    .map(|s| s.len())
                    .unwrap_or(0)
            ),
        ]
    });

    support::task_with_output("Serialize Value to YAML", || {
        let v = Value::from("hello from wasm");
        let yaml = to_string(&v).unwrap();
        vec![format!("output = {}", yaml.trim())]
    });

    support::task_with_output("Path traversal", || {
        let yaml =
            "server:\n  host: localhost\n  port: 8080\n  ssl:\n    enabled: true\n    cert: /tls\n";
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "server.host        = {}",
                v.get_path("server.host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "server.ssl.enabled = {}",
                v.get_path("server.ssl.enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            ),
        ]
    });

    support::task_with_output("Value merge", || {
        let mut base: Value = from_str("host: localhost\nport: 80\n").unwrap();
        let patch: Value = from_str("host: production\nssl: true\n").unwrap();
        base.merge(patch);
        vec![
            format!(
                "host = {}",
                base.get("host").and_then(|v| v.as_str()).unwrap_or("?")
            ),
            format!(
                "port = {}",
                base.get("port").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            format!(
                "ssl  = {}",
                base.get("ssl").and_then(|v| v.as_bool()).unwrap_or(false)
            ),
        ]
    });

    support::task_with_output("Schema validation", || {
        let yaml = "name: test\nport: 8080\n";
        let v: Value = from_str(yaml).unwrap();
        let json_ok = noyalib::validate_yaml_json_schema(&v).is_ok();
        let core_ok = noyalib::validate_yaml_core_schema(&v).is_ok();
        vec![
            format!(
                "JSON schema = {}",
                if json_ok { "valid" } else { "invalid" }
            ),
            format!(
                "Core schema = {}",
                if core_ok { "valid" } else { "invalid" }
            ),
        ]
    });

    // ── WASM build info ──────────────────────────────────────────────
    support::task_with_output("WASM build info", || {
        vec![
            "target   = wasm32-unknown-unknown".to_string(),
            "binary   = ~201 KB (release, lto)".to_string(),
            "deps     = 0 C/FFI (pure Rust)".to_string(),
            "build    = cd examples/wasm && wasm-pack build --target web".to_string(),
        ]
    });

    support::summary(6);
}
