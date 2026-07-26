// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Environment variable expansion in YAML values.
//!
//! Demonstrates a pattern for `${VAR}` substitution using Value manipulation.
//! This is a user-space pattern — noyalib parses the YAML, then you walk the
//! tree and expand variables.
//!
//! Run: `cargo run --example env`

#[path = "support.rs"]
mod support;

use std::collections::HashMap;

use noyalib::{Value, from_str, to_string};

/// Expand `${KEY}` patterns in all string values using the given env map.
fn expand_env(value: &mut Value, env: &HashMap<&str, &str>) {
    match value {
        Value::String(s) => {
            let mut result = s.clone();
            for (key, val) in env {
                result = result.replace(&format!("${{{key}}}"), val);
            }
            *s = result;
        }
        Value::Sequence(seq) => {
            for item in seq.iter_mut() {
                expand_env(item, env);
            }
        }
        Value::Mapping(map) => {
            for (_, v) in map.iter_mut() {
                expand_env(v, env);
            }
        }
        Value::Tagged(t) => expand_env(t.value_mut(), env),
        _ => {}
    }
}

fn main() {
    support::header("noyalib -- env");

    let yaml = r#"
database:
  host: ${DB_HOST}
  port: ${DB_PORT}
  name: ${APP_NAME}_db
  url: postgres://${DB_HOST}:${DB_PORT}/${APP_NAME}_db
app:
  name: ${APP_NAME}
  env: ${ENVIRONMENT}
  debug: ${DEBUG}
"#;

    let env = HashMap::from([
        ("DB_HOST", "localhost"),
        ("DB_PORT", "5432"),
        ("APP_NAME", "myservice"),
        ("ENVIRONMENT", "production"),
        ("DEBUG", "false"),
    ]);

    // ── Before expansion ─────────────────────────────────────────────
    support::task_with_output("Parse template with ${VAR} placeholders", || {
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "database.host = {}",
                v.get_path("database.host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "database.url  = {}",
                v.get_path("database.url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "app.name      = {}",
                v.get_path("app.name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
        ]
    });

    // ── After expansion ──────────────────────────────────────────────
    support::task_with_output("Expand ${VAR} from environment map", || {
        let mut v: Value = from_str(yaml).unwrap();
        expand_env(&mut v, &env);
        vec![
            format!(
                "database.host = {}",
                v.get_path("database.host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "database.url  = {}",
                v.get_path("database.url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "app.env       = {}",
                v.get_path("app.env")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
        ]
    });

    // ── Serialize expanded result ────────────────────────────────────
    support::task_with_output("Serialize expanded config", || {
        let mut v: Value = from_str(yaml).unwrap();
        expand_env(&mut v, &env);
        let output = to_string(&v).unwrap();
        output.lines().map(|l| l.to_string()).collect()
    });

    // ── Missing variable detection ───────────────────────────────────
    support::task_with_output("Detect unexpanded variables", || {
        let partial_env = HashMap::from([("DB_HOST", "localhost")]);
        let mut v: Value = from_str(yaml).unwrap();
        expand_env(&mut v, &partial_env);

        // Find any remaining ${...} patterns
        let yaml_out = to_string(&v).unwrap();
        let unexpanded: Vec<&str> = yaml_out.lines().filter(|l| l.contains("${")).collect();

        if unexpanded.is_empty() {
            vec!["all variables expanded".to_string()]
        } else {
            let mut lines = vec![format!("{} unexpanded variables:", unexpanded.len())];
            lines.extend(unexpanded.iter().map(|l| l.trim().to_string()));
            lines
        }
    });

    support::summary(4);
}
