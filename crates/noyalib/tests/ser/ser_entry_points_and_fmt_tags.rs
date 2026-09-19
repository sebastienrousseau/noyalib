// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The serializer's outer surface, and the formatting wrappers.
//!
//! Two things go untested when a suite standardises on `to_string`.
//!
//! First, the entry-point family: `to_writer`, `to_fmt_writer`,
//! `to_*_value`, `to_*_multi`, `to_*_tracking_shared`, each with and
//! without a config. They are thin, which is exactly why a mistake in
//! one — a dropped config, a writer never flushed — survives a suite
//! that only ever calls the shortest one.
//!
//! Second, the `fmt` wrappers (`FlowSeq`, `LitStr`, `Commented`, …)
//! serialize as private newtype tags the serializer intercepts. Each
//! interception has a fallback for when the wrapper does not hold the
//! shape it names — `FlowSeq` around an integer, say. Those fallbacks
//! exist so a wrapper can never make output invalid, and nothing
//! reached them.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use noyalib::fmt::{
    Commented, FlowMap, FlowSeq, FoldStr, FoldString, LitStr, LitString, SpaceAfter,
};
use noyalib::{ArcAnchor, RcAnchor};
use noyalib::{SerializerConfig, Value};
use serde::Serialize;

/// The same value through every entry point must produce the same
/// bytes — that is the only thing that makes them interchangeable.
#[test]
fn every_entry_point_agrees_on_the_same_value() {
    #[derive(Serialize)]
    struct Doc {
        name: String,
        items: Vec<i64>,
        nested: BTreeMap<String, bool>,
    }
    let doc = Doc {
        name: "n".into(),
        items: vec![1, 2, 3],
        nested: BTreeMap::from([("k".to_string(), true)]),
    };
    let cfg = SerializerConfig::new();

    let baseline = noyalib::to_string(&doc).expect("to_string");

    let mut buf = Vec::new();
    noyalib::to_writer(&mut buf, &doc).expect("to_writer");
    assert_eq!(String::from_utf8(buf).expect("utf8"), baseline, "to_writer");

    let mut buf = Vec::new();
    noyalib::to_writer_with_config(&mut buf, &doc, &cfg).expect("to_writer_with_config");
    assert_eq!(
        String::from_utf8(buf).expect("utf8"),
        baseline,
        "to_writer_with_config"
    );

    assert_eq!(
        noyalib::to_string_with_config(&doc, &cfg).expect("to_string_with_config"),
        baseline,
        "to_string_with_config"
    );

    let mut s = String::new();
    noyalib::to_fmt_writer(&mut s, &doc).expect("to_fmt_writer");
    assert_eq!(s, baseline, "to_fmt_writer");

    let mut s = String::new();
    noyalib::to_fmt_writer_with_config(&mut s, &doc, &cfg).expect("to_fmt_writer_with_config");
    assert_eq!(s, baseline, "to_fmt_writer_with_config");

    assert_eq!(
        noyalib::to_string_tracking_shared(&doc).expect("tracking_shared"),
        baseline,
        "to_string_tracking_shared"
    );
    assert_eq!(
        noyalib::to_string_tracking_shared_with_config(&doc, &cfg)
            .expect("tracking_shared_with_config"),
        baseline,
        "to_string_tracking_shared_with_config"
    );

    let mut buf = Vec::new();
    noyalib::to_writer_tracking_shared(&mut buf, &doc).expect("to_writer_tracking_shared");
    assert_eq!(
        String::from_utf8(buf).expect("utf8"),
        baseline,
        "to_writer_tracking_shared"
    );

    let mut buf = Vec::new();
    noyalib::to_writer_tracking_shared_with_config(&mut buf, &doc, &cfg)
        .expect("to_writer_tracking_shared_with_config");
    assert_eq!(
        String::from_utf8(buf).expect("utf8"),
        baseline,
        "to_writer_tracking_shared_with_config"
    );
}

/// The `Value`-typed entry points take the same path without serde in
/// front of them.
#[test]
fn the_value_entry_points_agree_with_each_other() {
    let v: Value = noyalib::from_str("a: 1\nb:\n  - x\n  - y\n").expect("parse");
    let cfg = SerializerConfig::new();
    let baseline = noyalib::to_string_value(&v).expect("to_string_value");

    assert_eq!(
        noyalib::to_string_value_with_config(&v, &cfg).expect("with_config"),
        baseline
    );

    let mut buf = Vec::new();
    noyalib::to_writer_value(&mut buf, &v).expect("to_writer_value");
    assert_eq!(String::from_utf8(buf).expect("utf8"), baseline);

    let mut buf = Vec::new();
    noyalib::to_writer_value_with_config(&mut buf, &v, &cfg).expect("to_writer_value_with_config");
    assert_eq!(String::from_utf8(buf).expect("utf8"), baseline);
}

