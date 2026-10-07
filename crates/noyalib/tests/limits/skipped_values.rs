// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Content a typed target skips is charged like content it reads.
//!
//! `IgnoredAny` and the values of unknown struct fields are consumed
//! without being built. That path used to skip the depth, mapping-key,
//! sequence-length and merge-key limits and the duplicate-key `Error`
//! policy, so a document the `Value` target refused was accepted the
//! moment the hostile part sat under a field the target ignores.

use noyalib::{DuplicateKeyPolicy, ParserConfig, Value, from_str_with_config};
use serde::de::IgnoredAny;

/// A target that reads `keep` and ignores everything else.
#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct KeepOnly {
    keep: i32,
}

fn cases() -> Vec<(&'static str, String, ParserConfig)> {
    let d = ParserConfig::default;
    let seq = |n: usize| {
        format!(
            "[{}]",
            (0..n).map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
        )
    };
    let map = |n: usize| {
        format!(
            "{{{}}}",
            (0..n)
                .map(|i| format!("k{i}: {i}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    vec![
        (
            "max_depth",
            format!("{}1{}", "[".repeat(10), "]".repeat(10)),
            d().max_depth(8),
        ),
        ("max_sequence_length", seq(6), d().max_sequence_length(4)),
        ("max_mapping_keys", map(6), d().max_mapping_keys(4)),
        (
            "max_merge_keys",
            "[{<<: {x: 1}}, {<<: {x: 1}}, {<<: {x: 1}}]".into(),
            d().max_merge_keys(2),
        ),
        (
            "duplicate keys",
            "{a: 1, a: 2}".into(),
            d().duplicate_key_policy(DuplicateKeyPolicy::Error),
        ),
    ]
}

#[test]
fn ignored_any_charges_what_it_skips() {
    let mut gaps = Vec::new();
    for (name, hostile, cfg) in cases() {
        let yaml = format!("keep: 1\nskipped: {hostile}\n");
        assert!(
            from_str_with_config::<Value>(&yaml, &cfg).is_err(),
            "{name}: Value refuses"
        );
        if from_str_with_config::<IgnoredAny>(&yaml, &cfg).is_ok() {
            gaps.push(format!("{name}: IgnoredAny accepted"));
        }
        if from_str_with_config::<KeepOnly>(&yaml, &cfg).is_ok() {
            gaps.push(format!("{name}: an unknown field's value was accepted"));
        }
    }
    assert!(gaps.is_empty(), "{gaps:#?}");
}

#[test]
fn skipped_content_inside_the_limits_still_parses() {
    let cfg = ParserConfig::default()
        .max_depth(8)
        .max_sequence_length(4)
        .max_mapping_keys(4)
        .duplicate_key_policy(DuplicateKeyPolicy::Error);
    let yaml = "keep: 1\nskipped: [[[1, 2]], {a: 1, b: [x, y]}]\nother: {a: {a: {a: 1}}}\n";
    let k: KeepOnly = from_str_with_config(yaml, &cfg).expect("parses");
    assert_eq!(k.keep, 1);
    let _: IgnoredAny = from_str_with_config(yaml, &cfg).expect("parses");
}

/// The alias-depth charge reaches skipped content too.
#[test]
fn ignored_any_refuses_an_alias_chain_deeper_than_max_depth() {
    let mut y = format!("a0: &a0 {}1{}\n", "[".repeat(100), "]".repeat(100));
    for i in 1..=3 {
        y.push_str(&format!(
            "a{i}: &a{i} {}*a{}{}\n",
            "[".repeat(100),
            i - 1,
            "]".repeat(100)
        ));
    }
    let err = noyalib::from_str::<IgnoredAny>(&y).expect_err("too deep once expanded");
    assert!(
        matches!(err, noyalib::Error::RecursionLimitExceeded { .. }),
        "{err:?}"
    );
}
