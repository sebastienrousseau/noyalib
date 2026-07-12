// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Large-document soak benchmark.
//!
//! Throughput on inputs sized 1 MiB / 10 MiB / 50 MiB. Drives the
//! full parse path — the SIMD-accelerated scanner, the loader's
//! anchor / alias bookkeeping, and the `Value` allocator. Useful
//! for catching:
//!
//! - Quadratic regressions on long scalars or dense mappings.
//! - Regressions in the SIMD hot-path `find_any_of` /
//!   `clean_prefix_len` primitives the scanner relies on.
//! - Memory-fragmentation issues that only surface at 10× / 100×
//!   the typical input size.
//!
//! Soak is NOT in the default CodSpeed CI run (it's bandwidth-
//! intensive); invoke with:
//!
//!     cargo bench --bench large_doc_soak
//!
//! On a 2025 MacBook Pro / M-series, expect ~150-250 MB/s on the
//! `from_str::<Value>` path for the 50 MiB input.

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use noyalib::{ParserConfig, StreamingDeserializer, Value, from_str_with_config};
use serde::Deserialize;

/// Budgets sized for the soak fixtures. The default `ParserConfig` DoS
/// guards (max_events 1M, max_nodes 250k, etc.) legitimately reject a
/// 50 MiB document; a large-doc soak is exactly the case where a caller
/// raises them, so parse the fixtures with the guards lifted.
fn soak_config() -> ParserConfig {
    let mut cfg = ParserConfig::default();
    cfg.max_events = usize::MAX;
    cfg.max_nodes = usize::MAX;
    cfg.max_document_length = usize::MAX;
    cfg.max_total_scalar_bytes = usize::MAX;
    cfg.max_sequence_length = usize::MAX;
    cfg.max_mapping_keys = usize::MAX;
    cfg
}
use std::hint::black_box;
use std::time::Duration;

/// Build a synthetic mapping-of-records YAML document of approximately
/// `target_bytes` total length. The shape mirrors a Kubernetes-ish
/// resource manifest catalogue: each "record" is a small mapping with
/// a name, version, and short comment-style description.
fn synthetic_yaml(target_bytes: usize) -> String {
    let template = "  - name: service-{i}\n    \
        version: 0.{i}.0\n    \
        replicas: {i}\n    \
        description: a synthetic record for soak benchmarking\n    \
        labels:\n      \
        tier: backend\n      \
        team: platform\n";
    // Estimate per-record bytes from the template.
    let approx_per_record = template.len() + 8;
    let count = (target_bytes / approx_per_record).max(1);

    let mut s = String::with_capacity(target_bytes + 64);
    s.push_str("services:\n");
    for i in 0..count {
        let block = template.replace("{i}", &i.to_string());
        s.push_str(&block);
    }
    s
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Doc {
    services: Vec<Record>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Record {
    name: String,
    version: String,
    replicas: u32,
    description: String,
    labels: std::collections::BTreeMap<String, String>,
}

fn bench_soak(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_doc_soak");
    // Soak runs are dominated by the largest single sample; cap to
    // a small sample count so a full run still finishes in minutes.
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));

    // 1 MiB / 10 MiB / 50 MiB.
    for &(label, target_bytes) in &[
        ("1MiB", 1usize << 20),
        ("10MiB", 10 * (1usize << 20)),
        ("50MiB", 50 * (1usize << 20)),
    ] {
        let yaml = synthetic_yaml(target_bytes);
        let cfg = soak_config();
        group.throughput(Throughput::Bytes(yaml.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("from_str_value", label),
            &yaml,
            |b, yaml| {
                b.iter(|| {
                    let v: Value = from_str_with_config(black_box(yaml.as_str()), &cfg).unwrap();
                    black_box(v);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("streaming_typed", label),
            &yaml,
            |b, yaml| {
                b.iter(|| {
                    let mut de = StreamingDeserializer::with_config(black_box(yaml.as_str()), &cfg);
                    let doc: Doc = Deserialize::deserialize(&mut de).unwrap();
                    black_box(doc);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_soak);
criterion_main!(benches);
