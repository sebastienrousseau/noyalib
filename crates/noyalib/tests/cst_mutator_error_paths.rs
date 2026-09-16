// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The refusals, not the successes.
//!
//! Every CST mutator resolves a path, checks the shape it found, and
//! declines when the two disagree. Those declines are most of the code
//! in `cst/document.rs` and almost none of what the happy-path suites
//! reach, so a regression there is invisible: a mutator that silently
//! edits the wrong bytes instead of refusing still passes a test that
//! only ever hands it a valid path.
//!
//! Each case here asserts three things — that it is an error, that the
//! message says which path and why, and that the document is byte-for-
//! byte unchanged. The last is the one that matters: a refusal that has
//! already half-spliced is worse than no refusal at all.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use noyalib::Value;
use noyalib::cst::parse_document;

/// One document with every shape the mutators branch on: a scalar, a
/// nested block mapping, a block sequence, a flow mapping, a flow
/// sequence, an anchor and an alias.
const DOC: &str = "a: 1\n\
                   outer:\n  inner: 2\n\
                   xs:\n  - a\n  - b\n\
                   m: {k: 1}\n\
                   fs: [a, b]\n\
                   anc: &A 1\n\
                   ali: *A\n";

/// Run one refusal and assert all three properties at once.
#[track_caller]
fn refuses(
    label: &str,
    op: impl FnOnce(&mut noyalib::cst::Document) -> noyalib::Result<()>,
    expect_substrings: &[&str],
) {
    let mut doc = parse_document(DOC).expect("fixture parses");
    let err = op(&mut doc).expect_err(label);
    let msg = err.to_string();
    for want in expect_substrings {
        assert!(
            msg.contains(want),
            "{label}: message does not explain the refusal\n  wanted substring: {want:?}\n  \
             got: {msg}"
        );
    }
    assert_eq!(
        doc.to_string(),
        DOC,
        "{label}: the document was edited despite the refusal"
    );
}

// ── rename_key ──────────────────────────────────────────────────

#[test]
fn rename_key_refuses_paths_it_cannot_address() {
    refuses(
        "empty path",
        |d| d.rename_key("", "z"),
        &["rename_key", "non-empty path"],
    );
    refuses(
        "sequence item",
        |d| d.rename_key("xs[0]", "z"),
        &["rename_key", "not a sequence item"],
    );
    refuses(
        "index past the end is still a sequence item, not a key",
        |d| d.rename_key("xs[9]", "z"),
        &["rename_key", "not a sequence item"],
    );
    refuses(
        "missing top-level key",
        |d| d.rename_key("nope", "z"),
        &["path not found", "nope"],
    );
    refuses(
        "missing nested key",
        |d| d.rename_key("outer.nope", "z"),
        &["path not found", "nope"],
    );
    refuses(
        "descending through a scalar",
        |d| d.rename_key("a.b", "z"),
        &["path not found"],
    );
}

#[test]
fn renaming_a_key_to_its_own_spelling_is_an_accepted_no_op() {
    // The early return exists so a caller looping over keys does not
    // have to special-case the unchanged one. It must not splice.
    let mut doc = parse_document(DOC).expect("parse");
    doc.rename_key("a", "a").expect("self-rename is accepted");
    assert_eq!(doc.to_string(), DOC, "a self-rename spliced anyway");
}

#[test]
fn rename_key_reaches_into_a_flow_mapping() {
    let mut doc = parse_document(DOC).expect("parse");
    doc.rename_key("m.k", "renamed")
        .expect("a flow-mapping member is addressable");
    assert!(
        doc.to_string().contains("m: {renamed: 1}"),
        "flow mapping not rewritten in place: {doc}"
    );
}

// ── swap_items / move_item ──────────────────────────────────────

#[test]
fn the_reorderers_refuse_anything_that_is_not_a_sequence() {
    for (label, is_swap) in [("swap_items", true), ("move_item", false)] {
        // Both report through `swap_items`' message; assert what is
        // actually emitted rather than what symmetry would suggest.
        refuses(
            &format!("{label}: scalar target"),
            |d| {
                if is_swap {
                    d.swap_items("a", 0, 1)
                } else {
                    d.move_item("a", 0, 1)
                }
            },
            &["does not address a sequence", "`a`"],
        );
        refuses(
            &format!("{label}: mapping target"),
            |d| {
                if is_swap {
                    d.swap_items("outer", 0, 1)
                } else {
                    d.move_item("outer", 0, 1)
                }
            },
            &["does not address a sequence", "`outer`"],
        );
        refuses(
            &format!("{label}: missing key"),
            |d| {
                if is_swap {
                    d.swap_items("nope", 0, 1)
                } else {
                    d.move_item("nope", 0, 1)
                }
            },
            &["path not found", "nope"],
        );
        refuses(
            &format!("{label}: through a scalar"),
            |d| {
                if is_swap {
                    d.swap_items("a.b", 0, 1)
                } else {
                    d.move_item("a.b", 0, 1)
                }
            },
            &["does not resolve to a sequence"],
        );
    }
}

#[test]
fn the_reorderers_work_on_a_flow_sequence() {
    for is_swap in [true, false] {
        let mut doc = parse_document(DOC).expect("parse");
        if is_swap {
            doc.swap_items("fs", 0, 1).expect("swap in a flow sequence");
        } else {
            doc.move_item("fs", 0, 1).expect("move in a flow sequence");
        }
        assert!(
            doc.to_string().contains("fs: [b, a]"),
            "flow sequence not reordered in place: {doc}"
        );
    }
}

