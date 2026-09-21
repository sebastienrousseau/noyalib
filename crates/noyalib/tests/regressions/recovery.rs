// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Integration tests for the `recovery` feature.

#![cfg(feature = "recovery")]

use noyalib::Value;
use noyalib::recovery::{LenientConfig, parse_lenient, parse_lenient_with};

#[test]
fn clean_input_returns_complete_result() {
    let r = parse_lenient("name: noyalib\nversion: 0.0.6\n");
    assert!(r.is_complete);
    assert!(r.errors.is_empty());
    let m = r.value.as_mapping().unwrap();
    assert_eq!(m.get("name").unwrap().as_str(), Some("noyalib"));
}

#[test]
fn malformed_flow_collects_error() {
    let r = parse_lenient("a: [unclosed\n");
    assert!(!r.is_complete);
    assert!(!r.errors.is_empty());
}

#[test]
fn trailing_garbage_recovers_clean_prefix() {
    let yaml = "a: 1\nb: 2\nc: [unclosed\n";
    let r = parse_lenient(yaml);
    assert!(!r.is_complete);
    if let Value::Mapping(m) = &r.value {
        assert_eq!(m.get("a").unwrap().as_i64(), Some(1));
    } else {
        panic!("expected mapping, got {:?}", r.value);
    }
}

#[test]
fn multi_document_partial_recovery() {
    let yaml = "---\nid: 1\n---\nid: 2\n---\nid: [unclosed\n";
    let r = parse_lenient(yaml);
    assert!(!r.is_complete);
    let seq = match r.value {
        Value::Sequence(s) => s,
        other => panic!("expected sequence, got {other:?}"),
    };
    assert_eq!(seq.len(), 3);
    assert!(matches!(&seq[0], Value::Mapping(_)));
    assert!(matches!(&seq[1], Value::Mapping(_)));
}

#[test]
fn error_cap_limits_diagnostics() {
    let cfg = LenientConfig {
        max_errors: 2,
        ..LenientConfig::default()
    };
    let yaml = "---\na: [x\n---\nb: [x\n---\nc: [x\n---\nd: [x\n";
    let r = parse_lenient_with(yaml, &cfg);
    assert!(r.errors.len() <= 2);
}

#[test]
fn empty_input_is_null_and_complete() {
    let r = parse_lenient("");
    assert!(r.is_complete);
    assert!(matches!(r.value, Value::Null));
}
