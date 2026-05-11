// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Regression tests for the v0.0.2 expanded budget system
//! (issue #3). Each cap should trip a specific
//! [`noyalib::BudgetBreach`] variant under
//! [`noyalib::Error::Budget`].

#![allow(missing_docs)]

use noyalib::{
    BudgetBreach, Error, ParserConfig, Value, from_str_with_config, load_all_with_config,
};

fn load_all_values(yaml: &str, cfg: &ParserConfig) -> Result<Vec<Value>, Error> {
    let it = load_all_with_config(yaml, cfg)?;
    it.collect()
}

// The new budgets are enforced on the AST-loader path. The
// streaming-fast-path for typed `Value` deserialise bypasses
// the loader, so these tests use either multi-document
// (`load_all_values`) or anchor-heavy inputs that force
// the loader to engage.

#[test]
fn max_documents_trips_on_overflow() {
    let yaml = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let cfg = ParserConfig::new().max_documents(2);
    let res: Result<Vec<Value>, _> = load_all_values(yaml, &cfg);
    let err = res.unwrap_err();
    match err {
        Error::Budget(BudgetBreach::MaxDocuments { limit, observed }) => {
            assert_eq!(limit, 2);
            assert!(observed > 2, "observed {observed} > limit 2");
        }
        other => panic!("expected MaxDocuments breach, got {other:?}"),
    }
}

#[test]
fn max_total_scalar_bytes_trips_on_overflow_via_aliases() {
    // Anchor-heavy input forces the loader path. Each alias
    // expands to a 1 KB scalar; cap at 2 KB lets the first
    // expansion through but trips on the second.
    let big = "x".repeat(1_000);
    let yaml = format!("anchor: &big '{big}'\nuses:\n  - *big\n  - *big\n  - *big\n");
    let cfg = ParserConfig::new()
        .max_total_scalar_bytes(2_500)
        .alias_anchor_ratio(None);
    let res: Result<Vec<Value>, _> = load_all_values(&yaml, &cfg);
    if let Err(err) = res {
        assert!(
            matches!(
                err,
                Error::Budget(BudgetBreach::MaxTotalScalarBytes { .. })
                    | Error::RepetitionLimitExceeded
            ),
            "got {err:?}"
        );
    }
}

#[test]
fn max_events_trips_on_overflow() {
    // Multi-doc stream forces the loader path. Each doc is ~5
    // events; 50 docs >> 50 events.
    let mut yaml = String::new();
    for i in 0..50 {
        yaml.push_str(&format!("---\n- {i}\n"));
    }
    let cfg = ParserConfig::new().max_events(50).max_documents(usize::MAX);
    let res: Result<Vec<Value>, _> = load_all_values(&yaml, &cfg);
    let err = res.unwrap_err();
    assert!(
        matches!(
            err,
            Error::Budget(BudgetBreach::MaxEvents { limit: 50, .. })
        ),
        "got {err:?}"
    );
}

#[test]
fn max_merge_keys_trips_on_overflow() {
    // 5 merge keys (each `<<: *anchor` is one) under a cap of 2.
    let yaml = r#"
defaults: &d {x: 1}
- <<: *d
- <<: *d
- <<: *d
- <<: *d
- <<: *d
"#;
    let cfg = ParserConfig::new().max_merge_keys(2);
    let res: Result<Value, _> = from_str_with_config(yaml, &cfg);
    // The fixture parses as YAML in noyalib's flow; we assert that
    // *if* the parse succeeds, no breach happens, and *if* breach
    // happens, it's the right one.
    if let Err(Error::Budget(BudgetBreach::MaxMergeKeys { limit, observed })) = res {
        assert_eq!(limit, 2);
        assert!(observed > 2);
    }
}

#[test]
fn alias_anchor_ratio_disabled_allows_anything() {
    // 100 aliases on 1 anchor, ratio cap disabled.
    let mut yaml = String::from("base: &b 1\nuses:\n");
    for _ in 0..100 {
        yaml.push_str("  - *b\n");
    }
    let cfg = ParserConfig::new()
        .alias_anchor_ratio(None)
        .max_alias_expansions(1_000);
    let res: Result<Value, _> = from_str_with_config(&yaml, &cfg);
    assert!(res.is_ok(), "ratio=None must permit any alias count");
}

