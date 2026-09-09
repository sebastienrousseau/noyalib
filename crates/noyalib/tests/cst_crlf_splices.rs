//! CRLF-aware splices: an edit that adds a line writes the break the
//! document already uses.
//!
//! The mutators derive a splice's indentation from the site rather than
//! assuming it. The line terminator was the exception — hard-coded `\n`
//! — so every insertion into a CRLF document left a bare LF behind, and
//! the inline-comment path landed between the `\r` and the `\n` and
//! stranded a lone `\r`. No data was lost (values round-trip, and YAML
//! 1.2 accepts a lone `\r` as a break), but a CRLF file came back with
//! two or three terminators in it.
//!
//! A document that already mixes terminators is left alone: there is no
//! convention to honour, and picking one would rewrite bytes the caller
//! did not ask about.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::cst::{CommentPosition, parse_document};
use noyalib::{Mapping, Value};

// ── Typed insertion (the `Emit` tier) ───────────────────────────────

#[test]
fn insert_entry_value_uses_the_documents_break() {
    let mut doc = parse_document("m:\r\n  a: 1\r\n").expect("parse");
    doc.insert_entry_value("m", "b", &2i64).expect("insert");
    assert_eq!(doc.source(), "m:\r\n  a: 1\r\n  b: 2\r\n");
}

#[test]
fn push_back_value_uses_the_documents_break() {
    let mut doc = parse_document("s:\r\n  - 1\r\n").expect("parse");
    doc.push_back_value("s", &3i64).expect("push");
    assert_eq!(doc.source(), "s:\r\n  - 1\r\n  - 3\r\n");
}

#[test]
fn insert_after_value_uses_the_documents_break() {
    let mut doc = parse_document("s:\r\n  - 1\r\n").expect("parse");
    doc.insert_after_value("s[0]", &9i64).expect("insert");
    assert_eq!(doc.source(), "s:\r\n  - 1\r\n  - 9\r\n");
}

// ── Fragment insertion ──────────────────────────────────────────────

#[test]
fn insert_entry_uses_the_documents_break() {
    let mut doc = parse_document("m:\r\n  a: 1\r\n").expect("parse");
    doc.insert_entry("m", "b", "2").expect("insert");
    assert_eq!(doc.source(), "m:\r\n  a: 1\r\n  b: 2\r\n");
}

#[test]
fn push_back_uses_the_documents_break() {
    let mut doc = parse_document("s:\r\n  - 1\r\n").expect("parse");
    doc.push_back("s", "3").expect("push");
    assert_eq!(doc.source(), "s:\r\n  - 1\r\n  - 3\r\n");
}

#[test]
fn insert_after_uses_the_documents_break() {
    let mut doc = parse_document("s:\r\n  - 1\r\n").expect("parse");
    doc.insert_after("s[0]", "9").expect("insert");
    assert_eq!(doc.source(), "s:\r\n  - 1\r\n  - 9\r\n");
}

// ── Multi-line values: every added break, not just the last ─────────

#[test]
fn a_block_scalar_insert_is_crlf_on_every_line() {
    let mut doc = parse_document("m:\r\n  a: 1\r\n").expect("parse");
    doc.insert_entry_value("m", "b", "x\ny").expect("insert");
    assert_eq!(
        doc.source(),
        "m:\r\n  a: 1\r\n  b: |-\r\n    x\r\n    y\r\n"
    );

    // The spelling changed; the value did not.
    let reparsed = parse_document(doc.source()).expect("reparse");
    let Value::Mapping(root) = reparsed.as_value().clone() else {
        panic!("expected a mapping")
    };
    let Some(Value::Mapping(m)) = root.get("m").cloned() else {
        panic!("expected m to be a mapping")
    };
    assert_eq!(m.get("b").cloned(), Some(Value::String("x\ny".to_owned())));
}

#[test]
fn a_nested_collection_insert_is_crlf_on_every_line() {
    let mut doc = parse_document("m:\r\n  a: 1\r\n").expect("parse");
    let mut inner = Mapping::new();
    let _ = inner.insert("k", Value::from(1i64));
    doc.insert_entry_value("m", "b", &Value::Mapping(inner))
        .expect("insert");
    assert_eq!(doc.source(), "m:\r\n  a: 1\r\n  b:\r\n    k: 1\r\n");
}

// ── Comments ────────────────────────────────────────────────────────

