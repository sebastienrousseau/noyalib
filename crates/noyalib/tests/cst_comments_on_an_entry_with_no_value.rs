// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Comments on an entry written with nothing after its `:` (issue #425).
//!
//! Such an entry is an implicit null. It has a key token of its own, and
//! `write_span` has resolved it since #310/#311, which is what lets
//! `set_value` fill it in. The comment API asked for the value's span
//! instead, so a `# todo` sitting beside the entry was invisible to
//! `comments_at` and unreachable by the mutators.
//!
//! The sequence half is deliberately left alone: see the controls at the
//! bottom, and the note in `comment_anchor_span`.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

// ── read ────────────────────────────────────────────────────────────

#[test]
fn the_comment_beside_an_entry_with_no_value_is_reported() {
    let doc = parse_document("k:   # todo\nafter: 1  # kept\n").unwrap();
    assert_eq!(
        doc.comments_at("k").inline.map(|c| c.text),
        Some(" todo".into())
    );
    // The sibling is unaffected, which is what says the two are told apart.
    assert_eq!(
        doc.comments_at("after").inline.map(|c| c.text),
        Some(" kept".into())
    );
}

#[test]
fn depth_and_the_end_of_input_do_not_change_the_answer() {
    let nested = parse_document("outer:\n  k:   # todo\n  after: 1\n").unwrap();
    assert_eq!(
        nested.comments_at("outer.k").inline.map(|c| c.text),
        Some(" todo".into())
    );
    // No trailing newline, so there is nothing after the comment either.
    let tail = parse_document("a: 1\nk:   # todo").unwrap();
    assert_eq!(
        tail.comments_at("k").inline.map(|c| c.text),
        Some(" todo".into())
    );
}

#[test]
fn an_entry_with_no_value_and_no_comment_reports_none() {
    let doc = parse_document("k:\nafter: 1\n").unwrap();
    assert!(doc.comments_at("k").inline.is_none());
}

// ── set ─────────────────────────────────────────────────────────────

#[test]
fn a_comment_can_be_set_on_an_entry_with_no_value() {
    let mut doc = parse_document("k:\nafter: 1\n").unwrap();
    doc.set_inline_comment("k", "set").unwrap();
    assert_eq!(doc.source(), "k:  # set\nafter: 1\n");
}

#[test]
fn replacing_one_keeps_the_gutter_the_author_wrote() {
    let mut doc = parse_document("k:   # todo\nafter: 1\n").unwrap();
    doc.set_inline_comment("k", "set").unwrap();
    assert_eq!(doc.source(), "k:   # set\nafter: 1\n");
}

#[test]
fn setting_one_at_the_end_of_input_adds_no_newline() {
    let mut doc = parse_document("a: 1\nk:   # todo").unwrap();
    doc.set_inline_comment("k", "set").unwrap();
    assert_eq!(doc.source(), "a: 1\nk:   # set");
}

#[test]
fn the_value_is_still_null_afterwards() {
    let mut doc = parse_document("k:\nafter: 1\n").unwrap();
    doc.set_inline_comment("k", "set").unwrap();
    let v: noyalib::Value = noyalib::from_str(doc.source()).unwrap();
    assert!(v.get_path("k").is_some_and(noyalib::Value::is_null));
}

// ── remove ──────────────────────────────────────────────────────────

#[test]
fn the_comment_can_be_removed_leaving_no_trailing_space() {
    let mut doc = parse_document("k:   # todo\nafter: 1\n").unwrap();
    doc.remove_inline_comment("k").unwrap();
    assert_eq!(doc.source(), "k:\nafter: 1\n");
}

#[test]
fn read_set_read_round_trips() {
    let mut doc = parse_document("k:   # todo\n").unwrap();
    let text = doc.comments_at("k").inline.map(|c| c.text).unwrap();
    doc.set_inline_comment("k", text.trim()).unwrap();
    assert_eq!(
        doc.comments_at("k").inline.map(|c| c.text),
        Some(" todo".into())
    );
}

// ── controls: what this must not change ─────────────────────────────

#[test]
fn an_empty_sequence_item_is_untouched() {
    // The same shape reached by the other indicator, and deliberately not
    // covered: neither go-yaml nor ruamel.yaml produces a usable result
    // there. Setting orphans the existing comment onto its own line, and
    // removing the item leaves it behind. This does not invent an answer.
    let src = "xs:\n  -   # todo\n  - 1\n";
    let doc = parse_document(src).unwrap();
    assert!(doc.comments_at("xs[0]").inline.is_none());
    let mut doc = parse_document(src).unwrap();
    assert!(doc.set_inline_comment("xs[0]", "set").is_err());
    assert_eq!(doc.source(), src);
}

#[test]
fn a_path_that_resolves_to_nothing_is_still_refused() {
    let mut doc = parse_document("a: 1\n").unwrap();
    assert!(doc.set_inline_comment("nope", "x").is_err());
    assert!(doc.comments_at("nope").inline.is_none());
}

#[test]
fn an_entry_that_has_a_value_is_unaffected() {
    let mut doc = parse_document("k: 1  # todo\n").unwrap();
    assert_eq!(
        doc.comments_at("k").inline.map(|c| c.text),
        Some(" todo".into())
    );
    doc.set_inline_comment("k", "set").unwrap();
    assert_eq!(doc.source(), "k: 1  # set\n");
}
