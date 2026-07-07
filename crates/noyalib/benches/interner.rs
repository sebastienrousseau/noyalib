// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Key-interning throughput on repeated-key workloads.
//!
//! Compares three shapes:
//!
//! 1. **Naïve `String::from(&str)`** — the baseline every fresh
//!    `Value` parse pays. One heap allocation per key occurrence.
//! 2. **`Arc::from(&str)`** — allocates a fresh `Arc<str>` each
//!    time. Same allocation count, different layout — proves the
//!    interner's win comes from the *cache*, not the `Arc`.
//! 3. **`KeyInterner::intern`** — the noyalib primitive. First
//!    call allocates, every re-intern returns a shared `Arc`.
//!
//! Run: `cargo bench --bench interner`

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use noyalib::interner::KeyInterner;
use std::hint::black_box;
use std::sync::Arc;

/// Realistic Kubernetes-shaped key set — small unique-count with
/// heavy repetition per record.
const KEYS: &[&str] = &[
    "apiVersion",
    "kind",
    "metadata",
    "name",
    "namespace",
    "labels",
    "annotations",
    "spec",
    "status",
    "creationTimestamp",
];

fn bench_interner(c: &mut Criterion) {
    let mut group = c.benchmark_group("interner");

    for &record_count in &[100usize, 1_000, 10_000] {
        let total_ops = record_count * KEYS.len();
        group.throughput(Throughput::Elements(total_ops as u64));

        group.bench_with_input(
            BenchmarkId::new("naive_string", record_count),
            &record_count,
            |b, &n| {
                b.iter(|| {
                    let mut buf: Vec<String> = Vec::with_capacity(n * KEYS.len());
                    for _ in 0..n {
                        for k in KEYS {
                            buf.push(black_box(*k).to_owned());
                        }
                    }
                    black_box(buf);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("naive_arc", record_count),
            &record_count,
            |b, &n| {
                b.iter(|| {
                    let mut buf: Vec<Arc<str>> = Vec::with_capacity(n * KEYS.len());
                    for _ in 0..n {
                        for k in KEYS {
                            buf.push(Arc::from(black_box(*k)));
                        }
                    }
                    black_box(buf);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("interner", record_count),
            &record_count,
            |b, &n| {
                b.iter(|| {
                    let mut interner = KeyInterner::new();
                    let mut buf: Vec<Arc<str>> = Vec::with_capacity(n * KEYS.len());
                    for _ in 0..n {
                        for k in KEYS {
                            buf.push(interner.intern(black_box(*k)));
                        }
                    }
                    black_box(buf);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_interner);
criterion_main!(benches);
