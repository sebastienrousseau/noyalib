//! Empty / comment-only / null documents deserializing into map-shaped
//! targets.
//!
//! Refs #349: an empty stream is the YAML null document. `serde_yaml`
//! deserializes null into a map or a struct as "no entries" instead of
//! erroring; `noyalib` used to reject it with `type mismatch: expected
//! mapping, found other`. `deserialize_any` (and therefore an `Option`
//! or scalar target) is unaffected — it still yields `Value::Null` /
//! `None`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Mapping, Value, from_str};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default, PartialEq)]
#[serde(default)]
struct S {
    title: Option<String>,
}

#[test]
fn comment_only_document_is_empty_mapping() {
    let m: Mapping = from_str("# c\n").expect("comment-only document should deserialize");
    assert!(m.is_empty());
}

#[test]
fn empty_document_is_empty_mapping() {
    let m: Mapping = from_str("").expect("empty document should deserialize");
    assert!(m.is_empty());
}

#[test]
fn empty_document_is_default_struct() {
    let s: S = from_str("").expect("empty document should deserialize into the default struct");
    assert_eq!(s, S::default());
    assert_eq!(s.title, None);
}

#[test]
fn empty_document_as_value_is_still_null() {
    // deserialize_any / the `Value` target must not change: an empty
    // document is Value::Null, not Value::Mapping(Mapping::new()).
    let v: Value = from_str("").expect("empty document parses as Value::Null");
    assert!(v.is_null());
}

#[test]
fn whitespace_only_document_behaves_like_empty() {
    let m: Mapping = from_str("   \n\t\n  ").expect("whitespace-only document should deserialize");
    assert!(m.is_empty());

    let v: Value = from_str("   \n\t\n  ").expect("whitespace-only document parses as Value::Null");
    assert!(v.is_null());
}

#[test]
fn document_end_marker_only_behaves_like_empty() {
    let m: Mapping = from_str("---\n").expect("a bare `---` document should deserialize");
    assert!(m.is_empty());

    let v: Value = from_str("---\n").expect("a bare `---` document parses as Value::Null");
    assert!(v.is_null());
}