/// Multi-document output: `---` between documents, and the writer form
/// must match the string form byte for byte.
#[test]
fn the_multi_document_entry_points_agree_and_separate_documents() {
    let docs = vec![
        BTreeMap::from([("a".to_string(), 1)]),
        BTreeMap::from([("b".to_string(), 2)]),
    ];
    let cfg = SerializerConfig::new();
    let baseline = noyalib::to_string_multi(&docs).expect("to_string_multi");
    assert!(
        baseline.contains("---"),
        "multi-document output has no document separator: {baseline:?}"
    );
    assert_eq!(
        noyalib::to_string_multi_with_config(&docs, &cfg).expect("multi_with_config"),
        baseline
    );

    let mut buf = Vec::new();
    noyalib::to_writer_multi(&mut buf, &docs).expect("to_writer_multi");
    assert_eq!(String::from_utf8(buf).expect("utf8"), baseline);

    let mut buf = Vec::new();
    noyalib::to_writer_multi_with_config(&mut buf, &docs, &cfg).expect("multi_writer_config");
    assert_eq!(String::from_utf8(buf).expect("utf8"), baseline);

    // Round-trips back to the same two documents.
    let back: Vec<BTreeMap<String, i64>> = noyalib::load_all_as(&baseline).expect("load_all_as");
    assert_eq!(back, docs, "multi-document output did not round-trip");
}

// ── fmt wrappers, used as intended ──────────────────────────────

#[test]
fn each_fmt_wrapper_produces_the_spelling_it_names() {
    #[derive(Serialize)]
    struct Doc<'a> {
        flow_seq: FlowSeq<Vec<i64>>,
        flow_map: FlowMap<BTreeMap<String, i64>>,
        lit: LitStr<'a>,
        lit_owned: LitString,
        fold: FoldStr<'a>,
        fold_owned: FoldString,
        commented: Commented<i64>,
        spaced: SpaceAfter<i64>,
    }
    let doc = Doc {
        flow_seq: FlowSeq(vec![1, 2, 3]),
        flow_map: FlowMap(BTreeMap::from([("k".to_string(), 1)])),
        lit: LitStr("one\ntwo\n"),
        lit_owned: LitString("three\nfour\n".into()),
        fold: FoldStr("a long folded line"),
        fold_owned: FoldString("another folded line".into()),
        commented: Commented::new(7, "why seven"),
        spaced: SpaceAfter(8),
    };
    let out = noyalib::to_string(&doc).expect("wrappers serialize");

    assert!(out.contains("flow_seq: [1, 2, 3]"), "flow seq: {out}");
    assert!(out.contains("flow_map: {k: 1}"), "flow map: {out}");
    assert!(out.contains("lit: |"), "literal block: {out}");
    assert!(out.contains("lit_owned: |"), "owned literal block: {out}");
    assert!(out.contains("fold: >"), "folded block: {out}");
    assert!(out.contains("fold_owned: >"), "owned folded block: {out}");
    assert!(out.contains("why seven"), "comment not emitted: {out}");
    assert!(
        out.contains("spaced: 8"),
        "space-after value missing: {out}"
    );

    // Whatever the wrappers do to layout, the result must still be YAML
    // that reads back with the same values.
    let back: BTreeMap<String, Value> = noyalib::from_str(&out).expect("wrapper output reparses");
    assert_eq!(back["lit"].as_str(), Some("one\ntwo\n"));
    assert_eq!(back["spaced"].as_i64(), Some(8));
}

