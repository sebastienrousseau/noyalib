// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! #![no_std] compatibility: what works without the standard library.
//!
//! noyalib supports `#![no_std]` with `extern crate alloc`. The core
//! parsing and serialization APIs work without std. Only I/O functions
//! (from_reader, to_writer) require the `std` feature.
//!
//! Run: `cargo run --example nostd`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str, to_string};

fn main() {
    support::header("noyalib -- nostd");

    // ── Core APIs that work without std ──────────────────────────────
    support::task_with_output("from_str: works in no_std (uses alloc)", || {
        let yaml = "name: embedded\nport: 8080\n";
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "name = {}",
                v.get("name").and_then(|v| v.as_str()).unwrap_or("?")
            ),
            format!(
                "port = {}",
                v.get("port").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            "Requires: alloc (String, Vec, IndexMap)".to_string(),
        ]
    });

    support::task_with_output("to_string: works in no_std (uses alloc)", || {
        let v = Value::String("hello from no_std".to_string());
        let yaml = to_string(&v).unwrap();
        vec![
            format!("output = {}", yaml.trim()),
            "Requires: alloc (String formatting)".to_string(),
        ]
    });

    support::task_with_output("Value operations: works in no_std", || {
        let yaml = "items:\n  - a\n  - b\n  - c\n";
        let v: Value = from_str(yaml).unwrap();
        let items = v
            .get("items")
            .and_then(|v| v.as_sequence())
            .map(|s| s.len())
            .unwrap_or(0);
        vec![
            format!("items.len() = {items}"),
            format!("get_path    = {:?}", v.get_path("items").map(|_| "found")),
            "Requires: alloc (Vec, IndexMap)".to_string(),
        ]
    });

    support::task_with_output("Schema validation: works in no_std", || {
        let yaml = "key: value\n";
        let v: Value = from_str(yaml).unwrap();
        let core_ok = noyalib::validate_yaml_core_schema(&v).is_ok();
        let json_ok = noyalib::validate_yaml_json_schema(&v).is_ok();
        vec![
            format!(
                "core schema = {}",
                if core_ok { "valid" } else { "invalid" }
            ),
            format!(
                "json schema = {}",
                if json_ok { "valid" } else { "invalid" }
            ),
            "Requires: alloc (String for error paths)".to_string(),
        ]
    });

    // ── APIs that require std ────────────────────────────────────────
    support::task_with_output("APIs requiring std feature", || {
        vec![
            "from_reader()          -- requires std::io::Read".to_string(),
            "to_writer()            -- requires std::io::Write".to_string(),
            "to_fmt_writer()        -- requires std::fmt::Write (alloc)".to_string(),
            "Spanned<T>             -- requires thread-local (std)".to_string(),
            "Error::Io              -- requires std::io::Error".to_string(),
        ]
    });

    // ── How to use in no_std ─────────────────────────────────────────
    support::task_with_output("Usage in no_std projects", || {
        vec![
            "[dependencies]".to_string(),
            "noyalib = { version = \"0.0.1\", default-features = false }".to_string(),
            String::new(),
            "// In your lib.rs:".to_string(),
            "#![no_std]".to_string(),
            "extern crate alloc;".to_string(),
            "use noyalib::{from_str, to_string, Value};".to_string(),
        ]
    });

    support::summary(6);
}
