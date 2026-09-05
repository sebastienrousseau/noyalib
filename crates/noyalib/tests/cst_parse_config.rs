// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `parse_document_with_config` / `parse_stream_with_config`: the CST
//! entry points honour a `ParserConfig`, and a `Document` keeps the
//! configuration it was opened with for every later re-parse.
//!
//! The motivating input is a Helm-style values file — a few anchored
//! default blocks merged into hundreds of entries with `<<: *anchor` —
//! which is valid YAML that the default alias-to-anchor ratio
//! heuristic (`Some(10.0)`) refuses. Before this API the CST path had
//! no way to relax it, while `from_str_with_config` always could.

#![allow(missing_docs)]

use noyalib::cst::{
    CommentPosition, Document, RepairScope, parse_document, parse_document_with_config,
    parse_stream, parse_stream_with_config,
};
use noyalib::{BudgetBreach, DuplicateKeyPolicy, Error, ParserConfig};

/// A values-file shape: `anchors` anchored default blocks, merged into
/// `aliases` entries round-robin with `<<: *anchor`.
fn merge_heavy(anchors: usize, aliases: usize) -> String {
    let mut src = String::from("defaults:\n");
    for a in 0..anchors {
        src.push_str(&format!("  d{a}: &a{a}\n    k: v{a}\n"));
    }
    src.push_str("tenants:\n");
    for t in 0..aliases {
        src.push_str(&format!("  t{t}:\n    <<: *a{}\n", t % anchors));
    }
    src
}

fn relaxed() -> ParserConfig {
    ParserConfig::new().alias_anchor_ratio(None)
}

fn is_ratio_breach(err: &Error) -> bool {
    matches!(err, Error::Budget(BudgetBreach::AliasAnchorRatio { .. }))
}

// ── the default entry points are unchanged ──────────────────────────

#[test]
fn default_entry_points_keep_the_ratio_heuristic() {
    // 221 merges over 22 anchors is ratio 10.05 — just past the default
    // cap of 10, and the shape of a real tenants file.
    let src = merge_heavy(22, 221);
    assert!(is_ratio_breach(&parse_document(&src).unwrap_err()));
    assert!(is_ratio_breach(&parse_stream(&src).unwrap_err()));
    // One alias fewer parses: the threshold is exact.
    assert!(parse_document(&merge_heavy(22, 220)).is_ok());
}

// ── the configured entry points ─────────────────────────────────────

#[test]
fn parse_document_with_config_disables_the_ratio_heuristic() {
    let src = merge_heavy(22, 221);
    let doc = parse_document_with_config(&src, &relaxed()).unwrap();
    assert_eq!(doc.to_string(), src);
    let v = doc.as_value();
    assert_eq!(v["tenants"]["t0"]["k"].as_str(), Some("v0"));
    assert_eq!(v["tenants"]["t21"]["k"].as_str(), Some("v21"));
    assert_eq!(v["tenants"]["t220"]["k"].as_str(), Some("v0"));
}

#[test]
fn parse_document_with_config_keeps_the_absolute_alias_budget() {
    // Disabling the ratio must not disable the guard that actually bounds
    // amplification: `max_alias_expansions` (default 1024) still refuses.
    let err = parse_document_with_config(&merge_heavy(22, 1025), &relaxed()).unwrap_err();
    assert!(
        matches!(err, Error::RepetitionLimitExceeded),
        "expected the alias-expansion cap, got {err:?}"
    );
}

#[test]
fn parse_document_with_config_honours_every_knob_not_just_the_ratio() {
    // Duplicate keys: last wins by default (YAML 1.2)...
    let src = "a: 1\na: 2\n";
    assert_eq!(
        parse_document(src).unwrap().as_value()["a"].as_i64(),
        Some(2)
    );
    // ...and refused when the configuration says so.
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    // The located form since #378; the kind is what the policy promises.
    assert!(matches!(
        parse_document_with_config(src, &cfg),
        Err(Error::DuplicateKeyAt { .. })
    ));
}

#[test]
fn a_default_config_behaves_like_the_bare_entry_point() {
    let src = merge_heavy(22, 221);
    assert!(is_ratio_breach(
        &parse_document_with_config(&src, &ParserConfig::default()).unwrap_err()
    ));
    let ok = merge_heavy(22, 220);
    let bare = parse_document(&ok).unwrap();
    let configured = parse_document_with_config(&ok, &ParserConfig::default()).unwrap();
    assert_eq!(bare.to_string(), configured.to_string());
    assert_eq!(*bare.as_value(), *configured.as_value());
}

// ── the document keeps its configuration ────────────────────────────