#[test]
fn an_inline_comment_lands_before_the_whole_break() {
    // Regression: this used to splice between the `\r` and the `\n`,
    // giving `  a: 1\r  # note\n` — a stranded lone CR.
    let mut doc = parse_document("m:\r\n  a: 1\r\n").expect("parse");
    doc.set_comment("m.a", CommentPosition::Inline, "note")
        .expect("set");
    assert_eq!(doc.source(), "m:\r\n  a: 1  # note\r\n");
}

#[test]
fn set_inline_comment_and_set_comment_agree_on_crlf() {
    let src = "m:\r\n  a: 1\r\n";
    let mut old_api = parse_document(src).expect("parse");
    old_api.set_inline_comment("m.a", "note").expect("set");
    let mut new_api = parse_document(src).expect("parse");
    new_api
        .set_comment("m.a", CommentPosition::Inline, "note")
        .expect("set");
    assert_eq!(old_api.source(), new_api.source());
}

#[test]
fn a_leading_comment_uses_the_documents_break() {
    let mut doc = parse_document("m:\r\n  a: 1\r\n").expect("parse");
    doc.set_comment("m.a", CommentPosition::Before, "note")
        .expect("set");
    assert_eq!(doc.source(), "m:\r\n  # note\r\n  a: 1\r\n");
}

#[test]
fn set_leading_comment_uses_the_documents_break() {
    let mut doc = parse_document("m:\r\n  a: 1\r\n").expect("parse");
    doc.set_leading_comment("m.a", "one\ntwo").expect("set");
    assert_eq!(doc.source(), "m:\r\n  # one\r\n  # two\r\n  a: 1\r\n");
}

// ── Controls: unaffected paths and non-CRLF documents ───────────────

#[test]
fn an_lf_document_is_unchanged_by_this() {
    let mut doc = parse_document("m:\n  a: 1\n").expect("parse");
    doc.insert_entry_value("m", "b", &2i64).expect("insert");
    assert_eq!(doc.source(), "m:\n  a: 1\n  b: 2\n");
}

#[test]
fn a_mixed_ending_document_keeps_the_lf_default() {
    // No single convention to honour, so the inserted line is not
    // "corrected" to a guess — that would rewrite bytes unasked.
    let mut doc = parse_document("m:\r\n  a: 1\n").expect("parse");
    doc.insert_entry_value("m", "b", &2i64).expect("insert");
    assert_eq!(doc.source(), "m:\r\n  a: 1\n  b: 2\n");
}

#[test]
fn an_unterminated_last_line_still_reads_the_documents_convention() {
    // The final line has no terminator, but the document's one break is
    // CRLF — so both the break this splice supplies for itself and the
    // one it ends with follow that.
    let mut doc = parse_document("m:\r\n  a: 1").expect("parse");
    doc.insert_entry_value("m", "b", &2i64).expect("insert");
    assert_eq!(doc.source(), "m:\r\n  a: 1\r\n  b: 2\r\n");
}

#[test]
fn a_document_with_no_break_at_all_gets_lf() {
    // Nothing here states a convention, so the default stands.
    let mut doc = parse_document("a: 1").expect("parse");
    doc.insert_entry_value("", "b", &2i64).expect("insert");
    assert_eq!(doc.source(), "a: 1\nb: 2\n");
}

#[test]
fn remove_was_never_affected() {
    // It adds no line; this pins that the fix did not disturb it.
    let mut doc = parse_document("m:\r\n  a: 1\r\n  b: 2\r\n").expect("parse");
    doc.remove("m.b").expect("remove");
    assert_eq!(doc.source(), "m:\r\n  a: 1\r\n");
}

#[test]
fn a_single_line_set_value_was_never_affected() {
    // A replacement that stays on one line grows no break, so it was
    // correct before the replacement arm learned the rule below.
    let mut doc = parse_document("m:\r\n  a: 1\r\n").expect("parse");
    doc.set_value("m.a", &Value::from(9i64)).expect("set");
    assert_eq!(doc.source(), "m:\r\n  a: 9\r\n");
}

// ── Replacement (`set_value`) ───────────────────────────────────────
//
// A replacement adds lines whenever the value written has more of them
// than the value replaced. That is the same event an insertion has, and
// it takes the same answer.

#[test]
fn a_multi_line_set_value_is_crlf_on_every_line() {
    let mut doc = parse_document("a: 1\r\nb: 2\r\n").expect("parse");
    doc.set_value("a", &Value::from("one\ntwo")).expect("set");
    assert_eq!(doc.source(), "a: |-\r\n  one\r\n  two\r\nb: 2\r\n");
    // The value itself is unchanged: the breaks re-spelled are the
    // document's, not the string's.
    assert_eq!(
        doc.as_value().get_path("a").and_then(Value::as_str),
        Some("one\ntwo")
    );
}

