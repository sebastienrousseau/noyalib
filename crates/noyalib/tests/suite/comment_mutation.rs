//! Comment mutation — the first sub-ask of #221.
//!
//! `comments_at` was read-only; editing a comment meant locating its
//! bytes by hand and losing the re-parse guard the other mutators give.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::cst::{CommentPosition, parse_document};

#[test]
fn sets_an_inline_comment_where_none_existed() {
    let mut doc = parse_document("port: 8080\n").expect("parse");
    doc.set_comment("port", CommentPosition::Inline, "listen port")
        .expect("set");
    assert_eq!(doc.source(), "port: 8080  # listen port\n");
}

#[test]
fn replaces_an_existing_inline_comment() {
    let mut doc = parse_document("port: 8080  # old\n").expect("parse");
    doc.set_comment("port", CommentPosition::Inline, "new")
        .expect("set");
    assert_eq!(doc.source(), "port: 8080  # new\n");
}

#[test]
fn removes_an_inline_comment_and_its_padding() {
    let mut doc = parse_document("port: 8080  # note\n").expect("parse");
    doc.remove_comment("port", CommentPosition::Inline)
        .expect("remove");
    assert_eq!(
        doc.source(),
        "port: 8080\n",
        "the spaces before the # must go too"
    );
}

#[test]
fn removing_an_absent_comment_is_a_no_op() {
    let mut doc = parse_document("port: 8080\n").expect("parse");
    doc.remove_comment("port", CommentPosition::Inline)
        .expect("remove");
    assert_eq!(doc.source(), "port: 8080\n");
}

#[test]
fn body_gets_exactly_one_leading_space() {
    let mut doc = parse_document("a: 1\n").expect("parse");
    doc.set_comment("a", CommentPosition::Inline, " already spaced")
        .expect("set");
    assert_eq!(doc.source(), "a: 1  # already spaced\n");
}

#[test]
fn writes_a_leading_comment_block() {
    let mut doc = parse_document("port: 8080\n").expect("parse");
    doc.set_comment("port", CommentPosition::Before, "why\nthis value")
        .expect("set");
    assert_eq!(doc.source(), "# why\n# this value\nport: 8080\n");
}

#[test]
fn replaces_an_existing_leading_block() {
    let src = "# one\n# two\nport: 8080\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set_comment("port", CommentPosition::Before, "only")
        .expect("set");
    assert_eq!(doc.source(), "# only\nport: 8080\n");
}

#[test]
fn removes_a_leading_block() {
    let src = "# one\n# two\nport: 8080\n";
    let mut doc = parse_document(src).expect("parse");
    doc.remove_comment("port", CommentPosition::Before)
        .expect("remove");
    assert_eq!(doc.source(), "port: 8080\n");
}

#[test]
fn leading_block_takes_the_nodes_indentation() {
    let src = "server:\n  port: 8080\n";
    let mut doc = parse_document(src).expect("parse");
    doc.set_comment("server.port", CommentPosition::Before, "nested")
        .expect("set");
    assert_eq!(
        doc.source(),
        "server:\n  # nested\n  port: 8080\n",
        "the comment must align with the node it decorates"
    );
}

#[test]
fn the_document_still_parses_to_the_same_value() {
    let mut doc = parse_document("port: 8080\nhost: x\n").expect("parse");
    let before: noyalib::Value = noyalib::from_str(doc.source()).expect("parse before");
    doc.set_comment("port", CommentPosition::Inline, "note")
        .expect("set");
    doc.set_comment("host", CommentPosition::Before, "the host")
        .expect("set");
    let after: noyalib::Value = noyalib::from_str(doc.source()).expect("parse after");
    assert_eq!(before, after, "comments must not change the value");
}

#[test]
fn round_trips_through_comments_at() {
    let mut doc = parse_document("port: 8080\n").expect("parse");
    doc.set_comment("port", CommentPosition::Inline, "readable")
        .expect("set");
    let bundle = doc.comments_at("port");
    assert_eq!(bundle.inline.expect("inline").text, " readable");
}

#[test]
fn unresolvable_path_is_an_error_not_a_silent_no_op() {
    let mut doc = parse_document("port: 8080\n").expect("parse");
    assert!(
        doc.set_comment("nope", CommentPosition::Inline, "x")
            .is_err()
    );
    assert!(doc.remove_comment("nope", CommentPosition::Inline).is_err());
}

// ── Value preservation ──────────────────────────────────────────────
//
// A comment edit must never change what the document means. That is not
// obvious to enforce case by case, because `#` is not always a comment:
// inside a block scalar it is content. The `fuzz_editors` target found
// this within a minute of existing, on a folded scalar.

/// Documents where a naive `  # ...` append would land inside content
/// rather than beside it.
const TRICKY: &[&str] = &[
    ">\n",
    "|\n",
    "a: >\n  folded\n",
    "a: |\n  literal\n",
    "a: |\n  has # inside\n",
    "a: >-\n  folded strip\n",
    "a: \"quoted # not comment\"\n",
    "a: 'single # not comment'\n",
];

#[test]
fn a_comment_edit_never_changes_the_value() {
    for src in TRICKY {
        for path in ["", "a"] {
            for position in [CommentPosition::Inline, CommentPosition::Before] {
                let Ok(before) = noyalib::from_str::<noyalib::Value>(src) else {
                    continue;
                };
                let mut doc = parse_document(src).expect("parse");

                // Either outcome is acceptable; changing the value is not.
                if doc.set_comment(path, position, "note").is_ok() {
                    let after =
                        noyalib::from_str::<noyalib::Value>(doc.source()).unwrap_or_else(|e| {
                            panic!("{src:?} {path:?} {position:?}: unparseable after edit: {e}")
                        });
                    assert_eq!(
                        after,
                        before,
                        "set_comment changed the value of {src:?} at {path:?} ({position:?})\n\
                         source became {:?}",
                        doc.source()
                    );
                }
            }
        }
    }
}

#[test]
fn a_refused_comment_edit_leaves_the_source_intact() {
    for src in TRICKY {
        for position in [CommentPosition::Inline, CommentPosition::Before] {
            let mut doc = parse_document(src).expect("parse");
            let before = doc.source().to_owned();
            if doc.set_comment("", position, "note").is_err() {
                assert_eq!(
                    doc.source(),
                    before,
                    "a refused comment edit modified {src:?}"
                );
            }
        }
    }
}
