// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Benchmark the MCP tool dispatch core — the work each
//! `tools/call` performs *after* the JSON-RPC transport
//! decodes the frame. Each tool delegates to the noyalib
//! library's stable surface, so this bench effectively measures
//! the public-API throughput AI agents see when calling the
//! server in a tight loop (e.g. iterating over a directory of
//! manifests).
//!
//! The benchmarks construct the same call shape the agent
//! sends, then dispatch to the library function the server
//! ultimately calls. The JSON-RPC framing overhead is
//! constant-per-call (microseconds) and not measured here —
//! see the `handshake.sh` example for end-to-end timings.

#![allow(missing_docs, unused_results)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use noyalib::cst::{format, parse_document};

const YAML: &str = r#"
server:
  host: api.example.com
  port: 8080
  tls:
    enabled: true
    cert: /etc/ssl/api.pem
"#;

fn tool_format(c: &mut Criterion) {
    c.bench_function("tools/call format", |b| {
        b.iter(|| black_box(format(black_box(YAML)).unwrap()));
    });
}

fn tool_parse(c: &mut Criterion) {
    c.bench_function("tools/call parse", |b| {
        b.iter(|| {
            let v: noyalib::Value = black_box(noyalib::from_str(black_box(YAML)).unwrap());
            black_box(v);
        });
    });
}

fn tool_get(c: &mut Criterion) {
    // `get` reads a value at a dotted path. Two distinct cost
    // profiles: a path that exists (walks to a scalar) and a
    // path that doesn't (walks until a missing key, returns
    // None).
    let v: noyalib::Value = noyalib::from_str(YAML).unwrap();
    c.bench_function("tools/call get (existing path)", |b| {
        b.iter(|| black_box(v.get_path(black_box("server.port"))));
    });
    c.bench_function("tools/call get (missing path)", |b| {
        b.iter(|| black_box(v.get_path(black_box("server.missing"))));
    });
}

fn tool_set(c: &mut Criterion) {
    c.bench_function("tools/call set (CST surgical edit)", |b| {
        b.iter(|| {
            let mut doc = parse_document(black_box(YAML)).unwrap();
            doc.set(black_box("server.port"), black_box("9090"))
                .unwrap();
            black_box(doc.to_string());
        });
    });
}

criterion_group!(mcp_tools, tool_format, tool_parse, tool_get, tool_set);
criterion_main!(mcp_tools);
