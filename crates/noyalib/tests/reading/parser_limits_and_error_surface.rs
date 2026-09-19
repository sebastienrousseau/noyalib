// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Every budget and policy the parser can refuse on, and what the
//! resulting error tells you.
//!
//! `ParserConfig`'s limits are the crate's defence against hostile
//! input, and each one has exactly one branch that raises it. A limit
//! whose branch is never taken in the test suite is a limit nobody has
//! confirmed *works* — it is set, read, compared against, and its
//! failure path is a guess.
//!
//! Each case below drives one limit past its threshold and then asks
//! the error four questions, because an error nobody can act on is
//! nearly as bad as no error: what kind is it, what does it say, what
//! does it look like against the source, and what stable code does it
//! carry. All four run through the same table so a new limit added
//! without an entry here is visible as a gap rather than silence.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use noyalib::{
    DuplicateKeyPolicy, Error, ErrorKind, MergeKeyPolicy, NonScalarKeyPolicy, ParserConfig,
    Spanned, Value, from_str, from_str_with_config,
};

/// One refusal: the input, the config that makes it a refusal, the
/// `ErrorKind` it must report, and a phrase the message must carry.
struct Case {
    label: &'static str,
    yaml: &'static str,
    configure: fn(&mut ParserConfig),
    kind: ErrorKind,
    phrase: &'static str,
}

const CASES: &[Case] = &[
    Case {
        label: "max_nodes",
        yaml: "a: 1\nb: 2\nc: 3\nd: 4\n",
        configure: |c| c.max_nodes = 3,
        kind: ErrorKind::Budget,
        phrase: "max_nodes",
    },
    Case {
        label: "max_events",
        yaml: "a: 1\nb: 2\nc: 3\n",
        configure: |c| c.max_events = 4,
        kind: ErrorKind::Budget,
        phrase: "max_events",
    },
    Case {
        label: "max_mapping_keys",
        yaml: "a: 1\nb: 2\nc: 3\n",
        configure: |c| c.max_mapping_keys = 2,
        kind: ErrorKind::Budget,
        phrase: "max_mapping_keys",
    },
    Case {
        label: "max_sequence_length",
        yaml: "- 1\n- 2\n- 3\n",
        configure: |c| c.max_sequence_length = 2,
        kind: ErrorKind::Budget,
        phrase: "max_sequence_length",
    },
    Case {
        label: "max_total_scalar_bytes",
        yaml: "a: abcdefghij\n",
        configure: |c| c.max_total_scalar_bytes = 4,
        kind: ErrorKind::Budget,
        phrase: "max_total_scalar_bytes",
    },
    Case {
        label: "max_merge_keys",
        yaml: "x: &x\n  a: 1\ny:\n  <<: *x\nz:\n  <<: *x\n",
        configure: |c| c.max_merge_keys = 1,
        kind: ErrorKind::Budget,
        phrase: "max_merge_keys",
    },
    Case {
        label: "alias_anchor_ratio",
        yaml: "x: &x 1\na: *x\nb: *x\nc: *x\n",
        configure: |c| c.alias_anchor_ratio = Some(1.0),
        kind: ErrorKind::Budget,
        phrase: "alias_anchor_ratio",
    },
    Case {
        label: "MergeKeyPolicy::Error",
        yaml: "x: &x\n  a: 1\ny:\n  <<: *x\n",
        configure: |c| c.merge_key_policy = MergeKeyPolicy::Error,
        kind: ErrorKind::Other,
        phrase: "merge key",
    },
    Case {
        label: "DuplicateKeyPolicy::Error",
        yaml: "a: 1\na: 2\n",
        configure: |c| c.duplicate_key_policy = DuplicateKeyPolicy::Error,
        kind: ErrorKind::DuplicateKey,
        phrase: "duplicate key",
    },
    Case {
        label: "NonScalarKeyPolicy::Error",
        yaml: "? [a, b]\n: 1\n",
        configure: |c| c.non_scalar_key_policy = NonScalarKeyPolicy::Error,
        kind: ErrorKind::NonScalarKey,
        phrase: "sequence",
    },
    Case {
        label: "integer_overflow_errors",
        yaml: "a: 99999999999999999999999\n",
        configure: |c| c.integer_overflow_errors = true,
        kind: ErrorKind::IntegerOverflow,
        phrase: "64 bits",
    },
];

