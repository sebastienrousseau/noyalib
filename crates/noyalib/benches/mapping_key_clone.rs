// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Mapping-key hot-path allocation cost.
//!
//! The v0.0.14 collision-guard fix retains the *typed* key value
//! before it is coerced to a string, so the value-arm can tell a
//! distinct-typed collision (`1` vs `"1"`) apart from a genuine
//! duplicate. That means one `Value::clone()` per non-merge
//! mapping key. Merge keys (`<<`) are exempted (the buffered path
//! never consults the typed key), so `<<`-heavy documents pay the
//! same cost as before.
//!
//! This bench guards against two regressions:
//!
//! 1. **Ordinary mapping-key throughput** stays close to the
//!    pre-fix baseline — the added clone is a small-integer or
//!    boolean, so the delta should be sub-percent per key.
//! 2. **Merge-heavy throughput** should not degrade at all — the
//!    merge-key clone is skipped, so this shape shows the win of
//!    the `is_buffered_merge_key` gate over an unconditional
//!    clone.
//!
//! Run: `cargo bench --bench mapping_key_clone`

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use noyalib::{ParserConfig, Value, from_str, from_str_with_config};
use std::hint::black_box;

/// Config with expanded alias/merge budgets so the merge-heavy
/// bench isn't limited by `max_alias_expansions` at large sizes.
/// The hot path being measured is the mapping-key insert/clone,
/// not the DoS budgets — those are exercised by the parity tests.
fn permissive_config() -> ParserConfig {
    let mut cfg = ParserConfig::default();
    cfg.max_alias_expansions = 100_000;
    cfg.max_merge_keys = 100_000;
    cfg
}

/// Build a mapping of `count` distinct integer keys with a
/// two-character string value each. Exercises the non-merge
/// mapping-key path — one `Value::clone()` per key.
fn integer_keyed_mapping(count: usize) -> String {
    let mut s = String::with_capacity(count * 12);
    for i in 0..count {
        s.push_str(&format!("{i}: v{i}\n"));
    }
    s
}

/// Build a mapping of `count` distinct string keys — same non-
/// merge shape but with the resolver taking the string branch.
fn string_keyed_mapping(count: usize) -> String {
    let mut s = String::with_capacity(count * 16);
    for i in 0..count {
        s.push_str(&format!("key{i}: value{i}\n"));
    }
    s
}

/// A `<<`-heavy document: one anchor holding a small map, then
/// `count` mappings that each merge from it. The clone gate
/// (`is_buffered_merge_key`) should skip the typed-key clone on
/// every `<<`.
fn merge_heavy_mapping(count: usize) -> String {
    let mut s = String::from("base: &b\n  x: 1\n  y: 2\n");
    s.push_str("docs:\n");
    for i in 0..count {
        s.push_str(&format!("  m{i}:\n    <<: *b\n    z: {i}\n"));
    }
    s
}

fn bench_mapping_key_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("mapping_key_clone");

    for &(label, count) in &[
        ("small", 32usize),
        ("medium", 1024usize),
        ("large", 8192usize),
    ] {
        let int_yaml = integer_keyed_mapping(count);
        let str_yaml = string_keyed_mapping(count);
        let merge_yaml = merge_heavy_mapping(count);

        group.throughput(Throughput::Bytes(int_yaml.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("integer_keys", label),
            &int_yaml,
            |b, y| {
                b.iter(|| {
                    let v: Value = from_str(black_box(y)).unwrap();
                    black_box(v);
                });
            },
        );

        group.throughput(Throughput::Bytes(str_yaml.len() as u64));
        group.bench_with_input(BenchmarkId::new("string_keys", label), &str_yaml, |b, y| {
            b.iter(|| {
                let v: Value = from_str(black_box(y)).unwrap();
                black_box(v);
            });
        });

        // The merge-heavy shape can produce more alias
        // expansions than the default `max_alias_expansions`
        // permits — lift that budget so the bench measures the
        // key-insert path, not the DoS guard.
        let merge_cfg = permissive_config();
        group.throughput(Throughput::Bytes(merge_yaml.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("merge_heavy", label),
            &merge_yaml,
            |b, y| {
                b.iter(|| {
                    let v: Value = from_str_with_config(black_box(y), &merge_cfg).unwrap();
                    black_box(v);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_mapping_key_clone);
criterion_main!(benches);
