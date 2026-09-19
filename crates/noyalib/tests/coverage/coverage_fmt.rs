//! Formatting wrapper coverage tests — into_inner, Deref, From, Debug, empty
//! collections, edge cases.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::BTreeMap;

use noyalib::fmt::{
    Commented, FlowMap, FlowSeq, FoldStr, FoldString, LitStr, LitString, SpaceAfter,
};
use noyalib::{from_str, to_string};

// ============================================================================
// into_inner
// ============================================================================

#[test]
fn flow_seq_into_inner() {
    let v = FlowSeq(vec![1, 2, 3]);
    assert_eq!(v.into_inner(), vec![1, 2, 3]);
}

#[test]
fn flow_map_into_inner() {
    let mut m = BTreeMap::new();
    let _ = m.insert("a", 1);
    let v = FlowMap(m.clone());
    assert_eq!(v.into_inner(), m);
}

#[test]
fn lit_str_into_inner() {
    let v = LitStr("hello");
    assert_eq!(v.into_inner(), "hello");
}

#[test]
fn lit_string_into_inner() {
    let v = LitString("hello".to_string());
    assert_eq!(v.into_inner(), "hello");
}

#[test]
fn fold_str_into_inner() {
    let v = FoldStr("hello");
    assert_eq!(v.into_inner(), "hello");
}

#[test]
fn fold_string_into_inner() {
    let v = FoldString("hello".to_string());
    assert_eq!(v.into_inner(), "hello");
}

#[test]
fn commented_into_inner() {
    let v = Commented::new(42, "test");
    assert_eq!(v.into_inner(), 42);
}

#[test]
fn space_after_into_inner() {
    let v = SpaceAfter(42);
    assert_eq!(v.into_inner(), 42);
}

// ============================================================================
// Deref
// ============================================================================

#[test]
fn flow_seq_deref() {
    let v = FlowSeq(vec![1, 2, 3]);
    assert_eq!(v.len(), 3); // Deref to Vec
}

#[test]
fn flow_map_deref() {
    let mut m = BTreeMap::new();
    let _ = m.insert("a", 1);
    let v = FlowMap(m);
    assert_eq!(v.len(), 1); // Deref to BTreeMap
}

#[test]
fn lit_str_deref() {
    let v = LitStr("hello");
    assert_eq!(v.len(), 5); // Deref to str
}

#[test]
fn lit_string_deref() {
    let v = LitString("hello".to_string());
    assert_eq!(v.len(), 5);
}

#[test]
fn fold_str_deref() {
    let v = FoldStr("hello");
    assert_eq!(v.len(), 5);
}

#[test]
fn fold_string_deref() {
    let v = FoldString("hello".to_string());
    assert_eq!(v.len(), 5);
}

#[test]
fn commented_deref() {
    let v = Commented::new(42i64, "test");
    let inner: &i64 = &v;
    assert_eq!(*inner, 42);
}

#[test]
fn space_after_deref() {
    let v = SpaceAfter(42i64);
    let inner: &i64 = &v;
    assert_eq!(*inner, 42);
}

// ============================================================================
// From conversions
// ============================================================================

#[test]
fn flow_seq_from() {
    let v: FlowSeq<Vec<i64>> = FlowSeq::from(vec![1, 2]);
    assert_eq!(v.0, vec![1, 2]);
}

#[test]
fn flow_map_from() {
    let m = BTreeMap::new();
    let v: FlowMap<BTreeMap<String, i64>> = FlowMap::from(m);
    assert!(v.0.is_empty());
}

#[test]
fn lit_str_from() {
    let v = LitStr::from("hello");
    assert_eq!(v.0, "hello");
}

#[test]
fn lit_string_from_string() {
    let v = LitString::from("hello".to_string());
    assert_eq!(v.0, "hello");
}

#[test]
fn lit_string_from_str() {
    let v = LitString::from("hello");
    assert_eq!(v.0, "hello");
}

#[test]
fn fold_str_from() {
    let v = FoldStr::from("hello");
    assert_eq!(v.0, "hello");
}

#[test]
fn fold_string_from_string() {
    let v = FoldString::from("hello".to_string());
    assert_eq!(v.0, "hello");
}

#[test]
fn fold_string_from_str() {
    let v = FoldString::from("hello");
    assert_eq!(v.0, "hello");
}

