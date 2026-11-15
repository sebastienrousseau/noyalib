// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Scanner cost must grow linearly with the length of a line.
//!
//! A plain scalar looks ahead for the end of its line (a break or a
//! comment). That lookahead used to restart at every scalar, so a single
//! line of many short flow entries cost the line length once per entry:
//! 400 KB took seconds, 1.6 MB close to a minute, and no budget applied
//! until the scan was done. The token queue's compaction had the same
//! shape: it shifted the whole backlog every 256 tokens.
//!
//! Scanner unit tests bound actual lookahead work and queue compactions,
//! including a cache-invalidation witness that must exceed the work bound.
//! These integration tests preserve all parsed values through each entry
//! point without depending on wall-clock scheduling.

/// One line of nested flow sequences holding `n` one-byte scalars, the
/// shape that exposed the per-scalar lookahead.
fn long_flow_line(n: usize) -> String {
    let inner = format!("[{}a]", "a,".repeat(999));
    format!("[{}]\n", vec![inner; n / 1000].join(","))
}

fn assert_long_flow_values(parse: fn(&str) -> noyalib::Value) {
    for n in [10_000, 80_000] {
        let parsed = parse(&long_flow_line(n));
        let rows = parsed.as_sequence().expect("outer sequence");
        assert_eq!(rows.len(), n / 1000);
        for row in rows {
            let values = row.as_sequence().expect("inner sequence");
            assert_eq!(values.len(), 1000);
            assert!(values.iter().all(|value| value.as_str() == Some("a")));
        }
    }
}

#[test]
fn long_flow_line_preserves_all_values() {
    assert_long_flow_values(|s| noyalib::from_str(s).expect("valid input"));
}

#[test]
fn long_flow_line_preserves_all_borrowed_values() {
    assert_long_flow_values(|s| {
        noyalib::borrowed::from_str_borrowed(s)
            .expect("valid input")
            .into_owned()
    });
}

#[cfg(feature = "std")]
#[test]
fn long_flow_line_preserves_all_cst_values() {
    assert_long_flow_values(|s| {
        noyalib::cst::parse_document(s)
            .expect("valid input")
            .as_value()
            .clone()
    });
}

/// `max_events` must be able to refuse a huge flow collection at
/// document start after a handful of events. The scanner used to hold
/// every token of the collection as a possible implicit key first; the
/// queue bound itself is pinned by the scanner's unit tests.
#[test]
fn max_events_refuses_a_ten_megabyte_flow_collection() {
    use noyalib::{BudgetBreach, Error, ParserConfig, Value};
    let cfg = ParserConfig::default()
        .max_document_length(64 * 1024 * 1024)
        .max_events(10);
    let one_line = format!("[{}a]\n", "a,".repeat(5 * 1024 * 1024));
    let many_lines = format!("[{}a]\n", "a,\n".repeat(3_500_000));
    for input in [&one_line, &many_lines] {
        match noyalib::from_str_with_config::<Value>(input, &cfg) {
            Err(Error::Budget(BudgetBreach::MaxEvents { limit: 10, .. })) => {}
            other => panic!("expected the max_events budget, got {other:?}"),
        }
    }
}
