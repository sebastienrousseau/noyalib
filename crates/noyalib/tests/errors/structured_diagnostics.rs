// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Framework-neutral structured diagnostic contracts.

#![allow(missing_docs)]

use noyalib::{
    Diagnostic, DiagnosticCode, DiagnosticLabel, DiagnosticSeverity, Error, Location, SourceSpan,
    Value, from_str,
};

#[test]
fn parse_errors_expose_a_stable_code_and_primary_span() {
    let source = "first: ok\nsecond: [unclosed\n";
    let error = from_str::<Value>(source).unwrap_err();
    let diagnostic = error.diagnostic();

    assert_eq!(diagnostic.code(), DiagnosticCode::Parse);
    assert_eq!(diagnostic.code().as_str(), "noyalib::parse");
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Error);
    assert!(!diagnostic.message().is_empty());

    let label = diagnostic
        .primary_label()
        .expect("located parse errors need a primary label");
    assert!(label.is_primary());
    assert!(label.span().offset() <= source.len());
    assert_eq!(label.span().length(), 1);
}

#[test]
fn unknown_anchor_diagnostics_keep_primary_and_context_labels() {
    let source = "&known value\n*missing\n";
    let error = Error::UnknownAnchorAt {
        name: "missing".into(),
        location: Location::from_index(source, 13),
        suggestion: Some(("known".into(), Location::from_index(source, 0))),
    };
    let diagnostic = error.diagnostic();

    assert_eq!(diagnostic.code(), DiagnosticCode::UnknownAnchor);
    assert_eq!(diagnostic.labels().len(), 2);
    assert!(diagnostic.labels()[0].is_primary());
    assert!(!diagnostic.labels()[1].is_primary());
    assert!(diagnostic.labels()[1].message().contains("known"));
    assert!(diagnostic.help().is_some());
}

#[test]
fn unlocated_errors_do_not_invent_source_spans() {
    let diagnostic = Error::Custom("service-defined failure".into()).diagnostic();
    assert_eq!(diagnostic.code(), DiagnosticCode::Other);
    assert!(diagnostic.labels().is_empty());
    assert!(diagnostic.primary_label().is_none());
}

#[test]
fn callers_can_build_diagnostics_without_a_rendering_framework() {
    let diagnostic = Diagnostic::new(
        DiagnosticCode::Other,
        DiagnosticSeverity::Warning,
        "deprecated key",
    )
    .with_help("rename the key")
    .with_label(DiagnosticLabel::primary(
        SourceSpan::new(8, 3),
        "deprecated here",
    ))
    .with_label(DiagnosticLabel::secondary(
        SourceSpan::new(0, 4),
        "replacement declared here",
    ));

    assert_eq!(diagnostic.help(), Some("rename the key"));
    assert_eq!(diagnostic.labels().len(), 2);
    assert_eq!(diagnostic.labels()[0].span().end(), 11);
}

#[test]
fn source_span_end_saturates() {
    let span = SourceSpan::new(usize::MAX, 10);
    assert_eq!(span.end(), usize::MAX);
}

#[cfg(feature = "miette")]
#[test]
fn miette_reads_the_same_code_help_and_labels() {
    use miette::Diagnostic as _;

    let source = "key: one\nkey: two\n";
    let error = Error::DuplicateKeyAt {
        key: "key".into(),
        path: "key".into(),
        location: Location::from_index(source, 9),
    };
    let structured = error.diagnostic();

    assert_eq!(
        error.code().expect("miette code").to_string(),
        structured.code().as_str()
    );
    assert_eq!(
        error.help().expect("miette help").to_string(),
        structured.help().expect("structured help")
    );
    assert_eq!(
        error.labels().expect("miette labels").count(),
        structured.labels().len()
    );
}
