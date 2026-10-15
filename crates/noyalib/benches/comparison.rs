//! Head-to-head benchmarks: noyalib vs yaml-rust2 and yaml-spanned
//! (plus serde-saphyr under `--features compare-saphyr`).
//!
//! `serde_yaml_ng` was retired from this harness in v0.0.58: it
//! depends on `unsafe-libyaml`, which RUSTSEC-2024-0449 marks
//! unmaintained, and the policy in `deny.toml` is to retire such
//! contenders rather than exempt them (as `serde_yml` was in v0.0.6).
//! The typed, serialize and roundtrip groups now measure noyalib on
//! its own.
//!
//! Run: `cargo bench --bench comparison`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

// ── Test Data ────────────────────────────────────────────────────────

const SIMPLE: &str = "name: myapp\nversion: 1\nenabled: true\n";

const NESTED: &str = "\
server:
  host: localhost
  port: 8080
  ssl:
    enabled: true
    cert: /etc/ssl/cert.pem
    key: /etc/ssl/key.pem
database:
  host: db.example.com
  port: 5432
  name: production
  pool:
    min: 5
    max: 25
    timeout: 30
logging:
  level: info
  format: json
  outputs:
    - type: stdout
    - type: file
      path: /var/log/app.log
";

const LARGE_LIST: &str = include_str!("fixtures/large_list.yaml");
// Real-world-shape fixtures: deep indent, comments, multi-document.
// `large_list` is synthetic; the two below exercise the parser the way
// production YAML config files actually do.
const K8S_DEPLOYMENT: &str = include_str!("fixtures/k8s_deployment.yaml");
const GITHUB_ACTIONS: &str = include_str!("fixtures/github_actions.yaml");

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Simple {
    name: String,
    version: u32,
    enabled: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Nested {
    server: Server,
    database: Database,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Server {
    host: String,
    port: u16,
    ssl: Ssl,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Ssl {
    enabled: bool,
    cert: String,
    key: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Database {
    host: String,
    port: u16,
    name: String,
}

// ── Deserialization Benchmarks ───────────────────────────────────────

fn bench_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize");

    for (name, yaml) in [
        ("simple", SIMPLE),
        ("nested", NESTED),
        ("large_list", LARGE_LIST),
        // Real-world fixtures: deep indent, comments, anchors.
        ("github_actions", GITHUB_ACTIONS),
    ] {
        group.bench_with_input(BenchmarkId::new("noyalib", name), yaml, |b, input| {
            b.iter(|| {
                let _: noyalib::Value = noyalib::from_str(black_box(input)).unwrap();
            });
        });

        group.bench_with_input(BenchmarkId::new("yaml-rust2", name), yaml, |b, input| {
            b.iter(|| {
                let _ = yaml_rust2::YamlLoader::load_from_str(black_box(input)).unwrap();
            });
        });

        // (`serde_yml` was previously benched here as the maintainer's
        // prior crate. RUSTSEC-2025-0068 marked it unsound +
        // unmaintained; the comparison row was retired so it no
        // longer appears in `Cargo.lock` and the OpenSSF Scorecard
        // Vulnerabilities check is clean. `serde_yaml_ng` followed
        // in v0.0.58 for the same reason: RUSTSEC-2024-0449.)

        // `yaml-spanned` — the closest other span-tracking parser in
        // the ecosystem. Returns a sequence of spanned values.
        group.bench_with_input(BenchmarkId::new("yaml-spanned", name), yaml, |b, input| {
            b.iter(|| {
                let _ = yaml_spanned::from_str(black_box(input)).unwrap();
            });
        });

        #[cfg(feature = "compare-saphyr")]
        group.bench_with_input(BenchmarkId::new("serde-saphyr", name), yaml, |b, input| {
            b.iter(|| {
                let _: serde_json::Value = serde_saphyr::from_str(black_box(input)).unwrap();
            });
        });
    }

    // Multi-document load — represents real-world Kubernetes /
    // Helm / Compose workflows where one file holds 3-30 manifests.
    // yaml-rust2 returns `Vec<Yaml>` from one call.
    group.bench_with_input(
        BenchmarkId::new("noyalib", "k8s_multidoc"),
        K8S_DEPLOYMENT,
        |b, input| {
            b.iter(|| {
                let _ = noyalib::load_all_as::<noyalib::Value>(black_box(input)).unwrap();
            });
        },
    );
    group.bench_with_input(
        BenchmarkId::new("yaml-rust2", "k8s_multidoc"),
        K8S_DEPLOYMENT,
        |b, input| {
            b.iter(|| {
                let _ = yaml_rust2::YamlLoader::load_from_str(black_box(input)).unwrap();
            });
        },
    );

    group.finish();
}

// ── Typed Deserialization Benchmarks ─────────────────────────────────

fn bench_typed_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("typed_deserialize");

    group.bench_function("noyalib/simple", |b| {
        b.iter(|| {
            let _: Simple = noyalib::from_str(black_box(SIMPLE)).unwrap();
        });
    });

    group.bench_function("noyalib/nested", |b| {
        b.iter(|| {
            let _: Nested = noyalib::from_str(black_box(NESTED)).unwrap();
        });
    });

    group.finish();
}

// ── Serialization Benchmarks ─────────────────────────────────────────

fn bench_serialize(c: &mut Criterion) {
    let noya_simple: noyalib::Value = noyalib::from_str(SIMPLE).unwrap();
    let noya_nested: noyalib::Value = noyalib::from_str(NESTED).unwrap();

    let mut group = c.benchmark_group("serialize");

    group.bench_function("noyalib/simple", |b| {
        b.iter(|| {
            let _ = noyalib::to_string(black_box(&noya_simple)).unwrap();
        });
    });

    group.bench_function("noyalib/nested", |b| {
        b.iter(|| {
            let _ = noyalib::to_string(black_box(&noya_nested)).unwrap();
        });
    });

    group.finish();
}

// ── Roundtrip Benchmarks ─────────────────────────────────────────────

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    group.bench_function("noyalib/nested", |b| {
        b.iter(|| {
            let v: noyalib::Value = noyalib::from_str(black_box(NESTED)).unwrap();
            let _ = noyalib::to_string(&v).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_deserialize,
    bench_typed_deserialize,
    bench_serialize,
    bench_roundtrip,
);
criterion_main!(benches);
