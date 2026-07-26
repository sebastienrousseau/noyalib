// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Value type example for noyalib.
//!
//! Demonstrates working with the dynamic Value type.
//!
//! Run: `cargo run --example dynamic`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str, to_string};

fn main() {
    support::header("noyalib -- dynamic");

    let yaml = r#"
name: my-config
version: 1
enabled: true
ratio: 3.125
tags:
  - alpha
  - beta
  - gamma
settings:
  timeout: 30
  retries: 3
  debug: false
"#;

    let value: Value = support::task("Parse YAML into Value", || from_str(yaml).unwrap());

    support::task_with_output("Access fields dynamically", || {
        vec![
            format!(
                "name    = {}",
                value.get("name").and_then(|v| v.as_str()).unwrap_or("?")
            ),
            format!(
                "version = {}",
                value.get("version").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            format!(
                "enabled = {}",
                value
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            ),
            format!(
                "ratio   = {}",
                value.get("ratio").and_then(|v| v.as_f64()).unwrap_or(0.0)
            ),
        ]
    });

    support::task_with_output("Access nested values", || {
        let mut lines = Vec::new();
        if let Some(tags) = value.get("tags").and_then(|v| v.as_sequence()) {
            for (i, tag) in tags.iter().enumerate() {
                lines.push(format!("tags[{i}] = {}", tag.as_str().unwrap_or("?")));
            }
        }
        if let Some(settings) = value.get("settings") {
            lines.push(format!(
                "settings.timeout = {}",
                settings
                    .get("timeout")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
            ));
        }
        lines
    });

    support::task_with_output("Missing key returns None (no panic)", || {
        vec![
            format!("value.get(\"missing\") = {:?}", value.get("missing")),
            format!("value.get_path(\"a.b.c\") = {:?}", value.get_path("a.b.c")),
        ]
    });

    support::task("Serialize Value back to YAML", || {
        let _ = to_string(&value).unwrap();
    });

    support::summary(5);
}