#[test]
fn a_document_keeps_its_configuration_across_local_repair_edits() {
    // A local-repair edit drops the typed cache; the next read rebuilds
    // it from the source. That rebuild must run under the opening
    // configuration — under the defaults it would refuse the source, and
    // the cache refresh treats a refused source as an invariant violation.
    let src = merge_heavy(22, 221);
    let mut doc = parse_document_with_config(&src, &relaxed()).unwrap();
    doc.set("defaults.d0.k", "changed").unwrap();
    assert_eq!(
        doc.as_value()["tenants"]["t0"]["k"].as_str(),
        Some("changed")
    );
    assert!(doc.validate().is_ok());
    assert_eq!(doc.to_string(), src.replacen("k: v0\n", "k: changed\n", 1));
}

#[test]
fn a_document_keeps_its_configuration_across_the_full_reparse_safety_net() {
    // Replacing the whole source has no local repair to fit — it takes the
    // full re-parse fallback, which validates the new source under the
    // document's configuration.
    let src = merge_heavy(22, 221);
    let next = merge_heavy(22, 230);
    let mut doc = parse_document_with_config(&src, &relaxed()).unwrap();
    doc.replace_span(0, src.len(), &next).unwrap();
    assert_eq!(doc.last_repair_scope(), Some(RepairScope::Document));
    assert_eq!(doc.to_string(), next);
    assert_eq!(doc.as_value()["tenants"]["t229"]["k"].as_str(), Some("v9"));
}

#[test]
fn a_clone_keeps_the_configuration() {
    let src = merge_heavy(22, 221);
    let doc = parse_document_with_config(&src, &relaxed()).unwrap();
    let mut copy = doc.clone();
    copy.set("defaults.d1.k", "x").unwrap();
    assert_eq!(copy.as_value()["tenants"]["t1"]["k"].as_str(), Some("x"));
    // The original is untouched.
    assert_eq!(doc.as_value()["tenants"]["t1"]["k"].as_str(), Some("v1"));
}

#[test]
fn comment_edits_stay_guarded_under_the_configuration() {
    // A comment edit is checked to change comments only, by loading the
    // document before and after. That load runs under the document's own
    // configuration: on a relaxed-budget document the default limits would
    // refuse the "before" load and the guard would be skipped silently.
    let mut src = merge_heavy(22, 221);
    src.push_str("note: |\n  text\n");
    let mut doc = parse_document_with_config(&src, &relaxed()).unwrap();
    doc.set_inline_comment("defaults.d0.k", "the default")
        .unwrap();
    assert_eq!(
        doc.to_string(),
        src.replacen("k: v0\n", "k: v0  # the default\n", 1)
    );
    assert_eq!(doc.as_value()["tenants"]["t0"]["k"].as_str(), Some("v0"));
    // On a block scalar the `#` lands inside the content and changes the
    // value; the edit is refused with the source unchanged — the guard
    // ran. (The same edit on a default-configuration document is refused
    // the same way; this pins that the relaxed document is no exception.)
    let before = doc.to_string();
    let err = doc
        .set_comment("note", CommentPosition::Inline, "x")
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("would change the document's value"),
        "unexpected error: {err}"
    );
    assert_eq!(doc.to_string(), before);
    assert!(
        parse_document("note: |\n  text\n")
            .unwrap()
            .set_comment("note", CommentPosition::Inline, "x")
            .is_err()
    );
}

// ── streams ─────────────────────────────────────────────────────────

#[test]
fn parse_stream_with_config_applies_to_every_document() {
    let first = merge_heavy(22, 221);
    let second = merge_heavy(3, 40);
    let src = format!("---\n{first}---\n{second}");
    assert!(is_ratio_breach(&parse_stream(&src).unwrap_err()));

    let docs = parse_stream_with_config(&src, &relaxed()).unwrap();
    assert_eq!(docs.len(), 2);
    let joined: String = docs.iter().map(Document::source).collect();
    assert_eq!(joined, src);
    assert_eq!(
        docs[0].as_value()["tenants"]["t220"]["k"].as_str(),
        Some("v0")
    );
    assert_eq!(
        docs[1].as_value()["tenants"]["t39"]["k"].as_str(),
        Some("v0")
    );

    // Each document keeps the configuration for its own edits.
    let mut second_doc = docs.into_iter().nth(1).unwrap();
    second_doc.set("defaults.d0.k", "z").unwrap();
    assert_eq!(
        second_doc.as_value()["tenants"]["t0"]["k"].as_str(),
        Some("z")
    );
}

#[test]
fn parse_stream_with_config_single_document_matches_parse_document() {
    let src = merge_heavy(22, 221);
    let docs = parse_stream_with_config(&src, &relaxed()).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].source(), src);
    let single = parse_document_with_config(&src, &relaxed()).unwrap();
    assert_eq!(*docs[0].as_value(), *single.as_value());
}
