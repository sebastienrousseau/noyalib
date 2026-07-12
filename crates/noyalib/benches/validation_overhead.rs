// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Microbenchmark that stresses span resolution on a large `Spanned<T>`
//! document with a trailing error, isolating the cost of the span
//! lookup + `Location` computation per field.
//!
//! Run: `cargo bench --bench validation_overhead`

#![allow(missing_docs, unused_results, clippy::unwrap_used)]

use criterion::{Criterion, criterion_group, criterion_main};
use noyalib::{Spanned, from_str};
use serde::Deserialize;
use std::hint::black_box;

#[derive(Deserialize)]
#[allow(dead_code)]
struct ErrorHeavy {
    items: Vec<Spanned<u8>>,
}

fn bench_span_resolution_on_error(c: &mut Criterion) {
    let mut items = Vec::new();
    for i in 0..1000 {
        items.push(format!("  - {}", i % 256));
    }
    // Trailing invalid u8 forces a Spanned<u8> deserialize error that
    // walks the full span resolution path.
    items.push("  - 300".to_string());
    let yaml = format!("items:\n{}", items.join("\n"));

    let _ = c.bench_function("span_resolution_error_path", |b| {
        b.iter(|| {
            let res: Result<ErrorHeavy, _> = from_str(black_box(&yaml));
            let _ = black_box(res);
        });
    });
}

criterion_group!(benches, bench_span_resolution_on_error);
criterion_main!(benches);
