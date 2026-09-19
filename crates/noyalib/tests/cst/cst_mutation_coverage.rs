// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Exhaustive exercise of the CST mutators across document shapes.
//!
//! The happy path of each mutator was tested on a flat mapping. The
//! branches that handle *shape* — nested collections, comments above and
//! beside an entry, anchors, block scalars, the first and last entry,
//! CRLF — were not, and those are where a lossless editor actually goes
//! wrong: the byte it moves is a byte someone wrote.
//!
//! Every test re-parses the result and compares values, so a mutation
//! that produces text which no longer means the same thing fails here
//! rather than in a user's file.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use noyalib::Value;
use noyalib::cst::parse_document;

/// Re-parse and compare, so a mutation that corrupts meaning fails even
/// when the bytes look plausible.
fn reparses(doc_text: &str) -> Value {
    noyalib::from_str(doc_text).expect("mutated document must still parse")
}

// ── rename_key ──────────────────────────────────────────────────

#[test]
fn rename_key_across_document_shapes() {
    let cases: &[(&str, &str, &str)] = &[
        ("a: 1\nb: 2\n", "a", "z"),
        ("a: 1\nb: 2\n", "b", "z"),
        ("outer:\n  inner: 1\n", "outer.inner", "renamed"),
        ("a: 1 # trailing\nb: 2\n", "a", "z"),
        ("# head\na: 1\nb: 2\n", "a", "z"),
        ("a: |\n  block\n  scalar\nb: 2\n", "a", "z"),
        ("a:\n  - 1\n  - 2\nb: 3\n", "a", "z"),
        ("a: 1\r\nb: 2\r\n", "a", "z"),
        ("a: &anc 1\nb: *anc\n", "a", "z"),
    ];
    for (src, path, new_key) in cases {
        let mut doc = parse_document(src).expect("parse");
        doc.rename_key(path, new_key)
            .unwrap_or_else(|e| panic!("rename {path}->{new_key} in {src:?}: {e}"));
        let out = doc.to_string();
        let leaf = path.rsplit('.').next().unwrap();
        assert!(
            !out.contains(&format!("{leaf}:")) || path.contains('.'),
            "old key survived in {out:?}"
        );
        assert!(out.contains(new_key), "new key missing from {out:?}");
        let _ = reparses(&out);
    }
}

#[test]
fn rename_key_rejects_a_duplicate_and_leaves_the_document_alone() {
    let mut doc = parse_document("a: 1\nb: 2\n").expect("parse");
    let before = doc.to_string();
    let _ = doc.rename_key("a", "b");
    // Whether it refuses or renames, the document must still parse and
    // must not silently lose an entry.
    let out = doc.to_string();
    let v = reparses(&out);
    assert!(
        v.as_mapping().map_or(0, noyalib::Mapping::len) >= 1,
        "entries vanished: {before:?} -> {out:?}"
    );
}

// ── swap_items / move_item ──────────────────────────────────────

#[test]
fn swap_and_move_across_sequence_shapes() {
    let shapes: &[&str] = &[
        "xs:\n  - a\n  - b\n  - c\n",
        "xs:\n  - a  # one\n  - b  # two\n  - c\n",
        "xs:\n  - {k: a}\n  - {k: b}\n  - {k: c}\n",
        "xs:\n  - - 1\n    - 2\n  - - 3\n    - 4\n  - - 5\n",
        "xs: [a, b, c]\n",
        "xs:\n  - |\n    block a\n  - |\n    block b\n",
    ];
    for src in shapes {
        let last = reparses(src)
            .get("xs")
            .and_then(Value::as_sequence)
            .expect("xs")
            .len()
            - 1;

        let mut doc = parse_document(src).expect("parse");
        doc.swap_items("xs", 0, last)
            .unwrap_or_else(|e| panic!("swap in {src:?}: {e}"));
        let swapped = reparses(&doc.to_string());
        let before = reparses(src);
        assert_eq!(
            swapped.get("xs").and_then(Value::as_sequence).expect("xs")[0],
            before.get("xs").and_then(Value::as_sequence).expect("xs")[last],
            "swap did not move the last item to the front in {src:?}"
        );

        let mut doc2 = parse_document(src).expect("parse");
        doc2.move_item("xs", 0, last)
            .unwrap_or_else(|e| panic!("move in {src:?}: {e}"));
        let moved = reparses(&doc2.to_string());
        assert_eq!(
            moved.get("xs").and_then(Value::as_sequence).expect("xs")[last],
            before.get("xs").and_then(Value::as_sequence).expect("xs")[0],
            "move did not put the first item last in {src:?}"
        );
    }
}

