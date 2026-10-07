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
//! Timing tests are noisy, so these compare two sizes eight times apart
//! and keep the fastest of several runs: linear work grows about eight
//! times, quadratic work about sixty-four. The bound sits well between.

use std::time::{Duration, Instant};

/// One line of nested flow sequences holding `n` one-byte scalars, the
/// shape that exposed the per-scalar lookahead.
fn long_flow_line(n: usize) -> String {
    let inner = format!("[{}a]", "a,".repeat(999));
    format!("[{}]\n", vec![inner; n / 1000].join(","))
}

fn fastest(runs: usize, mut f: impl FnMut()) -> Duration {
    (0..runs)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed()
        })
        .min()
        .unwrap_or_default()
}

fn assert_linear(entry: &str, parse: fn(&str)) {
    let small = long_flow_line(10_000);
    let large = long_flow_line(80_000);
    let t_small = fastest(5, || parse(&small));
    let t_large = fastest(3, || parse(&large));
    // Guard against a zero-duration small run on a very fast machine.
    let t_small = t_small.max(Duration::from_micros(200));
    let ratio = t_large.as_secs_f64() / t_small.as_secs_f64();
    assert!(
        ratio < 24.0,
        "{entry}: 8x longer line took {ratio:.1}x as long \
         ({t_small:?} -> {t_large:?}); expected linear growth",
    );
}

#[test]
fn long_flow_line_parses_in_linear_time_value() {
    assert_linear("from_str::<Value>", |s| {
        let _ = noyalib::from_str::<noyalib::Value>(s).expect("valid input");
    });
}

#[test]
fn long_flow_line_parses_in_linear_time_borrowed() {
    assert_linear("borrowed::from_str_borrowed", |s| {
        let _ = noyalib::borrowed::from_str_borrowed(s).expect("valid input");
    });
}

#[cfg(feature = "std")]
#[test]
fn long_flow_line_parses_in_linear_time_cst() {
    assert_linear("cst::parse_document", |s| {
        let _ = noyalib::cst::parse_document(s).expect("valid input");
    });
}

/// Deep flow nesting keeps one pending simple key per level. The
/// staleness check must not walk every level for every token, or a
/// 40,000-deep fragment costs seconds before the depth limit refuses it.
#[cfg(feature = "std")]
#[test]
fn deep_flow_nesting_is_refused_in_linear_time() {
    fn refuse(depth: usize) {
        let deep = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let mut doc = noyalib::cst::parse_document("m:\n  a: 1\n").expect("valid");
        assert!(doc.set("m.a", &deep).is_err(), "depth {depth} accepted");
    }
    let t_small = fastest(5, || refuse(5_000)).max(Duration::from_micros(200));
    let t_large = fastest(3, || refuse(40_000));
    let ratio = t_large.as_secs_f64() / t_small.as_secs_f64();
    assert!(
        ratio < 24.0,
        "8x deeper nesting took {ratio:.1}x as long ({t_small:?} -> {t_large:?})",
    );
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
