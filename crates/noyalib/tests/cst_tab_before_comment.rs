// SPDX-License-Identifier: MIT OR Apache-2.0

//! A tab before a comment is separation, not indentation (#428).
//!
//! YAML 1.2.2 gives `l-comment ::= s-separate-in-line c-nb-comment-text?
//! b-comment` with `s-white ::= s-space | s-tab` (§6.2, §6.6). The
//! no-tabs rule is about *indentation* (§6.1), and a comment line has no
//! indentation to satisfy — so a tab in front of `#` is as legal as a
//! space.
//!
//! The bug was that the answer depended on the quote style of the line
//! above: the two paths leave the scanner at different `indent` values,
//! and the top-level escape in `reject_tab_indentation` is keyed on
//! that.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use noyalib::cst::parse_document;

/// The reported pair: identical documents but for the quote style.
#[test]
fn quote_style_does_not_decide_whether_a_tab_comment_parses() {
    let plain = parse_document("k: 1\n\t# t\n");
    let quoted = parse_document("k: \"1\"\n\t# t\n");
    assert_eq!(
        plain.is_ok(),
        quoted.is_ok(),
        "plain={:?} quoted={:?}",
        plain.map(|_| ()).map_err(|e| e.to_string()),
        quoted.map(|_| ()).map_err(|e| e.to_string()),
    );
    assert!(parse_document("k: \"1\"\n\t# t\n").is_ok());
}

#[test]
fn a_tab_before_a_comment_parses_after_every_flow_scalar_style() {
    for doc in ["k: 1\n\t# t\n", "k: \"1\"\n\t# t\n", "k: '1'\n\t# t\n"] {
        assert!(parse_document(doc).is_ok(), "should parse: {doc:?}");
    }
}

/// Block scalars still reject a tab-indented comment, and this records
/// that rather than hiding it.
///
/// It is a different check — `tab characters are not allowed as
/// block-scalar indentation`, raised in the block-scalar scanner, not by
/// `reject_tab_indentation` — with its own rationale about where a block
/// scalar ends. #428 reported the plain-versus-quoted split, and that is
/// what the fix addresses.
///
/// The same spec argument arguably reaches here too: the line is less
/// indented than the scalar, so the scalar ends and what follows is a
/// comment line whose leading tab is separation. But changing where a
/// block scalar terminates is a different and riskier change than making
/// three flow styles agree, so it is not bundled in. Tracked separately.
///
/// If that path is ever changed, this test fails and points at the
/// decision instead of letting the behaviour drift unnoticed.
#[test]
fn block_scalars_still_reject_a_tab_indented_comment() {
    for doc in ["k: |\n  x\n\t# t\n", "k: >\n  x\n\t# t\n"] {
        let err = parse_document(doc).expect_err("still rejected today");
        assert!(
            err.to_string().contains("block-scalar indentation"),
            "unexpected error for {doc:?}: {err}"
        );
    }
}

#[test]
fn the_comment_survives_a_round_trip() {
    let doc = parse_document("k: \"1\"\n\t# t\n").unwrap();
    assert_eq!(format!("{doc}"), "k: \"1\"\n\t# t\n");
}

/// The fix must not make tabs legal as actual indentation. A tab before
/// *content* is still an error — that is §6.1 and it has not moved.
#[test]
fn a_tab_before_content_is_still_rejected() {
    for doc in ["a:\n\tb: 1\n", "a:\n\t- 1\n"] {
        assert!(
            parse_document(doc).is_err(),
            "a tab indenting content must stay an error: {doc:?}"
        );
    }
}
