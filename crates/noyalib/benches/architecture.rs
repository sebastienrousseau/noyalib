// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Architecture benchmarks: streaming vs AST, span overhead, security limits.
//!
//! Validates the performance wins from:
//! - Phase D (streaming deserializer bypassing Value AST)
//! - Phase C (NoSpanLoader vs span-tracking loader)
//! - Security limits (billion-laughs rejection speed)
//! - Zero-copy scalars (Cow::Borrowed vs String allocation)
//!
//! Run: `cargo bench --bench architecture`

#![allow(missing_docs, unused_results)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use serde::Deserialize;

// ── Test Payloads ────────────────────────────────────────────────────

/// Kubernetes-style config: medium depth, mixed types, sequences.
const K8S_DEPLOYMENT: &str = "\
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web-app
  namespace: production
  labels:
    app: web-app
    version: v2.1.0
    tier: frontend
spec:
  replicas: 3
  selector:
    matchLabels:
      app: web-app
  template:
    metadata:
      labels:
        app: web-app
    spec:
      containers:
        - name: web
          image: registry.io/web-app:v2.1.0
          ports:
            - containerPort: 8080
              protocol: TCP
          resources:
            limits:
              cpu: 500m
              memory: 256Mi
            requests:
              cpu: 100m
              memory: 128Mi
          env:
            - name: DATABASE_URL
              value: postgres://db:5432/prod
            - name: REDIS_URL
              value: redis://cache:6379
        - name: sidecar
          image: registry.io/envoy:v1.28
          ports:
            - containerPort: 9901
";

/// Zero-copy payload: all plain scalars, no escapes, no multiline.
const ZERO_COPY: &str = "\
host: localhost
port: 8080
name: myservice
version: 42
debug: false
timeout: 30000
retries: 3
workers: 8
log_level: info
format: json
output: stdout
max_connections: 1000
idle_timeout: 60
graceful_shutdown: 15
tls_enabled: true
tls_port: 8443
";

/// Deeply nested YAML (20 levels).
fn deep_yaml(depth: usize) -> String {
    let mut s = String::new();
    for i in 0..depth {
        let indent = "  ".repeat(i);
        s.push_str(&format!("{indent}level_{i}:\n"));
    }
    let indent = "  ".repeat(depth);
    s.push_str(&format!("{indent}value: leaf\n"));
    s
}

/// Billion-laughs attack payload.
fn billion_laughs(depth: usize) -> String {
    let mut yaml = String::from("a: &a x\n");
    for i in 1..depth {
        yaml.push_str(&format!(
            "{}: &{} [*{prev}, *{prev}, *{prev}]\n",
            (b'a' + i as u8) as char,
            (b'a' + i as u8) as char,
            prev = (b'a' + (i - 1) as u8) as char
        ));
    }
    yaml.push_str(&format!("z: *{}\n", (b'a' + (depth - 1) as u8) as char));
    yaml
}

// ── Typed structs for streaming vs AST comparison ────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct K8sMetadata {
    name: String,
    namespace: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct K8sDeployment {
    #[serde(rename = "apiVersion")]
    api_version: String,
    kind: String,
    metadata: K8sMetadata,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ZeroCopyConfig {
    host: String,
    port: u16,
    name: String,
    version: u32,
    debug: bool,
    timeout: u64,
    retries: u8,
    workers: u8,
}

// ── Benchmark: Streaming vs Value AST ────────────────────────────────

fn bench_streaming_vs_ast(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_vs_ast");

    // Typed deserialization uses streaming (bypasses Value)
    group.bench_function("streaming/k8s_typed", |b| {
        b.iter(|| {
            let _: K8sDeployment = noyalib::from_str(black_box(K8S_DEPLOYMENT)).unwrap();
        });
    });

    // Value deserialization builds full AST
    group.bench_function("ast/k8s_value", |b| {
        b.iter(|| {
            let _: noyalib::Value = noyalib::from_str(black_box(K8S_DEPLOYMENT)).unwrap();
        });
    });

    // Zero-copy typed (streaming) — most scalars borrow
    group.bench_function("streaming/zero_copy_typed", |b| {
        b.iter(|| {
            let _: ZeroCopyConfig = noyalib::from_str(black_box(ZERO_COPY)).unwrap();
        });
    });

    // Zero-copy Value — still builds AST but scalars borrow in scanner
    group.bench_function("ast/zero_copy_value", |b| {
        b.iter(|| {
            let _: noyalib::Value = noyalib::from_str(black_box(ZERO_COPY)).unwrap();
        });
    });

    group.finish();
}

// ── Benchmark: Span Tracking Overhead ────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SpannedConfig {
    host: noyalib::Spanned<String>,
    port: noyalib::Spanned<u16>,
    name: noyalib::Spanned<String>,
    version: noyalib::Spanned<u32>,
    debug: noyalib::Spanned<bool>,
    timeout: noyalib::Spanned<u64>,
    retries: noyalib::Spanned<u8>,
    workers: noyalib::Spanned<u8>,
}

