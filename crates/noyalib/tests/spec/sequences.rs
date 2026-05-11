// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Sequences

use noyalib::{Value, from_str};
use serde::Deserialize;

#[test]
fn block_sequence_of_scalars() {
    let v: Vec<String> = from_str("- a\n- b\n- c\n").unwrap();
    assert_eq!(v, vec!["a", "b", "c"]);
}

#[test]
fn block_sequence_of_integers() {
    let v: Vec<i64> = from_str("- 1\n- 2\n- 3\n").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn empty_block_sequence() {
    let v: Vec<String> = from_str("[]").unwrap();
    assert!(v.is_empty());
}

#[test]
fn nested_block_sequences() {
    let v: Vec<Vec<String>> = from_str("- - a\n  - b\n- - c\n  - d\n").unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], vec!["a", "b"]);
    assert_eq!(v[1], vec!["c", "d"]);
}

#[test]
fn sequence_in_block_sequence() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(untagged)]
    enum Item {
        Seq(Vec<String>),
        Str(String),
    }

    let v: Vec<Item> = from_str("- - s1_i1\n  - s1_i2\n- s2\n").unwrap();
    assert_eq!(v.len(), 2);
    match &v[0] {
        Item::Seq(inner) => assert_eq!(inner, &vec!["s1_i1".to_string(), "s1_i2".to_string()]),
        _ => panic!("first element should be a sequence"),
    }
    assert_eq!(v[1], Item::Str("s2".to_string()));
}

#[test]
fn sequence_of_mappings() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Player {
        name: String,
        hr: i32,
    }

    let v: Vec<Player> = from_str("- name: Mark\n  hr: 65\n- name: Sammy\n  hr: 63\n").unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].name, "Mark");
    assert_eq!(v[0].hr, 65);
    assert_eq!(v[1].name, "Sammy");
    assert_eq!(v[1].hr, 63);
}

#[test]
fn sequence_mixed_types() {
    let v: Vec<Value> = from_str("- 1\n- hello\n- true\n- null\n").unwrap();
    assert_eq!(v.len(), 4);
    assert_eq!(v[0].as_i64(), Some(1));
    assert_eq!(v[1].as_str(), Some("hello"));
    assert_eq!(v[2].as_bool(), Some(true));
    assert!(v[3].is_null());
}

#[test]
fn single_element_sequence() {
    let v: Vec<i64> = from_str("- 42\n").unwrap();
    assert_eq!(v, vec![42]);
}

#[test]
fn sequence_with_null_elements() {
    let v: Vec<Option<i64>> = from_str("- 1\n- ~\n- 3\n").unwrap();
    assert_eq!(v, vec![Some(1), None, Some(3)]);
}
