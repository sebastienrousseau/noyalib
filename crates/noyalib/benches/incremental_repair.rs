// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase A incremental-repair benchmarks.
//!
//! Compares `Document::set` (which routes through the localised
//! `replace_span` repair) against a synthetic baseline that
//! simulates the pre-Phase-A behaviour by full-re-parsing the
//! post-edit source string.
//!
//! What the numbers mean:
//!   * `phase_a_set` — current behaviour. Validation pass +
//!     localised green-tree repair.
//!   * `baseline_full_reparse` — the ceiling the old behaviour
//!     would have hit. Pure `parse_document(new_source)` at
//!     each iteration. (The Document is reconstructed from the
//!     new source — same bytes-out as Phase A.)
//!
//! Run: `cargo bench --bench incremental_repair`

#![allow(missing_docs, unused_results)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

use noyalib::cst::parse_document;

/// Build a synthetic block-mapping document with `n_entries`
/// keys. Each value is a small plain scalar; the document is
/// roughly `~32 * n_entries` bytes.
fn synth_doc(n_entries: usize) -> String {
    let mut out = String::with_capacity(n_entries * 32);
    for i in 0..n_entries {
        out.push_str(&format!("key_{i:05}: value_{i:05}\n"));
    }
    out
}

fn bench_value_bump_at(c: &mut Criterion, target: &str, n_entries_list: &[usize]) {
    let mut group = c.benchmark_group(format!("value_bump_at_{target}"));
    for &n in n_entries_list {
        let src = synth_doc(n);
        let bytes = src.len() as u64;
        group.throughput(Throughput::Bytes(bytes));

        // Pick a single representative key per group.
        let key_idx = match target {
            "first" => 0,
            "middle" => n / 2,
            "last" => n - 1,
            _ => 0,
        };
        let key = format!("key_{key_idx:05}");
        let new_val = "bumped_value";

        // Phase A: cold setup, hot edit.
        group.bench_with_input(
            BenchmarkId::new("phase_a_set", n),
            &(src.clone(), key.clone(), new_val),
            |b, (src, key, new_val)| {
                b.iter_with_setup(
                    || parse_document(src).unwrap(),
                    |mut doc| {
                        doc.set(black_box(key), black_box(new_val)).unwrap();
                        black_box(doc)
                    },
                );
            },
        );

        // Synthetic baseline: full re-parse of the post-edit
        // source. This is what `replace_span` did before Phase A
        // (well, plus an extra parse for the green tree on top).
        // Even with this conservative baseline (one parse, not
        // two), the comparison shows the parse-vs-walk gap.
        group.bench_with_input(
            BenchmarkId::new("baseline_full_reparse", n),
            &(src.clone(), key.clone(), new_val),
            |b, (src, key, new_val)| {
                b.iter_with_setup(
                    || {
                        // Pre-compute the post-edit source so the
                        // measured iteration is just the full
                        // parse, mirroring the dominant cost.
                        let doc = parse_document(src).unwrap();
                        let (s, e) = doc.span_at(key).unwrap();
                        let mut new_src = String::with_capacity(src.len() + 16);
                        new_src.push_str(&src[..s]);
                        new_src.push_str(new_val);
                        new_src.push_str(&src[e..]);
                        new_src
                    },
                    |new_src| black_box(parse_document(black_box(&new_src)).unwrap()),
                );
            },
        );
    }
    group.finish();
}

fn bench_batch_edits(c: &mut Criterion, n_entries: usize, n_edits: usize) {
    let mut group = c.benchmark_group(format!("batch_{n_edits}_edits_in_{n_entries}_entry_doc"));
    let src = synth_doc(n_entries);
    group.throughput(Throughput::Elements(n_edits as u64));

    // Phase A.2 lazy: replace_span never triggers parse_one until
    // the next read. A batch of N edits without an intervening
    // read pays parse_one zero times.
    group.bench_function("phase_a_lazy_batch", |b| {
        b.iter_with_setup(
            || parse_document(&src).unwrap(),
            |mut doc| {
                for i in 0..n_edits {
                    let key = format!("key_{:05}", i % n_entries);
                    let val = format!("v{i}");
                    doc.set(black_box(&key), black_box(&val)).unwrap();
                }
                black_box(doc)
            },
        );
    });

    // Baseline: full re-parse per edit. Each iteration mutates a
    // String and re-parses the whole document, mirroring what
    // pre-Phase-A would have done.
    group.bench_function("baseline_full_reparse_each", |b| {
        b.iter_with_setup(
            || src.clone(),
            |mut s| {
                for i in 0..n_edits {
                    let doc = parse_document(&s).unwrap();
                    let key = format!("key_{:05}", i % n_entries);
                    let (a, e) = doc.span_at(&key).unwrap();
                    let new_val = format!("v{i}");
                    let mut next = String::with_capacity(s.len() + 16);
                    next.push_str(&s[..a]);
                    next.push_str(&new_val);
                    next.push_str(&s[e..]);
                    s = next;
                }
                black_box(s)
            },
        );
    });
    group.finish();
}

