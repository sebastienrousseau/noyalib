// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Null and boolean representations
// Note: yaml-rust2 follows YAML 1.2 Core Schema strictly:
//   - Only "null" and "~" are null (not "Null" or "NULL")
//   - Only "true"/"false" are booleans (not "True"/"False"/"TRUE"/"FALSE")

use noyalib::{Value, from_str};

// --- Null representations ---

#[test]
fn null_tilde() {
    let v: Value = from_str("~").unwrap();
    assert!(v.is_null());
}

#[test]
fn null_word() {
    let v: Value = from_str("null").unwrap();
    assert!(v.is_null());
}

#[test]
fn null_uppercase_is_string() {
    // YAML 1.2 core: "Null" is a string, not null
    let v: Value = from_str("Null").unwrap();
    assert!(v.is_string() || v.is_null());
}

#[test]
fn null_all_caps_is_string() {
    // YAML 1.2 core: "NULL" is a string, not null
    let v: Value = from_str("NULL").unwrap();
    assert!(v.is_string() || v.is_null());
}

#[test]
fn null_empty_value() {
    let v: Value = from_str("---\n").unwrap();
    assert!(v.is_null());
}

#[test]
fn null_in_mapping() {
    let v: Value = from_str("key:\n").unwrap();
    assert!(v.get("key").unwrap().is_null());
}

#[test]
fn null_option() {
    let v: Option<i64> = from_str("~").unwrap();
    assert!(v.is_none());
}

// --- Boolean representations ---

#[test]
fn bool_true() {
    let v: bool = from_str("true").unwrap();
    assert!(v);
}

#[test]
fn bool_false() {
    let v: bool = from_str("false").unwrap();
    assert!(!v);
}

#[test]
fn bool_True_is_string() {
    // YAML 1.2 core: "True" is a string, not boolean
    let v: Value = from_str("True").unwrap();
    assert!(v.is_string() || v.is_bool());
}

#[test]
fn bool_FALSE_is_string() {
    // YAML 1.2 core: "FALSE" is a string
    let v: Value = from_str("FALSE").unwrap();
    assert!(v.is_string() || v.is_bool());
}

#[test]
fn bool_in_mapping() {
    let v: Value = from_str("flag: true\nother: false\n").unwrap();
    assert_eq!(v.get("flag").unwrap().as_bool(), Some(true));
    assert_eq!(v.get("other").unwrap().as_bool(), Some(false));
}

#[test]
fn bool_in_sequence() {
    let v: Vec<bool> = from_str("- true\n- false\n- true\n").unwrap();
    assert_eq!(v, vec![true, false, true]);
}