/// A wrapper naming a shape it does not hold must fall back to normal
/// output rather than emit something invalid. These fallbacks are the
/// serializer's only defence against a caller's type error becoming a
/// malformed document.
#[test]
fn a_wrapper_around_the_wrong_shape_falls_back_to_plain_output() {
    #[derive(Serialize)]
    struct Wrong {
        seq_of_scalar: FlowSeq<i64>,
        map_of_scalar: FlowMap<i64>,
        seq_of_map: FlowSeq<BTreeMap<String, i64>>,
        map_of_seq: FlowMap<Vec<i64>>,
    }
    let out = noyalib::to_string(&Wrong {
        seq_of_scalar: FlowSeq(1),
        map_of_scalar: FlowMap(2),
        seq_of_map: FlowSeq(BTreeMap::from([("k".to_string(), 3)])),
        map_of_seq: FlowMap(vec![4, 5]),
    })
    .expect("mismatched wrappers still serialize");

    let back: BTreeMap<String, Value> =
        noyalib::from_str(&out).expect("fallback output must still be valid YAML");
    assert_eq!(back["seq_of_scalar"].as_i64(), Some(1), "{out}");
    assert_eq!(back["map_of_scalar"].as_i64(), Some(2), "{out}");
    assert_eq!(
        back["seq_of_map"].get("k").and_then(Value::as_i64),
        Some(3),
        "{out}"
    );
    assert_eq!(
        back["map_of_seq"]
            .as_sequence()
            .map(|s| s.iter().filter_map(Value::as_i64).collect::<Vec<_>>()),
        Some(vec![4, 5]),
        "{out}"
    );
}

// ── anchors ─────────────────────────────────────────────────────

/// A shared `Rc` serialized twice becomes an anchor and an alias. The
/// anchored value's own shape decides whether the inner starts on the
/// anchor's line or the next one, so all three shapes are checked.
#[test]
fn a_shared_value_is_anchored_once_and_aliased_after() {
    #[derive(Serialize)]
    struct Doc {
        first: RcAnchor<Inner>,
        second: RcAnchor<Inner>,
    }
    #[derive(Serialize)]
    struct Inner {
        m: BTreeMap<String, i64>,
        s: Vec<i64>,
        scalar: i64,
    }
    let shared = Rc::new(Inner {
        m: BTreeMap::from([("k".to_string(), 1)]),
        s: vec![1, 2],
        scalar: 3,
    });
    let out = noyalib::to_string_tracking_shared(&Doc {
        first: RcAnchor(Rc::clone(&shared)),
        second: RcAnchor(shared),
    })
    .expect("anchored serialize");

    assert!(out.contains('&'), "no anchor was emitted: {out}");
    assert!(out.contains('*'), "no alias was emitted: {out}");
    let back: BTreeMap<String, Value> =
        noyalib::from_str(&out).expect("anchored output must reparse");
    assert_eq!(
        back["first"], back["second"],
        "the alias did not resolve to the anchored value: {out}"
    );
}

/// The same, through `Arc`, and with the anchored value being each of
/// the three shapes on its own so every arm of the anchor writer runs.
#[test]
fn anchors_cover_mapping_sequence_and_scalar_inners() {
    #[derive(Serialize)]
    struct Pair<T> {
        a: ArcAnchor<T>,
        b: ArcAnchor<T>,
    }
    // Mapping inner.
    let m = Arc::new(BTreeMap::from([("k".to_string(), 1_i64)]));
    let out = noyalib::to_string_tracking_shared(&Pair {
        a: ArcAnchor(Arc::clone(&m)),
        b: ArcAnchor(m),
    })
    .expect("mapping anchor");
    let back: BTreeMap<String, Value> = noyalib::from_str(&out).expect("reparse");
    assert_eq!(back["a"], back["b"], "mapping anchor: {out}");

    // Sequence inner.
    let s = Arc::new(vec![1_i64, 2, 3]);
    let out = noyalib::to_string_tracking_shared(&Pair {
        a: ArcAnchor(Arc::clone(&s)),
        b: ArcAnchor(s),
    })
    .expect("sequence anchor");
    let back: BTreeMap<String, Value> = noyalib::from_str(&out).expect("reparse");
    assert_eq!(back["a"], back["b"], "sequence anchor: {out}");

    // Scalar inner.
    let n = Arc::new(42_i64);
    let out = noyalib::to_string_tracking_shared(&Pair {
        a: ArcAnchor(Arc::clone(&n)),
        b: ArcAnchor(n),
    })
    .expect("scalar anchor");
    let back: BTreeMap<String, Value> = noyalib::from_str(&out).expect("reparse");
    assert_eq!(back["a"].as_i64(), Some(42), "scalar anchor: {out}");
    assert_eq!(back["a"], back["b"], "scalar anchor: {out}");

    // Empty collections take the scalar-shaped arm rather than opening
    // a block that has nothing in it.
    let e: Arc<Vec<i64>> = Arc::new(Vec::new());
    let out = noyalib::to_string_tracking_shared(&Pair {
        a: ArcAnchor(Arc::clone(&e)),
        b: ArcAnchor(e),
    })
    .expect("empty-sequence anchor");
    let back: BTreeMap<String, Value> = noyalib::from_str(&out).expect("reparse");
    assert_eq!(back["a"], back["b"], "empty sequence anchor: {out}");
}

