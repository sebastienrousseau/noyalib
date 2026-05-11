// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Special key scenarios

use std::collections::HashMap;

use noyalib::{Value, from_str};

#[test]
fn quoted_key_with_spaces() {
    let m: HashMap<String, String> = from_str("\"key with spaces\": value\n").unwrap();
    assert_eq!(m["key with spaces"], "value");
}

#[test]
fn quoted_key_with_colon() {
    let m: HashMap<String, String> = from_str("\"key:colon\": value\n").unwrap();
    assert_eq!(m["key:colon"], "value");
}

#[test]
fn integer_key() {
    let v: Value = from_str("42: value\n").unwrap();
    let map = v.as_mapping().unwrap();
    assert_eq!(map.get("42").unwrap().as_str(), Some("value"));
}

#[test]
fn boolean_key() {
    let v: Value = from_str("true: yes\n").unwrap();
    let map = v.as_mapping().unwrap();
    assert_eq!(map.get("true").unwrap().as_str(), Some("yes"));
}

#[test]
fn empty_key() {
    let v: Value = from_str("\"\": empty key\n").unwrap();
    let map = v.as_mapping().unwrap();
    assert_eq!(map.get("").unwrap().as_str(), Some("empty key"));
}

#[test]
fn explicit_key_indicator() {
    let v: Value = from_str("? explicit key\n: value\n").unwrap();
    let map = v.as_mapping().unwrap();
    assert_eq!(map.get("explicit key").unwrap().as_str(), Some("value"));
}

#[test]
fn multiword_plain_key() {
    let m: HashMap<String, String> = from_str("multi word key: value\n").unwrap();
    assert_eq!(m["multi word key"], "value");
}

#[test]
fn keys_with_special_chars() {
    let m: HashMap<String, String> = from_str("\"key#hash\": v1\n\"key!bang\": v2\n").unwrap();
    assert_eq!(m["key#hash"], "v1");
    assert_eq!(m["key!bang"], "v2");
}
