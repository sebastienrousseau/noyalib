// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `Document::remove` on flow members and sole entries — issue #221,
//! sub-ask 4.
//!
//! Both classes used to refuse. The refusals were correct at the time:
//! v0.0.21 turned them from *silent data loss* into errors, because a
//! flow member shares its line with its siblings and its parent, so
//! "delete the line" deleted the parent too. This suite pins the
//! completed behaviour — the edits now succeed, and succeed narrowly.
//!
//! Every case asserts the byte output rather than just the typed value:
//! the point of a lossless CST is that everything not addressed by the
//! path is returned unchanged, and only a byte comparison shows that.

use noyalib::Value;
use noyalib::cst::parse_document;

fn remove_ok(src: &str, path: &str, expected: &str) {
    let mut doc = parse_document(src).unwrap();
    doc.remove(path)
        .unwrap_or_else(|e| panic!("remove({path:?}) on {src:?} failed: {e}"));
    assert_eq!(doc.to_string(), expected, "source after remove({path:?})");
    // The result must re-parse; a lossless edit that produces something
    // only *this* parser accepts is not a fix.
    let _reparsed = parse_document(&doc.to_string()).expect("result re-parses");
}

// ── Flow mappings ───────────────────────────────────────────────────

#[test]
fn flow_mapping_first_member_takes_its_trailing_separator() {
    remove_ok("a: {x: 1, y: 2}\n", "a.x", "a: {y: 2}\n");
}

#[test]
fn flow_mapping_last_member_takes_its_leading_separator() {
    // The comma sits *before* the last member, so the backward branch
    // has to fire or the result is `{x: 1, }`.
    remove_ok("a: {x: 1, y: 2}\n", "a.y", "a: {x: 1}\n");
}

#[test]
fn flow_mapping_middle_member() {
    remove_ok("a: {x: 1, y: 2, z: 3}\n", "a.y", "a: {x: 1, z: 3}\n");
}

#[test]
fn flow_mapping_sibling_entries_are_untouched() {
    // The v0.0.21 data-loss case: `a.x` shares its line with `a` itself,
    // so the old line-splice removed `a` entirely, and for a one-entry
    // document removed the document.
    remove_ok(
        "keep: 0\na: {x: 1, y: 2}\nafter: 9\n",
        "a.x",
        "keep: 0\na: {y: 2}\nafter: 9\n",
    );
}

// ── Flow sequences ──────────────────────────────────────────────────

#[test]
fn flow_sequence_by_index() {
    remove_ok("a: [1, 2, 3]\n", "a[1]", "a: [1, 3]\n");
}

#[test]
fn flow_sequence_first_and_last() {
    remove_ok("a: [1, 2, 3]\n", "a[0]", "a: [2, 3]\n");
    remove_ok("a: [1, 2, 3]\n", "a[2]", "a: [1, 2]\n");
}

#[test]
fn flow_sequence_of_strings_keeps_quoting() {
    remove_ok(
        r#"a: ["one", "two", "three"]
"#,
        "a[1]",
        r#"a: ["one", "three"]
"#,
    );
}

// ── Sole entries ────────────────────────────────────────────────────
//
// Deleting the bytes of the last entry leaves `a:` behind, which
// re-parses as null. That is a type change, not a removal, so the
// collection is written out explicitly instead.

#[test]
fn sole_block_mapping_entry_becomes_an_empty_mapping() {
    remove_ok("a:\n  x: 1\n", "a.x", "a:\n  {}\n");
}

#[test]
fn sole_block_sequence_item_becomes_an_empty_sequence() {
    remove_ok("a:\n  - 1\n", "a[0]", "a:\n  []\n");
}

#[test]
fn sole_flow_members_empty_in_place() {
    remove_ok("a: {x: 1}\n", "a.x", "a: {}\n");
    remove_ok("a: [1]\n", "a[0]", "a: []\n");
}

#[test]
fn sole_root_entry_leaves_an_empty_document_mapping() {
    remove_ok("only: 1\n", "only", "{}\n");
    remove_ok("- 1\n", "[0]", "[]\n");
}

#[test]
fn emptied_collection_is_a_collection_not_null() {
    // The whole reason `{}` is written rather than nothing.
    let mut doc = parse_document("a:\n  x: 1\n").unwrap();
    doc.remove("a.x").unwrap();
    let v = doc.as_value();
    let Value::Mapping(root) = &*v else {
        panic!("root is not a mapping")
    };
    match root.get("a") {
        Some(Value::Mapping(inner)) => assert!(inner.is_empty(), "inner should be empty"),
        other => panic!("expected an empty mapping at `a`, got {other:?}"),
    }
}

// ── The trailing newline ────────────────────────────────────────────