// ── regressions ─────────────────────────────────────────────────

/// A wrapper around a block **mapping** used to be written at the
/// parent's column, so the inner keys parsed as siblings of the key
/// they belonged to: the value silently became null and its contents
/// moved up a level. Valid YAML, different document.
///
/// `SpaceAfter` and `Commented` around a struct are ordinary usage, so
/// this was not only a mismatched-wrapper problem.
#[test]
fn a_wrapper_around_a_mapping_keeps_the_mapping_under_its_key() {
    #[derive(Serialize)]
    struct Doc<T> {
        head: i64,
        wrapped: T,
        tail: i64,
    }
    fn inner() -> BTreeMap<String, i64> {
        BTreeMap::from([("k".to_string(), 3)])
    }

    #[track_caller]
    fn check<T: Serialize>(label: &str, wrapped: T) {
        let out = noyalib::to_string(&Doc {
            head: 0,
            wrapped,
            tail: 9,
        })
        .unwrap_or_else(|e| panic!("{label}: {e}"));
        let back: BTreeMap<String, Value> =
            noyalib::from_str(&out).unwrap_or_else(|e| panic!("{label}: reparse: {e}\n{out}"));
        assert_eq!(
            back.len(),
            3,
            "{label}: the wrapped mapping escaped to the top level\n{out}"
        );
        assert_eq!(
            back["wrapped"].get("k").and_then(Value::as_i64),
            Some(3),
            "{label}: the wrapped mapping did not stay under its key\n{out}"
        );
        assert_eq!(back["tail"].as_i64(), Some(9), "{label}: {out}");
    }

    check("SpaceAfter", SpaceAfter(inner()));
    check("Commented", Commented::new(inner(), "why"));
    check("FlowSeq holding a mapping", FlowSeq(inner()));

    // The sequence-shaped mismatch is not corrupting — a block sequence
    // at the parent's column still belongs to the key — but it must
    // still round-trip to the same items.
    let out = noyalib::to_string(&Doc {
        head: 0,
        wrapped: FlowMap(vec![4_i64, 5]),
        tail: 9,
    })
    .expect("serialize");
    let back: BTreeMap<String, Value> = noyalib::from_str(&out).expect("reparse");
    assert_eq!(
        back["wrapped"]
            .as_sequence()
            .map(|s| s.iter().filter_map(Value::as_i64).collect::<Vec<_>>()),
        Some(vec![4, 5]),
        "a flow-map wrapper around a sequence lost its items\n{out}"
    );
    assert_eq!(back["tail"].as_i64(), Some(9), "{out}");
}

/// `SpaceAfter` is documented as "emit a blank line after the value".
/// It emitted one newline, which `start_line` then absorbed when it
/// opened the next entry — so between mapping entries, where the blank
/// line is the entire point, it did nothing at all.
///
/// The only prior test serialized `SpaceAfter(42)` at the document
/// root, where a bare `42` produces the same bytes, so it passed either
/// way.
#[test]
fn space_after_actually_emits_a_blank_line_between_entries() {
    #[derive(Serialize)]
    struct Doc {
        head: i64,
        spaced: SpaceAfter<i64>,
        tail: i64,
    }
    let out = noyalib::to_string(&Doc {
        head: 0,
        spaced: SpaceAfter(5),
        tail: 9,
    })
    .expect("serialize");
    assert_eq!(
        out, "head: 0\nspaced: 5\n\ntail: 9",
        "no blank line after the wrapped value"
    );
    let back: BTreeMap<String, i64> = noyalib::from_str(&out).expect("reparse");
    assert_eq!(
        back,
        BTreeMap::from([
            ("head".to_string(), 0),
            ("spaced".to_string(), 5),
            ("tail".to_string(), 9),
        ])
    );

    // In a sequence, each item gets its own blank line.
    let out = noyalib::to_string(&vec![SpaceAfter(1_i64), SpaceAfter(2)]).expect("serialize");
    assert_eq!(
        out, "- 1\n\n- 2\n\n",
        "no blank line between sequence items"
    );

    // At the document root there is no following entry to separate
    // from, so one newline is the terminator and nothing more.
    assert_eq!(
        noyalib::to_string(&SpaceAfter(42_i64)).expect("serialize"),
        "42\n",
        "the root case must not gain a spurious blank line"
    );
}
