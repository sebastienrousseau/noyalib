// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Tags

use noyalib::{Value, from_str};
use serde::Deserialize;

#[test]
fn explicit_str_tag() {
    let v: String = from_str("!!str 42").unwrap();
    assert_eq!(v, "42");
}

#[test]
fn explicit_int_tag() {
    let v: i64 = from_str("!!int 42").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn explicit_float_tag() {
    let v: f64 = from_str("!!float 2.75").unwrap();
    assert!((v - 2.75).abs() < 0.001);
}

#[test]
fn explicit_bool_tag() {
    let v: bool = from_str("!!bool true").unwrap();
    assert!(v);
}

#[test]
fn explicit_null_tag() {
    let v: Option<i64> = from_str("!!null ~").unwrap();
    assert!(v.is_none());
}

#[test]
fn explicit_seq_tag() {
    let v: Vec<String> = from_str("!!seq\n- a\n- b\n").unwrap();
    assert_eq!(v, vec!["a", "b"]);
}

#[test]
fn explicit_map_tag() {
    use std::collections::HashMap;
    let m: HashMap<String, i64> = from_str("!!map\na: 1\nb: 2\n").unwrap();
    assert_eq!(m.len(), 2);
    assert_eq!(m["a"], 1);
    assert_eq!(m["b"], 2);
}

#[test]
fn tagged_sequence_with_scalar_tags() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct TaggedSeq(String, String, i64, String);

    let v: TaggedSeq = from_str("- !!str a\n- b\n- !!int 42\n- d\n").unwrap();
    assert_eq!(v, TaggedSeq("a".into(), "b".into(), 42, "d".into()));
}

#[test]
fn tag_in_value() {
    let v: Value = from_str("!!str hello").unwrap();
    assert!(v.is_tagged() || v.is_string());
}

#[test]
fn unordered_set_as_map() {
    use std::collections::HashMap;
    let m: HashMap<String, Option<String>> =
        from_str("--- !!set\n? Mark McGwire\n? Sammy Sosa\n").unwrap();
    assert!(m.contains_key("Mark McGwire"));
    assert!(m.contains_key("Sammy Sosa"));
    assert_eq!(m["Mark McGwire"], None);
    assert_eq!(m["Sammy Sosa"], None);
}

#[test]
fn tags_for_block_objects() {
    #[derive(Debug, Deserialize)]
    struct Doc {
        foo: Vec<Value>,
    }

    let d: Doc = from_str("foo: !!seq\n  - !!str a\n  - !!map\n    key: !!str value\n").unwrap();
    assert_eq!(d.foo.len(), 2);
}
