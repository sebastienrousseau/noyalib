// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Block scalars the serializer wrote in a shape its own parser read
//! back differently (issues #383, #385, #387), and the CST insert that
//! the same layout made fail its integrity check (#386).

#![allow(missing_docs)]

use noyalib::cst::parse_document;
use noyalib::{Mapping, SerializerConfig, Value, from_str, to_string};

fn s(v: &str) -> Value {
    Value::String(v.into())
}

fn map(pairs: &[(&str, Value)]) -> Value {
    let mut m = Mapping::new();
    for (k, v) in pairs {
        let _ = m.insert(*k, v.clone());
    }
    Value::Mapping(m)
}

fn seq(items: &[Value]) -> Value {
    Value::Sequence(items.to_vec())
}

fn round_trip(value: &Value) -> Value {
    let out = to_string(value).unwrap();
    from_str(&out).unwrap_or_else(|e| panic!("{e}\n--- written ---\n{out}"))
}

// ── #383: a text with no content line ──────────────────────────────

#[test]
fn a_lone_newline_takes_keep_chomping_and_reads_back() {
    // `k: |` followed by one empty line read back as "": clip keeps the
    // final break of the last content line, and there is none.
    let value = map(&[("k", s("\n"))]);
    assert_eq!(to_string(&value).unwrap(), "k: |+\n\n");
    assert_eq!(round_trip(&value), value);

    for text in ["\n\n", "\n\n\n"] {
        let value = map(&[("k", s(text))]);
        assert_eq!(round_trip(&value), value, "{text:?}");
    }
    // As a whole document, and under the single-quote preference.
    assert_eq!(round_trip(&s("\n")), s("\n"));
    let cfg = SerializerConfig::new().prefer_single_quotes(true);
    let out = noyalib::to_string_value_with_config(&map(&[("k", s("\n"))]), &cfg).unwrap();
    assert_eq!(from_str::<Value>(&out).unwrap(), map(&[("k", s("\n"))]));
}

// ── #385: a keep-chomped block followed by a sibling ───────────────

#[test]
fn a_keep_chomped_block_followed_by_a_sibling_keeps_its_count() {
    let value = map(&[("a", map(&[("off", s("\n\n")), ("b", s("x"))]))]);
    let out = to_string(&value).unwrap();
    assert_eq!(out, "a:\n  off: |+\n\n\n  b: x");
    assert_eq!(round_trip(&value), value);

    // Stable across repeated round trips.
    let mut current = value.clone();
    for _ in 0..3 {
        current = round_trip(&current);
    }
    assert_eq!(current, value);

    // A sequence item followed by another item.
    let value = map(&[("a", seq(&[s("x\n\n"), s("x")]))]);
    assert_eq!(to_string(&value).unwrap(), "a:\n  - |+\n    x\n\n  - x");
    assert_eq!(round_trip(&value), value);

    // The last entry of a nested mapping, followed by the parent's next key.
    let value = map(&[("a", map(&[("off", s("\n\n"))])), ("b", s("x"))]);
    assert_eq!(round_trip(&value), value);
}

#[test]
fn a_clip_block_followed_by_a_sibling_opens_no_blank_line() {
    // The block closes its own last line; the next entry starts on the
    // line after it, not after a blank one.
    let value = map(&[("a", map(&[("k", s("x\n")), ("b", s("y"))]))]);
    assert_eq!(to_string(&value).unwrap(), "a:\n  k: |\n    x\n  b: y");
    assert_eq!(round_trip(&value), value);

    let value = map(&[("k", s("x\ny")), ("b", s("z"))]);
    assert_eq!(to_string(&value).unwrap(), "k: |-\n  x\n  y\nb: z");
    assert_eq!(round_trip(&value), value);
}

// ── #387: an indentation indicator on a sequence item ──────────────

#[test]
fn a_sequence_item_with_an_indentation_indicator_reads_back() {
    let value = map(&[("a", seq(&[s("\t\n")]))]);
    assert_eq!(to_string(&value).unwrap(), "a:\n  - |2\n    \t\n");
    assert_eq!(round_trip(&value), value);

    for text in [" x\n", "\n x", "\t\nx", "x\n"] {
        let value = map(&[("a", seq(&[s(text)]))]);
        assert_eq!(round_trip(&value), value, "{text:?}");
        let nested = map(&[("a", seq(&[seq(&[s(text)])]))]);
        assert_eq!(round_trip(&nested), nested, "nested {text:?}");
        let root = seq(&[s(text), s("plain")]);
        assert_eq!(round_trip(&root), root, "root {text:?}");
    }

    let cfg = SerializerConfig::new().compact_list_indent(true);
    let value = map(&[("a", seq(&[s("\t\n"), seq(&[s(" x\n")])]))]);
    let out = noyalib::to_string_value_with_config(&value, &cfg).unwrap();
    assert_eq!(out, "a:\n- |2\n  \t\n- - |2\n    \x20x\n");
    assert_eq!(from_str::<Value>(&out).unwrap(), value);
}

#[test]
fn a_sequence_item_block_body_sits_one_indent_past_the_dash() {
    // The same column a mapping value's body takes past its key, so the
    // indicator (`config.indent` beyond the parent node) is right for both.
    let value = map(&[
        ("a", seq(&[s("x\ny\n")])),
        ("m", map(&[("k", s("x\ny\n"))])),
    ]);
    assert_eq!(
        to_string(&value).unwrap(),
        "a:\n  - |\n    x\n    y\nm:\n  k: |\n    x\n    y\n"
    );
    assert_eq!(round_trip(&value), value);
}

// ── #386: the CST insert of such items ─────────────────────────────

#[test]
fn set_path_writes_a_sequence_item_that_needs_an_indicator_or_keep() {
    for (item, expected) in [
        ("\t\n", "title: T\nk:\n  - |2\n    \t\n"),
        ("\n\n", "title: T\nk:\n  - |+\n\n\n"),
        (" x\n", "title: T\nk:\n  - |2\n     x\n"),
    ] {
        let mut doc = parse_document("title: T\n").unwrap();
        let value = seq(&[s(item)]);
        doc.set_path("k", &value)
            .unwrap_or_else(|e| panic!("{item:?}: {e}"));
        assert_eq!(doc.to_string(), expected, "{item:?}");
        let back: Value = from_str(&doc.to_string()).unwrap();
        assert_eq!(back["k"], value, "{item:?}");
    }

    let mut doc = parse_document("title: T\n").unwrap();
    let value = seq(&[seq(&[seq(&[s("\t\n")])])]);
    doc.set_path("k", &value).unwrap();
    let back: Value = from_str(&doc.to_string()).unwrap();
    assert_eq!(back["k"], value);

    // A keep-chomped item spliced in front of a sibling entry: the new
    // key lands at the end of `a`, and the parent's `next` follows it.
    let mut doc = parse_document("a:\n  x: 1\nnext: 1\n").unwrap();
    doc.set_path("a.k", &seq(&[s("x\n\n")])).unwrap();
    assert_eq!(
        doc.to_string(),
        "a:\n  x: 1\n  k:\n    - |+\n      x\n\nnext: 1\n"
    );
    let back: Value = from_str(&doc.to_string()).unwrap();
    assert_eq!(back["a"]["k"], seq(&[s("x\n\n")]));
    assert_eq!(back["next"].as_i64(), Some(1));
}