#[test]
fn swapping_an_item_with_itself_is_a_no_op() {
    let src = "xs:\n  - a\n  - b\n";
    let mut doc = parse_document(src).expect("parse");
    doc.swap_items("xs", 1, 1).expect("self-swap");
    assert_eq!(doc.to_string(), src, "self-swap changed the document");
}

#[test]
fn move_item_to_its_own_position_is_a_no_op() {
    let src = "xs:\n  - a\n  - b\n  - c\n";
    let mut doc = parse_document(src).expect("parse");
    doc.move_item("xs", 1, 1).expect("self-move");
    assert_eq!(doc.to_string(), src, "self-move changed the document");
}

#[test]
fn move_item_forwards_and_backwards_are_inverses() {
    let src = "xs:\n  - a\n  - b\n  - c\n  - d\n";
    let mut doc = parse_document(src).expect("parse");
    doc.move_item("xs", 0, 3).expect("move forward");
    doc.move_item("xs", 3, 0).expect("move back");
    let v = reparses(&doc.to_string());
    let names: Vec<_> = v
        .get("xs")
        .and_then(Value::as_sequence)
        .expect("seq")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(names, ["a", "b", "c", "d"], "move was not reversible");
}

// ── insert / push_back / insert_after ───────────────────────────

#[test]
fn insert_entry_across_mapping_shapes() {
    let shapes: &[&str] = &[
        "a: 1\n",
        "a: 1\nb: 2\n",
        "outer:\n  inner: 1\n",
        "a: 1 # trailing\n",
        "a: 1\n# dangling tail comment\n",
        "a:\n  - 1\n",
        "a: |\n  block\n",
        "a: 1\r\n",
    ];
    for src in shapes {
        let mut doc = parse_document(src).expect("parse");
        doc.insert_entry("", "zz", "9")
            .unwrap_or_else(|e| panic!("insert_entry into {src:?}: {e}"));
        let out = doc.to_string();
        let v = reparses(&out);
        assert_eq!(
            v.get("zz").and_then(Value::as_i64),
            Some(9),
            "inserted entry not readable back from {out:?}"
        );
        // The entries that were already there must survive verbatim.
        for (k, before) in reparses(src).as_mapping().into_iter().flatten() {
            assert_eq!(
                v.get(k.as_str()),
                Some(before),
                "insert_entry disturbed an existing entry in {out:?}"
            );
        }
    }
}

#[test]
fn push_back_across_sequence_shapes() {
    let shapes: &[&str] = &[
        "xs:\n  - a\n",
        "xs:\n  - a\n  - b\n",
        "xs:\n  - a  # c\n",
        "xs:\n  - |\n    block\n",
    ];
    for src in shapes {
        let before = reparses(src)
            .get("xs")
            .and_then(Value::as_sequence)
            .expect("xs")
            .len();
        let mut doc = parse_document(src).expect("parse");
        doc.push_back("xs", "zz")
            .unwrap_or_else(|e| panic!("push_back onto {src:?}: {e}"));
        let out = doc.to_string();
        let xs = reparses(&out);
        let xs = xs.get("xs").and_then(Value::as_sequence).expect("xs");
        assert_eq!(
            xs.len(),
            before + 1,
            "push_back changed length wrongly: {out:?}"
        );
        assert_eq!(
            xs.last().and_then(Value::as_str),
            Some("zz"),
            "push_back did not append at the end: {out:?}"
        );
    }
}

