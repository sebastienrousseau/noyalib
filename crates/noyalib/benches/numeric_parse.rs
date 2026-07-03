// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! SWAR vs. stdlib decimal-integer parse throughput.
//!
//! Compares two paths for converting an ASCII-digit byte slice to
//! `i64`/`u64`:
//!
//! 1. **Stdlib** — `<i64 as FromStr>::from_str(...).ok()`. The
//!    portable byte-by-byte loop the standard library walks.
//! 2. **SWAR** — `noyalib::simd::parse_decimal_{i64,u64}` — the
//!    SIMD-Within-A-Register pipeline that folds 8 digits per
//!    iteration via three pair-wise multiply-add phases.
//!
//! The SWAR path's win shows up on data-heavy workloads
//! (telemetry, port numbers, IDs in mappings) where every value
//! is parsed.
//!
//! This harness measures **scalar decimal parsing primitives** only,
//! not full YAML `from_str` resolution (see `benches/lossless_u64.rs`).
//!
//! Run: `cargo bench --bench numeric_parse`

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use noyalib::simd::{parse_decimal_i64, parse_decimal_u64};
use std::hint::black_box;

fn bench_parse_u64(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_decimal_u64");

    // Each input width is a representative real-world digit count:
    // - 1-3 digits: small ports, status codes, replica counts.
    // - 8-10 digits: timestamps, IDs, mid-size counters.
    // - 16-19 digits: nanosecond timestamps, BigInt counters.
    for &(label, val) in &[
        ("3_digits", 999u64),
        ("5_digits", 12_345u64),
        ("8_digits", 99_999_999u64),
        ("10_digits", 1_234_567_890u64),
        ("16_digits", 1_234_567_890_123_456u64),
        ("19_digits", 9_223_372_036_854_775_807u64),
        ("20_digits_u64_max", u64::MAX),
        ("i64_max_plus_1", i64::MAX as u64 + 1),
    ] {
        let s = val.to_string();
        let bytes = s.as_bytes();
        group.throughput(Throughput::Bytes(bytes.len() as u64));

        group.bench_with_input(BenchmarkId::new("stdlib", label), bytes, |b, bs| {
            b.iter(|| {
                let v: u64 = std::str::from_utf8(black_box(bs)).unwrap().parse().unwrap();
                black_box(v);
            });
        });

        group.bench_with_input(BenchmarkId::new("swar", label), bytes, |b, bs| {
            b.iter(|| {
                let v = parse_decimal_u64(black_box(bs)).unwrap();
                black_box(v);
            });
        });
    }

    group.finish();
}

fn bench_parse_i64(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_decimal_i64");

    for &(label, val) in &[
        ("positive_8_digits", 99_999_999i64),
        ("negative_8_digits", -99_999_999i64),
        ("i64_max", i64::MAX),
        ("i64_min", i64::MIN),
    ] {
        let s = val.to_string();
        let bytes = s.as_bytes();
        group.throughput(Throughput::Bytes(bytes.len() as u64));

        group.bench_with_input(BenchmarkId::new("stdlib", label), bytes, |b, bs| {
            b.iter(|| {
                let v: i64 = std::str::from_utf8(black_box(bs)).unwrap().parse().unwrap();
                black_box(v);
            });
        });

        group.bench_with_input(BenchmarkId::new("swar", label), bytes, |b, bs| {
            b.iter(|| {
                let v = parse_decimal_i64(black_box(bs)).unwrap();
                black_box(v);
            });
        });
    }

    group.finish();
}

/// Bulk parse — simulates a YAML document of port numbers / counter
/// values where every record carries a fresh integer parse.
fn bench_bulk_parse(c: &mut Criterion) {
    let values: Vec<String> = (0u64..1000)
        .map(|i| (i * 12345 + 100).to_string())
        .collect();
    let byte_slices: Vec<&[u8]> = values.iter().map(|s| s.as_bytes()).collect();
    let total_bytes: u64 = values.iter().map(|s| s.len() as u64).sum();

    let mut group = c.benchmark_group("bulk_parse_1000_integers");
    group.throughput(Throughput::Bytes(total_bytes));

    group.bench_function("stdlib", |b| {
        b.iter(|| {
            let mut sum: u64 = 0;
            for s in black_box(&byte_slices) {
                let v: u64 = std::str::from_utf8(s).unwrap().parse().unwrap();
                sum = sum.wrapping_add(v);
            }
            black_box(sum);
        });
    });

    group.bench_function("swar", |b| {
        b.iter(|| {
            let mut sum: u64 = 0;
            for s in black_box(&byte_slices) {
                sum = sum.wrapping_add(parse_decimal_u64(s).unwrap());
            }
            black_box(sum);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_parse_u64, bench_parse_i64, bench_bulk_parse);
criterion_main!(benches);
