// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Alias expansion is charged the same way on every path.
//!
//! The owned loaders, the typed streaming path and the borrowed builder
//! each expand aliases. They share one meter and one estimate of what
//! an expansion costs; these tests pin that a document refused by one
//! of them is refused by all of them, and that the estimate bounds the
//! memory an expansion really allocates.

use noyalib::borrowed::from_str_borrowed_with_config;
use noyalib::{BudgetBreach, DuplicateKeyPolicy, Error, ParserConfig, Value, from_str_with_config};

/// A billion-laughs document: `levels` anchors, each a sequence of
/// nine aliases to the one before.
fn laughs(levels: usize) -> String {
    let mut y = String::from("l0: &l0 [lol, lol, lol, lol, lol, lol, lol, lol, lol]\n");
    for i in 1..=levels {
        let refs = vec![format!("*l{}", i - 1); 9].join(", ");
        y.push_str(&format!("l{i}: &l{i} [{refs}]\n"));
    }
    y
}

/// One anchored sequence of `n` empty strings, expanded once.
fn one_expansion(n: usize) -> String {
    format!("a: &a [{}]\nb: *a\n", vec!["''"; n].join(", "))
}

#[test]
fn borrowed_refuses_the_billion_laughs_the_owned_loader_refuses() {
    let yaml = laughs(9);
    assert!(yaml.len() < 600, "the bomb is small: {} bytes", yaml.len());
    let cfg = ParserConfig::default();
    let owned = from_str_with_config::<Value>(&yaml, &cfg).expect_err("owned refuses");
    let borrowed = from_str_borrowed_with_config(&yaml, &cfg).expect_err("borrowed refuses");
    assert!(matches!(owned, Error::RepetitionLimitExceeded), "{owned:?}");
    assert!(
        matches!(borrowed, Error::RepetitionLimitExceeded),
        "{borrowed:?}"
    );
}

#[test]
fn borrowed_charges_alias_bytes_against_the_document_length() {
    let yaml = laughs(3);
    let cfg = ParserConfig::default().max_document_length(4096);
    assert!(from_str_with_config::<Value>(&yaml, &cfg).is_err());
    let err = from_str_borrowed_with_config(&yaml, &cfg).expect_err("borrowed refuses");
    assert!(matches!(err, Error::RepetitionLimitExceeded), "{err:?}");
}

#[test]
fn borrowed_enforces_every_per_event_and_per_collection_budget() {
    let seq = |n: usize| {
        format!(
            "[{}]\n",
            (0..n).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
        )
    };
    let map = |n: usize| (0..n).map(|i| format!("k{i}: {i}\n")).collect::<String>();
    let d = ParserConfig::default;
    let cases: Vec<(&str, String, ParserConfig)> = vec![
        ("max_events", seq(30), d().max_events(20)),
        ("max_nodes", seq(30), d().max_nodes(10)),
        (
            "max_total_scalar_bytes",
            format!("k: {}\n", "x".repeat(40)),
            d().max_total_scalar_bytes(16),
        ),
        ("max_mapping_keys", map(6), d().max_mapping_keys(4)),
        ("max_sequence_length", seq(6), d().max_sequence_length(4)),
        (
            "max_documents",
            "---\na: 1\n".repeat(5),
            d().max_documents(2),
        ),
        (
            "alias_anchor_ratio",
            "a: &a 1\nb: [*a,*a,*a,*a,*a]\n".into(),
            d().alias_anchor_ratio(Some(2.0)),
        ),
        (
            "max_merge_keys",
            "b: &b {x: 1}\nm: {<<: *b}\nn: {<<: *b}\n".into(),
            d().max_merge_keys(1),
        ),
        (
            "duplicate keys",
            "a: 1\na: 2\n".into(),
            d().duplicate_key_policy(DuplicateKeyPolicy::Error),
        ),
    ];
    for (name, yaml, cfg) in cases {
        assert!(
            from_str_with_config::<Value>(&yaml, &cfg).is_err(),
            "{name}: the owned loader must refuse"
        );
        assert!(
            from_str_borrowed_with_config(&yaml, &cfg).is_err(),
            "{name}: the borrowed builder must refuse what the owned loader refuses"
        );
    }
}

