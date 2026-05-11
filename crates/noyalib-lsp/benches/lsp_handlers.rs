// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Benchmark the LSP request-handler core — the synchronous
//! work each `textDocument/*` method does *after* the JSON-RPC
//! transport layer has decoded the frame. Isolates the
//! formatter and parse-error-diagnostic latency from the
//! transport overhead so a regression in either is visible
//! independently.
//!
//! End-to-end editor latency = transport round-trip + handler
//! cost. For real-world editor experience the round-trip is
//! microseconds (local stdio); this bench measures the handler
//! half.

#![allow(missing_docs, unused_results)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use noyalib::cst::{format, parse_document};

const SMALL: &str = "host: api.example.com\nport: 8080\n";

const KUBE_MANIFEST: &str = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: noyalib-api
  labels:
    app: noyalib
    tier: backend
spec:
  replicas: 3
  selector:
    matchLabels:
      app: noyalib
  template:
    metadata:
      labels:
        app: noyalib
    spec:
      containers:
        - name: api
          image: registry.example.com/noyalib:1.0.0
          ports:
            - containerPort: 8080
          env:
            - name: PORT
              value: "8080"
          resources:
            limits:
              cpu: "500m"
              memory: "256Mi"
"#;

fn format_request(c: &mut Criterion) {
    let mut g = c.benchmark_group("textDocument/formatting");
    g.bench_function("small (2 keys)", |b| {
        b.iter(|| black_box(format(black_box(SMALL)).unwrap()));
    });
    g.bench_function("kube manifest (~30 keys)", |b| {
        b.iter(|| black_box(format(black_box(KUBE_MANIFEST)).unwrap()));
    });
    g.finish();
}

fn parse_for_diagnostics(c: &mut Criterion) {
    // `textDocument/publishDiagnostics` runs on every didChange.
    // The cost is dominated by the parse — the diagnostic
    // production itself is just walking the parser's error
    // location once.
    let mut g = c.benchmark_group("publishDiagnostics: parse cost");
    g.bench_function("small (2 keys)", |b| {
        b.iter(|| black_box(parse_document(black_box(SMALL)).unwrap()));
    });
    g.bench_function("kube manifest", |b| {
        b.iter(|| black_box(parse_document(black_box(KUBE_MANIFEST)).unwrap()));
    });
    g.finish();
}

criterion_group!(handlers, format_request, parse_for_diagnostics);
criterion_main!(handlers);
