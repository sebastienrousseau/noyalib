//! Coverage tests for the loader module — load_all, load_all_with_config,
//! try_load_all, load_all_as.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::document::{load_all, load_all_as, load_all_with_config, try_load_all};
use noyalib::{ParserConfig, Value};

// ============================================================================
// load_all
// ============================================================================

#[test]
fn load_all_single_document() {
    let yaml = "key: value\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn load_all_multiple_documents() {
    let yaml = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].get("a").unwrap().as_i64(), Some(1));
    assert_eq!(docs[1].get("b").unwrap().as_i64(), Some(2));
    assert_eq!(docs[2].get("c").unwrap().as_i64(), Some(3));
}

#[test]
fn load_all_empty_string() {
    let docs: Vec<Value> = load_all("").unwrap().map(|r| r.unwrap()).collect();
    assert!(docs.is_empty());
}

#[test]
fn load_all_just_separator() {
    let docs: Vec<Value> = load_all("---\n").unwrap().map(|r| r.unwrap()).collect();
    // A document with just "---" yields a null document
    assert_eq!(docs.len(), 1);
}

#[test]
fn load_all_scalar_documents() {
    let yaml = "---\n42\n---\nhello\n---\ntrue\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].as_i64(), Some(42));
    assert_eq!(docs[1].as_str(), Some("hello"));
    assert_eq!(docs[2].as_bool(), Some(true));
}

#[test]
fn load_all_invalid_yaml() {
    let result = load_all("{{{{invalid");
    assert!(result.is_err());
}

// ============================================================================
// DocumentIterator
// ============================================================================

#[test]
fn document_iterator_len() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let iter = load_all(yaml).unwrap();
    assert_eq!(iter.len(), 2);
    assert!(!iter.is_empty());
}

#[test]
fn document_iterator_empty() {
    let iter = load_all("").unwrap();
    assert_eq!(iter.len(), 0);
    assert!(iter.is_empty());
}

#[test]
fn document_iterator_size_hint() {
    let yaml = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let mut iter = load_all(yaml).unwrap();
    assert_eq!(iter.size_hint(), (3, Some(3)));

    let _ = iter.next();
    assert_eq!(iter.size_hint(), (2, Some(2)));

    let _ = iter.next();
    let _ = iter.next();
    assert_eq!(iter.size_hint(), (0, Some(0)));
}

#[test]
fn document_iterator_exact_size() {
    let yaml = "---\n1\n---\n2\n";
    let iter = load_all(yaml).unwrap();
    assert_eq!(iter.len(), 2);
}

// ============================================================================
// load_all_with_config
// ============================================================================

#[test]
fn load_all_with_config_max_document_length() {
    let yaml = "this is a long string that exceeds the limit";
    let config = ParserConfig::new().max_document_length(10);
    let result = load_all_with_config(yaml, &config);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("maximum length"), "got: {err}");
}

#[test]
fn load_all_with_config_basic() {
    let yaml = "---\nkey: value\n";
    let config = ParserConfig::new();
    let docs: Vec<Value> = load_all_with_config(yaml, &config)
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(docs.len(), 1);
}

#[test]
fn load_all_with_config_depth_limit() {
    // The depth limit is now enforced during parsing (the native parser checks
    // depth as it builds the Value tree).
    let yaml = "a:\n  b:\n    c:\n      d:\n        e: 1\n";
    let config = ParserConfig::new().max_depth(2);
    let result = load_all_with_config(yaml, &config);
    assert!(result.is_err());
}

// ============================================================================
// try_load_all
// ============================================================================

#[test]
fn try_load_all_basic() {
    let yaml = "---\n42\n---\nhello\n";
    let iter = try_load_all(yaml).unwrap();
    assert_eq!(iter.len(), 2);
}

#[test]
fn try_load_all_invalid() {
    let result = try_load_all("{{{{invalid");
    assert!(result.is_err());
}

// ============================================================================
// load_all_as
// ============================================================================

#[test]
fn load_all_as_basic() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Doc {
        name: String,
    }

    let yaml = "---\nname: first\n---\nname: second\n";
    let docs: Vec<Doc> = load_all_as(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].name, "first");
    assert_eq!(docs[1].name, "second");
}

#[test]
fn load_all_as_scalar() {
    let yaml = "---\n42\n---\n99\n";
    let docs: Vec<i64> = load_all_as(yaml).unwrap();
    assert_eq!(docs, vec![42, 99]);
}

#[test]
fn load_all_as_type_mismatch() {
    let yaml = "---\nhello\n---\n42\n";
    let result: Result<Vec<i64>, _> = load_all_as(yaml);
    assert!(result.is_err());
}

#[test]
fn load_all_as_invalid_yaml() {
    let result: Result<Vec<i64>, _> = load_all_as("{{{{");
    assert!(result.is_err());
}

#[test]
fn load_all_as_empty() {
    let docs: Vec<i64> = load_all_as("").unwrap();
    assert!(docs.is_empty());
}

// ============================================================================
// Documents with end markers
// ============================================================================

#[test]
fn load_all_with_end_markers() {
    let yaml = "---\na: 1\n...\n---\nb: 2\n...\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(docs.len(), 2);
}

#[test]
fn load_all_mixed_types() {
    let yaml = "---\n42\n---\nhello\n---\n- 1\n- 2\n---\na: b\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(docs.len(), 4);
    assert!(docs[0].is_number());
    assert!(docs[1].is_string());
    assert!(docs[2].is_sequence());
    assert!(docs[3].is_mapping());
}
