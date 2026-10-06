// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Cross-path parity for the per-event budgets.
//!
//! A typed target with the default configuration is served by the
//! streaming deserializer; a `Value` target is served by the loaders.
//! Until v0.0.53 the streaming path never charged `max_events`,
//! `max_nodes`, `max_total_scalar_bytes`, `max_merge_keys` or the
//! alias ratio, so tightening any of them had no effect on the common
//! case. Every test here drives the same input through both paths and
//! asserts the same `BudgetBreach`, so a budget cannot again exist on
//! one path only.

#![allow(missing_docs)]

use std::collections::HashMap;

use noyalib::{BudgetBreach, Error, ParserConfig, Value, from_str_with_config};

#[derive(serde::Deserialize, Debug)]
struct Records {
    items: Vec<HashMap<String, String>>,
}

#[derive(serde::Deserialize, Debug)]
struct Merged {
    base: HashMap<String, i64>,
    items: Vec<HashMap<String, i64>>,
}

#[derive(serde::Deserialize, Debug)]
struct Aliased {
    base: i64,
    items: Vec<i64>,
}

/// `n` single-key records: about `2n + 6` parser events and `2n + 3`
/// nodes, so a limit of 50 trips well before the end for `n = 200`.
fn records(n: usize) -> String {
    let mut y = String::from("items:\n");
    for i in 0..n {
        y.push_str(&format!("  - k{i}: v{i}\n"));
    }
    y
}

fn breach(err: &Error) -> &BudgetBreach {
    match err {
        Error::Budget(b) => b,
        other => panic!("expected a budget breach, got {other:?}"),
    }
}

/// Both paths must fail, with the same breach, and the breach must be
/// the one the test is about.
fn assert_same_breach<T>(yaml: &str, cfg: &ParserConfig, is_expected: fn(&BudgetBreach) -> bool)
where
    T: for<'de> serde::Deserialize<'de> + std::fmt::Debug + 'static,
{
    let typed = from_str_with_config::<T>(yaml, cfg).expect_err("typed target must fail");
    let value = from_str_with_config::<Value>(yaml, cfg).expect_err("Value target must fail");
    assert!(is_expected(breach(&typed)), "typed path: {typed:?}");
    assert!(is_expected(breach(&value)), "Value path: {value:?}");
    assert_eq!(
        breach(&typed),
        breach(&value),
        "both paths must report the same breach"
    );
}

#[test]
fn max_events_is_charged_on_the_typed_path() {
    let cfg = ParserConfig::default().max_events(50);
    assert_same_breach::<Records>(&records(200), &cfg, |b| {
        matches!(b, BudgetBreach::MaxEvents { limit: 50, .. })
    });
}

#[test]
fn max_nodes_is_charged_on_the_typed_path() {
    let cfg = ParserConfig::default().max_nodes(50);
    assert_same_breach::<Records>(&records(200), &cfg, |b| {
        matches!(b, BudgetBreach::MaxNodes { limit: 50, .. })
    });
}

#[test]
fn max_total_scalar_bytes_is_charged_on_the_typed_path() {
    let cfg = ParserConfig::default().max_total_scalar_bytes(100);
    assert_same_breach::<Records>(&records(200), &cfg, |b| {
        matches!(b, BudgetBreach::MaxTotalScalarBytes { limit: 100, .. })
    });
}

#[test]
fn max_merge_keys_is_charged_on_the_typed_path() {
    let yaml = "base: &b {x: 1}\nitems:\n  - <<: *b\n  - <<: *b\n  - <<: *b\n";
    let cfg = ParserConfig::default().max_merge_keys(2);
    assert_same_breach::<Merged>(yaml, &cfg, |b| {
        matches!(b, BudgetBreach::MaxMergeKeys { limit: 2, .. })
    });
}

#[test]
fn alias_anchor_ratio_is_charged_on_the_typed_path() {
    let yaml = "base: &b 1\nitems: [*b, *b, *b]\n";
    let cfg = ParserConfig::default().alias_anchor_ratio(Some(2.0));
    assert_same_breach::<Aliased>(yaml, &cfg, |b| {
        matches!(b, BudgetBreach::AliasAnchorRatio { anchors: 1, .. })
    });
}

#[test]
fn max_mapping_keys_is_a_budget_breach_on_the_typed_path() {
    let cfg = ParserConfig::default().max_mapping_keys(2);
    assert_same_breach::<HashMap<String, i64>>("a: 1\nb: 2\nc: 3\n", &cfg, |b| {
        matches!(
            b,
            BudgetBreach::MaxMappingKeys {
                limit: 2,
                observed: 3
            }
        )
    });
}

#[test]
fn max_sequence_length_is_a_budget_breach_on_the_typed_path() {
    let cfg = ParserConfig::default().max_sequence_length(2);
    assert_same_breach::<Vec<i64>>("[1, 2, 3]", &cfg, |b| {
        matches!(
            b,
            BudgetBreach::MaxSequenceLength {
                limit: 2,
                observed: 3
            }
        )
    });
}

/// The charges must not over-count either: a document comfortably
/// inside every limit still parses on both paths.
#[test]
fn budgets_inside_the_limit_still_parse_on_both_paths() {
    let cfg = ParserConfig::default()
        .max_events(1000)
        .max_nodes(1000)
        .max_total_scalar_bytes(10_000)
        .max_merge_keys(1)
        .alias_anchor_ratio(Some(10.0));
    let yaml = records(10);
    let typed = from_str_with_config::<Records>(&yaml, &cfg).expect("typed target parses");
    assert_eq!(typed.items.len(), 10, "every record must survive");
    let value = from_str_with_config::<Value>(&yaml, &cfg).expect("Value target parses");
    assert_eq!(value["items"].as_sequence().map(Vec::len), Some(10));
}

/// Merge keys and aliases inside their limits still expand on the
/// typed path, with the counters charged but not tripped.
#[test]
fn merges_and_aliases_inside_the_limit_still_expand() {
    let merged_yaml = "base: &b {x: 1}\nitems:\n  - <<: *b\n  - <<: *b\n  - <<: *b\n";
    let cfg = ParserConfig::default().max_merge_keys(3);
    let merged = from_str_with_config::<Merged>(merged_yaml, &cfg).expect("three merges fit");
    assert_eq!(merged.base.get("x"), Some(&1));
    assert_eq!(merged.items.len(), 3, "every merged record must survive");
    assert!(merged.items.iter().all(|m| m.get("x") == Some(&1)));

    let aliased_yaml = "base: &b 1\nitems: [*b, *b, *b]\n";
    let cfg = ParserConfig::default().alias_anchor_ratio(Some(3.0));
    let aliased = from_str_with_config::<Aliased>(aliased_yaml, &cfg).expect("three aliases fit");
    assert_eq!(aliased.base, 1);
    assert_eq!(aliased.items, vec![1, 1, 1]);
}
