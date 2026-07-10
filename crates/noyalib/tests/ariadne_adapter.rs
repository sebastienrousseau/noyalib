// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Integration tests for the ariadne adapter.

#![cfg(feature = "ariadne")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use ariadne::Source;
use noyalib::ariadne_adapter::error_to_ariadne_report;
use noyalib::{Value, from_str};

fn render(source: &str) -> Vec<u8> {
    let err = from_str::<Value>(source).unwrap_err();
    let report = error_to_ariadne_report(&err, "input.yaml", source);
    let mut out = Vec::new();
    report
        .write(("input.yaml", Source::from(source)), &mut out)
        .unwrap();
    out
}

#[test]
fn unclosed_flow_renders_with_source_excerpt() {
    let bytes = render("a: [unclosed\n");
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("input.yaml"), "filename should appear: {s}");
    assert!(
        s.contains("unclosed") || s.contains("Error") || s.contains("error"),
        "report should carry the error context: {s}"
    );
}

#[test]
fn typed_target_error_renders() {
    #[derive(Debug, serde::Deserialize)]
    struct Cfg {
        #[allow(dead_code)]
        port: u16,
    }
    let source = "port: not-a-number\n";
    let err = from_str::<Cfg>(source).unwrap_err();
    let report = error_to_ariadne_report(&err, "config.yaml", source);
    let mut out = Vec::new();
    report
        .write(("config.yaml", Source::from(source)), &mut out)
        .unwrap();
    let s = String::from_utf8_lossy(&out);
    assert!(
        s.contains("port") || s.contains("Error") || s.contains("error"),
        "{s}"
    );
}

#[test]
fn report_without_location_still_renders() {
    use noyalib::Error;
    let err = Error::Custom("synthetic".into());
    let report = error_to_ariadne_report(&err, "input.yaml", "source: value\n");
    let mut out = Vec::new();
    report
        .write(("input.yaml", Source::from("source: value\n")), &mut out)
        .unwrap();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("synthetic"), "{s}");
}

#[test]
fn label_clamps_when_index_past_source_end() {
    use noyalib::Error;
    let err = Error::Custom("late".into());
    let report = error_to_ariadne_report(&err, "input.yaml", "");
    let mut out = Vec::new();
    report
        .write(("input.yaml", Source::from("")), &mut out)
        .unwrap();
    assert!(!out.is_empty());
}

#[test]
fn multibyte_unicode_at_label_does_not_panic() {
    let source = "name: 日本語\nbroken: [unclosed\n";
    let bytes = render(source);
    assert!(!bytes.is_empty());
}

#[test]
fn label_span_normal_path_with_mid_source_location() {
    // Force a parse error whose location lands mid-source so the
    // normal `label_span` path (start < source.len(), char extract,
    // char-width range) fires.
    let source = "key: value\nnext: bad: scalar\nthird: value\n";
    let err = from_str::<Value>(source).unwrap_err();
    if let Some(loc) = err.location() {
        assert!(
            loc.index() < source.len(),
            "expected mid-source location, got {}",
            loc.index()
        );
    }
    let report = error_to_ariadne_report(&err, "config.yaml", source);
    let mut out = Vec::new();
    report
        .write(("config.yaml", Source::from(source)), &mut out)
        .unwrap();
    let rendered = String::from_utf8_lossy(&out);
    assert!(rendered.contains("config.yaml"), "{rendered}");
}

#[test]
fn label_span_handles_utf8_boundary() {
    // Multi-byte UTF-8 at the label position must not panic.
    let source = "name: 日本語の設定\nbad: [unclosed\n";
    let err = from_str::<Value>(source).unwrap_err();
    let report = error_to_ariadne_report(&err, "input.yaml", source);
    let mut out = Vec::new();
    report
        .write(("input.yaml", Source::from(source)), &mut out)
        .unwrap();
    assert!(!out.is_empty());
}
