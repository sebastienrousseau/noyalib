// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `simdjson`-style structural-discovery throughput.
//!
//! Compares three paths for "find every structural-byte position
//! in the haystack":
//!
//! 1. **Scalar baseline** — the byte-by-byte `iter().position`
//!    loop a hand-written scanner would use.
//! 2. **memchr / find_any_of loop** — the existing scanner hot
//!    path (memchr arity 1/2/3, SWAR for 4+).
//! 3. **StructuralIter (32-byte bitmask + trailing_zeros)** — the
//!    new dense-bitmask discovery loop. Loads 32 bytes, produces
//!    a `u32` mask via `structural_bitmask_32`, walks set bits
//!    via `trailing_zeros()`.
//!
//! The bitmask path's win shows up on documents with many
//! delimiters per chunk (typical YAML), where the scanner state
//! machine would otherwise restart the SIMD scan after each match.
//!
//! Run: `cargo bench --bench structural_bitmask`

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use noyalib::simd::{SimdScanner, StructuralIter, find_any_of};

const STRUCTURAL_NEEDLES: &[u8] = b":-[]{}\n#";

fn scalar_walk(haystack: &[u8], needles: &[u8]) -> usize {
    let mut count = 0;
    for &b in haystack {
        if needles.contains(&b) {
            count += 1;
        }
    }
    count
}

fn memchr_walk(haystack: &[u8], needles: &[u8]) -> usize {
    let mut count = 0;
    let mut start = 0;
    while start < haystack.len() {
        match find_any_of(&haystack[start..], needles) {
            Some(pos) => {
                count += 1;
                start += pos + 1;
            }
            None => break,
        }
    }
    count
}

fn structural_iter_walk(haystack: &[u8], scanner: &SimdScanner) -> usize {
    StructuralIter::new(scanner, haystack).count()
}

fn make_yaml_haystack(target_bytes: usize) -> Vec<u8> {
    // Each "record" mirrors a typical config-file line shape: it
    // contains exactly two structural delimiters (`:` and `\n`) per
    // line. The bitmask path's win comes from the structural
    // density that real YAML exhibits.
    let template = b"  - key_name: value-data-payload-here\n";
    let mut out = Vec::with_capacity(target_bytes + template.len());
    while out.len() < target_bytes {
        out.extend_from_slice(template);
    }
    out
}

fn bench_structural(c: &mut Criterion) {
    let mut group = c.benchmark_group("structural_discovery");
    let scanner = SimdScanner::new(STRUCTURAL_NEEDLES);

    for &(label, size) in &[
        ("4KiB", 4 * 1024usize),
        ("64KiB", 64 * 1024usize),
        ("1MiB", 1024 * 1024usize),
    ] {
        let haystack = make_yaml_haystack(size);
        group.throughput(Throughput::Bytes(haystack.len() as u64));

        group.bench_with_input(BenchmarkId::new("scalar", label), &haystack, |b, h| {
            b.iter(|| scalar_walk(black_box(h), STRUCTURAL_NEEDLES));
        });

        group.bench_with_input(
            BenchmarkId::new("memchr_find_any_of", label),
            &haystack,
            |b, h| {
                b.iter(|| memchr_walk(black_box(h), STRUCTURAL_NEEDLES));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("structural_iter", label),
            &haystack,
            |b, h| {
                b.iter(|| structural_iter_walk(black_box(h), &scanner));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_structural);
criterion_main!(benches);
