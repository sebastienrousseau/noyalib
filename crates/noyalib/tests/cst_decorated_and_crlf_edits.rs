// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Editing values that carry decorations, and documents that use CRLF.
//!
//! A value is not always just its content. It can be preceded by an
//! anchor (`&name`), a shorthand tag (`!Foo`), or a verbatim tag
//! (`!<tag:example.com,2026:x>`), and the editor has to skip past those
//! to find where the content actually starts. It can also sit in a file
//! whose line breaks are CRLF, where every "step back over the newline"
//! is two bytes rather than one.
//!
//! Both are ordinary in real files and absent from the suite's
//! fixtures, so the code that handles them ran only by accident.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use noyalib::Value;
use noyalib::cst::{Document, parse_document};

/// One CST mutator, pinned to a fixed path and argument.
type Mutator = fn(&mut Document) -> noyalib::Result<()>;

/// Re-parse and return the value, so a mutation that corrupts meaning
/// fails here rather than in a user's file.
#[track_caller]
fn reparse(doc_text: &str, label: &str) -> Value {
    noyalib::from_str(doc_text).unwrap_or_else(|e| panic!("{label}: {e}\n{doc_text}"))
}

// ── decorated values ────────────────────────────────────────────

/// Setting a value whose old text carries an anchor or a tag. The
/// decoration belongs to the entry, not to the content, so the editor
/// has to find where the content starts before replacing it.
#[test]
fn a_value_behind_an_anchor_or_a_tag_can_be_replaced() {
    let cases: &[(&str, &str)] = &[
        ("an anchor", "a: &anc 1\nz: 2\n"),
        ("a shorthand tag", "a: !!int 1\nz: 2\n"),
        ("a custom tag", "a: !Custom 1\nz: 2\n"),
        ("a verbatim tag", "a: !<tag:example.com,2026:x> 1\nz: 2\n"),
        ("an anchor and a tag", "a: !!int &anc 1\nz: 2\n"),
        (
            "a verbatim tag in a sequence",
            "xs:\n  - !<tag:e,1:t> one\n  - two\nz: 2\n",
        ),
    ];
    for (label, src) in cases {
        let mut doc = parse_document(src).unwrap_or_else(|e| panic!("{label}: parse: {e}"));
        let path = if src.starts_with("xs:") { "xs[0]" } else { "a" };
        match doc.set_value(path, &Value::from(42_i64)) {
            Ok(()) => {
                let out = doc.to_string();
                let v = reparse(&out, label);
                assert_eq!(
                    v.get("z").and_then(Value::as_i64),
                    Some(2),
                    "{label}: the edit disturbed the following entry\n{out}"
                );
            }
            Err(e) => {
                // Refusing a decorated value is a defensible contract;
                // silently mangling it is not.
                assert_eq!(
                    doc.to_string(),
                    *src,
                    "{label}: refused ({e}) but edited the document"
                );
            }
        }
    }
}

/// Reading a decorated value back out, which is the same span logic in
/// the other direction.
#[test]
fn a_decorated_value_reads_back_with_its_decoration() {
    for (label, src, path) in [
        ("anchor", "a: &anc 1\n", "a"),
        ("verbatim tag", "a: !<tag:e,1:t> 1\n", "a"),
        ("custom tag", "a: !Custom 1\n", "a"),
    ] {
        let doc = parse_document(src).unwrap_or_else(|e| panic!("{label}: {e}"));
        let got = doc.get(path);
        assert!(got.is_some(), "{label}: the value has no span");
        assert!(!got.unwrap().is_empty(), "{label}: the span is empty");
    }
}

// ── CRLF ────────────────────────────────────────────────────────

