// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Streaming-path coverage for the YAML 1.2 failsafe-schema
//! `!!`-prefixed tags (`!!int`, `!!float`, `!!str`, `!!bool`,
//! `!!null`, `!!seq`, `!!map`). The streaming deserialiser
//! short-circuits these to the appropriate visitor without
//! enum-dispatch.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::from_str;

#[test]
fn int_tag_resolves_to_integer() {
    let n: i64 = from_str("!!int 42").unwrap();
    assert_eq!(n, 42);
}

#[test]
fn float_tag_resolves_to_float() {
    let f: f64 = from_str("!!float 2.5").unwrap();
    assert!((f - 2.5).abs() < 1e-9);
}

#[test]
fn str_tag_keeps_numeric_as_string() {
    let s: String = from_str("!!str 8080").unwrap();
    assert_eq!(s, "8080");
}

#[test]
fn bool_tag_resolves_to_bool() {
    let b: bool = from_str("!!bool true").unwrap();
    assert!(b);
}

#[test]
fn null_tag_resolves_to_unit() {
    #[derive(Debug, serde::Deserialize)]
    struct N {
        x: Option<i64>,
    }
    let n: N = from_str("x: !!null ~").unwrap();
    assert!(n.x.is_none());
}

#[test]
fn seq_tag_resolves_to_sequence() {
    let v: Vec<i64> = from_str("!!seq [1, 2, 3]").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn map_tag_resolves_to_mapping() {
    #[derive(Debug, serde::Deserialize)]
    struct Doc {
        a: i64,
        b: i64,
    }
    let d: Doc = from_str("!!map {a: 1, b: 2}").unwrap();
    assert_eq!(d.a, 1);
    assert_eq!(d.b, 2);
}