#[test]
fn space_after_from() {
    let v: SpaceAfter<i64> = SpaceAfter::from(42);
    assert_eq!(v.0, 42);
}

// ============================================================================
// Debug impls
// ============================================================================

#[test]
fn debug_impls() {
    assert!(format!("{:?}", FlowSeq(vec![1])).contains("FlowSeq"));
    assert!(format!("{:?}", FlowMap(BTreeMap::<String, i64>::new())).contains("FlowMap"));
    assert!(format!("{:?}", LitStr("hello")).contains("LitStr"));
    assert!(format!("{:?}", LitString("hello".into())).contains("LitString"));
    assert!(format!("{:?}", FoldStr("hello")).contains("FoldStr"));
    assert!(format!("{:?}", FoldString("hello".into())).contains("FoldString"));
    assert!(format!("{:?}", Commented::new(42, "test")).contains("Commented"));
    assert!(format!("{:?}", SpaceAfter(42)).contains("SpaceAfter"));
}

// ============================================================================
// Clone / PartialEq / Eq / Hash
// ============================================================================

#[test]
fn clone_and_eq() {
    let v = FlowSeq(vec![1, 2]);
    let v2 = v.clone();
    assert_eq!(v, v2);

    let v = LitString("hello".into());
    let v2 = v.clone();
    assert_eq!(v, v2);

    let v = FoldString("hello".into());
    let v2 = v.clone();
    assert_eq!(v, v2);

    let v = Commented::new(42, "test");
    let v2 = v.clone();
    assert_eq!(v, v2);

    let v = SpaceAfter(42);
    let v2 = v.clone();
    assert_eq!(v, v2);
}

// ============================================================================
// Empty collections
// ============================================================================

#[test]
fn flow_seq_empty() {
    let v: FlowSeq<Vec<i64>> = FlowSeq(vec![]);
    let yaml = to_string(&v).unwrap();
    assert_eq!(yaml.trim(), "[]");
}

#[test]
fn flow_map_empty() {
    let v: FlowMap<BTreeMap<String, i64>> = FlowMap(BTreeMap::new());
    let yaml = to_string(&v).unwrap();
    assert_eq!(yaml.trim(), "{}");
}

// ============================================================================
// Deserialization roundtrips
// ============================================================================

#[test]
fn commented_deserialize_loses_comment() {
    let v = Commented::new(42i64, "important");
    let yaml = to_string(&v).unwrap();
    let parsed: Commented<i64> = from_str(&yaml).unwrap();
    assert_eq!(parsed.value, 42);
    assert_eq!(parsed.comment, ""); // Comment lost on roundtrip
}

#[test]
fn space_after_deserialize() {
    let v = SpaceAfter(42i64);
    let yaml = to_string(&v).unwrap();
    let parsed: SpaceAfter<i64> = from_str(&yaml).unwrap();
    assert_eq!(parsed.0, 42);
}

#[test]
fn flow_seq_deserialize() {
    let v: FlowSeq<Vec<i64>> = from_str("[1, 2, 3]").unwrap();
    assert_eq!(v.0, vec![1, 2, 3]);
}

#[test]
fn flow_map_deserialize() {
    let v: FlowMap<BTreeMap<String, i64>> = from_str("{a: 1, b: 2}").unwrap();
    assert_eq!(v.0["a"], 1);
    assert_eq!(v.0["b"], 2);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn lit_str_empty_string() {
    let v = LitStr("");
    let yaml = to_string(&v).unwrap();
    let parsed: LitString = from_str(&yaml).unwrap();
    assert_eq!(parsed.0, "");
}

#[test]
fn fold_str_empty_string() {
    let v = FoldStr("");
    let yaml = to_string(&v).unwrap();
    let parsed: FoldString = from_str(&yaml).unwrap();
    assert_eq!(parsed.0, "");
}

#[test]
fn lit_str_with_trailing_newline() {
    let v = LitStr("hello\nworld\n");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.starts_with('|'));
    assert!(!yaml.starts_with("|-"));
}

#[test]
fn fold_str_with_trailing_newlines() {
    let v = FoldStr("hello\nworld\n\n");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.starts_with(">+"));
}

#[test]
fn commented_with_empty_comment() {
    let v = Commented::new(42i64, "");
    let yaml = to_string(&v).unwrap();
    assert!(yaml.contains("42"));
}
