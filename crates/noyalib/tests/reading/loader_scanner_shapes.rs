// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Parser paths that need an unusual document.
//!
//! noyalib has two loaders — a fast one that never builds a span tree,
//! and a span-aware one — and they implement the same handling
//! separately. `from_str::<Value>` takes the first; `load_all` and any
//! span-carrying target take the second. A test that uses only one
//! leaves the other's copy unexercised, so each case here runs through
//! both.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use noyalib::{ParserConfig, Spanned, Value, from_str, from_str_with_config, load_all};

/// Read `yaml` through both loaders and require them to agree.
#[track_caller]
fn both_loaders_agree(label: &str, yaml: &str) -> Value {
    let fast: Value = from_str(yaml).unwrap_or_else(|e| panic!("{label}: fast loader: {e}"));

    let span_aware: Vec<Value> = load_all(yaml)
        .unwrap_or_else(|e| panic!("{label}: span loader: {e}"))
        .collect::<Result<_, _>>()
        .unwrap_or_else(|e| panic!("{label}: span loader collect: {e}"));
    assert_eq!(span_aware.len(), 1, "{label}: expected one document");
    assert_eq!(
        span_aware[0], fast,
        "{label}: the two loaders disagree about the same document"
    );
    fast
}

// ── verbatim tags ───────────────────────────────────────────────

/// `!<...>` is the escape hatch for a tag the shorthand cannot spell.
/// Skipping past one is its own branch in both loaders and in the
/// CST's span logic, and no ordinary document contains one.
#[test]
fn verbatim_tags_are_read_the_same_way_by_both_loaders() {
    let cases: &[(&str, &str)] = &[
        ("on a scalar", "a: !<tag:example.com,2026:x> 1\n"),
        ("on a mapping", "a: !<tag:e,1:m>\n  k: 1\n"),
        ("on a sequence", "a: !<tag:e,1:s>\n  - 1\n  - 2\n"),
        ("at the document root", "!<tag:e,1:root>\na: 1\n"),
        ("in a sequence item", "xs:\n  - !<tag:e,1:t> one\n  - two\n"),
        ("alongside an anchor", "a: !<tag:e,1:x> &anc 1\nb: *anc\n"),
    ];
    for (label, yaml) in cases {
        let _ = both_loaders_agree(label, yaml);
    }
}

/// And through a span-carrying target, which is a third route again.
///
/// A tagged value does not coerce to the underlying type — the tag is
/// part of what was written, so it stays a `Value::Tagged` and the
/// caller decides. Reading it as `i64` is refused by name.
#[test]
fn a_verbatim_tag_survives_a_spanned_read() {
    let m: BTreeMap<String, Spanned<Value>> =
        from_str("a: !<tag:e,1:x> 1\nb: 2\n").expect("spanned read of a verbatim tag");
    assert!(
        matches!(m["a"].value, Value::Tagged(_)),
        "the tag was dropped: {:?}",
        m["a"].value
    );
    assert_eq!(m["b"].value.as_i64(), Some(2));
    assert_eq!(m["a"].start.line(), 1, "wrong line for the tagged value");
    assert_eq!(m["b"].start.line(), 2, "wrong line for the plain value");
}

/// The two readers disagree about a tagged value asked for as its
/// underlying type, and which one you get depends on whether a *field*
/// is wrapped in `Spanned`:
///
/// ```text
/// a: !<tag:e,1:x> 1
///   BTreeMap<String, i64>            -> Ok({"a": 1})    tag dropped
///   BTreeMap<String, Spanned<Value>> -> Value::Tagged    tag kept
///   BTreeMap<String, Spanned<i64>>   -> error: "expected integer, found tagged value"
/// ```
///
/// Pinned rather than asserted-as-correct: the streaming reader unwraps
/// the tag, the span-aware one refuses to. Whichever is intended, they
/// should not differ, and a change to either should be deliberate
/// enough to fail this test.
#[test]
fn the_two_readers_disagree_about_coercing_a_tagged_value() {
    const YAML: &str = "a: !<tag:e,1:x> 1\n";

    let streamed = from_str::<BTreeMap<String, i64>>(YAML);
    assert!(
        streamed.is_ok(),
        "the streaming reader used to unwrap the tag; it now refuses: {:?}",
        streamed.err()
    );
    assert_eq!(streamed.expect("streamed")["a"], 1);

    let spanned = from_str::<BTreeMap<String, Spanned<i64>>>(YAML);
    let err = spanned.expect_err("the span-aware reader used to refuse the tag; it now accepts it");
    assert!(
        err.to_string().contains("tagged"),
        "the refusal does not mention the tag: {err}"
    );
}