#[test]
fn every_limit_refuses_when_it_is_exceeded() {
    for case in CASES {
        let mut cfg = ParserConfig::new();
        (case.configure)(&mut cfg);
        let err = from_str_with_config::<Value>(case.yaml, &cfg)
            .err()
            .unwrap_or_else(|| panic!("{}: the limit did not refuse", case.label));

        assert_eq!(
            err.kind(),
            case.kind,
            "{}: wrong ErrorKind for `{}`",
            case.label,
            err
        );
        assert!(
            err.to_string().contains(case.phrase),
            "{}: message does not mention {:?}: {err}",
            case.label,
            case.phrase
        );
    }
}

/// The same inputs must be *accepted* at the default settings —
/// otherwise the case above proves nothing about the limit, only that
/// the document was bad.
#[test]
fn every_limit_case_is_accepted_at_the_default_settings() {
    for case in CASES {
        let _accepted: Value = from_str(case.yaml).unwrap_or_else(|e| {
            panic!(
                "{}: the input is refused even with the limit at its default, so the \
                 test above does not measure the limit: {e}",
                case.label
            )
        });
    }
}

/// `max_documents` is the one limit `from_str` cannot demonstrate: it
/// refuses *any* multi-document input before the budget is consulted.
/// The limit belongs to the multi-document readers, so it is measured
/// where it applies.
#[test]
fn max_documents_refuses_in_the_multi_document_readers() {
    let yaml = "a: 1\n---\nb: 2\n";
    let mut cfg = ParserConfig::new();
    cfg.max_documents = 1;

    let err = noyalib::load_all_with_config(yaml, &cfg)
        .and_then(|it| it.collect::<Result<Vec<Value>, _>>())
        .expect_err("max_documents must refuse the second document");
    assert_eq!(err.kind(), ErrorKind::Budget, "wrong kind for `{err}`");
    assert!(
        err.to_string().contains("max_documents"),
        "message does not name the limit: {err}"
    );

    // And the same input is accepted once the limit allows both.
    let docs: Vec<Value> = noyalib::load_all_with_config(yaml, &ParserConfig::new())
        .expect("default limits")
        .collect::<Result<_, _>>()
        .expect("default limits accept two documents");
    assert_eq!(
        docs.len(),
        2,
        "the control case did not read both documents"
    );
}

/// Rendering an error against its source must work for every one of
/// them, at every radius, without panicking and without losing the
/// message.
#[test]
fn every_refusal_renders_against_its_source() {
    for case in CASES {
        let mut cfg = ParserConfig::new();
        (case.configure)(&mut cfg);
        let err = from_str_with_config::<Value>(case.yaml, &cfg).expect_err(case.label);

        for rendered in [
            err.format_with_source(case.yaml),
            err.format_with_source_radius(case.yaml, 0),
            err.format_with_source_radius(case.yaml, 100),
            err.format_with_source_truncated(case.yaml, 20),
            err.format_with_source_radius_truncated(case.yaml, 1, 40),
            err.render(case.yaml),
        ] {
            assert!(!rendered.is_empty(), "{}: rendered to nothing", case.label);
        }
    }
}

/// `into_shared` / `from_shared` / `as_inner` / `is_shared` are a
/// four-method round trip; each is trivial and none was exercised
/// together, so a mismatch between them would not show.
#[test]
fn the_shared_error_round_trip_is_reversible() {
    let err = from_str::<Value>("a: [unclosed").expect_err("parse error");
    let text = err.to_string();
    let kind = err.kind();

    assert!(!err.is_shared(), "a fresh error is not shared");
    assert!(err.as_inner().is_none(), "a fresh error has no inner");

    let shared = err.into_shared();
    let wrapped = Error::from_shared(shared);
    assert!(
        wrapped.is_shared(),
        "from_shared did not produce a shared error"
    );
    assert_eq!(
        wrapped.as_inner().map(ToString::to_string),
        Some(text.clone()),
        "the inner error did not survive sharing"
    );
    assert_eq!(wrapped.kind(), kind, "sharing changed the error kind");
    assert_eq!(wrapped.to_string(), text, "sharing changed the message");
}

// ── parser refusals that no configuration turns on ──────────────

