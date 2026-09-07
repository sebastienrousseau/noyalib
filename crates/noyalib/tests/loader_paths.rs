// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The same diagnostics through every loader.
//!
//! There are two loaders behind the public API: one that records spans
//! and one that does not, chosen by which entry point the caller uses.
//! The v0.0.39 diagnostics were added to both, but only the span-full
//! one had a test, so the other copy was never executed. A message that
//! depends on which function you called is a bug waiting to happen.

#![allow(missing_docs)]

use noyalib::{Value, load_all, load_all_as, try_load_all};

const CROSS_DOCUMENT: &str = "a: &x 1\n---\nb: *x\n";
const SELF_REFERENTIAL: &str = "a: &x\n  self: *x\n";

fn assert_cross_document(msg: &str, from: &str) {
    assert!(
        msg.starts_with("unknown anchor: x at line 3, column 4"),
        "{from}: {msg}"
    );
    assert!(
        msg.contains("`&x` is defined at line 1, column 4, in an earlier document"),
        "{from}: {msg}"
    );
    assert!(msg.contains("anchors do not cross `---`"), "{from}: {msg}");
}

#[test]
fn the_cross_document_hint_is_the_same_from_every_loader() {
    assert_cross_document(
        &load_all_as::<Value>(CROSS_DOCUMENT)
            .unwrap_err()
            .to_string(),
        "load_all_as",
    );
    assert_cross_document(
        &load_all(CROSS_DOCUMENT).unwrap_err().to_string(),
        "load_all",
    );
    // `try_load_all` is lazy, so the error surfaces while iterating.
    let err = try_load_all(CROSS_DOCUMENT)
        .map(|it| it.collect::<Result<Vec<_>, _>>())
        .and_then(|r| r)
        .unwrap_err();
    assert_cross_document(&err.to_string(), "try_load_all");
}

#[test]
fn the_cycle_message_is_the_same_from_every_loader() {
    for (from, msg) in [
        (
            "load_all_as",
            load_all_as::<Value>(SELF_REFERENTIAL)
                .unwrap_err()
                .to_string(),
        ),
        (
            "load_all",
            load_all(SELF_REFERENTIAL).unwrap_err().to_string(),
        ),
        (
            "from_str",
            noyalib::from_str::<Value>(SELF_REFERENTIAL)
                .unwrap_err()
                .to_string(),
        ),
    ] {
        assert!(msg.contains("alias `*x` points at `&x`"), "{from}: {msg}");
        assert!(msg.contains("still being defined"), "{from}: {msg}");
        assert!(
            msg.contains("cannot be represented as a tree"),
            "{from}: {msg}"
        );
        assert!(!msg.contains("earlier document"), "{from}: {msg}");
    }
}

#[test]
fn a_similar_anchor_is_suggested_from_every_loader() {
    let src = "a: &foo 1\nb: *fooo\n";
    for (from, msg) in [
        (
            "load_all_as",
            load_all_as::<Value>(src).unwrap_err().to_string(),
        ),
        ("load_all", load_all(src).unwrap_err().to_string()),
        (
            "from_str",
            noyalib::from_str::<Value>(src).unwrap_err().to_string(),
        ),
    ] {
        assert!(msg.contains("did you mean `foo`?"), "{from}: {msg}");
    }
}

#[cfg(feature = "parallel")]
mod parallel_split {
    use noyalib::parallel::split;

    /// The split is a partition of the input in every shape, including
    /// the ones where the tail after the last `---` is only whitespace
    /// or only a comment.
    #[test]
    fn every_shape_is_a_partition() {
        for src in [
            "---\na: 1\n---\nb: 2\n",
            "---\na: 1\n---\n",
            "---\na: 1\n---\n   \n",
            "---\na: 1\n---\n# only a comment\n",
            "# prologue\n---\na: 1\n",
            "a: 1\n---\nb: 2\n",
            "a: 1\n",
            "",
            "---\n",
            "\n\n",
        ] {
            let chunks = split(src);
            assert_eq!(chunks.concat(), src, "split lost bytes for {src:?}");
            assert!(
                chunks.iter().all(|c| !c.is_empty()),
                "empty chunk for {src:?}"
            );
        }
    }

    /// A tail of nothing but whitespace after the final marker is not a
    /// document of its own; it belongs to the one before it.
    #[test]
    fn a_whitespace_tail_is_not_a_document() {
        assert_eq!(split("---\na: 1\n").len(), 1);
        assert_eq!(split("---\na: 1\n\n  \n").len(), 1);
        assert_eq!(split("---\na: 1\n---\nb: 2\n\n").len(), 2);
    }
}
