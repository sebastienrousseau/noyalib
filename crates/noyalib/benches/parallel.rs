// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Parallel multi-document parsing vs sequential `load_all_as`.
//!
//! Sweeps document count × per-document size to expose the
//! break-even point where Rayon's task-hand-off cost is worth
//! paying. Guides the "when should I use `parallel::parse`?"
//! decision.
//!
//! Requires `--features parallel`.
//!
//! Run: `cargo bench --bench parallel --features parallel`

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use serde::Deserialize;
use std::hint::black_box;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Record {
    id: u32,
    kind: String,
    payload: String,
}

fn build_stream(doc_count: usize, body_repeats: usize) -> String {
    let mut s = String::with_capacity(doc_count * (48 + body_repeats * 24));
    for i in 0..doc_count {
        s.push_str(&format!("---\nid: {i}\nkind: audit\npayload: "));
        for j in 0..body_repeats {
            s.push_str(&format!("field-{j}-{i} "));
        }
        s.push('\n');
    }
    s
}

fn bench_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_vs_sequential");

    // Doc counts stay under the default `max_documents = 1000`
    // budget so both paths run without a config override. The
    // per-document *size* is the axis we sweep — that's what
    // shifts the break-even point between sequential and
    // parallel.
    // (label, doc_count, body_repeats)
    let shapes = [
        ("tiny_docs", 900usize, 0usize), // sequential likely wins
        ("medium_docs", 900, 24),        // parallel likely wins
        ("large_docs", 300, 96),         // parallel wins by more
    ];

    for &(label, doc_count, body_repeats) in &shapes {
        let stream = build_stream(doc_count, body_repeats);
        group.throughput(Throughput::Bytes(stream.len() as u64));

        group.bench_with_input(BenchmarkId::new("sequential", label), &stream, |b, y| {
            b.iter(|| {
                let v: Vec<Record> = noyalib::load_all_as(black_box(y)).unwrap();
                black_box(v);
            });
        });

        group.bench_with_input(BenchmarkId::new("parallel", label), &stream, |b, y| {
            b.iter(|| {
                // `parallel::parse` calls `from_str::<T>` per
                // chunk, so per-chunk parsing runs with default
                // config. The `max_documents` limit applies per
                // chunk (each chunk has 1 doc); no lift needed.
                let v: Vec<Record> = noyalib::parallel::parse(black_box(y)).unwrap();
                black_box(v);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_parallel);
criterion_main!(benches);