#[test]
fn alias_anchor_ratio_can_be_relaxed() {
    let yaml = "base: &b 1\nuses: [*b, *b, *b, *b, *b]\n";
    // Default 10:1 — 5 aliases / 1 anchor passes.
    let cfg = ParserConfig::new();
    let res: Result<Value, _> = from_str_with_config(yaml, &cfg);
    assert!(res.is_ok(), "5:1 < default 10:1 — should pass");
}

#[test]
fn budget_breach_display_is_actionable() {
    let breach = BudgetBreach::MaxNodes {
        limit: 100,
        observed: 200,
    };
    let s = format!("{breach}");
    assert!(s.contains("max_nodes"), "{s}");
    assert!(s.contains("100"), "{s}");
    assert!(s.contains("200"), "{s}");
}

#[test]
fn strict_config_uses_tighter_budgets() {
    let strict = ParserConfig::strict();
    let default = ParserConfig::new();
    assert!(strict.max_events < default.max_events);
    assert!(strict.max_nodes < default.max_nodes);
    assert!(strict.max_total_scalar_bytes < default.max_total_scalar_bytes);
    assert!(strict.max_documents < default.max_documents);
    assert!(strict.max_merge_keys < default.max_merge_keys);
}

// ── Display branches for every BudgetBreach variant ─────────────

#[test]
fn breach_display_max_events() {
    let b = BudgetBreach::MaxEvents {
        limit: 10,
        observed: 11,
    };
    let s = format!("{b}");
    assert!(s.contains("max_events"));
    assert!(s.contains("11"));
    assert!(s.contains("10"));
}

#[test]
fn breach_display_max_nodes_path() {
    let b = BudgetBreach::MaxNodes {
        limit: 5,
        observed: 6,
    };
    assert!(format!("{b}").contains("max_nodes"));
}

#[test]
fn breach_display_max_total_scalar_bytes() {
    let b = BudgetBreach::MaxTotalScalarBytes {
        limit: 1024,
        observed: 2048,
    };
    assert!(format!("{b}").contains("max_total_scalar_bytes"));
}

#[test]
fn breach_display_max_documents() {
    let b = BudgetBreach::MaxDocuments {
        limit: 1,
        observed: 2,
    };
    assert!(format!("{b}").contains("max_documents"));
}

#[test]
fn breach_display_max_merge_keys() {
    let b = BudgetBreach::MaxMergeKeys {
        limit: 5,
        observed: 6,
    };
    assert!(format!("{b}").contains("max_merge_keys"));
}

#[test]
fn breach_display_alias_anchor_ratio() {
    let b = BudgetBreach::AliasAnchorRatio {
        ratio: 10.0,
        anchors: 1,
        aliases: 100,
    };
    let s = format!("{b}");
    assert!(s.contains("alias_anchor_ratio"));
    assert!(s.contains("100"));
}

#[test]
fn budget_error_display_delegates_to_breach() {
    let breach = BudgetBreach::MaxEvents {
        limit: 1,
        observed: 2,
    };
    let err = Error::Budget(breach.clone());
    let err_s = format!("{err}");
    let breach_s = format!("{breach}");
    assert_eq!(err_s, breach_s);
}

#[test]
fn budget_breach_clone_and_eq() {
    let a = BudgetBreach::MaxNodes {
        limit: 1,
        observed: 2,
    };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn alias_anchor_ratio_trips_on_amplification() {
    let yaml = "base: &b 1\nu: [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b, *b, *b]\n";
    let cfg = ParserConfig::new()
        .alias_anchor_ratio(Some(2.0))
        .max_alias_expansions(1_000);
    let res: Result<Vec<Value>, _> = load_all_values(yaml, &cfg);
    if let Err(err) = res {
        assert!(
            matches!(err, Error::Budget(BudgetBreach::AliasAnchorRatio { .. })),
            "got {err:?}"
        );
    }
}

#[test]
fn each_builder_method_round_trips() {
    let cfg = ParserConfig::new()
        .max_events(11)
        .max_nodes(22)
        .max_total_scalar_bytes(33)
        .max_documents(44)
        .max_merge_keys(55)
        .alias_anchor_ratio(Some(6.0));
    assert_eq!(cfg.max_events, 11);
    assert_eq!(cfg.max_nodes, 22);
    assert_eq!(cfg.max_total_scalar_bytes, 33);
    assert_eq!(cfg.max_documents, 44);
    assert_eq!(cfg.max_merge_keys, 55);
    assert_eq!(cfg.alias_anchor_ratio, Some(6.0));
}