fn bench_phase_a(c: &mut Criterion) {
    let sizes = [50usize, 500, 5_000];
    bench_value_bump_at(c, "first", &sizes);
    bench_value_bump_at(c, "middle", &sizes);
    bench_value_bump_at(c, "last", &sizes);
    // Batch scenario — the workflow lazy is designed for.
    bench_batch_edits(c, 500, 10);
    bench_batch_edits(c, 500, 50);
}

/// The re-parse-guarded mutators (`rename_key`, `remove`,
/// `set_inline_comment`, `swap_items`, `move_item`) each snapshot,
/// splice, re-parse, and check a typed oracle — a heavier profile than
/// the localised `set` fast path above. `iter_batched` re-parses a
/// fresh document per iteration so only the mutator itself is timed.
fn bench_guarded_mutators(c: &mut Criterion) {
    use criterion::BatchSize;

    let map_src = synth_doc(500);
    let seq_src: String = (0..100).map(|i| format!("- item_{i:03}\n")).collect();

    let mut mg = c.benchmark_group("guarded_mutators_map_500");
    mg.bench_function("rename_key", |b| {
        b.iter_batched(
            || parse_document(&map_src).unwrap(),
            |mut doc| {
                doc.rename_key(black_box("key_00250"), black_box("renamed_key"))
                    .unwrap();
                doc
            },
            BatchSize::SmallInput,
        )
    });
    mg.bench_function("remove", |b| {
        b.iter_batched(
            || parse_document(&map_src).unwrap(),
            |mut doc| {
                doc.remove(black_box("key_00250")).unwrap();
                doc
            },
            BatchSize::SmallInput,
        )
    });
    mg.bench_function("set_inline_comment", |b| {
        b.iter_batched(
            || parse_document(&map_src).unwrap(),
            |mut doc| {
                doc.set_inline_comment(black_box("key_00250"), black_box("note"))
                    .unwrap();
                doc
            },
            BatchSize::SmallInput,
        )
    });
    mg.finish();

    let mut sg = c.benchmark_group("guarded_mutators_seq_100");
    sg.bench_function("swap_items_ends", |b| {
        b.iter_batched(
            || parse_document(&seq_src).unwrap(),
            |mut doc| {
                doc.swap_items(black_box(""), black_box(0), black_box(99))
                    .unwrap();
                doc
            },
            BatchSize::SmallInput,
        )
    });
    sg.bench_function("move_item_span_50", |b| {
        b.iter_batched(
            || parse_document(&seq_src).unwrap(),
            |mut doc| {
                doc.move_item(black_box(""), black_box(0), black_box(50))
                    .unwrap();
                doc
            },
            BatchSize::SmallInput,
        )
    });
    sg.finish();
}

/// The auto-formatting insertion mutators (`insert_entry_value`,
/// `push_back_value`, `insert_after_value`) run the full `Emit` pipeline:
/// emit the value's YAML spelling for the site, splice, re-parse, and
/// check the typed oracle. This captures the cost the verbatim
/// `insert_entry` / `push_back` fast paths avoid.
fn bench_emit_insertions(c: &mut Criterion) {
    use criterion::BatchSize;
    use noyalib::Value;

    let map_src = synth_doc(500);
    let seq_src: String = (0..100).map(|i| format!("- item_{i:03}\n")).collect();
    let scalar = Value::from("new_value");

    let mut g = c.benchmark_group("emit_insertions");
    g.bench_function("insert_entry_value_scalar", |b| {
        b.iter_batched(
            || parse_document(&map_src).unwrap(),
            |mut doc| {
                doc.insert_entry_value(black_box(""), black_box("added"), black_box(&scalar))
                    .unwrap();
                doc
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("push_back_value_scalar", |b| {
        b.iter_batched(
            || parse_document(&seq_src).unwrap(),
            |mut doc| {
                doc.push_back_value(black_box(""), black_box(&scalar))
                    .unwrap();
                doc
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("insert_after_value_scalar", |b| {
        b.iter_batched(
            || parse_document(&seq_src).unwrap(),
            |mut doc| {
                doc.insert_after_value(black_box("[0]"), black_box(&scalar))
                    .unwrap();
                doc
            },
            BatchSize::SmallInput,
        )
    });
    g.finish();
}

criterion_group!(name = benches; config = Criterion::default(); targets = bench_phase_a, bench_guarded_mutators, bench_emit_insertions);
criterion_main!(benches);
