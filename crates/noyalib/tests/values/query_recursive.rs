// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Recursive path queries keep traversal bounded and first-match order.

use noyalib::Value;

#[test]
fn recursive_get_path_missing_leaf_does_not_expand_equivalent_routes() {
    let mut value = Value::Null;
    for _ in 0..28 {
        value = Value::Sequence(vec![value]);
    }
    let path = format!("{}missing", "..".repeat(28));
    assert!(value.get_path(&path).is_none());
    let compiled = noyalib::QueryPath::parse(&path).unwrap();
    assert!(value.get_query_path(&compiled).is_none());
    assert_eq!(value.query(&path), Vec::<&Value>::new());
    assert_eq!(value.query_path(&compiled), Vec::<&Value>::new());
}

#[test]
fn recursive_get_path_keeps_the_first_query_match() {
    let value: Value =
        noyalib::from_str("name: root\nitems:\n  - name: first\n  - name: second\n").unwrap();
    for path in [
        "..name",
        "items[*].name",
        "..items..name",
        "..missing",
        "items[*]",
    ] {
        let expected = value.query(path).into_iter().next();
        assert_eq!(value.get_path(path), expected, "{path}");
        let compiled = noyalib::QueryPath::parse(path).unwrap();
        assert_eq!(value.get_query_path(&compiled), expected, "{path}");
    }
}

#[test]
fn document_path_timeout_corpus_returns_without_a_match() {
    let source = include_str!("../fixtures/query-recursive-timeout.bin");
    let (yaml, path) = source.split_once('\0').unwrap();
    let value: Value = noyalib::from_str(yaml).unwrap();
    assert!(value.get_path(path).is_none());
    assert_eq!(value.query(path), Vec::<&Value>::new());
}
