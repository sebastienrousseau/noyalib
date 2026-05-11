// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Benchmark the CLI dispatch path — clap argv parse + the
//! noyafmt / noyavalidate command-tree construction. Captures
//! the overhead a process-spawning user pays *before* any YAML
//! work begins.
//!
//! For end-to-end formatter throughput, see the per-document
//! benches in `crates/noyalib/benches/comparison.rs` — those
//! exercise the same `noyalib::cst::format_with_config` engine
//! that `noyafmt` ultimately calls.

#![allow(missing_docs, unused_results)]

use clap::Parser;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use noya_cli::{NoyafmtCli, NoyavalidateCli, noyafmt_command, noyavalidate_command};

fn parse_noyafmt_args(c: &mut Criterion) {
    c.bench_function("noyafmt: parse `--check ci/*.yaml`", |b| {
        b.iter(|| {
            let argv = ["noyafmt", "--check", "a.yaml", "b.yaml", "c.yaml"];
            let _ = NoyafmtCli::try_parse_from(black_box(argv)).unwrap();
        });
    });

    c.bench_function("noyafmt: parse `--write --indent 4 a.yaml`", |b| {
        b.iter(|| {
            let argv = ["noyafmt", "--write", "--indent", "4", "a.yaml"];
            let _ = NoyafmtCli::try_parse_from(black_box(argv)).unwrap();
        });
    });
}

fn parse_noyavalidate_args(c: &mut Criterion) {
    c.bench_function("noyavalidate: parse `--schema s.yaml --fix in.yaml`", |b| {
        b.iter(|| {
            let argv = ["noyavalidate", "--schema", "s.yaml", "--fix", "in.yaml"];
            let _ = NoyavalidateCli::try_parse_from(black_box(argv)).unwrap();
        });
    });
}

fn build_command_tree(c: &mut Criterion) {
    c.bench_function(
        "noyafmt: build clap::Command (xtask / build.rs path)",
        |b| {
            b.iter(|| {
                let _ = black_box(noyafmt_command());
            });
        },
    );

    c.bench_function("noyavalidate: build clap::Command", |b| {
        b.iter(|| {
            let _ = black_box(noyavalidate_command());
        });
    });
}

criterion_group!(
    cli_dispatch,
    parse_noyafmt_args,
    parse_noyavalidate_args,
    build_command_tree,
);
criterion_main!(cli_dispatch);
