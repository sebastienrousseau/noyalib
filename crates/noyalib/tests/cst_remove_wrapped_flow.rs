//! Regression: removing a member of a *wrapped* flow collection left the
//! member's indentation behind as a whitespace-only line (#294, reported
//! by @zoosky).
//!
//!     ports: [          remove("ports[0]")     ports: [
//!       80,                     ->              ␣␣
//!       443,                                    443,
//!     ]                                       ]
//!
//! The value round-tripped correctly, so this was never corruption. What
//! it was is trailing whitespace written onto a line that had none, which
//! `git diff --check`, `yamllint`'s `trailing-spaces` and
//! `editorconfig-checker` all reject.
//!
//! The fix gives the member its whole line when — and only when — the
//! member is alone on it. Everything else that could hold the line
//! standing (an opening indicator, a sibling, a comment, the closing
//! indicator) must leave the output byte-identical to before, and the
//! bulk of this file is those negative cases.
//!
//! It was unreachable before #285: a wrapped flow collection did not
//! parse, so nothing downstream of the scanner had ever seen one.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::cst::parse_document;

/// Remove `path` and require the exact resulting source.
#[track_caller]
fn removes_exactly(src: &str, path: &str, want: &str) {
    let mut doc = parse_document(src).expect("parse");
    doc.remove(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    assert_eq!(doc.source(), want, "removing {path:?} from {src:?}");
    let _reparsed = parse_document(doc.source()).expect("result re-parses");
}

/// The invariant behind the whole issue: no line may be non-empty yet
/// consist only of blanks. An empty line is fine; `"  "` is not.
#[track_caller]
fn no_whitespace_only_line(src: &str, path: &str) {
    let mut doc = parse_document(src).expect("parse");
    doc.remove(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    for (n, line) in doc.source().lines().enumerate() {
        assert!(
            line.is_empty() || !line.trim().is_empty(),
            "line {} of {:?} is whitespace-only after removing {path:?}",
            n + 1,
            doc.source()
        );
    }
}

// ── the reported cases ───────────────────────────────────────────────

#[test]
fn wrapped_sequence_first_member_takes_its_line() {
    removes_exactly(
        "ports: [\n  80,\n  443,\n]\n",
        "ports[0]",
        "ports: [\n  443,\n]\n",
    );
}

#[test]
fn wrapped_sequence_last_member_takes_its_line() {
    removes_exactly(
        "ports: [\n  80,\n  443,\n]\n",
        "ports[1]",
        "ports: [\n  80,\n]\n",
    );
}

#[test]
fn wrapped_mapping_member_takes_its_line() {
    removes_exactly(
        "cfg: {\n  a: 1,\n  b: 2,\n}\n",
        "cfg.a",
        "cfg: {\n  b: 2,\n}\n",
    );
}

#[test]
fn the_reported_shapes_leave_no_whitespace_only_line() {
    no_whitespace_only_line("ports: [\n  80,\n  443,\n]\n", "ports[0]");
    no_whitespace_only_line("ports: [\n  80,\n  443,\n]\n", "ports[1]");
    no_whitespace_only_line("cfg: {\n  a: 1,\n  b: 2,\n}\n", "cfg.a");
}

// ── negative cases: something else holds the line ────────────────────

#[test]
fn an_opening_indicator_holds_the_line() {
    // `80` is not alone — `ports: [` precedes it.
    removes_exactly(
        "ports: [80,\n  443,\n]\n",
        "ports[0]",
        "ports: [\n  443,\n]\n",
    );
}

#[test]
fn a_sibling_member_holds_the_line() {
    removes_exactly(
        "ports: [\n  80, 443,\n  8080,\n]\n",
        "ports[0]",
        "ports: [\n  443,\n  8080,\n]\n",
    );
}

#[test]
fn a_closing_indicator_holds_the_line() {
    removes_exactly(
        "ports: [\n  80,\n  443]\n",
        "ports[1]",
        "ports: [\n  80,\n  ]\n",
    );
}

#[test]
fn a_trailing_comment_holds_the_line() {
    removes_exactly(
        "ports: [\n  80, # why\n  443,\n]\n",
        "ports[0]",
        "ports: [\n  # why\n  443,\n]\n",
    );
}

#[test]
fn single_line_collections_are_untouched() {
    removes_exactly("ports: [80, 443]\n", "ports[0]", "ports: [443]\n");
    removes_exactly("ports: [80, 443]\n", "ports[1]", "ports: [80]\n");
    removes_exactly("cfg: {a: 1, b: 2}\n", "cfg.a", "cfg: {b: 2}\n");
    removes_exactly("cfg: {a: 1, b: 2}\n", "cfg.b", "cfg: {a: 1}\n");
}

// ── shapes around the edges ──────────────────────────────────────────

#[test]
fn a_middle_member_takes_its_line() {
    removes_exactly(
        "ports: [\n  80,\n  443,\n  8080,\n]\n",
        "ports[1]",
        "ports: [\n  80,\n  8080,\n]\n",
    );
}

#[test]
fn a_last_member_without_a_trailing_comma() {
    removes_exactly(
        "ports: [\n  80,\n  443\n]\n",
        "ports[1]",
        "ports: [\n  80,\n]\n",
    );
}

#[test]
fn crlf_line_endings_are_preserved() {
    removes_exactly(
        "ports: [\r\n  80,\r\n  443,\r\n]\r\n",
        "ports[0]",
        "ports: [\r\n  443,\r\n]\r\n",
    );
}

#[test]
fn nested_inside_a_block_mapping() {
    removes_exactly(
        "a:\n  b: [\n    1,\n    2,\n  ]\n",
        "a.b[0]",
        "a:\n  b: [\n    2,\n  ]\n",
    );
}

#[test]
fn siblings_and_the_document_survive() {
    removes_exactly(
        "keep: 0\nports: [\n  80,\n  443,\n]\ntail: 9\n",
        "ports[0]",
        "keep: 0\nports: [\n  443,\n]\ntail: 9\n",
    );
}

#[test]
fn removing_every_member_one_at_a_time_stays_clean() {
    // Each step must re-parse and leave no whitespace-only line, which is
    // the property that would break if the widening ever over-reached.
    let mut doc = parse_document("ports: [\n  80,\n  443,\n  8080,\n]\n").expect("parse");
    doc.remove("ports[2]").expect("remove 2");
    doc.remove("ports[1]").expect("remove 1");
    assert_eq!(doc.source(), "ports: [\n  80,\n]\n");
    for line in doc.source().lines() {
        assert!(
            line.is_empty() || !line.trim().is_empty(),
            "{:?}",
            doc.source()
        );
    }
}

// ── the value must not move ──────────────────────────────────────────

#[test]
fn the_edit_changes_the_value_by_exactly_one_member() {
    let src = "ports: [\n  80,\n  443,\n]\nother: keep\n";
    let mut doc = parse_document(src).expect("parse");
    doc.remove("ports[0]").expect("remove");

    let after: noyalib::Value = noyalib::from_str(doc.source()).expect("after");
    let expect: noyalib::Value = noyalib::from_str("ports: [443]\nother: keep\n").expect("expect");
    assert_eq!(after, expect);
}

#[test]
fn a_refused_remove_leaves_the_source_untouched() {
    let src = "ports: [\n  80,\n  443,\n]\n";
    let mut doc = parse_document(src).expect("parse");
    assert!(doc.remove("ports[9]").is_err());
    assert_eq!(doc.source(), src, "source untouched after a refusal");
}

// ── the block path, which already answered this correctly ────────────

#[test]
fn the_block_path_is_unchanged_and_still_agrees() {
    // The argument for the fix was that these two shapes answer the same
    // question. Pin the block side so it cannot drift away again.
    removes_exactly("ports:\n  - 80\n  - 443\n", "ports[0]", "ports:\n  - 443\n");
    removes_exactly("ports:\n  - 80\n  - 443\n", "ports[1]", "ports:\n  - 80\n");
    removes_exactly("cfg:\n  a: 1\n  b: 2\n", "cfg.a", "cfg:\n  b: 2\n");
}
