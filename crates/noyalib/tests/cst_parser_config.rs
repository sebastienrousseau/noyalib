//! #372: the lossless CST honors a caller-supplied `ParserConfig`.
//!
//! `parse_document_with_config` / `parse_stream_with_config` mirror
//! `from_str_with_config`, and the `Document` keeps the
//! configuration for every internal re-parse — the lazy cache, the
//! `replace_span` safety net, `validate`, and the edit oracles — so
//! a document that only opens under a relaxed limit stays readable
//! and editable instead of panicking on its second read.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![cfg(feature = "std")]

use noyalib::ParserConfig;
use noyalib::cst::{parse_document, parse_document_with_config, parse_stream_with_config};

/// The reporter's file shape: `anchors` default blocks, merged into
/// `aliases` tenant entries round-robin.
fn tenants_file(anchors: usize, aliases: usize) -> String {
    let mut s = String::from("defaults:\n");
    for i in 0..anchors {
        s.push_str(&format!("  d{i}: &a{i}\n    k: v{i}\n"));
    }
    s.push_str("tenants:\n");
    for i in 0..aliases {
        s.push_str(&format!("  t{i}:\n    <<: *a{}\n", i % anchors));
    }
    s
}

#[test]
fn ratio_tripping_values_file_gets_its_lossless_path_back() {
    // 221 aliases over 22 anchors is 10.05 — just past the default
    // heuristic. The defaults must keep refusing it, unchanged.
    let src = tenants_file(22, 221);
    let err = parse_document(&src).expect_err("defaults must still trip the ratio");
    assert!(
        err.to_string().contains("alias_anchor_ratio"),
        "expected the ratio heuristic, got: {err}"
    );

    let cfg = ParserConfig::new().alias_anchor_ratio(None);
    let doc = parse_document_with_config(&src, &cfg).expect("relaxed config must open it");
    assert_eq!(doc.to_string(), src, "byte-preserving read");
}

#[test]
fn relaxed_document_survives_edit_then_read() {
    // The reporter's panic path: an edit invalidates the lazy cache,
    // and the next read re-parses. Before #372 that re-parse ran on
    // the defaults and hit the `expect` in `ensure_cache`.
    let src = tenants_file(22, 221);
    let cfg = ParserConfig::new().alias_anchor_ratio(None);
    let mut doc = parse_document_with_config(&src, &cfg).unwrap();

    doc.set("defaults.d0.k", "edited").expect("edit must apply");
    let v = doc.as_value();
    assert_eq!(
        v["defaults"]["d0"]["k"].as_str(),
        Some("edited"),
        "read after edit re-parses under the document's own config"
    );
    doc.validate().expect("validate honors the config too");
}

#[test]
fn relaxed_document_comment_edit_guard_stays_active() {
    // Before #372 the comment-edit guard parsed with the defaults,
    // failed silently on a relaxed-only document, and skipped its
    // value check. It must run — and succeed — under the document's
    // configuration.
    let src = tenants_file(22, 221);
    let cfg = ParserConfig::new().alias_anchor_ratio(None);
    let mut doc = parse_document_with_config(&src, &cfg).unwrap();
    doc.set_comment(
        "defaults.d0",
        noyalib::cst::CommentPosition::Before,
        "leading",
    )
    .expect("comment edit on a relaxed document");
    assert!(doc.to_string().contains("# leading"));
}

#[test]
fn stream_documents_keep_the_config() {
    let one = tenants_file(22, 221);
    let src = format!("---\n{one}---\n{one}");
    let cfg = ParserConfig::new().alias_anchor_ratio(None);
    let docs = parse_stream_with_config(&src, &cfg).expect("stream under relaxed config");
    assert_eq!(docs.len(), 2);
    for mut doc in docs {
        doc.set("defaults.d1.k", "x").expect("edit");
        let _ = doc.as_value(); // must not panic
    }
}

#[test]
fn disabling_the_ratio_does_not_disable_amplification_budgets() {
    // The issue's "what must not change": with the ratio off, the
    // absolute `max_alias_expansions` budget (default 1024) still
    // refuses 1025 merges.
    let src = tenants_file(22, 1025);
    let cfg = ParserConfig::new().alias_anchor_ratio(None);
    let err = parse_document_with_config(&src, &cfg)
        .expect_err("1025 alias expansions must exceed the absolute budget");
    assert!(
        err.to_string().contains("alias") || err.to_string().contains("expansion"),
        "expected the expansion budget, got: {err}"
    );
}

#[test]
fn default_entry_points_are_unchanged() {
    // 220 aliases over 22 anchors is exactly 10.0 — not past the
    // strict `>` comparison — and must keep parsing on defaults.
    let src = tenants_file(22, 220);
    let doc = parse_document(&src).expect("at-threshold file parses on defaults");
    assert_eq!(doc.to_string(), src);
}