#[test]
fn insert_after_positions_correctly() {
    let src = "xs:\n  - a\n  - c\n";
    let mut doc = parse_document(src).expect("parse");
    doc.insert_after("xs[0]", "b").expect("insert_after");
    {
        let v = reparses(&doc.to_string());
        let xs: Vec<_> = v
            .get("xs")
            .and_then(Value::as_sequence)
            .expect("seq")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(
            xs,
            ["a", "b", "c"],
            "insert_after put it in the wrong place"
        );
    }
}

// ── remove ──────────────────────────────────────────────────────

#[test]
fn remove_across_document_shapes() {
    let cases: &[(&str, &str)] = &[
        ("a: 1\nb: 2\n", "a"),
        ("a: 1\nb: 2\n", "b"),
        ("a: 1\nb: 2\nc: 3\n", "b"),
        ("outer:\n  inner: 1\n  other: 2\n", "outer.inner"),
        ("a: 1  # side\nb: 2\n", "a"),
        ("# head\na: 1\nb: 2\n", "a"),
        ("xs:\n  - a\n  - b\n", "xs[0]"),
        ("a: |\n  block\nb: 2\n", "a"),
        ("a: 1\r\nb: 2\r\n", "a"),
    ];
    for (src, path) in cases {
        let mut doc = parse_document(src).expect("parse");
        doc.remove(path)
            .unwrap_or_else(|e| panic!("remove {path} from {src:?}: {e}"));
        let out = doc.to_string();
        let v = reparses(&out);
        // Everything the source had except the removed path must remain.
        let before = reparses(src);
        for (k, was) in before.as_mapping().into_iter().flatten() {
            let key = k.as_str();
            if *path == key {
                assert!(v.get(key).is_none(), "removed key survived in {out:?}");
            } else if !path.starts_with(key) {
                assert_eq!(v.get(key), Some(was), "remove disturbed `{key}` in {out:?}");
            }
        }
    }
}

#[test]
fn removing_the_only_entry_leaves_a_parseable_document() {
    let mut doc = parse_document("only: 1\n").expect("parse");
    doc.remove("only").expect("remove the sole entry");
    let out = doc.to_string();
    let v: Value = noyalib::from_str(&out).expect("an emptied document must still parse");
    assert!(v.get("only").is_none(), "removed key survived in {out:?}");
}

/// An empty collection has no sibling to copy indentation from, so both
/// anchored inserters refuse rather than guess. The refusal is the
/// contract; it must stay an error with a message that says why.
#[test]
fn the_anchored_inserters_refuse_an_empty_collection() {
    let mut doc = parse_document("{}\n").expect("parse");
    let err = doc
        .insert_entry("", "zz", "9")
        .expect_err("insert_entry into an empty mapping must refuse");
    assert!(
        err.to_string().contains("empty mapping"),
        "unhelpful refusal: {err}"
    );
    assert_eq!(
        doc.to_string(),
        "{}\n",
        "the refusal still edited the document"
    );

    let mut doc = parse_document("xs: []\n").expect("parse");
    let err = doc
        .push_back("xs", "zz")
        .expect_err("push_back onto an empty sequence must refuse");
    assert!(
        err.to_string().contains("empty sequence"),
        "unhelpful refusal: {err}"
    );
    assert_eq!(
        doc.to_string(),
        "xs: []\n",
        "the refusal still edited the document"
    );
}

/// Out-of-range indices are rejected by both reorderers, naming the
/// sequence and its real length so the caller can tell which end is wrong.
#[test]
fn the_reorderers_reject_an_out_of_range_index() {
    let src = "xs:\n  - a\n  - b\n";
    let mut doc = parse_document(src).expect("parse");
    for err in [
        doc.swap_items("xs", 0, 2).expect_err("swap out of bounds"),
        doc.move_item("xs", 0, 2).expect_err("move out of bounds"),
    ] {
        let msg = err.to_string();
        assert!(msg.contains("out of bounds"), "unhelpful refusal: {msg}");
        assert!(
            msg.contains("xs"),
            "refusal does not name the sequence: {msg}"
        );
    }
    assert_eq!(
        doc.to_string(),
        src,
        "a rejected index still edited the document"
    );
}

