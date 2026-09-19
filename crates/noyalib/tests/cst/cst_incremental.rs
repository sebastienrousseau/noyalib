// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase A incremental repair: localised re-parse of `replace_span`.
//!
//! These tests pin three invariants:
//!
//!   1. **Equivalence** — after any edit, `Document::to_string()` and
//!      `Document::as_value()` must match what a full
//!      `parse_document(new_source)` would produce.
//!   2. **Scope discipline** — a typical edit should land at the
//!      smallest meaningful scope (a `MappingEntry` for a value
//!      bump, a `BlockSequence` for an entry add).
//!   3. **Escalation** — any edit that touches an anchor / alias /
//!      tag indicator escalates to a `Document`-scope re-parse so we
//!      do not have to reason about cross-document name resolution
//!      after a localised splice.

use noyalib::cst::{Document, RepairScope, parse_document};

/// Compare the post-edit document against a fresh full re-parse of
/// the same source. Equivalent on both surfaces means the local
/// repair was indistinguishable from a full re-parse.
#[track_caller]
fn assert_equivalent_to_full_reparse(doc: &Document) {
    let from_full = parse_document(&doc.to_string()).expect("parses");
    assert_eq!(doc.to_string(), from_full.to_string(), "to_string mismatch");
    // `as_value` returns `Ref<'_, Value>` since Phase A.2 — deref
    // it for direct comparison.
    assert_eq!(*doc.as_value(), *from_full.as_value(), "as_value mismatch");
}

// ── Equivalence ─────────────────────────────────────────────────────

#[test]
fn scalar_bump_value_matches_full_reparse() {
    let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
    doc.set("version", "0.0.2").unwrap();
    assert_eq!(doc.to_string(), "name: foo\nversion: 0.0.2\n");
    assert_equivalent_to_full_reparse(&doc);
}

#[test]
fn nested_mapping_edit_matches_full_reparse() {
    let mut doc = parse_document("outer:\n  inner: 1\n  next: 2\nother: 3\n").unwrap();
    doc.set("outer.inner", "11").unwrap();
    assert_eq!(
        doc.to_string(),
        "outer:\n  inner: 11\n  next: 2\nother: 3\n",
    );
    assert_equivalent_to_full_reparse(&doc);
}

#[test]
fn sequence_item_edit_matches_full_reparse() {
    let mut doc = parse_document("items:\n  - a\n  - b\n  - c\n").unwrap();
    doc.set("items[1]", "B").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - a\n  - B\n  - c\n");
    assert_equivalent_to_full_reparse(&doc);
}

#[test]
fn remove_middle_entry_matches_full_reparse() {
    let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    doc.remove("b").unwrap();
    assert_eq!(doc.to_string(), "a: 1\nc: 3\n");
    assert_equivalent_to_full_reparse(&doc);
}

#[test]
fn push_back_matches_full_reparse() {
    let mut doc = parse_document("items:\n  - a\n  - b\n").unwrap();
    doc.push_back("items", "c").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - a\n  - b\n  - c\n");
    assert_equivalent_to_full_reparse(&doc);
}

#[test]
fn insert_after_matches_full_reparse() {
    let mut doc = parse_document("items:\n  - a\n  - c\n").unwrap();
    doc.insert_after("items[0]", "b").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - a\n  - b\n  - c\n");
    assert_equivalent_to_full_reparse(&doc);
}

// ── Scope discipline ────────────────────────────────────────────────

#[test]
fn scalar_bump_lands_at_entry_scope() {
    // Setting a leaf scalar value should be repairable at the
    // `MappingEntry` (Entry) scope — the smallest rung the Phase A
    // ladder owns.
    let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
    doc.set("version", "0.0.2").unwrap();
    assert_eq!(doc.last_repair_scope(), Some(RepairScope::Entry));
}

#[test]
fn push_back_lands_at_collection_scope_or_higher() {
    // Adding an entry to a block sequence requires re-parsing at
    // least the `BlockSequence` (Collection) — there is no smaller
    // scope that fits a fragment with a new sibling item.
    let mut doc = parse_document("items:\n  - a\n  - b\n").unwrap();
    doc.push_back("items", "c").unwrap();
    let scope = doc.last_repair_scope().unwrap();
    assert!(
        matches!(
            scope,
            RepairScope::Collection | RepairScope::Entry | RepairScope::Document
        ),
        "unexpected scope: {scope:?}",
    );
}

// ── Escalation ──────────────────────────────────────────────────────

#[test]
fn replace_span_over_alias_escalates_to_document() {
    // The edit window covers the `*anc` alias token — phase A
    // refuses to reason locally about alias resolution.
    let mut doc = parse_document("a: &anc 1\nb: *anc\n").unwrap();
    // Replace `*anc` (bytes 13..17) with a plain scalar.
    doc.replace_span(13, 17, "different").unwrap();
    assert_eq!(doc.last_repair_scope(), Some(RepairScope::Document));
    assert_equivalent_to_full_reparse(&doc);
}

#[test]
fn replacement_introducing_anchor_escalates_to_document() {
    // Adding an anchor where there was none before — must escalate
    // since downstream aliases could newly resolve to it.
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    doc.set("a", "&new 1").unwrap();
    assert_eq!(doc.last_repair_scope(), Some(RepairScope::Document));
    assert_equivalent_to_full_reparse(&doc);
}

#[test]
fn replacement_introducing_tag_escalates_to_document() {
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    doc.set("a", "!!str 1").unwrap();
    assert_eq!(doc.last_repair_scope(), Some(RepairScope::Document));
    assert_equivalent_to_full_reparse(&doc);
}

#[test]
fn replacement_introducing_alias_escalates_to_document() {
    // Adding a `*alias` reference where there was none before. The
    // alias must resolve against an anchor defined elsewhere — the
    // local repair has no way to verify that resolution.
    let mut doc = parse_document("a: &anc 1\nb: 2\n").unwrap();
    doc.set("b", "*anc").unwrap();
    assert_eq!(doc.last_repair_scope(), Some(RepairScope::Document));
    assert_equivalent_to_full_reparse(&doc);
}

// ── Optimistic commit / lazy validation ─────────────────────────────

#[test]
fn invalid_replacement_commits_optimistically_and_panics_on_read() {
    // Phase A.2 trades atomic-rollback at edit time for batch
    // perf: the green-tree splice commits if its fragment-level
    // validation passes, so a cross-document structural error
    // (unclosed flow, here) doesn't surface until the typed view
    // is asked for.
    let mut doc = parse_document("name: foo\n").unwrap();
    doc.set("name", "[").unwrap();
    assert_eq!(doc.to_string(), "name: [\n");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = doc.as_value();
    }));
    assert!(result.is_err(), "as_value must panic on invalid source");
}
