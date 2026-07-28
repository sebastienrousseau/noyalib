//! Formatting wrapper tests.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::BTreeMap;

use noyalib::fmt::{
    Commented, FlowMap, FlowSeq, FoldStr, FoldString, LitStr, LitString, SpaceAfter,
};
use noyalib::{from_str, to_string};

#[test]
fn test_flow_seq_serialize() {
    let val = FlowSeq(vec![1i64, 2, 3]);
    let yaml = to_string(&val).unwrap();
    assert_eq!(yaml.trim(), "[1, 2, 3]");
}

#[test]
fn test_flow_map_serialize() {
    // Use BTreeMap for deterministic ordering
    let mut map = BTreeMap::new();
    let _ = map.insert("a".to_string(), 1i64);
    let _ = map.insert("b".to_string(), 2);
    let val = FlowMap(map);
    let yaml = to_string(&val).unwrap();
    assert_eq!(yaml.trim(), "{a: 1, b: 2}");
}

#[test]
fn test_lit_str_serialize() {
    let val = LitStr("line1\nline2");
    let yaml = to_string(&val).unwrap();
    assert!(yaml.starts_with("|-"));
    assert!(yaml.contains("  line1"));
    assert!(yaml.contains("  line2"));
}

#[test]
fn test_fold_str_serialize() {
    let val = FoldStr("line1\nline2");
    let yaml = to_string(&val).unwrap();
    assert!(yaml.starts_with(">-"));
    assert!(yaml.contains("  line1"));
    assert!(yaml.contains("  line2"));
}

#[test]
fn test_commented_serialize() {
    let val = Commented::new(42i64, "max");
    let yaml = to_string(&val).unwrap();
    assert!(yaml.contains("42 # max"), "got: {yaml}");
}

#[test]
fn test_space_after_serialize() {
    let val = SpaceAfter(42i64);
    let yaml = to_string(&val).unwrap();
    // Should end with extra newline (blank line after value)
    assert!(yaml.ends_with('\n'), "got: {:?}", yaml);
    assert_eq!(yaml, "42\n");
}

#[test]
fn test_lit_string_deserialize() {
    let yaml = "hello world";
    let val: LitString = from_str(yaml).unwrap();
    assert_eq!(val.0, "hello world");
}

#[test]
fn test_fold_string_deserialize() {
    let yaml = "hello world";
    let val: FoldString = from_str(yaml).unwrap();
    assert_eq!(val.0, "hello world");
}

#[test]
fn test_flow_seq_in_struct() {
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Config {
        tags: FlowSeq<Vec<String>>,
    }

    let config = Config {
        tags: FlowSeq(vec!["a".into(), "b".into(), "c".into()]),
    };

    let yaml = to_string(&config).unwrap();
    assert!(yaml.contains("[a, b, c]"), "got: {yaml}");

    // Deserialize back (transparent)
    let parsed: Config = from_str(&yaml).unwrap();
    assert_eq!(parsed.tags.0, vec!["a", "b", "c"]);
}

#[test]
fn test_flow_map_in_struct() {
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Config {
        env: FlowMap<BTreeMap<String, String>>,
    }

    let mut env = BTreeMap::new();
    let _ = env.insert("HOME".into(), "/home/user".into());
    let config = Config { env: FlowMap(env) };

    let yaml = to_string(&config).unwrap();
    assert!(yaml.contains("HOME:"), "got: {yaml}");
    assert!(yaml.contains("{"), "got: {yaml}");

    let parsed: Config = from_str(&yaml).unwrap();
    assert_eq!(parsed.env.0["HOME"], "/home/user");
}
