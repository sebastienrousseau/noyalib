//! Regression coverage for `Value::Tagged` losing its tag through the
//! generic `Serialize` pipeline (Refs #350, serializer half).
//!
//! `to_string`/`to_value` on a `Value::Tagged` used to emit a degenerate
//! single-entry mapping keyed by the tag string (`k:\n  "!vault": ABC`)
//! instead of a real YAML tag (`k: !vault ABC`) -- `Value::Tagged`'s own
//! inline `Serialize` arm routes through `serialize_map` (the shape a
//! generic serializer with no YAML-tag concept, like `serde_json`, needs
//! for interop), and nothing on the way back into a `Value` recognised
//! that shape as a tag. This crate's own [`Serializer`](noyalib) now
//! reconstructs `Value::Tagged` from the marker newtype
//! `TaggedValue::serialize` wraps that single-entry map in (never from
//! the shape alone, see #377), so `to_string` agrees with the
//! tag-preserving [`to_string_value`].

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Mapping, Tag, TaggedValue, Value, from_str, to_string, to_string_value};

#[test]
fn tagged_scalar_exact_output() {
    let v: Value = from_str("k: !vault ABC\n").unwrap();
    let out = to_string(&v).unwrap();
    assert_eq!(out, "k: !vault ABC");
    // `to_string_value` (the direct, `Serialize`-pipeline-bypassing path)
    // must agree.
    assert_eq!(to_string_value(&v).unwrap(), out);
}

#[test]
fn tagged_scalar_round_trips() {
    let original: Value = from_str("!vault ABC\n").unwrap();
    let out = to_string(&original).unwrap();
    let back: Value = from_str(&out).unwrap();
    assert_eq!(back, original);
}

#[test]
fn tagged_sequence_round_trips() {
    let original: Value = from_str("!tag\n- a\n- b\n").unwrap();
    let out = to_string(&original).unwrap();
    let back: Value = from_str(&out).unwrap();
    assert_eq!(back, original);
}

#[test]
fn tagged_mapping_round_trips() {
    let original: Value = from_str("!tag\na: 1\nb: 2\n").unwrap();
    let out = to_string(&original).unwrap();
    let back: Value = from_str(&out).unwrap();
    assert_eq!(back, original);
}

#[test]
fn tagged_mapping_nested_under_a_key_round_trips() {
    let mut inner = Mapping::new();
    let _ = inner.insert("a", Value::from(1));
    let _ = inner.insert("b", Value::from(2));
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!tag"),
        Value::Mapping(inner),
    )));
    let mut outer = Mapping::new();
    let _ = outer.insert("k", tagged);
    let original = Value::Mapping(outer);

    let out = to_string(&original).unwrap();
    assert_eq!(out, "k: !tag\n  a: 1\n  b: 2");
    let back: Value = from_str(&out).unwrap();
    assert_eq!(back, original);
}

#[test]
fn serde_json_interop_still_sees_a_single_entry_map() {
    // The single-entry-map wire form is still what a generic serializer
    // with no YAML-tag concept receives -- only noyalib's own serializer
    // recognises the shape and reconstructs `Value::Tagged`.
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!Color"),
        Value::from("#ff8800"),
    )));
    let json = serde_json::to_string(&tagged).unwrap();
    assert_eq!(json, r##"{"!Color":"#ff8800"}"##);
}
