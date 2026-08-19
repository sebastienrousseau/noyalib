// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Flow collections (inline)

use std::collections::HashMap;

use noyalib::{Value, from_str};

#[test]
fn flow_sequence_basic() {
    let v: Vec<String> = from_str("[a, b, c]").unwrap();
    assert_eq!(v, vec!["a", "b", "c"]);
}

#[test]
fn flow_sequence_integers() {
    let v: Vec<i64> = from_str("[1, 2, 3]").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn flow_sequence_empty() {
    let v: Vec<i64> = from_str("[]").unwrap();
    assert!(v.is_empty());
}

#[test]
fn flow_sequence_nested() {
    let v: Vec<Vec<i64>> = from_str("[[1, 2], [3, 4]]").unwrap();
    assert_eq!(v, vec![vec![1, 2], vec![3, 4]]);
}

#[test]
fn flow_mapping_basic() {
    let m: HashMap<String, String> = from_str("{name: John, city: NYC}").unwrap();
    assert_eq!(m["name"], "John");
    assert_eq!(m["city"], "NYC");
}

#[test]
fn flow_mapping_empty() {
    let m: HashMap<String, i64> = from_str("{}").unwrap();
    assert!(m.is_empty());
}

#[test]
fn flow_mapping_quoted_keys() {
    let m: HashMap<String, String> = from_str("{\"key one\": v1, 'key two': v2}").unwrap();
    assert_eq!(m["key one"], "v1");
    assert_eq!(m["key two"], "v2");
}

#[test]
fn flow_mapping_nested() {
    let v: Value = from_str("{a: {b: 1}, c: {d: 2}}").unwrap();
    assert_eq!(v.get("a").unwrap().get("b").unwrap().as_i64(), Some(1));
    assert_eq!(v.get("c").unwrap().get("d").unwrap().as_i64(), Some(2));
}

#[test]
fn flow_sequence_in_block_mapping() {
    let v: Value = from_str("items: [1, 2, 3]\nname: test\n").unwrap();
    let items = v.get("items").unwrap().as_sequence().unwrap();
    assert_eq!(items.len(), 3);
}

#[test]
fn flow_mapping_in_block_sequence() {
    let v: Vec<HashMap<String, i64>> = from_str("- {a: 1, b: 2}\n- {c: 3}\n").unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0]["a"], 1);
    assert_eq!(v[1]["c"], 3);
}

#[test]
fn flow_mapping_separate_values() {
    let m: HashMap<String, Option<String>> =
        from_str("{\nunquoted : \"separate\",\nomitted value:,\n}\n").unwrap();
    assert_eq!(
        m.get("unquoted").cloned().flatten().as_deref(),
        Some("separate")
    );
    assert_eq!(m.get("omitted value").and_then(|v| v.clone()), None);
}

#[test]
fn flow_sequence_multiline() {
    let v: Vec<i64> = from_str("[\n  1,\n  2,\n  3\n]").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

// A flow collection wrapped over several lines, with its closing
// indicator on a line of its own at or below the parent key's column.
// This is ordinary hand-written config — a long `args:` or `ports:` list
// wrapped for width — and it used to be refused outright: the
// more-indented rule for flow continuation was applied to the closing
// indicator as well as to content.
#[test]
fn flow_sequence_wrapped_with_closer_at_the_parent_column() {
    let v: Value = from_str("ports: [\n  80,\n  443,\n]\n").unwrap();
    let Value::Mapping(m) = &v else {
        panic!("not a mapping")
    };
    assert_eq!(
        m.get("ports"),
        Some(&Value::Sequence(vec![Value::from(80), Value::from(443)]))
    );
}

#[test]
fn flow_mapping_wrapped_with_closer_at_the_parent_column() {
    let v: Value = from_str("env: {\n  A: 1,\n  B: 2,\n}\nx: 1\n").unwrap();
    let Value::Mapping(m) = &v else {
        panic!("not a mapping")
    };
    assert!(matches!(m.get("env"), Some(Value::Mapping(e)) if e.len() == 2));
    assert_eq!(m.get("x"), Some(&Value::from(1)));
}

#[test]
fn a_wrapped_flow_closer_may_sit_below_its_own_key() {
    // Inside a flow collection indentation closes nothing, so a closer
    // further left than the key that opened it is still unambiguous.
    let v: Value = from_str("a:\n  b: [\n    1,\n]\nc: 2\n").unwrap();
    let Value::Mapping(m) = &v else {
        panic!("not a mapping")
    };
    assert_eq!(m.get("c"), Some(&Value::from(2)));
}

#[test]
fn mixed_flow_and_block() {
    let v: Value = from_str("mapping:\n  key: [1, 2, 3]\nscalar: value\n").unwrap();
    assert!(v.get("mapping").unwrap().get("key").unwrap().is_sequence());
    assert_eq!(v.get("scalar").unwrap().as_str(), Some("value"));
}
