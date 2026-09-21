// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Borrowed-path alias resolution.
//!
//! Anchors (`&name`) and aliases (`*name`) are eagerly resolved on
//! the borrowed path — the anchored value is stored in a side-table
//! keyed by name, and each alias clones the value into the tree.
//! String fields stay `Cow::Borrowed`, so a clone is mostly free —
//! only sequences and mappings actually duplicate.
//!
//! Total expansions are bounded by `max_alias_expansions` to
//! neutralise YAML bombs the same way the owned path does.

#![allow(missing_docs)]

use noyalib::ParserConfig;
use noyalib::borrowed::{BorrowedValue, from_str_borrowed, from_str_borrowed_with_config};

// ── Simple anchor + alias (scalar) ─────────────────────────────────

#[test]
fn scalar_alias_clones_string() {
    let yaml = "first: &greeting hello\nsecond: *greeting\n";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let m = v.as_mapping().unwrap();
    assert_eq!(m.get("first").and_then(|v| v.as_str()), Some("hello"));
    assert_eq!(m.get("second").and_then(|v| v.as_str()), Some("hello"));
}

#[test]
fn scalar_alias_clones_integer() {
    let yaml = "limit: &cap 42\nbackoff: *cap\n";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let m = v.as_mapping().unwrap();
    assert_eq!(m.get("limit").and_then(|v| v.as_i64()), Some(42));
    assert_eq!(m.get("backoff").and_then(|v| v.as_i64()), Some(42));
}

// ── Sequence anchor + alias ────────────────────────────────────────

#[test]
fn sequence_alias_clones_subtree() {
    let yaml = "shared: &flags [a, b, c]\nuse_a: *flags\nuse_b: *flags\n";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let m = v.as_mapping().unwrap();
    let a = m.get("use_a").and_then(|v| v.as_sequence()).unwrap();
    let b = m.get("use_b").and_then(|v| v.as_sequence()).unwrap();
    assert_eq!(a.len(), 3);
    assert_eq!(b.len(), 3);
    assert_eq!(a[0].as_str(), Some("a"));
    assert_eq!(b[2].as_str(), Some("c"));
}

// ── Mapping anchor + alias ─────────────────────────────────────────

#[test]
fn mapping_alias_clones_subtree() {
    let yaml = "\
defaults: &base
  retries: 3
  timeout: 30
service_a: *base
service_b: *base
";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let m = v.as_mapping().unwrap();
    let a = m.get("service_a").and_then(|v| v.as_mapping()).unwrap();
    let b = m.get("service_b").and_then(|v| v.as_mapping()).unwrap();
    assert_eq!(a.get("retries").and_then(|v| v.as_i64()), Some(3));
    assert_eq!(b.get("timeout").and_then(|v| v.as_i64()), Some(30));
}

// ── Multiple distinct anchors in one document ──────────────────────

#[test]
fn distinct_anchors_resolve_independently() {
    let yaml = "\
a: &x apple
b: &y banana
ref_x: *x
ref_y: *y
";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let m = v.as_mapping().unwrap();
    assert_eq!(m.get("ref_x").and_then(|v| v.as_str()), Some("apple"));
    assert_eq!(m.get("ref_y").and_then(|v| v.as_str()), Some("banana"));
}

// ── Anchor namespace resets between documents ──────────────────────

#[test]
fn anchor_does_not_leak_into_next_document() {
    // Per YAML spec each document has its own anchor namespace.
    // The first document's anchor `x` must not be visible to the
    // second — so a `*x` reference there errors as a missing anchor
    // rather than silently resolving to the prior doc's value.
    let yaml = "first: &x hi\n---\nsecond: *x\n";
    let err = from_str_borrowed(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("anchor") && msg.contains('x'),
        "expected unknown-anchor error from leaked-anchor reference, got: {msg}"
    );
}

// ── Unknown alias errors out ───────────────────────────────────────

#[test]
fn unknown_alias_is_an_error() {
    let yaml = "ref: *missing\n";
    let err = from_str_borrowed(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("missing") && msg.contains("anchor"),
        "expected message to mention the missing anchor, got: {msg}"
    );
}

// ── Alias as a mapping key ────────────────────────────────────────

#[test]
fn alias_resolved_to_string_works_as_key() {
    let yaml = "key_anchor: &k production\n*k : enabled\n";
    let v: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
    let m = v.as_mapping().unwrap();
    assert_eq!(
        m.get("production").and_then(|v| v.as_str()),
        Some("enabled")
    );
}

#[test]
fn alias_resolved_to_non_scalar_cannot_be_a_key() {
    // `&seq [a, b]` then using `*seq` as a key — YAML's owned
    // path coerces non-scalar keys via Display; the borrowed path
    // rejects them explicitly to keep the type system honest
    // (mapping keys are `Cow<'a, str>`).
    let yaml = "src: &seq [a, b]\n*seq : value\n";
    let err = from_str_borrowed(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("alias") || msg.contains("non-scalar"),
        "expected non-scalar-key error, got: {msg}"
    );
}

// ── Bomb defence: alias expansion is bounded ───────────────────────

#[test]
fn excess_alias_expansions_error() {
    // Ten-fan-out structure with a low cap. We don't need a
    // billion-laughs-style exponential to verify the limit fires.
    let yaml = "\
shared: &x leaf
items:
  - *x
  - *x
  - *x
  - *x
  - *x
";
    let cfg = ParserConfig::new().max_alias_expansions(2);
    let err = from_str_borrowed_with_config(yaml, &cfg).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("alias") && msg.contains("exceed"),
        "expected expansion-cap error, got: {msg}"
    );
}

#[test]
fn alias_expansion_within_limit_succeeds() {
    let yaml = "\
shared: &x leaf
items:
  - *x
  - *x
";
    let cfg = ParserConfig::new().max_alias_expansions(10);
    let v: BorrowedValue<'_> = from_str_borrowed_with_config(yaml, &cfg).unwrap();
    let items = v
        .as_mapping()
        .unwrap()
        .get("items")
        .and_then(|v| v.as_sequence())
        .unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].as_str(), Some("leaf"));
    assert_eq!(items[1].as_str(), Some("leaf"));
}

// ── Round-trip parity with owned path ──────────────────────────────

#[test]
fn alias_resolution_matches_owned_path() {
    let yaml = "\
defaults: &base
  retries: 3
  timeout: 30
prod: *base
staging: *base
";
    let borrowed = from_str_borrowed(yaml).unwrap().into_owned();
    let owned: noyalib::Value = noyalib::from_str(yaml).unwrap();
    assert_eq!(borrowed, owned);
}
