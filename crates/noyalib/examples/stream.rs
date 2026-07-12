// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Multi-document YAML streams separated by `---`.
//!
//! Run: `cargo run --example stream`

#[path = "support.rs"]
mod support;

use noyalib::{Value, load_all, load_all_as, try_load_all};
use serde::Deserialize;

/// Format a Value as a compact one-liner (no debug dump).
fn compact(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("\"{s}\""),
        Value::Sequence(seq) => {
            let items: Vec<String> = seq.iter().map(compact).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Mapping(map) => {
            let entries: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{k}: {}", compact(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
        Value::Tagged(t) => format!("!{} {}", t.tag(), compact(t.value())),
    }
}

fn main() {
    support::header("noyalib -- stream");

    let yaml = "---\nname: document1\nvalue: 100\n---\nname: document2\nvalue: 200\n---\nname: document3\nvalue: 300\n";

    // ── load_all: iterate ────────────────────────────────────────────
    support::task_with_output("load_all: iterate documents", || {
        load_all(yaml)
            .unwrap()
            .enumerate()
            .filter_map(|(i, r)| {
                r.ok().map(|doc| {
                    let name = doc.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    let val = doc.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    format!("[Doc {}] name = {}, value = {}", i + 1, name, val)
                })
            })
            .collect()
    });

    // ── try_load_all: collected ──────────────────────────────────────
    support::task_with_output("try_load_all: collected results", || {
        let iter = try_load_all(yaml).unwrap();
        let mut lines = vec![format!("Status: {} documents found", iter.len())];
        for (i, result) in iter.enumerate() {
            if let Ok(doc) = result {
                lines.push(format!("[Doc {}] {}", i + 1, compact(&doc)));
            }
        }
        lines
    });

    // ── load_all_as: typed structs ───────────────────────────────────
    support::task_with_output("load_all_as: typed deserialization", || {
        #[derive(Debug, Deserialize)]
        struct Config {
            name: String,
            value: i32,
        }
        let docs: Vec<Config> = load_all_as(yaml).unwrap();
        docs.iter()
            .map(|d| format!("{} = {}", d.name, d.value))
            .collect()
    });

    // ── Mixed types ──────────────────────────────────────────────────
    support::task_with_output("Different document types", || {
        let mixed = "---\n42\n---\nhello world\n---\n- item1\n- item2\n- item3\n---\nkey: value\nnested:\n  a: 1\n  b: 2\n";
        load_all(mixed)
            .unwrap()
            .enumerate()
            .filter_map(|(i, r)| {
                r.ok().map(|doc| {
                    let kind = match &doc {
                        Value::Number(_) => "Number",
                        Value::String(_) => "String",
                        Value::Sequence(_) => "Sequence",
                        Value::Mapping(_) => "Mapping",
                        _ => "Other",
                    };
                    format!("[Doc {}] {} ({})", i + 1, compact(&doc), kind)
                })
            })
            .collect()
    });

    support::summary(4);
}
