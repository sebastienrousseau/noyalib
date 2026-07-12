// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Serde flatten and untagged enums in YAML context.
//!
//! Run: `cargo run --example flatten`

#[path = "support.rs"]
mod support;

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

// ── Flatten: merge nested struct fields into parent ──────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Metadata {
    created_by: String,
    version: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Document {
    title: String,
    #[serde(flatten)]
    meta: Metadata,
}

// ── Untagged enum: infer variant from structure ──────────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Endpoint {
    Simple(String),
    Detailed { url: String, timeout: u32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Service {
    name: String,
    endpoints: Vec<Endpoint>,
}

// ── Internally tagged enum ───────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
enum Resource {
    #[serde(rename = "database")]
    Database { host: String, port: u16 },
    #[serde(rename = "cache")]
    Cache { host: String, ttl: u32 },
}

fn main() {
    support::header("noyalib -- flatten");

    // ── Flatten ──────────────────────────────────────────────────────
    support::task_with_output("#[serde(flatten)]: merge fields into parent", || {
        let doc = Document {
            title: "Design Doc".to_string(),
            meta: Metadata {
                created_by: "alice".to_string(),
                version: 3,
            },
        };
        let yaml = to_string(&doc).unwrap();
        let parsed: Document = from_str(&yaml).unwrap();
        assert_eq!(doc, parsed);
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // ── Untagged ─────────────────────────────────────────────────────
    support::task_with_output("#[serde(untagged)]: infer variant from shape", || {
        let svc = Service {
            name: "api".to_string(),
            endpoints: vec![
                Endpoint::Simple("https://api.example.com".to_string()),
                Endpoint::Detailed {
                    url: "https://api.staging.com".to_string(),
                    timeout: 30,
                },
            ],
        };
        let yaml = to_string(&svc).unwrap();
        let parsed: Service = from_str(&yaml).unwrap();
        assert_eq!(svc, parsed);
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // ── Internally tagged ────────────────────────────────────────────
    support::task_with_output("#[serde(tag = \"type\")]: discriminator field", || {
        let resources = vec![
            Resource::Database {
                host: "db.local".to_string(),
                port: 5432,
            },
            Resource::Cache {
                host: "redis.local".to_string(),
                ttl: 300,
            },
        ];
        let yaml = to_string(&resources).unwrap();
        let parsed: Vec<Resource> = from_str(&yaml).unwrap();
        assert_eq!(resources, parsed);
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // ── Roundtrip proof ──────────────────────────────────────────────
    support::task_with_output("All patterns roundtrip correctly", || {
        let doc = Document {
            title: "Test".to_string(),
            meta: Metadata {
                created_by: "bot".to_string(),
                version: 1,
            },
        };
        let yaml = to_string(&doc).unwrap();
        let rt: Document = from_str(&yaml).unwrap();
        vec![
            format!("title      = {}", rt.title),
            format!("created_by = {}", rt.meta.created_by),
            format!("version    = {}", rt.meta.version),
            format!("match      = {}", doc == rt),
        ]
    });

    support::summary(4);
}
