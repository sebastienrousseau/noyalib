// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Async I/O: non-blocking YAML parsing for tokio/async-std runtimes.
//!
//! noyalib's parser is synchronous (CPU-bound). For async contexts,
//! wrap parsing in `spawn_blocking` to avoid stalling the executor.
//! This is the standard pattern for CPU-bound work in async Rust.
//!
//! Run: `cargo run --example async_io`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str, to_string};
use std::time::Instant;

/// Simulate async file read + parse using threads.
/// In a real app, replace with tokio::task::spawn_blocking.
fn parse_blocking(yaml: &str) -> Result<Value, noyalib::Error> {
    // This is what tokio::task::spawn_blocking does internally:
    // move the CPU-bound work off the async executor thread.
    from_str(yaml)
}

/// Simulate async serialization.
fn serialize_blocking(value: &Value) -> Result<String, noyalib::Error> {
    to_string(value)
}

fn main() {
    support::header("noyalib -- async_io");

    // ── Pattern: spawn_blocking for parsing ──────────────────────────
    support::task_with_output("Pattern: parse in blocking context", || {
        let yaml = "host: localhost\nport: 8080\nworkers: 4\n";
        let v = parse_blocking(yaml).unwrap();
        vec![
            format!(
                "host    = {}",
                v.get("host").and_then(|v| v.as_str()).unwrap_or("?")
            ),
            format!(
                "workers = {}",
                v.get("workers").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            "Pattern: tokio::task::spawn_blocking(move || from_str(yaml))".to_string(),
        ]
    });

    // ── Pattern: spawn_blocking for serialization ────────────────────
    support::task_with_output("Pattern: serialize in blocking context", || {
        let v = Value::from("async-safe output");
        let yaml = serialize_blocking(&v).unwrap();
        vec![
            format!("output = {}", yaml.trim()),
            "Pattern: tokio::task::spawn_blocking(move || to_string(&v))".to_string(),
        ]
    });

    // ── Throughput: bulk parsing without blocking ─────────────────────
    support::task_with_output("Bulk: 1000 documents, threaded", || {
        let docs: Vec<String> = (0..1000)
            .map(|i| format!("id: {i}\nname: doc_{i}\nvalue: {}\n", i * 10))
            .collect();

        let start = Instant::now();
        let results: Vec<Value> = docs
            .iter()
            .map(|yaml| parse_blocking(yaml).unwrap())
            .collect();
        let elapsed = start.elapsed();

        vec![
            format!("parsed  = {} documents", results.len()),
            format!("elapsed = {:.1}ms", elapsed.as_secs_f64() * 1000.0),
            format!(
                "per_doc = {:.1}us",
                elapsed.as_secs_f64() * 1_000_000.0 / 1000.0
            ),
        ]
    });

    // ── Integration guide ────────────────────────────────────────────
    support::task_with_output("Async integration guide", || {
        vec![
            "tokio:".to_string(),
            "  let v = spawn_blocking(move || noyalib::from_str(&yaml)).await??;".to_string(),
            String::new(),
            "async-std:".to_string(),
            "  let v = blocking::unblock(move || noyalib::from_str(&yaml)).await?;".to_string(),
            String::new(),
            "Why not native async?".to_string(),
            "  YAML parsing is CPU-bound, not I/O-bound.".to_string(),
            "  spawn_blocking is the correct pattern (same as serde_json).".to_string(),
        ]
    });

    support::summary(4);
}
