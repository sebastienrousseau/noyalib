// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Lossless-u64 parser resolution overhead.
//!
//! Compares `from_str_with_config` with `ParserConfig::lossless_u64_integers`
//! off versus on for YAML documents carrying large unsigned scalars. The
//! mixed-integer control doc proves the default path does not regress when
//! the knob stays off.
//!
//! Run: `cargo bench --bench lossless_u64 --features lossless-u64`

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use noyalib::{ParserConfig, Value, from_str_with_config};
use std::hint::black_box;

const I64_MAX_PLUS_ONE: &str = "id: 9223372036854775808\n";
const U64_MAX: &str = "id: 18446744073709551615\n";

const MIXED_INTEGERS: &str = "\
a: 1
b: 42
c: 1000000000
d: 9223372036854775807
e: 0
f: -1
g: 1234567890
h: 999
";

fn bench_lossless_u64_resolution(c: &mut Criterion) {
    let off = ParserConfig::default();
    let on = ParserConfig::new().lossless_u64_integers(true);

    let mut group = c.benchmark_group("lossless_u64_resolution");

    for (label, yaml) in [
        ("i64_max_plus_1", I64_MAX_PLUS_ONE),
        ("u64_max", U64_MAX),
        ("mixed_i64_control", MIXED_INTEGERS),
    ] {
        group.bench_with_input(BenchmarkId::new("knob_off", label), yaml, |b, input| {
            b.iter(|| {
                let v: Value = from_str_with_config(black_box(input), black_box(&off)).unwrap();
                black_box(v);
            });
        });
        group.bench_with_input(BenchmarkId::new("knob_on", label), yaml, |b, input| {
            b.iter(|| {
                let v: Value = from_str_with_config(black_box(input), black_box(&on)).unwrap();
                black_box(v);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_lossless_u64_resolution);
criterion_main!(benches);
