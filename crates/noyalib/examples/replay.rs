// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Anchor event replay via the streaming deserializer.
//!
//! Demonstrates how noyalib handles YAML anchors and aliases natively
//! in the streaming path -- no AST fallback needed for the common case.
//!
//! Run: `cargo run --example replay`

#[path = "support.rs"]
mod support;

use std::collections::BTreeMap;

use noyalib::{Value, from_str};
use serde::Deserialize;

fn main() {
    support::header("noyalib -- replay (anchor event replay)");

    // ── Simple scalar anchors and aliases ────────────────────────────
    support::task_with_output("Scalar anchor and alias (string)", || {
        let yaml = "name: &who Alice\ngreeting: *who\n";
        #[derive(Debug, Deserialize)]
        struct Doc {
            name: String,
            greeting: String,
        }
        let doc: Doc = from_str(yaml).unwrap();
        assert_eq!(doc.name, "Alice");
        assert_eq!(doc.greeting, "Alice");
        vec![
            format!("name     = {}", doc.name),
            format!("greeting = {} (replayed from &who)", doc.greeting),
        ]
    });

    support::task_with_output("Scalar anchor and alias (integer)", || {
        let yaml = "width: &sz 1024\nheight: *sz\n";
        let map: BTreeMap<String, i64> = from_str(yaml).unwrap();
        assert_eq!(map["width"], 1024);
        assert_eq!(map["height"], 1024);
        vec![format!(
            "width = {}, height = {} (same anchor)",
            map["width"], map["height"]
        )]
    });

    // ── Compound anchor (mapping with anchor, aliased elsewhere) ────
    support::task_with_output("Compound anchor (mapping)", || {
        let yaml = r#"
defaults: &cfg
  host: localhost
  port: 8080
staging: *cfg
"#;
        #[derive(Debug, Deserialize, PartialEq)]
        struct Endpoint {
            host: String,
            port: u16,
        }
        #[derive(Debug, Deserialize)]
        struct Doc {
            defaults: Endpoint,
            staging: Endpoint,
        }
        let doc: Doc = from_str(yaml).unwrap();
        assert_eq!(doc.defaults, doc.staging);
        vec![
            format!("defaults = {}:{}", doc.defaults.host, doc.defaults.port),
            format!(
                "staging  = {}:{} (replayed)",
                doc.staging.host, doc.staging.port
            ),
        ]
    });

    // ── Nested anchors (anchor inside a sequence) ────────────────────
    support::task_with_output("Nested anchor inside a sequence", || {
        let yaml = r#"
items:
  - &first alpha
  - &second beta
  - gamma
copies:
  - *second
  - *first
"#;
        #[derive(Debug, Deserialize)]
        struct Doc {
            items: Vec<String>,
            copies: Vec<String>,
        }
        let doc: Doc = from_str(yaml).unwrap();
        assert_eq!(doc.copies, vec!["beta", "alpha"]);
        vec![
            format!("items  = {:?}", doc.items),
            format!("copies = {:?} (replayed in reverse)", doc.copies),
        ]
    });

    // ── Multiple aliases to the same anchor ──────────────────────────
    support::task_with_output("Multiple aliases to the same anchor", || {
        let yaml = "origin: &pt\n  x: 0\n  y: 0\na: *pt\nb: *pt\nc: *pt\n";
        #[derive(Debug, Deserialize, PartialEq)]
        struct Point {
            x: i32,
            y: i32,
        }
        #[derive(Debug, Deserialize)]
        struct Doc {
            origin: Point,
            a: Point,
            b: Point,
            c: Point,
        }
        let doc: Doc = from_str(yaml).unwrap();
        assert_eq!(doc.origin, doc.a);
        assert_eq!(doc.a, doc.b);
        assert_eq!(doc.b, doc.c);
        vec![format!("4 references to &pt all equal: {:?}", doc.origin)]
    });

    // ── Typed deserialization with shared values ─────────────────────
    support::task_with_output("Typed struct with shared values", || {
        let yaml = r#"
database: &db
  host: db.internal
  port: 5432
  name: myapp
read_replica: *db
analytics_replica: *db
"#;
        #[derive(Debug, Deserialize, PartialEq)]
        struct DbConfig {
            host: String,
            port: u16,
            name: String,
        }
        #[derive(Debug, Deserialize)]
        struct Topology {
            database: DbConfig,
            read_replica: DbConfig,
            analytics_replica: DbConfig,
        }
        let t: Topology = from_str(yaml).unwrap();
        assert_eq!(t.database, t.read_replica);
        assert_eq!(t.database, t.analytics_replica);
        vec![
            format!(
                "primary  = {}:{}/{}",
                t.database.host, t.database.port, t.database.name
            ),
            format!("replicas share the same config via anchor replay"),
        ]
    });

    // ── Streaming speed demonstration ────────────────────────────────
    support::task_with_output("Streaming path handles anchors (no AST fallback)", || {
        // Build a document with many anchors to show streaming handles them.
        let mut yaml = String::new();
        for i in 0..50 {
            yaml.push_str(&format!("key{i}: &anchor{i} value{i}\n"));
        }
        for i in 0..50 {
            yaml.push_str(&format!("alias{i}: *anchor{i}\n"));
        }
        let start = std::time::Instant::now();
        let map: BTreeMap<String, String> = from_str(&yaml).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(map.len(), 100);
        for i in 0..50 {
            assert_eq!(map[&format!("key{i}")], map[&format!("alias{i}")]);
        }
        vec![
            format!(
                "100 entries (50 anchors + 50 aliases) parsed in {:?}",
                elapsed
            ),
            format!("Streaming path -- no intermediate Value allocation"),
        ]
    });

    // ── Anchor replay with Value (merge key triggers fallback) ──────
    support::task_with_output("Merge key still works via Value fallback", || {
        let yaml = r#"
base: &base
  timeout: 30
  retries: 3
server:
  <<: *base
  host: example.com
"#;
        let v: Value = from_str(yaml).unwrap();
        let server = v.get("server").unwrap();
        assert_eq!(server.get("timeout").unwrap(), &Value::from(30));
        assert_eq!(server.get("host").unwrap(), &Value::from("example.com"));
        vec![
            format!(
                "timeout = {} (inherited via merge)",
                server.get("timeout").unwrap()
            ),
            format!("host    = {} (local override)", server.get("host").unwrap()),
        ]
    });

    support::summary(8);
}
