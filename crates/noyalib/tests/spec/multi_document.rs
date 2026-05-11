// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Multi-document streams

use noyalib::{Value, from_str, load_all};

#[test]
fn document_start_marker() {
    let v: String = from_str("---\nhello").unwrap();
    assert_eq!(v, "hello");
}

#[test]
fn document_end_marker() {
    let v: String = from_str("hello\n...").unwrap();
    assert_eq!(v, "hello");
}

#[test]
fn multiple_documents() {
    let docs: Vec<Value> = load_all("---\n1\n---\n2\n---\n3\n")
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].as_i64(), Some(1));
    assert_eq!(docs[1].as_i64(), Some(2));
    assert_eq!(docs[2].as_i64(), Some(3));
}

#[test]
fn multiple_documents_with_end_marker() {
    let docs: Vec<Value> = load_all("---\na\n...\n---\nb\n...\n")
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].as_str(), Some("a"));
    assert_eq!(docs[1].as_str(), Some("b"));
}

#[test]
fn bare_document() {
    let v: String = from_str("bare document").unwrap();
    assert_eq!(v, "bare document");
}

#[test]
fn null_document() {
    let v: Option<i32> = from_str("---\n").unwrap();
    assert!(v.is_none());
}

#[test]
fn mixed_document_types() {
    let docs: Vec<Value> = load_all("---\n42\n---\nhello\n---\ntrue\n")
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(docs.len(), 3);
    assert!(docs[0].is_number());
    assert!(docs[1].is_string());
    assert!(docs[2].is_bool());
}

#[test]
fn document_with_yaml_directive() {
    let v: String = from_str("%YAML 1.2\n---\nhello\n").unwrap();
    assert_eq!(v, "hello");
}

#[test]
fn single_document_via_load_all() {
    let docs: Vec<Value> = load_all("hello\n").unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].as_str(), Some("hello"));
}
