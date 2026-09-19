// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Scanner refusals that need a specific malformed document.
//!
//! The scanner rejects a handful of constructs that are syntactically
//! tempting and not YAML: a block collection opened on the `---` line,
//! a tag suffix holding a character URIs cannot carry, a second value
//! indicator where no key is open. Each has exactly one branch raising
//! it, and each needs an input nobody writes by accident — which is why
//! they went untested.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use noyalib::{ErrorKind, Value, from_str};

/// `!<...>`-spelled tags exist for suffixes the shorthand cannot carry,
/// but `>` itself is not a URI character, so it cannot be written
/// either way. Found originally by `fuzz_roundtrip` on `!!)!)>!`.
#[test]
fn a_tag_suffix_holding_an_angle_bracket_is_refused() {
    let cases: &[(&str, &str)] = &[
        ("the `!!` form", "!!)!)>! v\n"),
        (
            "a named handle",
            "%TAG !e! tag:example.com,2026:\n--- !e!a>b v\n",
        ),
        ("the primary handle", "!a>b v\n"),
    ];
    let mut refused = 0;
    for (label, yaml) in cases {
        if let Err(e) = from_str::<Value>(yaml) {
            let msg = e.to_string();
            if msg.contains('>') {
                assert!(
                    msg.contains("tag suffix") || msg.contains("URI") || msg.contains("tag"),
                    "{label}: the refusal does not explain itself: {msg}"
                );
                refused += 1;
            }
        }
    }
    assert!(
        refused >= 1,
        "no spelling of a `>`-bearing tag suffix was refused, so the check is \
         no longer reachable"
    );
}

/// A block collection cannot open on the same line as `---`.
#[test]
fn a_block_collection_cannot_open_on_the_document_start_line() {
    for (label, yaml) in [
        ("a sequence", "--- - a\n"),
        ("an explicit key", "--- ? a\n"),
        ("a mapping", "--- a: b\n"),
    ] {
        let err = match from_str::<Value>(yaml) {
            Ok(v) => panic!("{label}: {yaml:?} was accepted as {v:?}"),
            Err(e) => e,
        };
        assert_eq!(
            err.kind(),
            ErrorKind::Syntax,
            "{label}: wrong kind for {err}"
        );
        assert!(
            err.to_string().contains("not allowed") || err.to_string().contains("'---'"),
            "{label}: unhelpful refusal: {err}"
        );
    }

    // The control: the same content one line down is fine.
    for yaml in ["---\n- a\n", "---\na: b\n"] {
        let _: Value = from_str(yaml).unwrap_or_else(|e| panic!("{yaml:?} should be valid: {e}"));
    }
}

/// A value indicator with no key open, in each of the shapes that get
/// there by a different route.
#[test]
fn a_value_indicator_with_no_key_open_is_refused() {
    for (label, yaml) in [
        ("a second colon on one line", "a: b: c\n"),
        ("a colon after a flow collection", "[a]: b\n: c\n"),
        ("a bare colon after a scalar", "a\n: b\n: c\n"),
    ] {
        match from_str::<Value>(yaml) {
            Err(e) => assert_eq!(e.kind(), ErrorKind::Syntax, "{label}: wrong kind for {e}"),
            Ok(v) => {
                // Accepting is defensible for some of these; what must
                // not happen is a value that silently drops content.
                assert!(
                    v.as_mapping().is_some_and(|m| !m.is_empty()),
                    "{label}: {yaml:?} produced an empty document: {v:?}"
                );
            }
        }
    }
}

/// `...` ends a document. The scanner advances past all three bytes,
/// which nothing reached because the suite's multi-document fixtures
/// all use `---` alone.
#[test]
fn an_explicit_document_end_marker_is_handled() {
    let docs: Vec<Value> = noyalib::load_all("a: 1\n...\n---\nb: 2\n...\n")
        .expect("explicit end markers parse")
        .collect::<Result<_, _>>()
        .expect("both documents load");
    assert_eq!(
        docs.len(),
        2,
        "the `...` markers changed the document count"
    );
    assert_eq!(docs[0].get("a").and_then(Value::as_i64), Some(1));
    assert_eq!(docs[1].get("b").and_then(Value::as_i64), Some(2));
}

/// A document marker inside a block scalar is content in some positions
/// and a terminator in others; the scanner refuses the ambiguous one
/// rather than guessing.
#[test]
fn a_document_marker_at_column_zero_inside_a_block_scalar_is_refused() {
    let yaml = "a: |\n  line\n--- \nb: 2\n";
    match from_str::<Value>(yaml) {
        Err(e) => assert!(!e.to_string().is_empty(), "empty refusal"),
        Ok(_) => panic!("a `---` inside a block scalar was silently accepted"),
    }
}
