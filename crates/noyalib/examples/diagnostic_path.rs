// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `serde_path_to_error` — pinpoint *which* nested key blew up.
//!
//! Vanilla serde reports the leaf-level error message ("invalid
//! type: integer 42, expected a string") but discards the
//! containing path: in a 200-line Kubernetes manifest with three
//! levels of nesting, knowing that *some* field is wrong is much
//! less helpful than knowing the field is
//! `spec.template.spec.containers[0].image`.
//!
//! [`serde_path_to_error`] composes with any `serde::Deserializer`
//! and emits the JSON-Pointer-style path to the offending node.
//! noyalib's [`Deserializer`](noyalib::Deserializer) plugs in directly — no
//! noyalib-specific glue.
//!
//! Run: `cargo run --example diagnostic_path`

#[path = "support.rs"]
mod support;

use noyalib::{Deserializer, Value, from_str};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct App {
    name: String,
    server: Server,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Server {
    host: String,
    port: u16,
    database: Database,
    #[serde(default)]
    replicas: Vec<Replica>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Database {
    url: String,
    pool_size: u16,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Replica {
    region: String,
    weight: u16,
}

fn try_parse(yaml: &str) -> Result<App, (String, String)> {
    let value: Value = from_str(yaml).map_err(|e| (String::new(), e.to_string()))?;
    let de = Deserializer::new(&value);
    serde_path_to_error::deserialize::<_, App>(de)
        .map_err(|err| (err.path().to_string(), err.inner().to_string()))
}

fn main() {
    support::header("serde_path_to_error — locate the offending nested key");

    // ── Valid input ─────────────────────────────────────────────────
    support::task_with_output("Valid document — no error path emitted", || {
        let yaml = "\
name: noyalib
server:
  host: api.example.com
  port: 8080
  database:
    url: postgres://api/db
    pool_size: 16
";
        let app = try_parse(yaml).unwrap();
        vec![format!("parsed App {{ name: {:?}, … }}", app.name)]
    });

    // ── Top-level field error ───────────────────────────────────────
    support::task_with_output("Top-level: `name` has the wrong type", || {
        let yaml = "\
name: 42
server:
  host: api.example.com
  port: 8080
  database:
    url: postgres://api/db
    pool_size: 16
";
        let (path, msg) = try_parse(yaml).unwrap_err();
        vec![format!("path:  {path}"), format!("error: {msg}")]
    });

    // ── 2-level nested field ────────────────────────────────────────
    support::task_with_output("Nested: `server.port` overflows u16", || {
        let yaml = "\
name: api
server:
  host: api.example.com
  port: 99999
  database:
    url: postgres://api/db
    pool_size: 16
";
        let (path, msg) = try_parse(yaml).unwrap_err();
        vec![format!("path:  {path}"), format!("error: {msg}")]
    });

    // ── 3-level nested field ────────────────────────────────────────
    support::task_with_output("Deeply nested: `server.database.pool_size`", || {
        let yaml = "\
name: api
server:
  host: api.example.com
  port: 8080
  database:
    url: postgres://api/db
    pool_size: \"sixteen\"
";
        let (path, msg) = try_parse(yaml).unwrap_err();
        vec![format!("path:  {path}"), format!("error: {msg}")]
    });

    // ── Sequence index pointed at by name ───────────────────────────
    support::task_with_output("Sequence: which `replicas[N].weight` is wrong", || {
        let yaml = "\
name: api
server:
  host: api.example.com
  port: 8080
  database:
    url: postgres://api/db
    pool_size: 16
  replicas:
    - region: eu-west-1
      weight: 50
    - region: us-east-1
      weight: -3
";
        let (path, msg) = try_parse(yaml).unwrap_err();
        vec![format!("path:  {path}"), format!("error: {msg}")]
    });

    println!();
    println!("  serde_path_to_error works because noyalib's Deserializer<'de>");
    println!("  is plain serde — no special hooks. Everything that wraps a");
    println!("  generic Deserializer composes for free: serde_ignored,");
    println!("  serde_path_to_error, format_serde_error, etc.");

    support::footer();
}
