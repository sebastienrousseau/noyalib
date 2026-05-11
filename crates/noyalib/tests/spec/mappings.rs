// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Mappings

use std::collections::HashMap;

use noyalib::{Value, from_str};
use serde::Deserialize;

#[test]
fn block_mapping_string_values() {
    let m: HashMap<String, String> = from_str("name: John\ncity: NYC\n").unwrap();
    assert_eq!(m["name"], "John");
    assert_eq!(m["city"], "NYC");
}

#[test]
fn block_mapping_integer_values() {
    let m: HashMap<String, i64> = from_str("a: 1\nb: 2\nc: 3\n").unwrap();
    assert_eq!(m["a"], 1);
    assert_eq!(m["b"], 2);
    assert_eq!(m["c"], 3);
}

#[test]
fn empty_mapping() {
    let m: HashMap<String, String> = from_str("{}").unwrap();
    assert!(m.is_empty());
}

#[test]
fn nested_mappings() {
    #[derive(Debug, Deserialize)]
    struct Config {
        database: Database,
    }
    #[derive(Debug, Deserialize)]
    struct Database {
        host: String,
        port: u16,
    }

    let c: Config = from_str("database:\n  host: localhost\n  port: 5432\n").unwrap();
    assert_eq!(c.database.host, "localhost");
    assert_eq!(c.database.port, 5432);
}

#[test]
fn mapping_with_sequence_value() {
    let v: HashMap<String, Vec<String>> =
        from_str("colors:\n  - red\n  - green\n  - blue\n").unwrap();
    assert_eq!(v["colors"], vec!["red", "green", "blue"]);
}

#[test]
fn mapping_with_null_values() {
    let m: HashMap<String, Option<String>> = from_str("a: hello\nb: ~\nc:\n").unwrap();
    assert_eq!(m["a"], Some("hello".into()));
    assert_eq!(m["b"], None);
    assert_eq!(m["c"], None);
}

#[test]
fn mapping_with_boolean_keys() {
    let v: Value = from_str("true: yes\nfalse: no\n").unwrap();
    assert!(v.is_mapping());
}

#[test]
fn mapping_with_integer_keys() {
    let v: Value = from_str("1: one\n2: two\n").unwrap();
    assert!(v.is_mapping());
}

#[test]
fn mapping_preserves_order() {
    let v: Value = from_str("z: 1\na: 2\nm: 3\n").unwrap();
    let map = v.as_mapping().unwrap();
    let keys: Vec<&str> = map.keys().map(|s| s.as_str()).collect();
    assert_eq!(keys, vec!["z", "a", "m"]);
}

#[test]
fn deeply_nested_mapping() {
    #[derive(Debug, Deserialize)]
    struct A {
        b: B,
    }
    #[derive(Debug, Deserialize)]
    struct B {
        c: C,
    }
    #[derive(Debug, Deserialize)]
    struct C {
        value: i64,
    }

    let a: A = from_str("b:\n  c:\n    value: 42\n").unwrap();
    assert_eq!(a.b.c.value, 42);
}

#[test]
fn mapping_with_complex_string_keys() {
    let m: HashMap<String, String> =
        from_str("\"key with spaces\": value1\n\"key:with:colons\": value2\n").unwrap();
    assert_eq!(m["key with spaces"], "value1");
    assert_eq!(m["key:with:colons"], "value2");
}