/// Every sequence mutator against a CRLF document. Stepping back from a
/// value to the `-` that introduces it crosses two bytes rather than
/// one, and getting that wrong lands the edit a byte off.
#[test]
fn the_sequence_mutators_work_on_a_crlf_document() {
    const SRC: &str = "xs:\r\n  - one\r\n  - two\r\n  - three\r\nafter: 1\r\n";

    let ops: &[(&str, Mutator)] = &[
        ("push_back", |d| d.push_back("xs", "four")),
        ("push_back_value", |d| {
            d.push_back_value("xs", &Value::from("four"))
        }),
        ("insert_after", |d| d.insert_after("xs[0]", "mid")),
        ("insert_after_value", |d| {
            d.insert_after_value("xs[0]", &Value::from("mid"))
        }),
        ("swap_items", |d| d.swap_items("xs", 0, 2)),
        ("move_item", |d| d.move_item("xs", 0, 2)),
        ("remove", |d| d.remove("xs[1]")),
        ("set_value", |d| {
            d.set_value("xs[1]", &Value::from("edited"))
        }),
    ];

    for (label, op) in ops {
        let mut doc = parse_document(SRC).expect("CRLF fixture parses");
        op(&mut doc).unwrap_or_else(|e| panic!("{label} on a CRLF document: {e}"));
        let out = doc.to_string();
        let v = reparse(&out, label);

        assert_eq!(
            v.get("after").and_then(Value::as_i64),
            Some(1),
            "{label}: the entry after the sequence was disturbed\n{out:?}"
        );
        assert!(
            v.get("xs").and_then(Value::as_sequence).is_some(),
            "{label}: `xs` stopped being a sequence\n{out:?}"
        );
        // The document was CRLF; the edit must not introduce a bare LF.
        let bare_lf = out
            .match_indices('\n')
            .filter(|(i, _)| *i == 0 || out.as_bytes()[i - 1] != b'\r')
            .count();
        assert_eq!(
            bare_lf, 0,
            "{label}: introduced {bare_lf} bare LF line break(s) into a CRLF document\n{out:?}"
        );
    }
}

/// The mapping mutators against CRLF, for the same reason.
#[test]
fn the_mapping_mutators_work_on_a_crlf_document() {
    const SRC: &str = "m:\r\n  a: 1\r\n  b: 2\r\nafter: 3\r\n";

    let ops: &[(&str, Mutator)] = &[
        ("insert_entry", |d| d.insert_entry("m", "c", "3")),
        ("insert_entry_value", |d| {
            d.insert_entry_value("m", "c", &Value::from(3_i64))
        }),
        ("remove", |d| d.remove("m.a")),
        ("rename_key", |d| d.rename_key("m.a", "renamed")),
        ("set_value", |d| d.set_value("m.a", &Value::from(9_i64))),
    ];

    for (label, op) in ops {
        let mut doc = parse_document(SRC).expect("CRLF fixture parses");
        op(&mut doc).unwrap_or_else(|e| panic!("{label} on a CRLF document: {e}"));
        let out = doc.to_string();
        let v = reparse(&out, label);
        assert_eq!(
            v.get("after").and_then(Value::as_i64),
            Some(3),
            "{label}: the entry after the mapping was disturbed\n{out:?}"
        );
        let bare_lf = out
            .match_indices('\n')
            .filter(|(i, _)| *i == 0 || out.as_bytes()[i - 1] != b'\r')
            .count();
        assert_eq!(bare_lf, 0, "{label}: introduced a bare LF\n{out:?}");
    }
}

/// A CRLF document whose sequence is nested one level deeper, so the
/// step back to the `-` crosses both the break and the indentation.
#[test]
fn a_nested_crlf_sequence_can_be_appended_to() {
    const SRC: &str = "outer:\r\n  inner:\r\n    - one\r\n    - two\r\nafter: 1\r\n";
    let mut doc = parse_document(SRC).expect("parse");
    doc.push_back("outer.inner", "three")
        .expect("append to a nested CRLF sequence");
    let out = doc.to_string();
    let v = reparse(&out, "nested CRLF append");
    let items: Vec<_> = v
        .get("outer")
        .and_then(|o| o.get("inner"))
        .and_then(Value::as_sequence)
        .expect("inner sequence")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(items, ["one", "two", "three"], "{out:?}");
    assert_eq!(v.get("after").and_then(Value::as_i64), Some(1));
}
