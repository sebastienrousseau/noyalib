// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 4 — throughput benchmark for the SIMD-friendly
//! `find_any_of` primitive vs. a byte-by-byte scalar baseline.
//!
//! Run with:
//!     cargo bench --features simd --bench simd
//!
//! The bench sweeps three needle arities (1, 3, 8) across two
//! haystack sizes (4 KiB, 64 KiB) and two needle densities (sparse
//! / dense). Memchr's vectorised arity-1/2/3 paths and the
//! arity-4+ SWAR path are exercised; the scalar baseline is the
//! same byte-by-byte loop a hand-written parser would use.

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use noyalib::simd::find_any_of;

fn scalar_find_any_of(haystack: &[u8], needles: &[u8]) -> Option<usize> {
    for (i, &b) in haystack.iter().enumerate() {
        if needles.contains(&b) {
            return Some(i);
        }
    }
    None
}

fn make_haystack(size: usize, dense: bool, needles: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; size];
    // Fill with non-needle high-bit bytes so we exercise the worst
    // case (no early termination).
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = 0x80 | ((i & 0x7F) as u8);
        while needles.contains(slot) {
            *slot = slot.wrapping_add(1);
        }
    }
    if dense {
        // One needle every 7 bytes — guarantees the chunk loop
        // hits a match in nearly every iteration.
        for i in (3..buf.len()).step_by(7) {
            buf[i] = needles[i % needles.len()];
        }
    }
    // Always place a needle near the end so neither path can
    // bail out at chunk granularity.
    let last = buf.len().saturating_sub(1);
    buf[last] = needles[0];
    buf
}

fn bench_find_any_of(c: &mut Criterion) {
    let arities: &[(&[u8], &str)] = &[
        (b":", "arity_1_memchr"),
        (b":\n#", "arity_3_memchr3"),
        (b"[]{}:,#\n", "arity_8_swar"),
    ];

    for &(needles, label) in arities {
        let mut group = c.benchmark_group(format!("find_any_of/{label}"));
        for size in [4 * 1024, 64 * 1024] {
            for dense in [false, true] {
                let haystack = make_haystack(size, dense, needles);
                let density = if dense { "dense" } else { "sparse" };
                let id = BenchmarkId::new(density, size);
                group.throughput(Throughput::Bytes(size as u64));

                group.bench_with_input(
                    BenchmarkId::new(format!("{density}_simd"), size),
                    &haystack,
                    |b, haystack| {
                        b.iter(|| find_any_of(black_box(haystack), black_box(needles)));
                    },
                );
                group.bench_with_input(
                    BenchmarkId::new(format!("{density}_scalar"), size),
                    &haystack,
                    |b, haystack| {
                        b.iter(|| scalar_find_any_of(black_box(haystack), black_box(needles)));
                    },
                );
                let _ = id; // discard the unused id (kept for parallel structure).
            }
        }
        group.finish();
    }
}

criterion_group!(simd_benches, bench_find_any_of);
criterion_main!(simd_benches);
