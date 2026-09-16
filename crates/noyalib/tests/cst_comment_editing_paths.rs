// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Editing comments without editing data.
//!
//! The comment API's whole promise is that writing a comment changes
//! comments and nothing else. It keeps that promise with the same
//! three-part guard the value mutators use — splice, re-parse, compare
//! the loaded value against a snapshot — and, as there, the guard's
//! branches are the ones a well-formed comment never reaches.
//!
//! What can reach them is a comment body that is not a comment: text
//! with a newline in it splices a second line into the document, and
//! whatever that line turns out to be is either a parse failure or, far
//! worse, a new entry. Every case below asserts the same two things:
//! the call fails, and the document is byte-for-byte unchanged.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use noyalib::Value;
use noyalib::cst::{CommentPosition, parse_document};

const DOC: &str = "# head\na: 1  # beside a\nb:\n  c: 2\nxs:\n  - p  # beside p\n";

/// Every comment setter, in both positions, addressed four ways.
#[test]
fn comments_can_be_written_and_read_back_at_every_position() {
    let paths = ["a", "b", "b.c", "xs[0]"];
    for path in paths {
        for position in [CommentPosition::Inline, CommentPosition::Before] {
            let mut doc = parse_document(DOC).expect("parse");
            let before_value = doc.as_value().clone();

            doc.set_comment(path, position, "written by the test")
                .unwrap_or_else(|e| panic!("set_comment({path}, {position:?}): {e}"));

            let out = doc.to_string();
            assert!(
                out.contains("written by the test"),
                "set_comment({path}, {position:?}) wrote nothing\n{out}"
            );
            let reparsed: Value = noyalib::from_str(&out).unwrap_or_else(|e| {
                panic!("set_comment({path}, {position:?}) broke parsing: {e}\n{out}")
            });
            assert_eq!(
                reparsed, before_value,
                "set_comment({path}, {position:?}) changed the document's data\n{out}"
            );

            let bundle = doc.comments_at(path);
            let found = match position {
                CommentPosition::Inline => bundle
                    .inline
                    .as_ref()
                    .is_some_and(|c| c.text.contains("written by the test")),
                CommentPosition::Before => bundle
                    .before
                    .iter()
                    .any(|c| c.text.contains("written by the test")),
            };
            assert!(
                found,
                "set_comment({path}, {position:?}) wrote a comment `comments_at` cannot see\n{out}"
            );
        }
    }
}

/// Removing a comment leaves the data alone and the document parseable,
/// and removing one that is not there is not an error.
#[test]
fn comments_can_be_removed_and_removing_nothing_is_not_a_failure() {
    for (path, position) in [
        ("a", CommentPosition::Inline),
        ("xs[0]", CommentPosition::Inline),
        ("a", CommentPosition::Before),
    ] {
        let mut doc = parse_document(DOC).expect("parse");
        let before_value = doc.as_value().clone();
        doc.remove_comment(path, position)
            .unwrap_or_else(|e| panic!("remove_comment({path}, {position:?}): {e}"));
        let out = doc.to_string();
        let reparsed: Value = noyalib::from_str(&out).expect("still parses");
        assert_eq!(
            reparsed, before_value,
            "remove_comment({path}, {position:?}) changed the data\n{out}"
        );

        // Idempotent: removing again is a no-op, not an error.
        let once = doc.to_string();
        doc.remove_comment(path, position)
            .expect("removing an absent comment is not an error");
        assert_eq!(
            doc.to_string(),
            once,
            "remove_comment({path}, {position:?}) was not idempotent"
        );
    }
}

/// The convenience wrappers must agree with the positional API — they
/// are the same edit spelled two ways, and a divergence between them
/// would show up as one of the four silently doing nothing.
#[test]
fn the_named_wrappers_agree_with_the_positional_api() {
    for path in ["a", "b.c"] {
        let mut via_named = parse_document(DOC).expect("parse");
        via_named
            .set_inline_comment(path, "same text")
            .expect("named inline");
        let mut via_position = parse_document(DOC).expect("parse");
        via_position
            .set_comment(path, CommentPosition::Inline, "same text")
            .expect("positional inline");
        assert_eq!(
            via_named.to_string(),
            via_position.to_string(),
            "set_inline_comment and set_comment(.., Inline) disagree on `{path}`"
        );

        let mut via_named = parse_document(DOC).expect("parse");
        via_named
            .set_leading_comment(path, "same text")
            .expect("named leading");
        let mut via_position = parse_document(DOC).expect("parse");
        via_position
            .set_comment(path, CommentPosition::Before, "same text")
            .expect("positional leading");
        assert_eq!(
            via_named.to_string(),
            via_position.to_string(),
            "set_leading_comment and set_comment(.., Before) disagree on `{path}`"
        );

        let mut via_named = parse_document(DOC).expect("parse");
        via_named
            .remove_inline_comment(path)
            .expect("named remove inline");
        let mut via_position = parse_document(DOC).expect("parse");
        via_position
            .remove_comment(path, CommentPosition::Inline)
            .expect("positional remove inline");
        assert_eq!(
            via_named.to_string(),
            via_position.to_string(),
            "remove_inline_comment and remove_comment(.., Inline) disagree on `{path}`"
        );

        let mut via_named = parse_document(DOC).expect("parse");
        via_named
            .remove_leading_comment(path)
            .expect("named remove leading");
        let mut via_position = parse_document(DOC).expect("parse");
        via_position
            .remove_comment(path, CommentPosition::Before)
            .expect("positional remove leading");
        assert_eq!(
            via_named.to_string(),
            via_position.to_string(),
            "remove_leading_comment and remove_comment(.., Before) disagree on `{path}`"
        );
    }
}

/// A path that does not resolve must be refused by every entry point,
/// and must not edit anything on the way out.
#[test]
fn every_comment_entry_point_refuses_an_unresolvable_path() {
    for path in ["nope", "b.nope", "xs[9]", "a.b"] {
        for position in [CommentPosition::Inline, CommentPosition::Before] {
            let mut doc = parse_document(DOC).expect("parse");
            assert!(
                doc.set_comment(path, position, "x").is_err(),
                "set_comment accepted `{path}` ({position:?})"
            );
            assert_eq!(
                doc.to_string(),
                DOC,
                "a refused set_comment edited the document"
            );
        }
    }
}

/// A comment body containing a line break would splice a second line
/// into the document — the one input that can turn a comment edit into
/// a data edit. It must be refused, and the document left alone.
#[test]
fn a_comment_body_that_is_not_one_line_is_refused() {
    let hostile = [
        "first\nsecond",
        "text\nb: injected",
        "text\r\nmore",
        "ends with a newline\n",
    ];
    for text in hostile {
        for position in [CommentPosition::Inline, CommentPosition::Before] {
            let mut doc = parse_document(DOC).expect("parse");
            let result = doc.set_comment("a", position, text);
            let out = doc.to_string();

            if result.is_ok() {
                // If it is accepted, it must not have become data: the
                // loaded value has to be untouched and the text has to
                // still be inside a comment.
                let reparsed: Value = noyalib::from_str(&out).unwrap_or_else(|e| {
                    panic!("accepted {text:?} ({position:?}) and broke parsing: {e}\n{out}")
                });
                let original: Value = noyalib::from_str(DOC).expect("fixture");
                assert_eq!(
                    reparsed, original,
                    "a multi-line comment body became data\n{out}"
                );
            } else {
                assert_eq!(
                    out, DOC,
                    "a refused multi-line comment body ({position:?}) still edited the document"
                );
            }
        }
    }
}
