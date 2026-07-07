// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Key interning for repeated-key workloads.
//!
//! YAML mappings frequently repeat the same key text across many
//! records — `metadata`, `name`, `version`, `apiVersion` in
//! Kubernetes manifests, or `time`, `level`, `service` in
//! structured logs. Every fresh parse allocates a brand-new
//! `String` for every key. For a 10 000-record stream that means
//! tens of thousands of duplicate heap allocations.
//!
//! [`KeyInterner`](noyalib::interner::KeyInterner) is the
//! primitive that lets you dedupe those allocations. This example
//! shows the allocation delta on a small synthetic workload:
//! parse 10 000 log records once, then intern the observed keys.
//!
//! Run:
//! ```text
//! cargo run --example interner --release
//! ```

use noyalib::Value;
use noyalib::interner::KeyInterner;
use std::sync::Arc;

fn build_records(count: usize) -> String {
    // Every record has the same three keys — the exact repeated-
    // key shape the interner is designed for.
    let mut s = String::with_capacity(count * 48);
    for i in 0..count {
        s.push_str(&format!(
            "- time: 2026-07-07T00:00:{i:02}Z\n  level: info\n  service: web-{i}\n",
        ));
    }
    s
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = build_records(10_000);
    let root: Value = noyalib::from_str(&yaml)?;
    let records = root.as_sequence().expect("root is a sequence");
    println!("parsed {} records", records.len());

    // Naïve baseline: every String key is a fresh allocation.
    // Sum up how many key-String heap allocations landed just so
    // the comparison is legible.
    let naive_key_allocs: usize = records
        .iter()
        .filter_map(|r| r.as_mapping())
        .map(|m| m.len())
        .sum();
    println!("naïve heap Strings for keys: {naive_key_allocs}");

    // Interned: one Arc<str> per unique key text, shared across
    // every record via `Arc` clones (16 bytes each on 64-bit).
    let mut interner = KeyInterner::new();
    let mut interned_refs: Vec<Arc<str>> = Vec::with_capacity(naive_key_allocs);
    for record in records {
        if let Some(mapping) = record.as_mapping() {
            for key in mapping.keys() {
                interned_refs.push(interner.intern(key));
            }
        }
    }
    println!(
        "interned: {} unique keys, {} Arc handles referencing them",
        interner.len(),
        interned_refs.len(),
    );

    // Sanity: every "time" reference points at the same allocation.
    let a = interner.intern("time");
    let b = interner.intern("time");
    assert!(Arc::ptr_eq(&a, &b), "re-intern must return the same Arc");
    println!("Arc::ptr_eq(intern(\"time\"), intern(\"time\")) = true");

    // Interner reset — useful between distinct streams so unused
    // keys aren't held forever.
    interner.clear();
    assert_eq!(interner.len(), 0);
    println!("interner.clear() → len = {}", interner.len());

    Ok(())
}
