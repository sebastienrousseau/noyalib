// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Self-documenting config: extract schema from YAML structure.
//!
//! Analyzes a YAML Value tree and generates a human-readable schema
//! or a type map — turning noyalib into a single source of truth
//! for configuration contracts.
//!
//! Run: `cargo run --example schema_ext`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str};

/// Infer the type name of a Value.
fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(n) => {
            if n.as_f64().fract() == 0.0 {
                "int"
            } else {
                "float"
            }
        }
        Value::String(_) => "string",
        Value::Sequence(_) => "list",
        Value::Mapping(_) => "map",
        Value::Tagged(_) => "tagged",
    }
}

/// Generate a schema description from a Value tree.
fn describe(value: &Value, prefix: &str, lines: &mut Vec<String>) {
    match value {
        Value::Mapping(map) => {
            for (key, val) in map.iter() {
                let path = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{prefix}.{key}")
                };
                match val {
                    Value::Mapping(_) => {
                        lines.push(format!("{path:<30} map"));
                        describe(val, &path, lines);
                    }
                    Value::Sequence(seq) => {
                        let item_type = seq.first().map(type_name).unwrap_or("any");
                        lines.push(format!("{path:<30} list<{item_type}>"));
                    }
                    _ => {
                        let example = match val {
                            Value::String(s) => format!("\"{}\"", truncate(s, 20)),
                            other => other.to_string(),
                        };
                        lines.push(format!("{path:<30} {:<8} (e.g. {example})", type_name(val)));
                    }
                }
            }
        }
        _ => {
            let path = if prefix.is_empty() { "root" } else { prefix };
            lines.push(format!("{path:<30} {}", type_name(value)));
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

fn main() {
    support::header("noyalib -- schema_ext");

    let yaml = r#"
server:
  host: localhost
  port: 8080
  ssl: true
  workers: 4
database:
  url: postgres://localhost:5432/mydb
  pool_size: 10
  timeout_ms: 5000
logging:
  level: info
  format: json
  outputs:
    - stdout
    - /var/log/app.log
features:
  - authentication
  - rate_limiting
  - caching
"#;

    let value: Value = from_str(yaml).unwrap();

    // ── Generate schema ──────────────────────────────────────────────
    support::task_with_output("Extract schema from YAML", || {
        let mut lines = Vec::new();
        describe(&value, "", &mut lines);
        lines
    });

    // ── Required vs optional detection ───────────────────────────────
    support::task_with_output("Detect field presence across documents", || {
        let docs = vec![
            "host: a\nport: 80\ndebug: true\n",
            "host: b\nport: 443\n",
            "host: c\nport: 8080\ndebug: false\ntls: true\n",
        ];

        let mut all_keys = std::collections::BTreeMap::<String, usize>::new();
        for doc in &docs {
            let v: Value = from_str(doc).unwrap();
            if let Some(map) = v.as_mapping() {
                for key in map.keys() {
                    *all_keys.entry(key.clone()).or_insert(0) += 1;
                }
            }
        }

        let total = docs.len();
        all_keys
            .iter()
            .map(|(key, count)| {
                let status = if *count == total {
                    "required"
                } else {
                    "optional"
                };
                format!("{key:<10} {status} ({count}/{total} docs)")
            })
            .collect()
    });

    // ── Type consistency check ───────────────────────────────────────
    support::task_with_output("Type consistency across documents", || {
        let docs = vec![
            "port: 80\nname: app1\n",
            "port: 443\nname: app2\n",
            "port: \"8080\"\nname: app3\n", // Type mismatch!
        ];

        let mut types = std::collections::BTreeMap::<String, Vec<&str>>::new();
        for doc in &docs {
            let v: Value = from_str(doc).unwrap();
            if let Some(map) = v.as_mapping() {
                for (key, val) in map.iter() {
                    types.entry(key.clone()).or_default().push(type_name(val));
                }
            }
        }

        types
            .iter()
            .map(|(key, type_list)| {
                let unique: std::collections::BTreeSet<&&str> = type_list.iter().collect();
                let consistent = unique.len() == 1;
                let types_str = type_list.join(", ");
                if consistent {
                    format!("{key:<10} consistent ({types_str})")
                } else {
                    format!("{key:<10} MISMATCH! ({types_str})")
                }
            })
            .collect()
    });

    support::summary(3);
}
