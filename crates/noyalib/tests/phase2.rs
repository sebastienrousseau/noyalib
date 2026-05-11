//! Phase 2 feature tests: anchors/aliases, merge keys, multi-document support.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::BTreeMap;

use noyalib::{Value, from_str, load_all, load_all_as, try_load_all};
use serde::Deserialize;

// ============================================================================
// Anchor and Alias Tests
// ============================================================================

#[test]
fn test_simple_anchor_alias() {
    let yaml = "first: &anchor 42\nsecond: *anchor\n";
    let value: BTreeMap<String, i32> = from_str(yaml).unwrap();
    assert_eq!(value.get("first"), Some(&42));
    assert_eq!(value.get("second"), Some(&42));
}

#[test]
fn test_anchor_alias_with_mapping() {
    let yaml = r#"
defaults: &defaults
  timeout: 30
  retries: 3

production:
  <<: *defaults
  host: prod.example.com

development:
  <<: *defaults
  host: localhost
"#;
    let value: Value = from_str(yaml).unwrap();

    let prod = value.get("production").unwrap();
    assert_eq!(prod.get("timeout").unwrap().as_i64(), Some(30));
    assert_eq!(prod.get("retries").unwrap().as_i64(), Some(3));
    assert_eq!(prod.get("host").unwrap().as_str(), Some("prod.example.com"));

    let dev = value.get("development").unwrap();
    assert_eq!(dev.get("timeout").unwrap().as_i64(), Some(30));
    assert_eq!(dev.get("host").unwrap().as_str(), Some("localhost"));
}

#[test]
fn test_anchor_alias_sequence() {
    let yaml = r#"
items: &items
  - one
  - two
  - three
copy: *items
"#;
    let value: Value = from_str(yaml).unwrap();

    let items = value.get("items").unwrap().as_sequence().unwrap();
    let copy = value.get("copy").unwrap().as_sequence().unwrap();

    assert_eq!(items.len(), 3);
    assert_eq!(copy.len(), 3);
    assert_eq!(items[0].as_str(), Some("one"));
    assert_eq!(copy[0].as_str(), Some("one"));
}

// ============================================================================
// Merge Key Tests
// ============================================================================

#[test]
fn test_merge_key_basic() {
    let yaml = r#"
base: &base
  key1: value1
  key2: value2

derived:
  <<: *base
  key3: value3
"#;
    let value: Value = from_str(yaml).unwrap();
    let derived = value.get("derived").unwrap();

    assert_eq!(derived.get("key1").unwrap().as_str(), Some("value1"));
    assert_eq!(derived.get("key2").unwrap().as_str(), Some("value2"));
    assert_eq!(derived.get("key3").unwrap().as_str(), Some("value3"));
}

#[test]
fn test_merge_key_override() {
    let yaml = r#"
base: &base
  key1: original
  key2: value2

derived:
  <<: *base
  key1: overridden
"#;
    let value: Value = from_str(yaml).unwrap();
    let derived = value.get("derived").unwrap();

    // Explicit key should override merged key
    assert_eq!(derived.get("key1").unwrap().as_str(), Some("overridden"));
    assert_eq!(derived.get("key2").unwrap().as_str(), Some("value2"));
}

#[test]
fn test_merge_key_multiple_sources() {
    let yaml = r#"
source1: &s1
  a: 1
  b: 2

source2: &s2
  c: 3
  d: 4

combined:
  <<: [*s1, *s2]
  e: 5
"#;
    let value: Value = from_str(yaml).unwrap();
    let combined = value.get("combined").unwrap();

    assert_eq!(combined.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(combined.get("b").unwrap().as_i64(), Some(2));
    assert_eq!(combined.get("c").unwrap().as_i64(), Some(3));
    assert_eq!(combined.get("d").unwrap().as_i64(), Some(4));
    assert_eq!(combined.get("e").unwrap().as_i64(), Some(5));
}

#[test]
fn test_merge_key_precedence() {
    let yaml = r#"
first: &first
  key: from_first

second: &second
  key: from_second

merged:
  <<: [*first, *second]
"#;
    let value: Value = from_str(yaml).unwrap();
    let merged = value.get("merged").unwrap();

    // First source in the array should take precedence
    assert_eq!(merged.get("key").unwrap().as_str(), Some("from_first"));
}

// ============================================================================
// Multi-Document Tests
// ============================================================================

#[test]
fn test_load_all_basic() {
    let yaml = "---\nfirst: 1\n---\nsecond: 2\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(Result::ok).collect();

    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].get("first").unwrap().as_i64(), Some(1));
    assert_eq!(docs[1].get("second").unwrap().as_i64(), Some(2));
}

#[test]
fn test_load_all_single_document() {
    let yaml = "name: test\nvalue: 42\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(Result::ok).collect();

    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].get("name").unwrap().as_str(), Some("test"));
}

#[test]
fn test_load_all_empty() {
    let yaml = "";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(Result::ok).collect();
    assert!(docs.is_empty());
}

#[test]
fn test_try_load_all() {
    let yaml = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let iter = try_load_all(yaml).unwrap();

    assert_eq!(iter.len(), 3);

    let docs: Vec<Value> = iter.filter_map(Result::ok).collect();
    assert_eq!(docs.len(), 3);
}

