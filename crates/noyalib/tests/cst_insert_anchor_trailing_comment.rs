// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Where a new key lands when the mapping's last entry is a nested
//! block collection followed by a comment.
//!
//! `insert_entry`, `insert_entry_value` and `set_path` all splice after
//! the anchor entry's last line. Two span sources disagree about where that entry
//! stops: the green tree trims a block collection to its content, while
//! the loader's span tree runs on to the next token and sweeps up the
//! blank and comment lines beneath it. Since v0.0.25 the anchor has come
//! from the span tree, so a document-final comment was counted as part
//! of the entry above it and the new key was written *below* the comment
//! — turning a document's closing comment into the new key's head
//! comment (#418).
//!
//! Indentation decides. A comment indented strictly deeper than the
//! anchor entry's key sits inside that entry's block and keeps the new
//! sibling below it; a comment at the entry's own column or shallower is
//! not inside it, and the sibling goes above.
//!
//! Every case here asserts the whole document byte for byte, because the
//! defect moves bytes the caller did not name and leaves a document that
//! still parses.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

/// `insert_entry_value(map, key, "2")` over `src`, returning the source.
fn insert(src: &str, map: &str, key: &str) -> String {
    let mut doc = parse_document(src).expect("source must parse");
    doc.insert_entry_value(map, key, &Value::from("2"))
        .expect("insert must succeed");
    doc.source().to_owned()
}

/// The fragment tier, `insert_entry`, which reaches the same anchor
/// through a second call site.
fn insert_fragment(src: &str, map: &str, key: &str) -> String {
    let mut doc = parse_document(src).expect("source must parse");
    doc.insert_entry(map, key, "2")
        .expect("insert must succeed");
    doc.source().to_owned()
}

/// `set_path`, which the report names as a third way in. It creates the
/// missing levels and hands the outermost to `insert_entry_value`.
fn set_path(src: &str, path: &str) -> String {
    let mut doc = parse_document(src).expect("source must parse");
    doc.set_path(path, &Value::from("2"))
        .expect("set_path must succeed");
    doc.source().to_owned()
}

// --- The reported shape -------------------------------------------

#[test]
fn new_key_goes_above_a_document_final_comment() {
    // #418 as reported. Before the fix the comment came first and `c`
    // was written under it.
    assert_eq!(
        insert("a:\n  b:\n    n: 1\n# trailing\n", "a", "c"),
        "a:\n  b:\n    n: 1\n  c: \"2\"\n# trailing\n"
    );
}

#[test]
fn the_comment_does_not_become_the_new_keys_head_comment() {
    // The byte assertion above says where the comment sits. This says
    // what that means to a reader of the comment API, which is how the
    // damage shows up in a tool built on it.
    let mut doc = parse_document("a:\n  b:\n    n: 1\n# trailing\n").expect("source must parse");
    doc.insert_entry_value("a", "c", &Value::from("2"))
        .expect("insert must succeed");
    assert!(
        doc.comments_at("a.c").before.is_empty(),
        "the new key must not inherit the document's closing comment"
    );
}

#[test]
fn a_key_follows_the_comment_unchanged() {
    // The comment is not at the end of the document, so nothing about
    // "document-final" is load-bearing: it is any comment outside the
    // anchor entry.
    assert_eq!(
        insert("a:\n  b:\n    n: 1\n# trailing\nz: 9\n", "a", "c"),
        "a:\n  b:\n    n: 1\n  c: \"2\"\n# trailing\nz: 9\n"
    );
}

// --- Shape of the anchor entry ------------------------------------

#[test]
fn a_deeper_nest_below_the_anchor_is_still_one_entry() {
    assert_eq!(
        insert("a:\n  b:\n    c:\n      n: 1\n# trailing\n", "a", "d"),
        "a:\n  b:\n    c:\n      n: 1\n  d: \"2\"\n# trailing\n"
    );
}

#[test]
fn a_nested_sequence_anchor_behaves_the_same() {
    // A control, as it turns out: this one was already right before the
    // fix. It is here because a reader would expect the sequence and
    // mapping anchors to agree, and now there is something saying they
    // do.
    assert_eq!(
        insert("a:\n  b:\n    - 1\n# trailing\n", "a", "c"),
        "a:\n  b:\n    - 1\n  c: \"2\"\n# trailing\n"
    );
}

// --- Shape of the trivia ------------------------------------------

#[test]
fn a_run_of_comment_lines_moves_together() {
    assert_eq!(
        insert("a:\n  b:\n    n: 1\n# one\n# two\n", "a", "c"),
        "a:\n  b:\n    n: 1\n  c: \"2\"\n# one\n# two\n"
    );
}

#[test]
fn a_blank_line_detaching_the_comment_keeps_both() {
    // The blank line is not the anchor's either, so the new key goes
    // above it and the detached comment keeps its distance.
    assert_eq!(
        insert("a:\n  b:\n    n: 1\n\n# trailing\n", "a", "c"),
        "a:\n  b:\n    n: 1\n  c: \"2\"\n\n# trailing\n"
    );
}

