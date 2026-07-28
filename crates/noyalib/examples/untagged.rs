// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Untagged enums: polymorphic deserialization by structure.
//!
//! Real-world YAML (K8s, CloudFormation, CI/CD) uses untagged enums
//! where the parser infers the variant from field presence.
//!
//! Run: `cargo run --example untagged`

#[path = "support.rs"]
mod support;

use noyalib::{from_str, to_string};

// ── K8s-style: infer resource type from fields ──────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
enum Resource {
    Service {
        name: String,
        port: u16,
        protocol: String,
    },
    ConfigMap {
        name: String,
        data: std::collections::BTreeMap<String, String>,
    },
    Secret {
        name: String,
        encoded: String,
    },
}

// ── CI/CD-style: step can be a string or a map ──────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
enum Step {
    Simple(String),
    Detailed {
        run: String,
        #[serde(default)]
        timeout: Option<u32>,
        #[serde(default)]
        env: std::collections::BTreeMap<String, String>,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct Pipeline {
    name: String,
    steps: Vec<Step>,
}

// ── Config value: string, number, or list ───────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
enum ConfigValue {
    Text(String),
    Number(i64),
    Flag(bool),
    List(Vec<String>),
}

fn main() {
    support::header("noyalib -- untagged");

    // ── K8s resources ────────────────────────────────────────────────
    support::task_with_output("Infer K8s resource type from fields", || {
        let yaml = r#"
- name: web
  port: 80
  protocol: TCP
- name: app-config
  data:
    DB_HOST: localhost
    DB_PORT: "5432"
- name: api-key
  encoded: c2VjcmV0
"#;
        let resources: Vec<Resource> = from_str(yaml).unwrap();
        resources
            .iter()
            .map(|r| match r {
                Resource::Service { name, port, .. } => {
                    format!("{name:<12} -> Service (port {port})")
                }
                Resource::ConfigMap { name, data } => {
                    format!("{name:<12} -> ConfigMap ({} keys)", data.len())
                }
                Resource::Secret { name, .. } => {
                    format!("{name:<12} -> Secret")
                }
            })
            .collect()
    });

    // ── CI/CD pipeline ───────────────────────────────────────────────
    support::task_with_output("CI/CD pipeline: string or detailed step", || {
        let yaml = r#"
name: build
steps:
  - cargo check
  - run: cargo test
    timeout: 300
  - cargo clippy
  - run: cargo publish
    env:
      CARGO_TOKEN: secret
"#;
        let pipeline: Pipeline = from_str(yaml).unwrap();
        pipeline
            .steps
            .iter()
            .map(|s| match s {
                Step::Simple(cmd) => format!("simple:   {cmd}"),
                Step::Detailed { run, timeout, env } => {
                    let extras: Vec<String> = [
                        timeout.map(|t| format!("timeout={t}s")),
                        if env.is_empty() {
                            None
                        } else {
                            Some(format!("{} env vars", env.len()))
                        },
                    ]
                    .into_iter()
                    .flatten()
                    .collect();
                    if extras.is_empty() {
                        format!("detailed: {run}")
                    } else {
                        format!("detailed: {run} ({})", extras.join(", "))
                    }
                }
            })
            .collect()
    });

    // ── Polymorphic config values ────────────────────────────────────
    support::task_with_output("Polymorphic config values", || {
        let yaml = "host: localhost\nport: 8080\ndebug: true\ntags:\n  - web\n  - api\n";
        let v: std::collections::BTreeMap<String, ConfigValue> = from_str(yaml).unwrap();
        v.iter()
            .map(|(k, v)| match v {
                ConfigValue::Text(s) => format!("{k:<6} = \"{s}\" (Text)"),
                ConfigValue::Number(n) => format!("{k:<6} = {n} (Number)"),
                ConfigValue::Flag(b) => format!("{k:<6} = {b} (Flag)"),
                ConfigValue::List(l) => format!("{k:<6} = [{}] (List)", l.join(", ")),
            })
            .collect()
    });

    // ── Roundtrip ────────────────────────────────────────────────────
    support::task_with_output("Untagged enum roundtrip", || {
        let pipeline = Pipeline {
            name: "deploy".into(),
            steps: vec![
                Step::Simple("cargo build --release".into()),
                Step::Detailed {
                    run: "deploy.sh".into(),
                    timeout: Some(600),
                    env: std::collections::BTreeMap::new(),
                },
            ],
        };
        let yaml = to_string(&pipeline).unwrap();
        let rt: Pipeline = from_str(&yaml).unwrap();
        vec![
            format!("match  = {}", pipeline == rt),
            format!("steps  = {}", rt.steps.len()),
        ]
    });

    support::summary(4);
}
