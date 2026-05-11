// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Streaming vs `Value` deserialisation throughput.
//!
//! Compares two paths for typed deserialisation:
//!
//! 1. **Streaming** — `StreamingDeserializer` walks events directly
//!    into `T` with no intermediate AST allocation. This is the
//!    path `from_str` takes by default for plain inputs.
//! 2. **AST** — parses into `Value` first, then runs
//!    `T::deserialize` against the tree. Used as a fallback when
//!    the input needs span info, anchors, or other features the
//!    streaming path doesn't carry.
//!
//! The streaming path's advertised win comes from skipping the
//! intermediate allocation. This bench measures it on three sizes:
//! small (1 KB / typical config), medium (50 KB / sizeable
//! manifest), and large (500 KB / IaC monolith).
//!
//! Run: `cargo bench --bench streaming_vs_value`

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use noyalib::{StreamingDeserializer, from_str};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Doc {
    name: String,
    version: u32,
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Item {
    id: u32,
    label: String,
    enabled: bool,
}

fn make_yaml(item_count: usize) -> String {
    let mut s = String::with_capacity(item_count * 64);
    s.push_str("name: bench-doc\nversion: 1\nitems:\n");
    for i in 0..item_count {
        s.push_str(&format!(
            "  - id: {i}\n    label: item-{i}-label-text\n    enabled: true\n"
        ));
    }
    s
}

fn bench_streaming_vs_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_vs_value");

    for &(label, count) in &[
        ("small", 8usize),
        ("medium", 800usize),
        ("large", 8000usize),
    ] {
        let yaml = make_yaml(count);
        group.throughput(Throughput::Bytes(yaml.len() as u64));

        // ── Streaming path (no AST allocation) ─────────────────────
        group.bench_with_input(BenchmarkId::new("streaming", label), &yaml, |b, yaml| {
            b.iter(|| {
                let mut de = StreamingDeserializer::new(black_box(yaml.as_str()));
                let doc: Doc = Deserialize::deserialize(&mut de).unwrap();
                black_box(doc);
            });
        });

        // ── AST path (build Value, then deserialise) ───────────────
        group.bench_with_input(BenchmarkId::new("ast", label), &yaml, |b, yaml| {
            b.iter(|| {
                let v: noyalib::Value = from_str(black_box(yaml.as_str())).unwrap();
                let doc: Doc = noyalib::from_value(&v).unwrap();
                black_box(doc);
            });
        });
    }

    group.finish();
}

fn bench_dyn_mapping(c: &mut Criterion) {
    // BTreeMap target — exercises the streaming MapAccess path
    // against an AST-mediated deserialise of the same shape.
    let yaml = (0..1000)
        .map(|i| format!("k{i}: value-{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let yaml = format!("{yaml}\n");

    let mut group = c.benchmark_group("dyn_mapping_deserialize");
    group.throughput(Throughput::Bytes(yaml.len() as u64));

    group.bench_function("streaming", |b| {
        b.iter(|| {
            let mut de = StreamingDeserializer::new(black_box(yaml.as_str()));
            let m: BTreeMap<String, String> = Deserialize::deserialize(&mut de).unwrap();
            black_box(m);
        });
    });

    group.bench_function("ast", |b| {
        b.iter(|| {
            let v: noyalib::Value = from_str(black_box(yaml.as_str())).unwrap();
            let m: BTreeMap<String, String> = noyalib::from_value(&v).unwrap();
            black_box(m);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_streaming_vs_value, bench_dyn_mapping);
criterion_main!(benches);