#[test]
fn crlf_line_endings_are_preserved() {
    assert_eq!(
        insert("a:\r\n  b:\r\n    n: 1\r\n# trailing\r\n", "a", "c"),
        "a:\r\n  b:\r\n    n: 1\r\n  c: \"2\"\r\n# trailing\r\n"
    );
}

// --- Where indentation puts the boundary --------------------------

#[test]
fn a_comment_indented_inside_the_block_keeps_the_new_key_below_it() {
    // `# inner` is deeper than `b`, so it is part of `b`'s block and
    // the sibling belongs after it. This is the answer #288 gave, and
    // it is the right one here; the fix must not take it away.
    assert_eq!(
        insert("a:\n  b:\n    n: 1\n    # inner\n", "a", "c"),
        "a:\n  b:\n    n: 1\n    # inner\n  c: \"2\"\n"
    );
}

#[test]
fn a_comment_at_the_anchors_own_column_is_not_inside_it() {
    // A comment aligned with the entries could be read either way in
    // principle. The report settles it: it names this shape as part of
    // the defect and expects the new key above. That is also what the
    // anchor did before v0.0.25, and it keeps the comment adjacent to
    // whatever follows it.
    assert_eq!(
        insert("a:\n  b:\n    n: 1\n  # sibling\n", "a", "c"),
        "a:\n  b:\n    n: 1\n  c: \"2\"\n  # sibling\n"
    );
}

// --- The root mapping ---------------------------------------------

#[test]
fn a_root_key_goes_above_a_document_final_comment() {
    // The root's own column is 0 and so is the comment's, so this case
    // rides on the same rule as the one above.
    assert_eq!(
        insert("a:\n  b:\n    n: 1\n# trailing\n", "", "c"),
        "a:\n  b:\n    n: 1\nc: \"2\"\n# trailing\n"
    );
}

#[test]
fn a_flat_root_mapping_was_already_right_and_stays_right() {
    // A scalar anchor sweeps up nothing, so this case never regressed.
    // It pins that the fix leaves it alone.
    assert_eq!(
        insert("a: 1\n# trailing\n", "", "c"),
        "a: 1\nc: \"2\"\n# trailing\n"
    );
}

// --- Controls -----------------------------------------------------

#[test]
fn a_scalar_anchor_is_unaffected() {
    assert_eq!(
        insert("a:\n  b: 1\n# trailing\n", "a", "c"),
        "a:\n  b: 1\n  c: \"2\"\n# trailing\n"
    );
}

#[test]
fn a_document_with_no_comment_is_unaffected() {
    assert_eq!(
        insert("a:\n  b:\n    n: 1\n", "a", "c"),
        "a:\n  b:\n    n: 1\n  c: \"2\"\n"
    );
}

#[test]
fn a_flow_mapping_does_not_use_this_anchor() {
    assert_eq!(insert("a: {}\n", "a", "c"), "a: {c: \"2\"}\n");
}

// --- The fragment tier reaches the same anchor --------------------

#[test]
fn insert_entry_lands_above_the_comment_too() {
    assert_eq!(
        insert_fragment("a:\n  b:\n    n: 1\n# trailing\n", "a", "c"),
        "a:\n  b:\n    n: 1\n  c: 2\n# trailing\n"
    );
}

#[test]
fn insert_entry_at_the_root_lands_above_the_comment_too() {
    assert_eq!(
        insert_fragment("a:\n  b:\n    n: 1\n# trailing\n", "", "c"),
        "a:\n  b:\n    n: 1\nc: 2\n# trailing\n"
    );
}

#[test]
fn insert_entry_keeps_a_comment_inside_the_block() {
    assert_eq!(
        insert_fragment("a:\n  b:\n    n: 1\n    # inner\n", "a", "c"),
        "a:\n  b:\n    n: 1\n    # inner\n  c: 2\n"
    );
}

// --- A keep-chomped block scalar's blank lines are content --------

// `|+` and `>+` keep the blank lines at the end of a block scalar, so
// those lines are the value rather than trivia below it — which is why
// `trim_value_span` hands such a span back untrimmed. An anchor walk
// that skipped them as blank would splice into the middle of the scalar
// and shorten a value the caller never named: worse than #418, because
// it changes a value rather than where a comment sits.
//
// Each of these asserts the value as well as the bytes. The byte
// assertion alone cannot see the damage — the lines are still there,
// just on the wrong side of the new key.

/// The scalar at `path` in the re-parsed document.
fn scalar_at(source: &str, path: &str) -> String {
    let v: Value = noyalib::from_str(source).expect("document must re-parse");
    v[path].as_str().expect("a string").to_owned()
}

