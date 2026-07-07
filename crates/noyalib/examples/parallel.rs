// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Parallel multi-document YAML parsing with Rayon.
//!
//! Requires `--features parallel`. For massive multi-document
//! streams (telemetry, audit exports, Kubernetes-resource
//! snapshots — anything emitting `---`-separated documents at
//! scale), single-threaded parsing is bounded by one CPU core.
//! The `parallel` module pre-scans the input, splits it into
//! per-document slices, and dispatches each slice to a Rayon
//! worker.
//!
//! Run:
//! ```text
//! cargo run --example parallel --features parallel --release
//! ```

use noyalib::Value;
use serde::Deserialize;
use std::time::Instant;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Event {
    id: u32,
    kind: String,
    payload: String,
}

fn build_stream(document_count: usize) -> String {
    // Give each document enough body that per-document parse time
    // outweighs Rayon's task-queue overhead. With tiny documents
    // the sequential path wins because the parser is faster than
    // the thread hand-off. A realistic multi-doc workload lives
    // somewhere between "trivial" and "huge single doc" — this
    // shape is representative of structured-audit records.
    let mut s = String::with_capacity(document_count * 320);
    for i in 0..document_count {
        s.push_str(&format!("---\nid: {i}\nkind: audit\npayload: entry-{i}-",));
        for j in 0..12 {
            s.push_str(&format!("field{j}-value-{i}-{j} "));
        }
        s.push('\n');
    }
    s
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 900 fits under the default `max_documents = 1000` budget so
    // the sequential baseline runs without a config override —
    // keeps the example dependency-light. A production caller
    // parsing millions of records would raise the budget via
    // `ParserConfig::new().max_documents(...)` and use
    // `load_all_with_config` / `from_str_with_config`.
    let stream = build_stream(900);
    let size_kb = stream.len() / 1024;
    println!("stream: 900 documents, {size_kb} KB");

    // Sequential baseline: `load_all_as` walks the events on the
    // calling thread, one document at a time.
    let t0 = Instant::now();
    let sequential: Vec<Event> = noyalib::load_all_as(&stream)?;
    let sequential_ms = t0.elapsed().as_secs_f64() * 1_000.0;
    println!(
        "sequential (load_all_as): {} events in {sequential_ms:.1} ms",
        sequential.len()
    );

    // Parallel: `parallel::parse` splits + dispatches across the
    // Rayon global thread pool. Same result set, just built with
    // multiple cores.
    let t1 = Instant::now();
    let parallel: Vec<Event> = noyalib::parallel::parse(&stream)?;
    let parallel_ms = t1.elapsed().as_secs_f64() * 1_000.0;
    println!(
        "parallel (parallel::parse): {} events in {parallel_ms:.1} ms",
        parallel.len()
    );

    println!("speedup: {:.2}×", sequential_ms / parallel_ms.max(0.001));

    // Dynamic-tree variant — same shape, but the caller wants
    // `Value` back so downstream code can route by document type.
    let values: Vec<Value> = noyalib::parallel::values(&stream)?;
    println!("parallel::values: {} Values", values.len());

    // Standalone boundary scan — for callers that drive their
    // own concurrency (async tasks, custom thread pools) instead
    // of Rayon.
    let slices = noyalib::parallel::split(&stream);
    println!("parallel::split: {} raw document slices", slices.len());

    Ok(())
}
