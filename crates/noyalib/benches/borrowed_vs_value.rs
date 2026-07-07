// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Zero-copy `BorrowedValue` vs allocating `Value` throughput.
//!
//! `BorrowedValue<'_>` reuses `&str` slices of the input instead
//! of allocating fresh `String`s for scalar values. On a document
//! dominated by long unquoted plain scalars — log payloads, YAML-
//! backed template files, translation catalogues — this can cut
//! per-parse allocations dramatically. This bench captures the
//! throughput delta so a future regression is visible.
//!
//! Run: `cargo bench --bench borrowed_vs_value`

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use noyalib::Value;
use noyalib::borrowed::{BorrowedValue, from_str_borrowed};
use std::hint::black_box;

/// Build `count` mapping records with a long unquoted string
/// payload each — the shape where zero-copy borrowing pays best.
fn string_heavy_yaml(count: usize) -> String {
    let mut s = String::with_capacity(count * 128);
    for i in 0..count {
        s.push_str(&format!(
            "- key: entry-{i}\n  \
             body: this-is-a-long-plain-scalar-value-{i}-that-is-designed-to-dominate-the-allocation-profile\n",
        ));
    }
    s
}

fn bench_borrowed_vs_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("borrowed_vs_value");

    for &(label, count) in &[
        ("small", 32usize),
        ("medium", 512usize),
        ("large", 4096usize),
    ] {
        let yaml = string_heavy_yaml(count);
        group.throughput(Throughput::Bytes(yaml.len() as u64));

        // Allocating: every scalar becomes a `String`.
        group.bench_with_input(BenchmarkId::new("value", label), &yaml, |b, y| {
            b.iter(|| {
                let v: Value = noyalib::from_str(black_box(y)).unwrap();
                black_box(v);
            });
        });

        // Borrowing: scalars are `Cow::Borrowed` from the input.
        group.bench_with_input(BenchmarkId::new("borrowed", label), &yaml, |b, y| {
            b.iter(|| {
                let v: BorrowedValue<'_> = from_str_borrowed(black_box(y)).unwrap();
                black_box(v);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_borrowed_vs_value);
criterion_main!(benches);