#[test]
fn test_load_all_as_typed() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Config {
        name: String,
        value: i32,
    }

    let yaml = "---\nname: first\nvalue: 1\n---\nname: second\nvalue: 2\n";
    let configs: Vec<Config> = load_all_as(yaml).unwrap();

    assert_eq!(configs.len(), 2);
    assert_eq!(configs[0].name, "first");
    assert_eq!(configs[0].value, 1);
    assert_eq!(configs[1].name, "second");
    assert_eq!(configs[1].value, 2);
}

#[test]
fn test_load_all_with_different_types() {
    let yaml = "---\n42\n---\nhello\n---\n- 1\n- 2\n- 3\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(Result::ok).collect();

    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].as_i64(), Some(42));
    assert_eq!(docs[1].as_str(), Some("hello"));
    assert!(docs[2].is_sequence());
}

#[test]
fn test_load_all_with_explicit_end() {
    let yaml = "---\nfirst: 1\n...\n---\nsecond: 2\n...\n";
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(Result::ok).collect();

    assert_eq!(docs.len(), 2);
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[test]
fn test_multi_doc_with_anchors() {
    let yaml = r#"---
common: &common
  timeout: 30

service1:
  <<: *common
  name: svc1
---
common: &common
  timeout: 60

service2:
  <<: *common
  name: svc2
"#;
    let docs: Vec<Value> = load_all(yaml).unwrap().filter_map(Result::ok).collect();

    assert_eq!(docs.len(), 2);

    let svc1 = docs[0].get("service1").unwrap();
    assert_eq!(svc1.get("timeout").unwrap().as_i64(), Some(30));
    assert_eq!(svc1.get("name").unwrap().as_str(), Some("svc1"));

    let svc2 = docs[1].get("service2").unwrap();
    assert_eq!(svc2.get("timeout").unwrap().as_i64(), Some(60));
    assert_eq!(svc2.get("name").unwrap().as_str(), Some("svc2"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_nested_merge() {
    let yaml = r#"
base1: &base1
  level1:
    a: 1
    b: 2

base2: &base2
  <<: *base1
  level2: extra

final:
  <<: *base2
  level3: more
"#;
    let value: Value = from_str(yaml).unwrap();
    let final_val = value.get("final").unwrap();

    // Should have merged keys from base1 through base2
    assert!(final_val.get("level1").is_some());
    assert_eq!(final_val.get("level2").unwrap().as_str(), Some("extra"));
    assert_eq!(final_val.get("level3").unwrap().as_str(), Some("more"));
}

#[test]
fn test_document_iterator_size_hint() {
    let yaml = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let iter = try_load_all(yaml).unwrap();

    assert_eq!(iter.size_hint(), (3, Some(3)));
    assert_eq!(iter.len(), 3);
}

// ============================================================================
// Additional Coverage Tests for loader.rs
// ============================================================================

#[test]
fn test_document_iterator_is_empty() {
    let yaml = "---\na: 1\n";
    let iter = try_load_all(yaml).unwrap();
    assert!(!iter.is_empty());

    // Empty iteration after consuming
    let empty_iter = try_load_all("").unwrap();
    assert!(empty_iter.is_empty());
}

#[test]
fn test_load_all_with_parse_error() {
    // Genuinely invalid YAML — an unterminated double-quoted scalar.
    // (`:\n:\n:` was previously used here, but per YAML 1.2 that is a
    // valid three-entry block mapping with duplicate empty keys; the
    // earlier "error" was a parser bug masking a real YAML construct.)
    let invalid_yaml = "key: \"unterminated";
    let result = load_all(invalid_yaml);
    if let Ok(iter) = result {
        assert!(iter.is_empty() || iter.count() == 0);
    }
}

#[test]
fn test_merge_key_with_array_of_maps() {
    let yaml = r#"
base1: &base1
  key1: value1

base2: &base2
  key2: value2

merged:
  <<: [*base1, *base2]
  key3: value3
"#;
    let value: Value = from_str(yaml).unwrap();
    let merged = value.get("merged").unwrap();

    assert_eq!(merged.get("key1").unwrap().as_str(), Some("value1"));
    assert_eq!(merged.get("key2").unwrap().as_str(), Some("value2"));
    assert_eq!(merged.get("key3").unwrap().as_str(), Some("value3"));
}

#[test]
fn test_mapping_with_integer_keys() {
    let yaml = r#"
1: one
2: two
3: three
"#;
    let value: Value = from_str(yaml).unwrap();
    assert_eq!(value.get("1").unwrap().as_str(), Some("one"));
    assert_eq!(value.get("2").unwrap().as_str(), Some("two"));
}

#[test]
fn test_mapping_with_boolean_keys() {
    let yaml = r#"
true: yes_value
false: no_value
"#;
    let value: Value = from_str(yaml).unwrap();
    assert_eq!(value.get("true").unwrap().as_str(), Some("yes_value"));
    assert_eq!(value.get("false").unwrap().as_str(), Some("no_value"));
}