/// Structural refusals the scanner raises regardless of config. Each is
/// a real YAML rule, and each has exactly one branch raising it.
#[test]
fn the_scanner_refuses_these_structures_by_name() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "a block sequence on the `---` line",
            "--- - a\n",
            "not allowed in this context",
        ),
        (
            "an explicit key on the `---` line",
            "--- ? a\n",
            "not allowed in this context",
        ),
        (
            "a second `:` on one line",
            "a: b: c\n",
            "mapping values are not allowed",
        ),
        (
            "a tag suffix containing `!`",
            "!e!foo!bar x\n",
            "tag suffix must not contain",
        ),
        (
            "a flow terminator with nothing open",
            "[a, b]]\n",
            "outside of any flow sequence",
        ),
    ];
    for (label, yaml, phrase) in cases {
        let err = from_str::<Value>(yaml).expect_err(label);
        assert!(
            err.to_string().contains(phrase),
            "{label}: message does not say why: {err}"
        );
        assert_eq!(err.kind(), ErrorKind::Syntax, "{label}: wrong kind");
    }
}

/// A verbatim tag (`!<…>`) is legal YAML and must be *accepted*; the
/// scanner has a dedicated skip for it that plain tags never reach.
#[test]
fn verbatim_tags_are_accepted_in_both_collection_positions() {
    let v: Value = from_str("!<tag:example.com,2026:x> v\n").expect("verbatim tag at the root");
    assert!(
        matches!(v, Value::Tagged(_)) || v.as_str() == Some("v"),
        "the verbatim tag produced neither a tagged value nor its content: {v:?}"
    );

    let v: Value = from_str("- !<tag:e,1:t> v\n- plain\n").expect("verbatim tag in a sequence");
    assert_eq!(
        v.as_sequence().map(noyalib::Sequence::len),
        Some(2),
        "verbatim tag disturbed the sequence: {v:?}"
    );
}

/// The same matrix, through the span-aware loader.
///
/// noyalib has two loaders: `from_str::<Value>` takes a fast path that
/// never builds a `SpanTree`, while `load_all` and any target needing
/// positions take the span-aware one. They implement the same limits
/// *separately*, so a test that only drives one leaves the other's
/// copy of every budget check unexercised — and a limit that stops
/// working in the span loader would not fail anything.
#[test]
fn every_limit_also_refuses_through_the_span_aware_loader() {
    for case in CASES {
        let mut cfg = ParserConfig::new();
        (case.configure)(&mut cfg);

        let result = noyalib::load_all_with_config(case.yaml, &cfg)
            .and_then(|it| it.collect::<Result<Vec<Value>, _>>());
        let err = result
            .err()
            .unwrap_or_else(|| panic!("{}: the span-aware loader did not refuse", case.label));

        assert_eq!(
            err.kind(),
            case.kind,
            "{}: the two loaders disagree on ErrorKind for `{}`",
            case.label,
            err
        );
        assert!(
            err.to_string().contains(case.phrase),
            "{}: the span-aware loader's message does not mention {:?}: {err}",
            case.label,
            case.phrase
        );
    }
}

/// And the control for it: the same inputs pass the span-aware loader
/// at default settings.
#[test]
fn the_span_aware_loader_accepts_every_case_at_the_default_settings() {
    for case in CASES {
        let _accepted: Vec<Value> = noyalib::load_all_with_config(case.yaml, &ParserConfig::new())
            .and_then(|it| it.collect::<Result<Vec<Value>, _>>())
            .unwrap_or_else(|e| {
                panic!(
                    "{}: the span-aware loader refuses the input even at default \
                     limits, so the test above measures nothing: {e}",
                    case.label
                )
            });
    }
}

/// A `Spanned` target also routes through the span-aware loader, and
/// through a different entry point again. Refusals must match.
#[test]
fn a_spanned_target_reports_the_same_refusals() {
    use std::collections::BTreeMap;

    let mut cfg = ParserConfig::new();
    cfg.max_mapping_keys = 2;
    let err = from_str_with_config::<BTreeMap<String, Spanned<i64>>>("a: 1\nb: 2\nc: 3\n", &cfg)
        .expect_err("max_mapping_keys must refuse a spanned target too");
    assert_eq!(err.kind(), ErrorKind::Budget, "wrong kind for `{err}`");
    assert!(
        err.to_string().contains("max_mapping_keys"),
        "message does not name the limit: {err}"
    );
}