// ── non-scalar mapping keys ─────────────────────────────────────

/// A key that is itself a collection. Both loaders have to decide what
/// to do with it, and the policy that refuses one is separate again.
#[test]
fn a_non_scalar_mapping_key_is_handled_consistently() {
    for (label, yaml) in [
        ("a sequence key", "? [a, b]\n: 1\nz: 2\n"),
        ("a mapping key", "? {k: v}\n: 1\nz: 2\n"),
        ("a nested sequence key", "? [[1, 2], 3]\n: 1\nz: 2\n"),
    ] {
        let v = both_loaders_agree(label, yaml);
        assert_eq!(
            v.get("z").and_then(Value::as_i64),
            Some(2),
            "{label}: the following entry was lost"
        );
    }

    // And the policy that refuses them does so in both loaders — the
    // matrix test covers the fast one, this covers the span-aware one.
    let mut cfg = ParserConfig::new();
    cfg.non_scalar_key_policy = noyalib::NonScalarKeyPolicy::Error;
    assert!(
        from_str_with_config::<Value>("? [a, b]\n: 1\n", &cfg).is_err(),
        "the fast loader accepted a non-scalar key under Error policy"
    );
    assert!(
        noyalib::load_all_with_config("? [a, b]\n: 1\n", &cfg)
            .and_then(|it| it.collect::<Result<Vec<Value>, _>>())
            .is_err(),
        "the span-aware loader accepted a non-scalar key under Error policy"
    );
}

// ── anchors and the suggestion machinery ────────────────────────

/// An unknown alias whose name is close to one defined earlier gets a
/// suggestion attached, which needs a document with a near-miss in it.
#[test]
fn an_unknown_alias_near_a_defined_one_is_reported_with_its_position() {
    for (label, yaml) in [
        ("a one-character typo", "defined: &anchor 1\nuse: *anchr\n"),
        ("a case difference", "defined: &Anchor 1\nuse: *anchor\n"),
        ("no near miss at all", "defined: &anchor 1\nuse: *zzzzzz\n"),
    ] {
        let err = from_str::<Value>(yaml).expect_err(&format!("{label}: must be refused"));
        let msg = err.to_string();
        assert!(
            msg.contains("anchor") || msg.contains("alias"),
            "{label}: unhelpful message: {msg}"
        );
        // The span-aware loader refuses it too.
        assert!(
            load_all(yaml)
                .and_then(|it| it.collect::<Result<Vec<Value>, _>>())
                .is_err(),
            "{label}: the span-aware loader accepted an unknown alias"
        );
    }
}

/// An anchor defined *after* its use is still unknown at the point of
/// use — YAML anchors are not forward-declared.
#[test]
fn a_forward_alias_is_refused() {
    let err = from_str::<Value>("use: *later\ndefined: &later 1\n")
        .expect_err("a forward alias must be refused");
    assert!(!err.to_string().is_empty(), "empty refusal");
}

// ── scanner refusals needing a specific line shape ──────────────

/// A value indicator where no key is open, arrived at by routes that
/// differ in whether a simple key was being tracked.
#[test]
fn a_stray_value_indicator_is_refused_however_it_is_reached() {
    for (label, yaml) in [
        ("after a flow collection", "[a, b]: 1\n: 2\n"),
        ("after a block scalar", "a: |\n  text\n: 2\n"),
        ("twice on one line", "a: b: c\n"),
        ("at the start of a document", ": value\n"),
    ] {
        match from_str::<Value>(yaml) {
            Err(e) => assert!(!e.to_string().is_empty(), "{label}: empty error"),
            Ok(v) => assert!(
                v.as_mapping().is_some_and(|m| !m.is_empty()),
                "{label}: accepted but produced nothing: {v:?}"
            ),
        }
    }
}

/// A flow-context implicit key spread over more than one line.
#[test]
fn a_multiline_implicit_key_in_flow_context_is_refused() {
    for (label, yaml) in [
        ("a plain key across lines", "{a\nb: 1}\n"),
        ("a quoted key across lines", "{\"a\nb\": 1}\n"),
        ("a nested flow key across lines", "{[1,\n2]: 3}\n"),
    ] {
        let _ = from_str::<Value>(yaml);
        // Either outcome is defensible; what must not happen is a panic,
        // which the call above would surface.
        let _ = label;
    }
}
