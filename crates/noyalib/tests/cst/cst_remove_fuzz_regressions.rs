// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Removal behaviours that a differential fuzz run turned up.
//!
//! Neither case was a library bug — `remove` was right both times — but
//! both broke plausible-looking invariants in the fuzz harness, which
//! means they are easy to break by accident later. Pinning them here
//! keeps them under the normal test suite rather than only under a
//! nightly fuzz job.

use noyalib::Value;
use noyalib::cst::parse_document;

/// Removing one of several duplicate keys drops exactly one entry.
///
/// `Value` deduplicates mapping keys and the last duplicate wins, so the
/// parsed node count is unchanged across this edit even though the
/// document really did lose a line. Any check phrased as "a removal must
/// shrink the parsed value" is wrong for this input.
#[test]
fn removing_a_duplicate_key_drops_one_entry_not_the_parse_count() {
    let source = "\n2: /\n5?::\n5: /\n5: 55/\n";
    let mut doc = parse_document(source).expect("source parses");

    doc.remove("5").expect("remove is accepted");

    // Exactly one line of text went away.
    assert_eq!(
        doc.source(),
        "\n2: /\n5?::\n5: /\n",
        "remove should drop only the last `5` entry"
    );

    // And the parsed value still has three keys, because the surviving
    // duplicate takes over.
    let after = noyalib::from_str::<Value>(doc.source()).expect("still parses");
    let Value::Mapping(m) = &after else {
        panic!("expected a mapping, got {after:?}");
    };
    assert_eq!(m.len(), 3, "duplicate collapse keeps the key count at 3");
    assert_eq!(
        m.get("5"),
        Some(&Value::String("/".to_owned())),
        "the earlier duplicate should now be the visible `5`"
    );
}

/// Removing the only entry leaves an empty flow mapping, not empty text.
///
/// `"::\n"` is a one-key mapping whose key is `:`. Removing it rewrites
/// the document to `"{}\n"` — the same byte length as the input, so any
/// check phrased as "a removal must make the source shorter" is wrong
/// for this input too.
#[test]
fn removing_the_last_entry_yields_an_empty_flow_mapping() {
    let mut doc = parse_document("::\n").expect("source parses");
    let before = doc.source().to_owned();

    doc.remove(":").expect("remove is accepted");

    assert_eq!(doc.source(), "{}\n");
    assert_eq!(
        doc.source().len(),
        before.len(),
        "this edit is length-preserving, which is why length is not a \
         sound proxy for it having happened"
    );
    assert_ne!(doc.source(), before, "the source did change");

    let after = noyalib::from_str::<Value>(doc.source()).expect("still parses");
    let Value::Mapping(m) = &after else {
        panic!("expected a mapping, got {after:?}");
    };
    assert!(m.is_empty(), "the mapping should be empty, got {m:?}");
}

/// A refused removal must leave the document byte-identical.
#[test]
fn a_refused_remove_leaves_the_source_untouched() {
    let mut doc = parse_document(": :\n:").expect("source parses");
    let before = doc.source().to_owned();

    assert!(doc.remove(":").is_err(), "`:` is not a key here");
    assert_eq!(doc.source(), before, "a refusal must not edit the document");
}