#[test]
fn emptying_a_collection_keeps_the_final_newline() {
    // A collection's span can reach the end of its last line. Replacing
    // that range wholesale would eat the document's final newline —
    // valid YAML, but a whole-file diff and a CI end-of-file failure.
    for (src, path, expected) in [
        ("only: 1\n", "only", "{}\n"),
        ("a:\n  x: 1\n", "a.x", "a:\n  {}\n"),
        ("- 1\n", "[0]", "[]\n"),
    ] {
        let mut doc = parse_document(src).unwrap();
        doc.remove(path).unwrap();
        assert_eq!(doc.to_string(), expected);
        assert!(
            doc.to_string().ends_with('\n'),
            "{src:?} lost its trailing newline"
        );
    }
}

#[test]
fn a_document_without_a_trailing_newline_does_not_gain_one() {
    let mut doc = parse_document("only: 1").unwrap();
    doc.remove("only").unwrap();
    assert_eq!(doc.to_string(), "{}");
}

// ── Controls: what must still refuse or still work ──────────────────

#[test]
fn block_removal_is_unchanged() {
    remove_ok("a: 1\nb: 2\n", "a", "b: 2\n");
    remove_ok("a:\n  x: 1\n  y: 2\n", "a.x", "a:\n  y: 2\n");
    remove_ok("a:\n  - 1\n  - 2\n", "a[0]", "a:\n  - 2\n");
}

#[test]
fn missing_paths_still_error_and_leave_the_document_alone() {
    for (src, path) in [
        ("a: {x: 1, y: 2}\n", "a.nope"),
        ("a: [1, 2]\n", "a[9]"),
        ("a: 1\n", ""),
    ] {
        let mut doc = parse_document(src).unwrap();
        let before = doc.to_string();
        assert!(doc.remove(path).is_err(), "{path:?} should not resolve");
        assert_eq!(doc.to_string(), before, "document must survive a refusal");
    }
}

#[test]
fn comments_outside_the_flow_collection_survive() {
    remove_ok(
        "# header\na: {x: 1, y: 2}  # trailing\nb: 2\n",
        "a.x",
        "# header\na: {y: 2}  # trailing\nb: 2\n",
    );
}

// ── Head comments on a sole entry (#280) ────────────────────────────
//
// `Removal::Line` owned the entry's head-comment run via
// `owned_entry_range`; `Removal::SoleEntry` replaced the *collection's*
// span, which begins at the first entry's content — below the comment.
// So the same comment on the same entry was taken when the entry had a
// sibling and stranded when it did not, left describing an empty
// collection. Invisible to the typed oracle, which cannot see comments.

#[test]
fn sole_entry_removal_takes_its_head_comment() {
    remove_ok(
        "a:\n  # documents x\n  x: 1\nb: 2\n",
        "a.x",
        "a:\n  {}\nb: 2\n",
    );
}

#[test]
fn sole_entry_removal_takes_the_whole_head_comment_run() {
    remove_ok(
        "a:\n  # one\n  # two\n  x: 1\nb: 2\n",
        "a.x",
        "a:\n  {}\nb: 2\n",
    );
}

#[test]
fn sole_sequence_item_takes_its_head_comment() {
    remove_ok("xs:\n  # about one\n  - one\n", "xs[0]", "xs:\n  []\n");
}

#[test]
fn sole_root_entry_takes_its_head_comment() {
    remove_ok("# doc for only\nonly: 1\n", "only", "{}\n");
}

#[test]
fn the_same_comment_is_treated_the_same_with_and_without_a_sibling() {
    // The heart of #280: these two differ only in whether `a` has a
    // second entry, so they must agree about who owns `# documents x`.
    remove_ok(
        "a:\n  # documents x\n  x: 1\n  y: 2\n",
        "a.x",
        "a:\n  y: 2\n",
    );
    remove_ok(
        "a:\n  # documents x\n  x: 1\nb: 2\n",
        "a.x",
        "a:\n  {}\nb: 2\n",
    );
}

#[test]
fn a_detached_comment_is_not_the_entrys_and_stays() {
    // A blank line severs ownership — `absorb_head_comments` stops there,
    // and the sole-entry path must inherit that, not widen it.
    remove_ok(
        "a:\n  # detached\n\n  x: 1\n",
        "a.x",
        "a:\n  # detached\n\n  {}\n",
    );
}

#[test]
fn an_inline_comment_was_already_correct_and_stays_correct() {
    // Inside the collection span, so it was never stranded.
    remove_ok("a:\n  x: 1  # trailing\n", "a.x", "a:\n  {}\n");
}

#[test]
fn indentation_survives_absorbing_the_comment_run() {
    // The splice now starts above the entry, so the entry's own leading
    // whitespace is inside the replaced range and must be written back.
    // Without that, `a:` loses its value entirely.
    let mut doc = parse_document("a:\n    # deep\n    x: 1\nb: 2\n").unwrap();
    doc.remove("a.x").unwrap();
    assert_eq!(doc.to_string(), "a:\n    {}\nb: 2\n");
    let v = doc.as_value();
    let Value::Mapping(root) = &*v else {
        panic!("not a mapping")
    };
    assert!(matches!(root.get("a"), Some(Value::Mapping(m)) if m.is_empty()));
}