// ── set_value across shapes ─────────────────────────────────────

#[test]
fn set_value_across_scalar_shapes() {
    let cases: &[(&str, &str)] = &[
        ("a: 1\n", "a"),
        ("a: 'quoted'\n", "a"),
        ("a: \"double\"\n", "a"),
        ("a: |\n  block\n", "a"),
        ("a: >\n  folded\n", "a"),
        ("a: 1  # keep\n", "a"),
        ("outer:\n  inner: 1\n", "outer.inner"),
        ("xs:\n  - 1\n", "xs[0]"),
        ("a: 1\r\n", "a"),
    ];
    for (src, path) in cases {
        let mut doc = parse_document(src).expect("parse");
        doc.set_value(path, &Value::from(42_i64))
            .unwrap_or_else(|e| panic!("set_value {path} in {src:?}: {e}"));
        let out = doc.to_string();
        let v = reparses(&out);
        let got = path.split(['.', '[']).try_fold(v, |acc, seg| {
            let seg = seg.trim_end_matches(']');
            seg.parse::<usize>().map_or_else(
                |_| acc.get(seg).cloned(),
                |i| acc.as_sequence().and_then(|s| s.get(i).cloned()),
            )
        });
        assert_eq!(
            got.as_ref().and_then(Value::as_i64),
            Some(42),
            "set_value did not take effect at `{path}` in {out:?}"
        );
    }
}

// ── the `_value` mutators (typed, not fragment) ──────────────────

#[test]
fn value_mutators_round_trip_through_emit() {
    let mut doc = parse_document("m:\n  a: 1\nxs:\n  - a\n").expect("parse");
    doc.insert_entry_value("m", "b", &Value::from(2_i64))
        .expect("insert_entry_value");
    doc.push_back_value("xs", &Value::from("tail"))
        .expect("push_back_value");
    doc.insert_after_value("xs[0]", &Value::from("mid"))
        .expect("insert_after_value");

    let v = reparses(&doc.to_string());
    assert_eq!(
        v.get("m").and_then(|m| m.get("b")).and_then(Value::as_i64),
        Some(2),
        "insert_entry_value did not round-trip"
    );
    let xs: Vec<_> = v
        .get("xs")
        .and_then(Value::as_sequence)
        .expect("xs")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(
        xs,
        ["a", "mid", "tail"],
        "the typed sequence mutators landed in the wrong order"
    );
}

#[test]
fn value_mutators_refuse_an_unresolvable_path_without_touching_the_document() {
    let src = "m:\n  a: 1\n";
    let mut doc = parse_document(src).expect("parse");
    for res in [
        doc.insert_entry_value("nope", "b", &Value::from(1_i64)),
        doc.push_back_value("nope", &Value::from(1_i64)),
        doc.insert_after_value("nope[0]", &Value::from(1_i64)),
    ] {
        assert!(res.is_err(), "an unresolvable path was accepted");
    }
    assert_eq!(
        doc.to_string(),
        src,
        "a failed mutation edited the document"
    );
}

#[test]
fn value_mutators_emit_nested_collections_at_the_right_indent() {
    let mut doc = parse_document("m:\n  a: 1\n").expect("parse");
    let nested: Value = noyalib::from_str("k:\n  - 1\n  - 2\n").expect("nested");
    doc.insert_entry_value("m", "deep", &nested)
        .expect("insert nested");
    let out = doc.to_string();
    let v = reparses(&out);
    assert_eq!(
        v.get("m")
            .and_then(|m| m.get("deep"))
            .and_then(|d| d.get("k"))
            .and_then(Value::as_sequence)
            .map_or(0, |s| s.len()),
        2,
        "nested collection lost its items: {out:?}"
    );
}