#[test]
fn borrowed_still_parses_inside_the_limits() {
    let yaml = "a: &a [1, 2]\nb: *a\nc: {x: y}\n";
    let v = from_str_borrowed_with_config(yaml, &ParserConfig::strict()).expect("parses");
    assert_eq!(v.into_owned(), noyalib::from_str::<Value>(yaml).unwrap());
}

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct Typed {
    a: Vec<String>,
    b: Vec<String>,
}

/// The typed streaming path used to weigh an expanded node at 8 bytes
/// where the loader weighed it at 32, so an expansion the loader
/// refused was accepted for a typed target.
#[test]
fn typed_and_value_targets_charge_an_expansion_the_same() {
    let yaml = one_expansion(100);
    let cfg = ParserConfig::default().max_document_length(2000);
    assert!(yaml.len() < 2000);
    let value = from_str_with_config::<Value>(&yaml, &cfg).expect_err("Value target refuses");
    let typed = from_str_with_config::<Typed>(&yaml, &cfg).expect_err("typed target refuses");
    assert!(matches!(value, Error::RepetitionLimitExceeded), "{value:?}");
    assert!(matches!(typed, Error::RepetitionLimitExceeded), "{typed:?}");
}

/// The estimate is at least the memory a `Value` node occupies, so the
/// alias-byte budget bounds the allocation rather than a fraction of it.
#[test]
fn an_expansion_is_charged_at_least_a_value_per_node() {
    let nodes = 1000;
    let yaml = one_expansion(nodes);
    let real = (nodes + 1) * size_of::<Value>();
    let cfg = ParserConfig::default().max_document_length(real - 1);
    for (path, err) in [
        ("Value", from_str_with_config::<Value>(&yaml, &cfg).err()),
        ("typed", from_str_with_config::<Typed>(&yaml, &cfg).err()),
        ("borrowed", from_str_borrowed_with_config(&yaml, &cfg).err()),
    ] {
        assert!(
            matches!(err, Some(Error::RepetitionLimitExceeded)),
            "{path}: an expansion of {nodes} nodes ({real} bytes of Value) must not fit in {} bytes: {err:?}",
            real - 1
        );
    }
    // And the same expansion fits once the budget covers it.
    let roomy = ParserConfig::default().max_document_length(4 * real);
    assert!(from_str_with_config::<Value>(&yaml, &roomy).is_ok());
    assert!(from_str_with_config::<Typed>(&yaml, &roomy).is_ok());
    assert!(from_str_borrowed_with_config(&yaml, &roomy).is_ok());
}

/// The 32 MiB ceiling on expanded bytes holds on the typed path too,
/// whatever the document length allows.
#[test]
fn the_absolute_alias_ceiling_holds_on_the_typed_path() {
    let yaml = laughs(6);
    let cfg = ParserConfig::default()
        .max_alias_expansions(usize::MAX)
        .alias_anchor_ratio(None);
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct Doc {
        l6: serde::de::IgnoredAny,
    }
    let typed = from_str_with_config::<Doc>(&yaml, &cfg).expect_err("typed refuses");
    let value = from_str_with_config::<Value>(&yaml, &cfg).expect_err("Value refuses");
    assert!(matches!(typed, Error::RepetitionLimitExceeded), "{typed:?}");
    assert!(matches!(value, Error::RepetitionLimitExceeded), "{value:?}");
}

/// The breach a borrowed parse reports for a per-event budget is the
/// same typed breach the owned loader reports.
#[test]
fn borrowed_reports_the_same_breach_as_the_owned_loader() {
    let cfg = ParserConfig::default().max_nodes(10);
    let yaml = "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]\n";
    let owned = from_str_with_config::<Value>(yaml, &cfg).unwrap_err();
    let borrowed = from_str_borrowed_with_config(yaml, &cfg).unwrap_err();
    assert!(matches!(
        owned,
        Error::Budget(BudgetBreach::MaxNodes { limit: 10, .. })
    ));
    assert_eq!(owned.to_string(), borrowed.to_string());
}
