// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Regression tests for the `max_nodes` AST budget.
//!
//! `ParserConfig::max_nodes` bounds the number of `Value` nodes the
//! loader will author for a document stream. It closes a DoS vector the
//! other budgets miss: a payload that *minimises* scalar bytes and stays
//! under `max_events` while *maximising* node count — e.g. a long run of
//! empty collections `[] [] [] …` or `{} {} …`. Each empty collection is
//! one node but only a couple of bytes, so `max_total_scalar_bytes`
//! never trips and `max_events` (1M default) is four times looser than
//! the documented 250k node cap.
//!
//! Like the other AST budgets (see `budget_breach.rs`), `max_nodes` is
//! enforced on the loader path; these tests drive it through
//! `load_all_with_config`, which under `std` engages the span-full
//! loader and shares the counter with the no-span fast path.

#![allow(missing_docs)]

use noyalib::{BudgetBreach, Error, ParserConfig, Value, load_all_with_config};

fn load_all(yaml: &str, cfg: &ParserConfig) -> Result<Vec<Value>, Error> {
    load_all_with_config(yaml, cfg)?.collect()
}

/// A flow document containing `n` empty flow mappings inside a sequence.
/// Node count is `n + 1` (the outer sequence plus `n` empty mappings);
/// scalar bytes are zero, so only `max_nodes` can bound it. Keep `n`
/// under `max_sequence_length` (65536) so the width cap does not trip
/// first; use [`chunked_empty_bomb`] for larger totals.
fn empty_collection_bomb(n: usize) -> String {
    let mut s = String::with_capacity(n * 3 + 2);
    s.push('[');
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        s.push_str("{}");
    }
    s.push(']');
    s
}

/// Like [`empty_collection_bomb`] but groups the empties into inner
/// sequences of `chunk` so no single collection exceeds the sequence
/// width cap. Lets the node bomb grow past `max_sequence_length` while
/// keeping every individual collection small — isolating `max_nodes`.
fn chunked_empty_bomb(total: usize, chunk: usize) -> String {
    let mut s = String::with_capacity(total * 3 + total / chunk * 2 + 2);
    s.push('[');
    let mut written = 0;
    let mut first_group = true;
    while written < total {
        if !first_group {
            s.push(',');
        }
        first_group = false;
        s.push('[');
        let group = chunk.min(total - written);
        for i in 0..group {
            if i > 0 {
                s.push(',');
            }
            s.push_str("{}");
        }
        s.push(']');
        written += group;
    }
    s.push(']');
    s
}

#[test]
fn custom_low_max_nodes_rejects_node_dense_input() {
    // Outer sequence (1) + 10 empty mappings = 11 nodes > cap of 5.
    let yaml = empty_collection_bomb(10);
    let cfg = ParserConfig::new().max_nodes(5);
    let err = load_all(&yaml, &cfg).unwrap_err();
    match err {
        Error::Budget(BudgetBreach::MaxNodes { limit, observed }) => {
            assert_eq!(limit, 5);
            assert!(observed > 5, "observed {observed} must exceed limit 5");
        }
        other => panic!("expected MaxNodes breach, got {other:?}"),
    }
}

#[test]
fn documents_under_the_cap_parse_cleanly() {
    // 1 outer mapping + 3 scalar values = 4 nodes, well under the cap.
    let yaml = "a: 1\nb: 2\nc: 3\n";
    let cfg = ParserConfig::new().max_nodes(100);
    let docs = load_all(yaml, &cfg).expect("must parse under the node cap");
    assert_eq!(docs.len(), 1);
}

#[test]
fn empty_collections_count_as_nodes_under_byte_and_event_caps() {
    // The distinguishing attack: many empty collections. Bytes are
    // trivial and events stay well under `max_events`, yet the node
    // count is large. Only `max_nodes` refuses it.
    let yaml = empty_collection_bomb(1_000);
    let cfg = ParserConfig::new()
        .max_nodes(500)
        // Prove it is the node cap doing the work, not another budget.
        .max_total_scalar_bytes(64 * 1024 * 1024)
        .max_events(1_000_000);
    let err = load_all(&yaml, &cfg).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Budget(BudgetBreach::MaxNodes { limit: 500, .. })
        ),
        "expected MaxNodes(500) breach, got {err:?}"
    );
}

#[test]
fn default_max_nodes_is_enforced_at_250k() {
    // The shipped default (250_000) must be live, not documentation.
    // 300k empties grouped into 1000-wide sequences exceed the node cap
    // while every collection stays under `max_sequence_length` and the
    // stream stays under the default 1M event and 64 MB byte caps.
    let yaml = chunked_empty_bomb(300_000, 1_000);
    let cfg = ParserConfig::default();
    let err = load_all(&yaml, &cfg).unwrap_err();
    match err {
        Error::Budget(BudgetBreach::MaxNodes { limit, observed }) => {
            assert_eq!(limit, 250_000);
            assert!(observed > 250_000);
        }
        other => panic!("expected default MaxNodes breach, got {other:?}"),
    }
}

#[test]
fn raising_max_nodes_admits_larger_documents() {
    // The knob relaxes as documented: what the default rejects, a
    // higher cap accepts (mirrors the large-doc soak bench).
    let yaml = empty_collection_bomb(2_000);
    let strict = ParserConfig::new().max_nodes(1_000);
    assert!(load_all(&yaml, &strict).is_err());
    let relaxed = ParserConfig::new().max_nodes(usize::MAX);
    assert!(
        load_all(&yaml, &relaxed).is_ok(),
        "raising max_nodes must admit the same document"
    );
}

#[test]
fn max_nodes_breach_display_is_actionable() {
    let breach = BudgetBreach::MaxNodes {
        limit: 250_000,
        observed: 250_001,
    };
    let msg = breach.to_string();
    assert!(msg.contains("max_nodes"), "message names the budget: {msg}");
    assert!(
        msg.contains("250000") && msg.contains("250001"),
        "shows numbers: {msg}"
    );
}