#[test]
fn a_hoisted_comment_keeps_the_documents_break() {
    // The comment is moved onto the block header (#333) *after* the
    // formatter returns, so a terminator threaded through the formatter
    // would miss this one.
    let mut doc = parse_document("title: Hello # note\r\n").expect("parse");
    doc.set_value("title", &Value::from("multi\nline"))
        .expect("set");
    assert_eq!(doc.source(), "title: |- # note\r\n  multi\r\n  line\r\n");
}

#[test]
fn a_single_quoted_multi_line_site_is_crlf_too() {
    // A line beginning with a space rules out a block literal, so the
    // fragment comes from the single-quoted formatter instead.
    let mut doc = parse_document("a: 'p'\r\nb: 2\r\n").expect("parse");
    doc.set_value("a", &Value::from("x\n y")).expect("set");
    assert_eq!(doc.source(), "a: 'x\r\n y'\r\nb: 2\r\n");
    assert!(!has_bare_lf(doc.source()), "{:?}", doc.source());
    // The value folds to `x y` on the way back, which is what YAML says
    // a line break inside a quoted scalar means. That is the site's own
    // semantics and predates this rule; what is pinned here is only that
    // the break the formatter wrote takes the document's spelling.
}

#[test]
fn a_carriage_return_in_the_value_is_escaped_not_doubled() {
    // The substitution is blind, which is safe only because a fragment
    // can never carry a raw `\r` (#335). This pins the premise.
    let mut doc = parse_document("a: 1\r\nb: 2\r\n").expect("parse");
    doc.set_value("a", &Value::from("x\ry")).expect("set");
    assert!(!doc.source().contains("\r\r"), "{:?}", doc.source());
    assert_eq!(
        doc.as_value().get_path("a").and_then(Value::as_str),
        Some("x\ry")
    );
}

#[test]
fn a_collection_replacement_that_grows_is_crlf_on_every_line() {
    let mut doc = parse_document("tags:\r\n  - a\r\nname: x\r\n").expect("parse");
    let value = Value::Sequence(vec![Value::from("a"), Value::from("b"), Value::from("c")]);
    doc.set_value("tags", &value).expect("set");
    assert!(!has_bare_lf(doc.source()), "{:?}", doc.source());
    assert_eq!(
        doc.source(),
        "tags:\r\n  - a\r\n  - b\r\n  - c\r\nname: x\r\n"
    );
}

#[test]
fn a_collection_replacement_of_the_same_line_count_is_still_clean() {
    let mut doc = parse_document("tags:\r\n  - a\r\nname: x\r\n").expect("parse");
    doc.set_value("tags", &Value::Sequence(vec![Value::from("z")]))
        .expect("set");
    assert_eq!(doc.source(), "tags:\r\n  - z\r\nname: x\r\n");
}

#[test]
fn a_flow_collection_replacement_is_still_one_line() {
    let mut doc = parse_document("tags: [a]\r\nname: x\r\n").expect("parse");
    doc.set_value(
        "tags",
        &Value::Sequence(vec![Value::from("a"), Value::from("b")]),
    )
    .expect("set");
    assert!(!has_bare_lf(doc.source()), "{:?}", doc.source());
}

#[test]
fn an_lf_document_is_unchanged_by_the_set_value_respelling() {
    let mut doc = parse_document("a: 1\nb: 2\n").expect("parse");
    doc.set_value("a", &Value::from("one\ntwo")).expect("set");
    assert_eq!(doc.source(), "a: |-\n  one\n  two\nb: 2\n");
}

#[test]
fn a_mixed_document_keeps_the_lf_default_for_set_value() {
    // Mixed input has no convention to honour, so the replacement takes
    // the same default every other splice takes there.
    let mut doc = parse_document("a: 1\r\nb: 2\n").expect("parse");
    doc.set_value("a", &Value::from("one\ntwo")).expect("set");
    // The `\r\n` after `two` is the document's own byte, outside the
    // replaced span; the two breaks the fragment grew are the default.
    assert_eq!(doc.source(), "a: |-\n  one\n  two\r\nb: 2\n");
}

/// A line feed with no carriage return before it.
fn has_bare_lf(source: &str) -> bool {
    let bytes = source.as_bytes();
    bytes
        .iter()
        .enumerate()
        .any(|(i, b)| *b == b'\n' && (i == 0 || bytes[i - 1] != b'\r'))
}
