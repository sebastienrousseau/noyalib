// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The competitor matrix from the architecture benchmarks: noyalib,
//! serde-saphyr and yaml-rust2 on the same Kubernetes and zero-copy
//! payloads that `crates/noyalib/benches/architecture.rs` measures.
//!
//! Run: `cargo bench -p noyalib-compare --bench competitors`

#![allow(missing_docs, unused_results)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

const K8S_DEPLOYMENT: &str = include_str!("../../noyalib/benches/fixtures/architecture_k8s.yaml");
const ZERO_COPY: &str = include_str!("../../noyalib/benches/fixtures/zero_copy.yaml");

// ── Benchmark: Competitor Matrix ─────────────────────────────────────

fn bench_competitors(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitors");

    // K8s payload across all libraries
    group.bench_function("noyalib/k8s", |b| {
        b.iter(|| {
            let _: noyalib::Value = noyalib::from_str(black_box(K8S_DEPLOYMENT)).unwrap();
        });
    });

    group.bench_function("serde_saphyr/k8s", |b| {
        b.iter(|| {
            let _: serde_json::Value = serde_saphyr::from_str(black_box(K8S_DEPLOYMENT)).unwrap();
        });
    });

    group.bench_function("yaml_rust2/k8s", |b| {
        b.iter(|| {
            let _ = yaml_rust2::YamlLoader::load_from_str(black_box(K8S_DEPLOYMENT)).unwrap();
        });
    });

    // Zero-copy payload
    group.bench_function("noyalib/zero_copy", |b| {
        b.iter(|| {
            let _: noyalib::Value = noyalib::from_str(black_box(ZERO_COPY)).unwrap();
        });
    });

    group.bench_function("serde_saphyr/zero_copy", |b| {
        b.iter(|| {
            let _: serde_json::Value = serde_saphyr::from_str(black_box(ZERO_COPY)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_competitors);
criterion_main!(benches);