// ── remove ──────────────────────────────────────────────────────

#[test]
fn remove_refuses_an_alias_because_its_bytes_belong_to_the_anchor() {
    refuses(
        "alias value",
        |d| d.remove("ali"),
        &["alias", "anchor", "remove the anchor's entry"],
    );
}

#[test]
fn remove_refuses_paths_it_cannot_address() {
    refuses(
        "missing key",
        |d| d.remove("nope"),
        &["path not found", "nope"],
    );
    refuses(
        "index past the end",
        |d| d.remove("xs[9]"),
        &["path not found", "index 9 out of bounds"],
    );
    refuses("through a scalar", |d| d.remove("a.b"), &["path not found"]);
}

#[test]
fn remove_works_inside_a_flow_sequence() {
    let mut doc = parse_document(DOC).expect("parse");
    doc.remove("fs[0]").expect("remove a flow-sequence item");
    assert!(
        doc.to_string().contains("fs: [b]"),
        "flow sequence not rewritten: {doc}"
    );
}

// ── insert_entry / insert_entry_value ───────────────────────────

#[test]
fn insert_entry_refuses_a_non_mapping_and_an_unknown_path() {
    refuses(
        "scalar target",
        |d| d.insert_entry("a", "n", "2"),
        &["not a mapping"],
    );
    refuses(
        "unknown path",
        |d| d.insert_entry("nope", "n", "2"),
        &["path not found", "nope"],
    );
}

#[test]
fn the_fragment_inserter_refuses_a_flow_mapping_but_the_typed_one_accepts_it() {
    // Splicing `key: value` text into `{k: 1}` would change the entry
    // count of the enclosing flow scalar, so the fragment path declines
    // and points at the variant that can do it.
    refuses(
        "flow mapping via fragment",
        |d| d.insert_entry("m", "n", "2"),
        &["added or removed entries", "_value"],
    );

    let mut doc = parse_document(DOC).expect("parse");
    doc.insert_entry_value("m", "q", &Value::from(2_i64))
        .expect("the typed inserter knows the flow spelling");
    assert!(
        doc.to_string().contains("m: {k: 1, q: 2}"),
        "flow mapping not extended in place: {doc}"
    );
}

// ── push_back / insert_after ────────────────────────────────────

#[test]
fn push_back_refuses_a_non_sequence_and_a_flow_sequence_fragment() {
    refuses(
        "scalar target",
        |d| d.push_back("a", "z"),
        &["push_back", "not a sequence"],
    );
    refuses(
        "flow sequence via fragment",
        |d| d.push_back("fs", "z"),
        &["only block sequences", "`-` anchor"],
    );

    let mut doc = parse_document(DOC).expect("parse");
    doc.push_back_value("fs", &Value::from("z"))
        .expect("the typed appender knows the flow spelling");
    assert!(
        doc.to_string().contains("fs: [a, b, z]"),
        "flow sequence not extended in place: {doc}"
    );
}

#[test]
fn insert_after_refuses_a_path_that_does_not_end_in_an_index() {
    refuses(
        "no index",
        |d| d.insert_after("a", "z"),
        &["insert_after", "sequence index"],
    );
    refuses(
        "index past the end",
        |d| d.insert_after("xs[9]", "z"),
        &["path not found", "xs[9]"],
    );
    refuses(
        "flow sequence via fragment",
        |d| d.insert_after("fs[0]", "z"),
        &["only block sequences", "`-` anchor"],
    );
}

// ── set_value ───────────────────────────────────────────────────

#[test]
fn set_value_refuses_an_alias_and_unresolvable_paths() {
    refuses(
        "alias target",
        |d| d.set_value("ali", &Value::from(1_i64)),
        &["alias", "edit the anchor definition"],
    );
    refuses(
        "unknown key",
        |d| d.set_value("nope", &Value::from(1_i64)),
        &["path not found", "nope"],
    );
    refuses(
        "through a scalar",
        |d| d.set_value("a.b", &Value::from(1_i64)),
        &["path not found", "a.b"],
    );
}

#[test]
fn set_value_works_inside_a_flow_sequence() {
    let mut doc = parse_document(DOC).expect("parse");
    doc.set_value("fs[0]", &Value::from(9_i64))
        .expect("a flow-sequence item is addressable");
    assert!(
        doc.to_string().contains("fs: [9, b]"),
        "flow sequence item not rewritten: {doc}"
    );
}

/// Every path-addressing mutator reaches a flow-mapping member, and
/// each rewrites only that member — the flow spelling around it stays
/// on one line. Pinned together because the four went in separately.
#[test]
fn flow_mapping_members_are_addressable_by_every_mutator() {
    let mut doc = parse_document(DOC).expect("parse");
    doc.rename_key("m.k", "k2")
        .expect("rename in a flow mapping");
    assert!(
        doc.to_string().contains("m: {k2: 1}"),
        "{}",
        doc.to_string()
    );

    let mut doc = parse_document(DOC).expect("parse");
    doc.set_value("m.k", &Value::from(2_i64))
        .expect("set in a flow mapping");
    assert!(doc.to_string().contains("m: {k: 2}"), "{}", doc.to_string());

    let mut doc = parse_document(DOC).expect("parse");
    doc.remove("m.k").expect("remove from a flow mapping");
    let out = doc.to_string();
    assert!(
        out.contains("m: {}") || out.contains("m: {  }"),
        "flow mapping not emptied in place: {out}"
    );
    let _: Value = noyalib::from_str(&out).expect("the result still parses");
}
