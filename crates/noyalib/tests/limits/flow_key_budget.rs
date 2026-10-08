// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `max_events` refuses a long flow-mapping key before it is buffered.
//!
//! An implicit key inside a flow mapping has no length limit (YAML 1.2.2
//! §7.4, suite case NJ66), so the scanner held back every token of
//! `{[a, a, ...]: v}` until the `:`. A budget of ten events was only
//! checked once the whole collection had been tokenised.

use noyalib::{ParserConfig, Value};

fn long_key(entries: usize) -> String {
    format!("{{[{}a]: v}}\n", "a,".repeat(entries))
}

#[test]
fn every_loader_refuses_the_backlog_under_a_small_event_budget() {
    let input = long_key(200_000);
    let cfg = ParserConfig::new().max_events(10);
    let started = std::time::Instant::now();
    let value: Result<Value, _> = noyalib::from_str_with_config(&input, &cfg);
    match value {
        Err(noyalib::Error::Budget(noyalib::BudgetBreach::MaxEvents { limit: 10, .. })) => {}
        other => panic!("expected the max_events budget, got {other:?}"),
    }
    let docs: Result<Vec<_>, _> = noyalib::load_all_with_config(&input, &cfg).map(|d| d.collect());
    assert!(docs.is_err());
    let borrowed = noyalib::borrowed::from_str_borrowed_with_config(&input, &cfg);
    assert!(borrowed.is_err());
    // Cut off early: well under the time the 400 KB scan takes in debug.
    assert!(started.elapsed().as_secs() < 5, "{:?}", started.elapsed());
}

#[test]
fn a_long_flow_mapping_key_within_budget_still_parses() {
    let input = long_key(2_000);
    let value: Value = noyalib::from_str(&input).expect("default budgets allow it");
    assert!(value.as_mapping().is_some_and(|m| m.len() == 1));
}