#[test]
fn a_keep_chomped_literal_keeps_its_trailing_blank() {
    let out = insert_fragment("a: |+\n  x\n\n", "", "c");
    assert_eq!(out, "a: |+\n  x\n\nc: 2\n");
    assert_eq!(
        scalar_at(&out, "a"),
        "x\n\n",
        "the kept blank line is content"
    );
}

#[test]
fn a_keep_chomped_folded_scalar_behaves_the_same() {
    let out = insert_fragment("a: >+\n  x\n\n", "", "c");
    assert_eq!(out, "a: >+\n  x\n\nc: 2\n");
    assert_eq!(scalar_at(&out, "a"), "x\n\n");
}

#[test]
fn a_keep_chomped_scalar_keeps_several_trailing_blanks() {
    let out = insert_fragment("a: |+\n  x\n\n\n", "", "c");
    assert_eq!(out, "a: |+\n  x\n\n\nc: 2\n");
    assert_eq!(scalar_at(&out, "a"), "x\n\n\n");
}

#[test]
fn a_keep_chomped_scalar_below_an_anchor_property_is_found() {
    // `is_keep_chomped_block_scalar` skips `&anchor` / `!tag` before
    // looking for the indicator, and the span walk has to reach it the
    // same way or the property hides the `|+`.
    let out = insert_fragment("a: &an |+\n  x\n\n", "", "c");
    assert_eq!(out, "a: &an |+\n  x\n\nc: 2\n");
    assert_eq!(scalar_at(&out, "a"), "x\n\n");
}

#[test]
fn a_comment_after_a_keep_chomped_scalar_still_stays_last() {
    // Both rules at once: the blank belongs to the scalar, the comment
    // does not belong to the entry. The new key goes between them.
    let out = insert_fragment("a: |+\n  x\n\n# trailing\n", "", "c");
    assert_eq!(out, "a: |+\n  x\n\nc: 2\n# trailing\n");
    assert_eq!(scalar_at(&out, "a"), "x\n\n");
}

#[test]
fn a_comment_indented_into_a_keep_chomped_scalar_is_its_content() {
    // `# in` is indented to the scalar's own column, so YAML reads it as
    // literal text, not as a comment. It must travel with the value.
    let out = insert_fragment("a: |+\n  x\n  # in\n\n", "", "c");
    assert_eq!(out, "a: |+\n  x\n  # in\n\nc: 2\n");
    assert_eq!(scalar_at(&out, "a"), "x\n# in\n\n");
}

#[test]
fn a_nested_keep_chomped_scalar_is_found_through_its_parent() {
    // The anchor here is `m`, whose value is a mapping; the keep-chomped
    // scalar is one level down. The blank still belongs to it, so the
    // question has to be asked of the whole span rather than of the
    // anchor's own value.
    let out = insert_fragment("m:\n  a: |+\n    x\n\n", "m", "c");
    assert_eq!(out, "m:\n  a: |+\n    x\n\n  c: 2\n");
    let v: Value = noyalib::from_str(&out).expect("document must re-parse");
    assert_eq!(v["m"]["a"].as_str().expect("a string"), "x\n\n");
}

#[test]
fn the_other_chomping_modes_are_unaffected() {
    // Clip (`|`) and strip (`|-`) own no trailing blank, so nothing
    // about them changes and the new key follows the content directly.
    assert_eq!(insert_fragment("a: |\n  x\n", "", "c"), "a: |\n  x\nc: 2\n");
    assert_eq!(
        insert_fragment("a: |-\n  x\n", "", "c"),
        "a: |-\n  x\nc: 2\n"
    );
    assert_eq!(
        insert_fragment("a: |\n  x\n# trailing\n", "", "c"),
        "a: |\n  x\nc: 2\n# trailing\n"
    );
}

// --- set_path reaches the same anchor -----------------------------

#[test]
fn set_path_lands_above_the_comment_too() {
    assert_eq!(
        set_path("a:\n  b:\n    n: 1\n# trailing\n", "a.c"),
        "a:\n  b:\n    n: 1\n  c: \"2\"\n# trailing\n"
    );
}

#[test]
fn set_path_at_the_root_lands_above_the_comment_too() {
    // The root shape from the report's follow-up: the root mapping's last
    // entry is `a`, whose value is a nested block, so the root has the
    // defect for the same reason a nested mapping does.
    assert_eq!(
        set_path("a:\n  b: 1\n# trailing\n", "z"),
        "a:\n  b: 1\nz: \"2\"\n# trailing\n"
    );
}

#[test]
fn set_path_creating_several_levels_lands_above_the_comment() {
    // The whole created chain goes in one splice at the anchor, so it
    // moves as a unit rather than straddling the comment.
    assert_eq!(
        set_path("a:\n  b:\n    n: 1\n# trailing\n", "a.x.y"),
        "a:\n  b:\n    n: 1\n  x:\n    y: \"2\"\n# trailing\n"
    );
}
