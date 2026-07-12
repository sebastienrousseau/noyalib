//! Value merge example for noyalib.
//!
//! Demonstrates merging YAML values together, useful for configuration
//! layering.
//!
//! Run: `cargo run --example overlay`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str, to_string};

fn main() {
    support::header("noyalib -- overlay");

    support::task_with_output("Merge override configuration into base", || {
        let base_yaml = r#"
server:
  host: localhost
  port: 8080
  timeout: 30
database:
  host: localhost
  port: 5432
logging:
  level: info
  format: text
"#;

        let override_yaml = r#"
server:
  host: prod.example.com
  ssl: true
database:
  host: db.example.com
  pool_size: 20
logging:
  level: warn
"#;

        let mut base: Value = from_str(base_yaml).unwrap();
        let overrides: Value = from_str(override_yaml).unwrap();

        let mut lines = vec!["=== Base configuration ===".to_string()];
        lines.extend(to_string(&base).unwrap().lines().map(|l| l.to_string()));
        lines.push(String::new());
        lines.push("=== Override configuration ===".to_string());
        lines.extend(
            to_string(&overrides)
                .unwrap()
                .lines()
                .map(|l| l.to_string()),
        );

        base.merge(overrides);

        lines.push(String::new());
        lines.push("=== Merged configuration ===".to_string());
        lines.extend(to_string(&base).unwrap().lines().map(|l| l.to_string()));
        lines
    });

    support::task_with_output("Verify merged values", || {
        let base_yaml = "server:\n  host: localhost\n  port: 8080\n  timeout: 30\ndatabase:\n  host: localhost\n  port: 5432\nlogging:\n  level: info\n  format: text\n";
        let override_yaml = "server:\n  host: prod.example.com\n  ssl: true\ndatabase:\n  host: db.example.com\n  pool_size: 20\nlogging:\n  level: warn\n";

        let mut base: Value = from_str(base_yaml).unwrap();
        let overrides: Value = from_str(override_yaml).unwrap();
        base.merge(overrides);

        vec![
            format!(
                "server.host     = {}",
                base.get_path("server.host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "server.port     = {}",
                base.get_path("server.port")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
            ),
            format!(
                "server.timeout  = {}",
                base.get_path("server.timeout")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
            ),
            format!(
                "server.ssl      = {}",
                base.get_path("server.ssl")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            ),
            format!(
                "database.pool   = {}",
                base.get_path("database.pool_size")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
            ),
            format!(
                "logging.level   = {}",
                base.get_path("logging.level")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
        ]
    });

    support::task_with_output("Sequence merge (concat)", || {
        let mut list1: Value = from_str("items:\n  - a\n  - b\n").unwrap();
        let list2: Value = from_str("items:\n  - c\n  - d\n").unwrap();

        let fmt_seq = |v: &Value| -> String {
            v.get("items")
                .and_then(|s| s.as_sequence())
                .map(|seq| {
                    let items: Vec<&str> = seq.iter().filter_map(|v| v.as_str()).collect();
                    format!("[{}]", items.join(", "))
                })
                .unwrap_or_default()
        };

        let base = format!("base     = {}", fmt_seq(&list1));
        let other = format!("other    = {}", fmt_seq(&list2));
        list1.merge_concat(list2);
        let merged = format!("merged   = {}", fmt_seq(&list1));

        vec![base, other, merged]
    });

    support::task_with_output("Sequence merge (replace)", || {
        let mut base: Value = from_str("tags:\n  - old1\n  - old2\n").unwrap();
        let new: Value = from_str("tags:\n  - new1\n  - new2\n  - new3\n").unwrap();

        let fmt_seq = |v: &Value| -> String {
            v.get("tags")
                .and_then(|s| s.as_sequence())
                .map(|seq| {
                    let items: Vec<&str> = seq.iter().filter_map(|v| v.as_str()).collect();
                    format!("[{}]", items.join(", "))
                })
                .unwrap_or_default()
        };

        let before = format!("before = {}", fmt_seq(&base));
        base.merge(new);
        let after = format!("after  = {}", fmt_seq(&base));

        vec![before, after]
    });

    support::task_with_output("Programmatic value modification", || {
        let mut config: Value = from_str("settings:\n  timeout: 30\n").unwrap();

        if let Some(settings) = config.get_mut("settings") {
            if let Some(map) = settings.as_mapping_mut() {
                let _ = map.insert("retries".to_string(), Value::from(3));
                let _ = map.insert("debug".to_string(), Value::from(false));
            }
        }

        let _ = config.insert("version", Value::from("1.0.0"));

        let mut lines = vec!["Modified config:".to_string()];
        lines.extend(to_string(&config).unwrap().lines().map(|l| l.to_string()));
        lines
    });

    support::summary(5);
}
