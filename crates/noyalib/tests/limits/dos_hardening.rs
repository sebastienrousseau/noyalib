// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Adversarial denial-of-service hardening regressions.
//!
//! A YAML parser that ingests untrusted input must reject resource
//! bombs with a bounded, typed error rather than exhausting the stack,
//! the heap, or CPU. These tests complement `budget_breach.rs`,
//! `stress_load.rs`, and `panic_free.rs` by covering vectors those miss:
//!
//! * deeply nested **flow** collections `[[[[…` / `{a:{a:…}}` (block
//!   nesting is already covered; flow was only tested for no-panic);
//! * the sequence-width and mapping-width caps, which must surface as
//!   structured [`noyalib::BudgetBreach`] variants, not opaque errors;
//! * merge-key (`<<`) amplification combined with alias reuse.

#![allow(missing_docs)]

use noyalib::{BudgetBreach, Error, ParserConfig, Value, from_str, load_all_with_config};

fn load_all(yaml: &str, cfg: &ParserConfig) -> Result<Vec<Value>, Error> {
    load_all_with_config(yaml, cfg)?.collect()
}

// ── Deep flow nesting: reject, never stack-overflow ─────────────────

#[test]
fn deep_flow_sequence_is_rejected_not_overflowed() {
    // 100k-deep `[[[…]]]`. The parser is iterative, so this must return
    // a clean recursion-limit error, not smash the stack.
    let depth = 100_000;
    let mut s = String::with_capacity(depth * 2 + 4);
    s.push_str("v: ");
    for _ in 0..depth {
        s.push('[');
    }
    for _ in 0..depth {
        s.push(']');
    }
    let err = from_str::<Value>(&s).unwrap_err();
    assert!(
        matches!(err, Error::RecursionLimitExceeded { .. }),
        "deep flow sequence must trip the recursion limit, got {err:?}"
    );
}

#[test]
fn deep_flow_mapping_is_rejected_not_overflowed() {
    // 100k-deep `{a: {a: {a: … 0 …}}}`.
    let depth = 100_000;
    let mut s = String::with_capacity(depth * 4 + 8);
    s.push_str("v: ");
    for _ in 0..depth {
        s.push_str("{a: ");
    }
    s.push('0');
    for _ in 0..depth {
        s.push('}');
    }
    let err = from_str::<Value>(&s).unwrap_err();
    assert!(
        matches!(err, Error::RecursionLimitExceeded { .. }),
        "deep flow mapping must trip the recursion limit, got {err:?}"
    );
}

#[test]
fn recursion_limit_is_configurable_below_the_default() {
    let yaml = "a: {b: {c: {d: 1}}}\n";
    let cfg = ParserConfig::new().max_depth(2);
    let err = load_all(yaml, &cfg).unwrap_err();
    assert!(
        matches!(err, Error::RecursionLimitExceeded { .. }),
        "a low max_depth must reject moderately nested input, got {err:?}"
    );
}

// ── Width caps surface as structured budget breaches ────────────────

#[test]
fn oversize_sequence_trips_max_sequence_length_budget() {
    let yaml = format!("[{}]", "0,".repeat(50).trim_end_matches(','));
    let cfg = ParserConfig::new().max_sequence_length(8);
    let err = load_all(&yaml, &cfg).unwrap_err();
    match err {
        Error::Budget(BudgetBreach::MaxSequenceLength { limit, observed }) => {
            assert_eq!(limit, 8);
            assert!(observed > 8, "observed {observed} > limit 8");
        }
        other => panic!("expected MaxSequenceLength budget breach, got {other:?}"),
    }
}

#[test]
fn oversize_mapping_trips_max_mapping_keys_budget() {
    let mut yaml = String::from("{");
    for i in 0..50 {
        if i > 0 {
            yaml.push(',');
        }
        yaml.push_str(&format!("k{i}: {i}"));
    }
    yaml.push('}');
    let cfg = ParserConfig::new().max_mapping_keys(8);
    let err = load_all(&yaml, &cfg).unwrap_err();
    match err {
        Error::Budget(BudgetBreach::MaxMappingKeys { limit, observed }) => {
            assert_eq!(limit, 8);
            assert!(observed > 8, "observed {observed} > limit 8");
        }
        other => panic!("expected MaxMappingKeys budget breach, got {other:?}"),
    }
}

#[test]
fn width_cap_breaches_classify_as_budget_errors() {
    // A DoS-aware caller routes on `ErrorKind::Budget`; both width caps
    // must land in that bucket (previously they were opaque `Serialize`
    // errors and would have been misrouted).
    use noyalib::ErrorKind;
    let seq = format!("[{}]", "0,".repeat(20).trim_end_matches(','));
    let err = load_all(&seq, &ParserConfig::new().max_sequence_length(4)).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Budget, "seq width cap: {err:?}");
}

// ── Merge-key amplification is bounded ──────────────────────────────

#[test]
fn merge_key_amplification_is_bounded() {
    // A document that merges an anchored map many times over. Whatever
    // the outcome, it must be a bounded error (or a bounded success),
    // never unbounded work.
    let mut yaml = String::from("base: &b\n  a: 1\n  b: 2\ntarget:\n");
    for _ in 0..10_000 {
        yaml.push_str("  <<: *b\n");
    }
    let cfg = ParserConfig::new()
        .max_merge_keys(100)
        .alias_anchor_ratio(None);
    let res = load_all(&yaml, &cfg);
    if let Err(err) = res {
        assert!(
            matches!(
                err,
                Error::Budget(BudgetBreach::MaxMergeKeys { .. }) | Error::RepetitionLimitExceeded
            ),
            "merge amplification must be a bounded budget error, got {err:?}"
        );
    }
}

fn assert_fixed_name_budget(source: &str, allowed: bool) {
    let value = from_str::<Value>(source);
    let typed = from_str::<std::collections::BTreeMap<String, Value>>(source);
    let cst = noyalib::cst::parse_document(source);
    assert_eq!(
        [value.is_ok(), typed.is_ok(), cst.is_ok()],
        [allowed; 3],
        "{source:?}"
    );
    if allowed {
        let value = value.unwrap();
        let output = noyalib::to_string(&value).unwrap();
        assert_eq!(from_str::<Value>(&output).unwrap(), value);
    } else {
        for error in [value.unwrap_err(), typed.unwrap_err()] {
            assert!(error.location().is_some(), "{error}");
            assert!(
                error.to_string().contains("maximum length of 1024 bytes"),
                "{error}"
            );
        }
    }
}

#[test]
fn resolved_tag_budget_is_uniform_across_loaders() {
    for bytes in [1024, 1025] {
        let unicode = format!(
            "{}{}",
            "é".repeat(512),
            if bytes == 1024 { "" } else { "a" }
        );
        for source in [
            format!("key: !<{}> value", "t".repeat(bytes)),
            format!("key: !{} value", "t".repeat(bytes - 1)),
            format!("key: !!{}!x value", "t".repeat(bytes - 4)),
            format!(
                "%TAG !h! tag:example:{}\n---\nkey: !h!v value",
                "t".repeat(bytes - 13)
            ),
            format!("key: !<{unicode}> value"),
        ] {
            assert_fixed_name_budget(&source, bytes == 1024);
        }
    }
}

#[test]
fn anchor_name_budget_also_applies_at_eof() {
    for bytes in [1024, 1025] {
        for name in [
            "a".repeat(bytes),
            format!(
                "{}{}",
                "é".repeat(512),
                if bytes == 1024 { "" } else { "a" }
            ),
        ] {
            assert_fixed_name_budget(&format!("key: &{name}"), bytes == 1024);
        }
    }
}
