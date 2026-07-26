// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! YAML merge keys (`<<`) with anchors and aliases.
//!
//! Merge keys are resolved automatically during parsing. `apply_merge()`
//! is available for post-parse resolution on manually constructed Values.
//!
//! Run: `cargo run --example inherit`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str};

/// Extract a scalar value at a dotted path, formatted for display.
fn val(v: &Value, path: &str) -> String {
    v.get_path(path)
        .map(|v| v.to_string().trim_matches('"').to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn main() {
    support::header("noyalib -- inherit");

    // ── Basic: overrides and inheritance ──────────────────────────────
    support::task_with_output("Basic merge (overrides and inheritance)", || {
        let yaml = r#"
defaults: &defaults
  timeout: 30
  retries: 3
  logging: true

development:
  <<: *defaults
  debug: true
  timeout: 60

production:
  <<: *defaults
  debug: false
  replicas: 5
"#;
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "dev.timeout  = {:>2} (overridden)",
                val(&v, "development.timeout")
            ),
            format!(
                "dev.retries  = {:>2} (inherited)",
                val(&v, "development.retries")
            ),
            format!("dev.debug    = {:>2} (local)", val(&v, "development.debug")),
            format!(
                "prod.replicas = {:>2} (local)",
                val(&v, "production.replicas")
            ),
            format!(
                "prod.timeout = {:>2} (inherited)",
                val(&v, "production.timeout")
            ),
        ]
    });

    // ── Multiple merge sources ───────────────────────────────────────
    support::task_with_output("Multiple merge sources", || {
        let yaml = r#"
base: &base
  adapter: postgres

connection: &connection
  host: localhost
  port: 5432

credentials: &credentials
  user: admin
  password: secret

database:
  <<: [*base, *connection, *credentials]
  database: myapp
"#;
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "database.adapter  = {} (from &base)",
                val(&v, "database.adapter")
            ),
            format!(
                "database.host     = {} (from &connection)",
                val(&v, "database.host")
            ),
            format!(
                "database.user     = {} (from &credentials)",
                val(&v, "database.user")
            ),
            format!(
                "database.database = {} (local)",
                val(&v, "database.database")
            ),
        ]
    });

    // ── Precedence ───────────────────────────────────────────────────
    support::task_with_output("Merge precedence (first source wins)", || {
        let yaml = r#"
first: &first
  a: 1
  b: 2

second: &second
  b: 20
  c: 30

result:
  <<: [*first, *second]
  c: 300
"#;
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!("a = {:>3} (from &first)", val(&v, "result.a")),
            format!("b = {:>3} (from &first, not &second)", val(&v, "result.b")),
            format!("c = {:>3} (local override)", val(&v, "result.c")),
        ]
    });

    // ── Merge within sequences ───────────────────────────────────────
    support::task_with_output("Merge within sequences", || {
        let yaml = r#"
defaults: &defaults
  type: worker
  replicas: 1

services:
  - name: api
    <<: *defaults
    type: web
    replicas: 3
  - name: worker-1
    <<: *defaults
  - name: worker-2
    <<: *defaults
    replicas: 2
"#;
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "api:      type = {}, replicas = {}",
                val(&v, "services[0].type"),
                val(&v, "services[0].replicas")
            ),
            format!(
                "worker-1: type = {}, replicas = {}",
                val(&v, "services[1].type"),
                val(&v, "services[1].replicas")
            ),
            format!(
                "worker-2: type = {}, replicas = {}",
                val(&v, "services[2].type"),
                val(&v, "services[2].replicas")
            ),
        ]
    });

    // ── Nested structure merge ───────────────────────────────────────
    support::task_with_output("Nested structure merge", || {
        let yaml = r#"
shared: &shared
  logging:
    level: info
    format: json

service_a:
  <<: *shared
  name: service-a

service_b:
  <<: *shared
  name: service-b
"#;
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!("service_a.name           = {}", val(&v, "service_a.name")),
            format!(
                "service_a.logging.level  = {}",
                val(&v, "service_a.logging.level")
            ),
            format!("service_b.name           = {}", val(&v, "service_b.name")),
            format!(
                "service_b.logging.format = {}",
                val(&v, "service_b.logging.format")
            ),
        ]
    });

    support::summary(5);
}