fn bench_span_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("span_overhead");

    // Typed struct with Spanned<T> fields — hits the HashMap lookup
    // path inside ValueSeqAccess / SpannedMapAccess for every field.
    group.bench_function("spanned_fields/8_fields", |b| {
        b.iter(|| {
            let _: SpannedConfig = noyalib::from_str_with_config(
                black_box(ZERO_COPY),
                &noyalib::ParserConfig::default(),
            )
            .unwrap();
        });
    });

    // from_str (streaming, no spans)
    group.bench_function("no_spans/from_str", |b| {
        b.iter(|| {
            let _: ZeroCopyConfig = noyalib::from_str(black_box(ZERO_COPY)).unwrap();
        });
    });

    // from_str_with_config (builds spans for Spanned<T> support)
    group.bench_function("with_spans/from_str_with_config", |b| {
        b.iter(|| {
            let _: ZeroCopyConfig = noyalib::from_str_with_config(
                black_box(ZERO_COPY),
                &noyalib::ParserConfig::default(),
            )
            .unwrap();
        });
    });

    // Value path: no spans (from_slice uses NoSpanLoader)
    group.bench_function("no_spans/value_from_slice", |b| {
        let bytes = ZERO_COPY.as_bytes();
        b.iter(|| {
            let _: noyalib::Value = noyalib::from_slice(black_box(bytes)).unwrap();
        });
    });

    // Value path: with spans (from_str_with_config)
    group.bench_function("with_spans/value_from_str_config", |b| {
        b.iter(|| {
            let _: noyalib::Value = noyalib::from_str_with_config(
                black_box(ZERO_COPY),
                &noyalib::ParserConfig::default(),
            )
            .unwrap();
        });
    });

    group.finish();
}

// ── Benchmark: Security (Billion Laughs) ─────────────────────────────

fn bench_security(c: &mut Criterion) {
    let mut group = c.benchmark_group("security");

    let attack_small = billion_laughs(8); // 8 levels of exponential expansion
    let attack_large = billion_laughs(12); // 12 levels — would be 531K nodes without limits

    // Config with very low alias limit to ensure rejection
    let strict = noyalib::ParserConfig::new()
        .max_alias_expansions(4)
        .max_depth(10);

    group.bench_function("reject_billion_laughs_8", |b| {
        b.iter(|| {
            let result: Result<noyalib::Value, _> =
                noyalib::from_str_with_config(black_box(&attack_small), &strict);
            let _ = black_box(result);
        });
    });

    group.bench_function("reject_billion_laughs_12", |b| {
        b.iter(|| {
            let result: Result<noyalib::Value, _> =
                noyalib::from_str_with_config(black_box(&attack_large), &strict);
            let _ = black_box(result);
        });
    });

    // Deep nesting rejection
    let deep = deep_yaml(50);
    let depth_config = noyalib::ParserConfig::new().max_depth(10);
    group.bench_function("reject_deep_nesting_50", |b| {
        b.iter(|| {
            let result: Result<noyalib::Value, _> =
                noyalib::from_str_with_config(black_box(&deep), &depth_config);
            let _ = black_box(result);
        });
    });

    group.finish();
}

// ── Benchmark: Competitor Matrix ─────────────────────────────────────

fn bench_competitors(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitors");

    // K8s payload across all libraries
    group.bench_function("noyalib/k8s", |b| {
        b.iter(|| {
            let _: noyalib::Value = noyalib::from_str(black_box(K8S_DEPLOYMENT)).unwrap();
        });
    });

    group.bench_function("serde_yaml_ng/k8s", |b| {
        b.iter(|| {
            let _: serde_yaml_ng::Value =
                serde_yaml_ng::from_str(black_box(K8S_DEPLOYMENT)).unwrap();
        });
    });

    #[cfg(feature = "compare-saphyr")]
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

    group.bench_function("serde_yaml_ng/zero_copy", |b| {
        b.iter(|| {
            let _: serde_yaml_ng::Value = serde_yaml_ng::from_str(black_box(ZERO_COPY)).unwrap();
        });
    });

    #[cfg(feature = "compare-saphyr")]
    group.bench_function("serde_saphyr/zero_copy", |b| {
        b.iter(|| {
            let _: serde_json::Value = serde_saphyr::from_str(black_box(ZERO_COPY)).unwrap();
        });
    });

    group.finish();
}

// ── Benchmark: Borrowed vs Owned Value ───────────────────────────────

fn bench_borrowed_vs_owned(c: &mut Criterion) {
    let mut group = c.benchmark_group("borrowed_vs_owned");

    group.bench_function("owned/k8s", |b| {
        b.iter(|| {
            let _: noyalib::Value = noyalib::from_str(black_box(K8S_DEPLOYMENT)).unwrap();
        });
    });

    group.bench_function("borrowed/k8s", |b| {
        b.iter(|| {
            let _ = noyalib::borrowed::from_str_borrowed(black_box(K8S_DEPLOYMENT)).unwrap();
        });
    });

    group.bench_function("owned/zero_copy", |b| {
        b.iter(|| {
            let _: noyalib::Value = noyalib::from_str(black_box(ZERO_COPY)).unwrap();
        });
    });

    group.bench_function("borrowed/zero_copy", |b| {
        b.iter(|| {
            let _ = noyalib::borrowed::from_str_borrowed(black_box(ZERO_COPY)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_streaming_vs_ast,
    bench_span_overhead,
    bench_security,
    bench_borrowed_vs_owned,
    bench_competitors,
);
criterion_main!(benches);
