// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! garde — declarative post-deserialise validation.
//!
//! Type-shape errors (a string where a `u16` was expected) are
//! caught by serde at parse time, but *logic* errors — a port that
//! must be in `1024..=65535`, a hostname that must match a regex, a
//! capacity that must not exceed a sibling field — need a separate
//! pass. [`garde`] is the modern, attribute-driven validation crate
//! that handles those checks. noyalib's [`Validated<T>`] wrapper
//! runs garde rules immediately after deserialise so a malformed
//! document fails *before* it reaches business logic.
//!
//! Run: `cargo run --example validation_garde --features garde`

#[path = "support.rs"]
mod support;

use garde::Validate;
use noyalib::Validated;

#[derive(Debug, serde::Deserialize, Validate)]
#[allow(dead_code)]
struct ServerConfig {
    #[garde(length(min = 1, max = 253))]
    host: String,

    #[garde(range(min = 1024, max = 65535))]
    port: u16,

    #[garde(range(min = 1, max = 1024))]
    workers: u32,

    #[garde(length(min = 1, max = 64))]
    service_name: String,
}

#[derive(Debug, serde::Deserialize, Validate)]
#[allow(dead_code)]
struct DeployConfig {
    #[garde(length(min = 1, max = 32))]
    environment: String,

    #[garde(dive)]
    server: ServerConfig,

    #[garde(length(min = 1))]
    #[garde(inner(length(min = 1, max = 64)))]
    allowed_origins: Vec<String>,
}

fn main() {
    support::header("garde — declarative validation of parsed YAML");

    // ── Happy path ──────────────────────────────────────────────────
    support::task_with_output("Valid document deserialises and validates", || {
        let yaml = "
host: api.example.com
port: 8080
workers: 16
service_name: noyalib-api
";
        let cfg: Validated<ServerConfig> = noyalib::from_str(yaml).unwrap();
        vec![
            format!("host         = {}", cfg.host),
            format!("port         = {}", cfg.port),
            format!("workers      = {}", cfg.workers),
            format!("service_name = {}", cfg.service_name),
        ]
    });

    // ── Single-field violation ──────────────────────────────────────
    support::task_with_output("Invalid port surfaces a typed error", || {
        let yaml = "
host: api.example.com
port: 80
workers: 16
service_name: api
";
        let res: Result<Validated<ServerConfig>, _> = noyalib::from_str(yaml);
        match res {
            Ok(_) => vec!["unexpected: parse succeeded".into()],
            Err(e) => vec![
                "Validation failed (as expected):".to_string(),
                format!("  {e}"),
            ],
        }
    });

    // ── Nested struct: garde dives into sub-structs ─────────────────
    support::task_with_output("Nested struct: invalid `server.port` rejected", || {
        let yaml = "
environment: production
server:
  host: api.example.com
  port: 22
  workers: 16
  service_name: api
allowed_origins:
  - https://example.com
";
        let res: Result<Validated<DeployConfig>, _> = noyalib::from_str(yaml);
        match res {
            Ok(_) => vec!["unexpected: parse succeeded".into()],
            Err(e) => vec![
                "Nested validation failed (as expected):".to_string(),
                format!("  {e}"),
            ],
        }
    });

    // ── Collection rule: empty Vec rejected ─────────────────────────
    support::task_with_output("Collection rule: empty `allowed_origins` rejected", || {
        let yaml = "
environment: staging
server:
  host: api.example.com
  port: 8080
  workers: 4
  service_name: api
allowed_origins: []
";
        let res: Result<Validated<DeployConfig>, _> = noyalib::from_str(yaml);
        match res {
            Ok(_) => vec!["unexpected: parse succeeded".into()],
            Err(e) => vec![
                "Empty-collection rule fired (as expected):".to_string(),
                format!("  {e}"),
            ],
        }
    });

    // ── Multiple violations reported together ───────────────────────
    support::task_with_output("Multiple violations surface in one error", || {
        let yaml = "
host: \"\"
port: 80
workers: 0
service_name: \"\"
";
        let res: Result<Validated<ServerConfig>, _> = noyalib::from_str(yaml);
        match res {
            Ok(_) => vec!["unexpected: parse succeeded".into()],
            Err(e) => {
                let msg = e.to_string();
                let mut lines = vec!["Combined error report:".to_string()];
                for chunk in msg.split(';') {
                    lines.push(format!("  {}", chunk.trim()));
                }
                lines
            }
        }
    });

    println!();
    println!("  garde catches *logic* errors (range, regex, length,");
    println!("  cross-field invariants) — the half of validation that");
    println!("  serde's type-shape check cannot reach. Pair it with");
    println!("  noyalib's `Validated<T>` and a malformed YAML document");
    println!("  fails at the boundary, never inside business logic.");

    support::footer();
}
