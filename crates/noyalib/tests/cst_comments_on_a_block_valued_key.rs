// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Leading comments on a key whose value is a block collection.
//!
//! A leading comment decorates the entry, so the line it sits above is the
//! entry's first line, which is the key's. The comment API measured from
//! the value's span instead. For a scalar, a flow collection or an implicit
//! null the key and the value share a line and either would do; for a block
//! collection the value starts on the next line, so the run above the key
//! was out of reach.
//!
//! One anchor, four faces: `comments_at` reported nothing, `set_comment`
//! spliced the comment inside the block where it documented the first child,
//! `set_leading_comment` stacked a second line instead of replacing the
//! first, and both removers reported success having removed nothing.

#![allow(missing_docs)]

use noyalib::cst::{CommentPosition, parse_document};

/// The same entry with each of the value shapes that start on the next line.
const BLOCK_VALUES: &[(&str, &str)] = &[
    ("mapping", "k:\n  n: 1\n"),
    ("sequence", "k:\n  - 1\n"),
    ("mapping, several entries", "k:\n  n: 1\n  m: 2\n"),
];

// ── read ────────────────────────────────────────────────────────────

#[test]
fn the_run_above_a_block_valued_key_is_reported() {
    for (shape, value) in BLOCK_VALUES {
        let doc = parse_document(&format!("# doc for k\n{value}")).unwrap();
        assert_eq!(
            doc.comments_at("k")
                .before
                .iter()
                .map(|c| c.text.clone())
                .collect::<Vec<_>>(),
            vec![" doc for k".to_string()],
            "{shape}"
        );
    }
}

#[test]
fn the_whole_run_is_reported_not_just_its_last_line() {
    let doc = parse_document("# one\n# two\nk:\n  n: 1\n").unwrap();
    assert_eq!(doc.comments_at("k").before.len(), 2);
}

#[test]
fn a_comment_indented_inside_the_block_is_not_the_keys() {
    // It sits below the key, so no upward walk from the key's line reaches
    // it. It belongs to the entry it precedes.
    let doc = parse_document("k:\n  # about n\n  n: 1\n").unwrap();
    assert!(doc.comments_at("k").before.is_empty());
    assert_eq!(
        doc.comments_at("k.n")
            .before
            .iter()
            .map(|c| c.text.clone())
            .collect::<Vec<_>>(),
        vec![" about n".to_string()]
    );
}

// ── write ───────────────────────────────────────────────────────────

#[test]
fn set_comment_writes_above_the_key_not_inside_the_block() {
    for (shape, value) in BLOCK_VALUES {
        let mut doc = parse_document(value).unwrap();
        doc.set_comment("k", CommentPosition::Before, "documents k")
            .unwrap();
        assert_eq!(doc.source(), &format!("# documents k\n{value}"), "{shape}");
    }
}

#[test]
fn the_two_leading_writers_agree() {
    // `set_comment(Before)` and `set_leading_comment` are two ways to the
    // same edit. They anchored differently, so they disagreed on exactly
    // this shape.
    for (shape, value) in BLOCK_VALUES {
        let mut a = parse_document(value).unwrap();
        a.set_comment("k", CommentPosition::Before, "why").unwrap();
        let mut b = parse_document(value).unwrap();
        b.set_leading_comment("k", "why").unwrap();
        assert_eq!(a.source(), b.source(), "{shape}");
    }
}

#[test]
fn a_second_write_replaces_the_run_rather_than_stacking_on_it() {
    let mut doc = parse_document("k:\n  n: 1\n").unwrap();
    doc.set_leading_comment("k", "first").unwrap();
    doc.set_leading_comment("k", "second").unwrap();
    assert_eq!(doc.source(), "# second\nk:\n  n: 1\n");
}

#[test]
fn the_comment_takes_the_keys_indent_not_the_values() {
    let mut doc = parse_document("outer:\n  inner:\n    n: 1\n").unwrap();
    doc.set_comment("outer.inner", CommentPosition::Before, "why")
        .unwrap();
    assert_eq!(doc.source(), "outer:\n  # why\n  inner:\n    n: 1\n");
}

#[test]
fn the_childs_comment_survives_a_write_aimed_at_the_key() {
    // The sharpest face of the old anchor. Walking up from the value's line
    // landed inside the block, so the run it found was the *child's*. A
    // caller asking to comment `k` overwrote the comment on `n`, and one
    // asking to remove `k`'s deleted it, both at `Ok(())`.
    let src = "k:\n  # about n\n  n: 1\n";

    let mut set = parse_document(src).unwrap();
    set.set_comment("k", CommentPosition::Before, "about k")
        .unwrap();
    assert_eq!(set.source(), "# about k\nk:\n  # about n\n  n: 1\n");

    let mut removed = parse_document(src).unwrap();
    removed
        .remove_comment("k", CommentPosition::Before)
        .unwrap();
    assert_eq!(removed.source(), src, "`k` has none to remove");
}

// ── remove ──────────────────────────────────────────────────────────

#[test]
fn both_removers_take_the_run_above_a_block_valued_key() {
    for (shape, value) in BLOCK_VALUES {
        let src = format!("# doc for k\n{value}");
        let mut a = parse_document(&src).unwrap();
        a.remove_comment("k", CommentPosition::Before).unwrap();
        assert_eq!(a.source(), *value, "remove_comment, {shape}");

        let mut b = parse_document(&src).unwrap();
        b.remove_leading_comment("k").unwrap();
        assert_eq!(b.source(), *value, "remove_leading_comment, {shape}");
    }
}

// ── controls ────────────────────────────────────────────────────────

#[test]
fn the_shapes_that_already_worked_do_not_move() {
    // Key and value share a line here, so the anchor moved by zero bytes.
    for (shape, src, after) in [
        ("scalar", "# old\nk: 1\n", "# new\nk: 1\n"),
        ("flow mapping", "# old\nk: {n: 1}\n", "# new\nk: {n: 1}\n"),
        ("implicit null", "# old\nk:\n", "# new\nk:\n"),
    ] {
        let doc = parse_document(src).unwrap();
        assert_eq!(doc.comments_at("k").before.len(), 1, "{shape}");
        let mut doc = parse_document(src).unwrap();
        doc.set_comment("k", CommentPosition::Before, "new")
            .unwrap();
        assert_eq!(doc.source(), after, "{shape}");
    }
}

#[test]
fn a_sequence_item_keeps_the_value_span_it_always_used() {
    // It has no key token, so there is no entry line to prefer.
    let src = "xs:\n  # mine\n  - 1\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(doc.comments_at("xs[0]").before.len(), 1);

    let mut doc = parse_document("xs:\n  - 1\n").unwrap();
    doc.set_comment("xs[0]", CommentPosition::Before, "mine")
        .unwrap();
    assert_eq!(doc.source(), src);
}

#[test]
fn the_inline_anchor_is_untouched() {
    // Only the leading run moved to the key. An inline comment still
    // follows the value, on the value's own line.
    let doc = parse_document("k: 1  # beside the value\n").unwrap();
    assert_eq!(
        doc.comments_at("k").inline.map(|c| c.text),
        Some(" beside the value".to_string())
    );
}
