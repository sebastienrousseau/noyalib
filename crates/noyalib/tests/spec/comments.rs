// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Comments

use std::collections::HashMap;

use noyalib::{Value, from_str};

#[test]
fn inline_comment() {
    let m: HashMap<String, String> = from_str("key: value # this is a comment\n").unwrap();
    assert_eq!(m["key"], "value");
}

#[test]
fn full_line_comment() {
    let v: Value = from_str("# comment\nkey: value\n").unwrap();
    assert_eq!(v.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn comment_between_entries() {
    let m: HashMap<String, i64> = from_str("a: 1\n# comment\nb: 2\n").unwrap();
    assert_eq!(m["a"], 1);
    assert_eq!(m["b"], 2);
}

#[test]
fn comment_at_end() {
    let v: i64 = from_str("42\n# trailing comment\n").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn comment_in_sequence() {
    let v: Vec<i64> = from_str("- 1\n# between\n- 2\n- 3\n").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn comment_in_flow_sequence() {
    let v: Vec<i64> = from_str("[1, # inline\n2, 3]").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn multiple_comments() {
    let m: HashMap<String, String> =
        from_str("# header comment\n# another comment\nkey: value\n# footer\n").unwrap();
    assert_eq!(m["key"], "value");
}

#[test]
fn comment_does_not_affect_string() {
    // Hash inside quotes is not a comment
    let m: HashMap<String, String> = from_str("key: \"value # not comment\"\n").unwrap();
    assert_eq!(m["key"], "value # not comment");
}
