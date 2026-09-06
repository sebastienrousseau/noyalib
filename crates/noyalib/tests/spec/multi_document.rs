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

// A tab before a top-level flow node is separation, not indentation
// (YAML 1.2.2 §6.2; the suite's 6CA3), and the rule is the same in every
// document of a stream. Before v0.0.36 the tab was accepted on the first
// line of the stream and rejected after `---` or `...`.
#[test]
fn tab_before_top_level_flow_node_in_every_document() {
    use noyalib::{Value, load_all_as};
    for src in [
        "\t[\n\t]\n",
        "---\n\t[\n\t]\n",
        "a: 1\n---\n\t[\n\t]\n",
        "a: 1\n...\n\t[\n\t]\n",
        "---\n\t{a: 1}\n",
        "a: 1\n...\n\tfoo\n",
        "---\n\t\"foo\"\n",
    ] {
        let docs = load_all_as::<Value>(src).unwrap_or_else(|e| panic!("{src:?}: {e}"));
        assert!(!docs.is_empty(), "{src:?}");
        let cst = noyalib::cst::parse_stream(src).unwrap_or_else(|e| panic!("cst {src:?}: {e}"));
        assert_eq!(cst.iter().map(|d| d.source()).collect::<String>(), src);
    }
}

// A tab is still indentation before a block collection, on every line
// of the stream including the first, and inside a block collection.
#[test]
fn tab_indentation_is_rejected_everywhere() {
    use noyalib::{Value, load_all_as};
    for src in [
        "\t- a\n",
        "---\n\t- a\n",
        "\t? a\n",
        "\tfoo: 1\n",
        "---\n\tfoo: 1\n",
        "a:\n\t[1]\n",
        "a:\n\tb: 1\n",
    ] {
        let err = load_all_as::<Value>(src).unwrap_err();
        assert!(
            err.to_string().contains("tab characters are not allowed"),
            "{src:?}: {err}"
        );
        let cst = noyalib::cst::parse_stream(src).unwrap_err();
        assert_eq!(cst.location(), err.location(), "{src:?}");
    }
    // A tab on an empty line, or after the indentation spaces, is fine.
    assert!(load_all_as::<Value>("a: 1\n\t\nb: 2\n").is_ok());
    assert!(load_all_as::<Value>("a:\n \t[1]\n").is_ok());
}
