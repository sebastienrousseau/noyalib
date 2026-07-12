// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! CST preservation: comment injection and formatting control.
//!
//! Demonstrates comment injection and formatting control on the
//! serialization side. For full lossless CST round-tripping (byte-for-byte
//! preserving edits), see the shipped `noyalib::cst` layer and the
//! `lossless_edit.rs` / `entry_api.rs` examples.
//!
//! Run: `cargo run --example preserve`

#[path = "support.rs"]
mod support;

use noyalib::fmt::{Commented, FlowMap, FlowSeq, LitString, SpaceAfter};
use noyalib::{Value, from_str, to_string};
use serde::Serialize;

fn main() {
    support::header("noyalib -- preserve");

    // ── Current: Commented<T> for write-only comments ────────────────
    support::task_with_output("Commented<T>: attach comments during serialization", || {
        #[derive(Serialize)]
        struct Config {
            host: Commented<String>,
            port: Commented<u16>,
            debug: Commented<bool>,
        }

        let config = Config {
            host: Commented::new("localhost".to_string(), "primary database host"),
            port: Commented::new(5432, "default postgres port"),
            debug: Commented::new(false, "set true for verbose logging"),
        };

        let yaml = to_string(&config).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // ── Current: SpaceAfter<T> for section spacing ───────────────────
    support::task_with_output("SpaceAfter<T>: blank lines between sections", || {
        #[derive(Serialize)]
        struct Doc {
            header: SpaceAfter<String>,
            body: String,
        }

        let doc = Doc {
            header: SpaceAfter("# Configuration".to_string()),
            body: "content here".to_string(),
        };

        let yaml = to_string(&doc).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // ── Current: FlowSeq/FlowMap for inline style preservation ───────
    support::task_with_output("FlowSeq/FlowMap: preserve inline style intent", || {
        #[derive(Serialize)]
        struct Manifest {
            tags: FlowSeq<Vec<String>>,
            metadata: FlowMap<std::collections::BTreeMap<String, String>>,
            description: LitString,
        }

        let mut meta = std::collections::BTreeMap::new();
        let _ = meta.insert("version".into(), "1.0".into());
        let _ = meta.insert("author".into(), "team".into());

        let manifest = Manifest {
            tags: FlowSeq(vec!["yaml".into(), "rust".into()]),
            metadata: FlowMap(meta),
            description: LitString("Multi-line\ndescription\nhere.\n".into()),
        };

        let yaml = to_string(&manifest).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // ── Current: Tagged values as extensibility hook ──────────────────
    support::task_with_output("Tagged values: extensibility hook for CST", || {
        let yaml = "!important critical-value\n";
        let v: Value = from_str(yaml).unwrap();

        // Tags survive round-trip via Value::Tagged
        let output = to_string(&v).unwrap();
        vec![
            format!("input:  {}", yaml.trim()),
            format!("output: {}", output.trim()),
            format!("tagged: {}", v.is_tagged()),
            "Tags provide structural hooks for future CST metadata.".to_string(),
        ]
    });

    // ── Limitation: comments stripped on parse ────────────────────────
    support::task_with_output("Limitation: comments stripped during parsing", || {
        let yaml = "# Database config\nhost: localhost  # primary\nport: 5432\n";
        let v: Value = from_str(yaml).unwrap();
        let output = to_string(&v).unwrap();
        vec![
            format!("input lines:  {} (with comments)", yaml.lines().count()),
            format!(
                "output lines: {} (comments stripped)",
                output.lines().count()
            ),
            "YAML spec: comments are not part of the data model.".to_string(),
            "Full CST preservation ships in noyalib::cst (see \
             lossless_edit.rs)."
                .to_string(),
        ]
    });

    // ── Two paths: serialization hooks vs the lossless CST ────────────
    support::task_with_output("Preservation options", || {
        vec![
            "Serialization hooks (this example):".to_string(),
            "  Commented<T>   — inject comments on write".to_string(),
            "  SpaceAfter<T>  — control blank lines".to_string(),
            "  FlowSeq/Map    — preserve inline style".to_string(),
            "  LitStr/FoldStr — preserve block scalar style".to_string(),
            "  Tagged values  — structural extensibility".to_string(),
            String::new(),
            "Lossless CST (noyalib::cst):".to_string(),
            "  cst::Document  — preserves all source bytes".to_string(),
            "  span_at/comments_at — retains comments + whitespace".to_string(),
            "  Surgical edits — modify one value, keep rest intact".to_string(),
            "  Full round-trip — read -> modify -> write without loss".to_string(),
        ]
    });

    support::summary(6);
}
