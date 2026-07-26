// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Quick performance overview.
//!
//! Run: `cargo run --example bench --release`

#![allow(unused_results)]

#[path = "support.rs"]
mod support;

use std::time::{Duration, Instant};

use noyalib::{Value, from_str, to_string};

/// Benchmark a closure, return per-op duration.
fn bench<F: FnMut()>(iterations: usize, mut f: F) -> Duration {
    // Warmup
    for _ in 0..iterations / 10 {
        f();
    }
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed() / iterations as u32
}

/// Format a duration as a fixed-width right-aligned string (10 chars).
fn fmt_dur(d: Duration) -> String {
    if d.as_nanos() < 1_000 {
        format!("{:>7.3} ns", d.as_nanos() as f64)
    } else if d.as_nanos() < 1_000_000 {
        format!("{:>7.3} us", d.as_nanos() as f64 / 1_000.0)
    } else {
        format!("{:>7.3} ms", d.as_nanos() as f64 / 1_000_000.0)
    }
}

fn main() {
    support::header("noyalib -- bench");

    // Warn if running in debug mode
    if cfg!(debug_assertions) {
        println!(
            "  \x1b[33m!\x1b[0m \x1b[90mRunning in DEBUG mode. Use --release for accurate numbers.\x1b[0m\n"
        );
    }

    // ── Test data ────────────────────────────────────────────────────
    let simple_yaml = "name: test\nversion: 1\nenabled: true\n";

    let nested_yaml = r#"
server:
  host: localhost
  port: 8080
  ssl:
    enabled: true
    cert: /path/to/cert
    key: /path/to/key
database:
  host: db.example.com
  port: 5432
  credentials:
    username: admin
    password: secret
"#;

    let sequence_yaml = r#"
items:
  - name: item1
    value: 100
    tags: [tag1, tag2]
  - name: item2
    value: 200
    tags: [tag3]
  - name: item3
    value: 300
    tags: [tag4, tag5, tag6]
"#;

    let large_yaml: String = (0..50).map(|i| format!("key{i}: value{i}\n")).collect();

    // ── Parse ────────────────────────────────────────────────────────
    support::task_with_output("Parsing performance (10k iterations)", || {
        vec![
            format!(
                "simple   = {}/op",
                fmt_dur(bench(10_000, || {
                    let _: Value = from_str(simple_yaml).unwrap();
                }))
            ),
            format!(
                "nested   = {}/op",
                fmt_dur(bench(10_000, || {
                    let _: Value = from_str(nested_yaml).unwrap();
                }))
            ),
            format!(
                "sequence = {}/op",
                fmt_dur(bench(10_000, || {
                    let _: Value = from_str(sequence_yaml).unwrap();
                }))
            ),
            format!(
                "large    = {}/op",
                fmt_dur(bench(5_000, || {
                    let _: Value = from_str(&large_yaml).unwrap();
                }))
            ),
        ]
    });

    // ── Serialize ────────────────────────────────────────────────────
    let simple_v: Value = from_str(simple_yaml).unwrap();
    let nested_v: Value = from_str(nested_yaml).unwrap();
    let large_v: Value = from_str(&large_yaml).unwrap();

    support::task_with_output("Serialization performance (10k iterations)", || {
        vec![
            format!(
                "simple = {}/op",
                fmt_dur(bench(10_000, || {
                    let _ = to_string(&simple_v).unwrap();
                }))
            ),
            format!(
                "nested = {}/op",
                fmt_dur(bench(10_000, || {
                    let _ = to_string(&nested_v).unwrap();
                }))
            ),
            format!(
                "large  = {}/op",
                fmt_dur(bench(5_000, || {
                    let _ = to_string(&large_v).unwrap();
                }))
            ),
        ]
    });

    // ── Value operations ─────────────────────────────────────────────
    let nested: Value = from_str(nested_yaml).unwrap();

    support::task_with_output("Value operations (100k iterations)", || {
        vec![
            format!(
                "get()      = {}/op",
                fmt_dur(bench(100_000, || {
                    let _ = nested.get("server");
                }))
            ),
            format!(
                "get_path() = {}/op",
                fmt_dur(bench(100_000, || {
                    let _ = nested.get_path("server.ssl.enabled");
                }))
            ),
        ]
    });

    // ── Bulk processing ──────────────────────────────────────────────
    support::task_with_output("Bulk processing (1000 documents)", || {
        let start = Instant::now();
        let values: Vec<Value> = (0..1000)
            .map(|i| {
                let yaml = format!("id: {i}\nname: item_{i}\nvalue: {}", i * 100);
                from_str(&yaml).unwrap()
            })
            .collect();
        let parse_time = start.elapsed();

        let start = Instant::now();
        let _: Vec<String> = values.iter().map(|v| to_string(v).unwrap()).collect();
        let ser_time = start.elapsed();

        vec![
            format!("parse     = {}", fmt_dur(parse_time)),
            format!("serialize = {}", fmt_dur(ser_time)),
        ]
    });

    support::summary(4);
    println!("  \x1b[90mFor precise metrics, run: cargo bench\x1b[0m\n");
}
