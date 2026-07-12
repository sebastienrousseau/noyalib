// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Comment handling: attach, serialize, and the preservation boundary.
//!
//! YAML comments are stripped during parsing (per the YAML spec, comments
//! are not part of the data model). noyalib provides `Commented<T>` for
//! attaching comments during serialization.
//!
//! For true comment preservation (read → modify → write with comments
//! intact), use the shipped CST layer: `noyalib::cst::parse_document` +
//! `Document::comments_at` (see `comments_at.rs`, `lossless_edit.rs`,
//! `entry_api.rs`).
//!
//! Run: `cargo run --example comments`

#[path = "support.rs"]
mod support;

use noyalib::fmt::Commented;
use noyalib::{Value, from_str, to_string};
use serde::Serialize;

fn main() {
    support::header("noyalib -- comments");

    // ── Comments are stripped during parsing ──────────────────────────
    support::task_with_output("Comments stripped during parsing (YAML spec)", || {
        let yaml = r#"
# Database configuration
host: localhost  # primary host
port: 5432       # default postgres port
# Connection pool
pool_size: 10    # max connections
"#;
        let v: Value = from_str(yaml).unwrap();
        let output = to_string(&v).unwrap();
        vec![
            format!("input lines   = {} (with comments)", yaml.lines().count()),
            format!(
                "output lines  = {} (comments stripped)",
                output.lines().count()
            ),
            format!(
                "data intact   = {}",
                v.get("host").and_then(|v| v.as_str()).unwrap_or("?")
            ),
            "Comments are not part of the YAML data model.".to_string(),
        ]
    });

    // ── Commented<T>: attach comments during serialization ───────────
    support::task_with_output("Commented<T>: attach inline comments", || {
        #[derive(Serialize)]
        struct Config {
            host: Commented<String>,
            port: Commented<u16>,
            pool_size: Commented<u32>,
        }

        let config = Config {
            host: Commented::new("localhost".to_string(), "primary database host"),
            port: Commented::new(5432, "default postgres port"),
            pool_size: Commented::new(10, "max concurrent connections"),
        };

        let yaml = to_string(&config).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // ── Programmatic comment injection via Value ─────────────────────
    support::task_with_output("Inject comments into existing Values", || {
        let yaml = "host: localhost\nport: 5432\n";
        let v: Value = from_str(yaml).unwrap();

        // Wrap values with Commented<T> for serialization
        #[derive(Serialize)]
        struct Annotated {
            host: Commented<String>,
            port: Commented<i64>,
        }

        let annotated = Annotated {
            host: Commented::new(
                v.get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                "do not change in production",
            ),
            port: Commented::new(
                v.get("port").and_then(|v| v.as_i64()).unwrap_or(0),
                "standard postgres port",
            ),
        };

        let output = to_string(&annotated).unwrap();
        output.lines().map(|l| l.to_string()).collect()
    });

    // ── Preservation: data path vs CST path ──────────────────────────
    support::task_with_output("Comment preservation status", || {
        vec![
            "Data path: comments stripped (YAML spec compliant)".to_string(),
            "Writing:   Commented<T> adds inline comments".to_string(),
            "Data roundtrip: comments NOT preserved (by design)".to_string(),
            "CST path:  comments preserved byte-for-byte via \
                        noyalib::cst (see comments_at.rs)"
                .to_string(),
        ]
    });

    support::summary(4);
}
