//! Multi-document serialization tests.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Value, load_all, to_string_multi, to_writer_multi};

#[test]
fn test_to_string_multi() {
    let docs = vec![1i64, 2, 3];
    let yaml = to_string_multi(&docs).unwrap();

    // Should have --- separators
    let doc_count = yaml.matches("---").count();
    assert_eq!(
        doc_count, 3,
        "expected 3 document markers, got {doc_count} in:\n{yaml}"
    );
    assert!(yaml.contains("1"), "missing value 1 in:\n{yaml}");
    assert!(yaml.contains("2"), "missing value 2 in:\n{yaml}");
    assert!(yaml.contains("3"), "missing value 3 in:\n{yaml}");
}

#[test]
fn test_to_writer_multi() {
    let docs = vec!["hello", "world"];
    let mut buf = Vec::new();
    to_writer_multi(&mut buf, &docs).unwrap();

    let yaml = String::from_utf8(buf).unwrap();
    assert!(yaml.contains("---"));
    assert!(yaml.contains("hello"));
    assert!(yaml.contains("world"));
}

#[test]
fn test_roundtrip_multi_doc() {
    let docs = vec![Value::from(42i64), Value::from("hello"), Value::Bool(true)];
    let yaml = to_string_multi(&docs).unwrap();

    let loaded: Vec<Value> = load_all(&yaml).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[0].as_i64(), Some(42));
    assert_eq!(loaded[1].as_str(), Some("hello"));
    assert_eq!(loaded[2].as_bool(), Some(true));
}

#[test]
fn test_to_string_multi_empty() {
    let docs: Vec<i64> = vec![];
    let yaml = to_string_multi(&docs).unwrap();
    assert!(yaml.is_empty());
}

#[test]
fn test_to_string_multi_single() {
    let docs = vec![42i64];
    let yaml = to_string_multi(&docs).unwrap();
    assert!(yaml.starts_with("---\n"));
    assert!(yaml.contains("42"));
}
