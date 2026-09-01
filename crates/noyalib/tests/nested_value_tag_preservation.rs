//! A `Value` reached through serde (nested inside `Mapping`, `Sequence`
//! / `Vec<Value>`, or a struct field of type `Value`) must keep the
//! YAML tag the parser saw, the same way the top-level `Value` target
//! already does.
//!
//! Refs #350 (deserializer half): `from_str::<Mapping>("k: !vault
//! ABC\n")` used to give `{"k": String("ABC")}` — the tag silently
//! dropped — while `from_str::<Value>` of the same text correctly gave
//! `{"k": Tagged(!vault, "ABC")}`. The `Mapping` target reaches `Value`
//! through serde (`impl Deserialize for Mapping` deserializes each
//! entry's value via `Value`'s own `Deserialize`), and the AST
//! deserializer's `deserialize_any` used to see through a tag
//! transparently for every caller, discarding it before `Value`'s
//! visitor ever saw it.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Mapping, Tag, Value, from_str};
use serde::Deserialize;

fn assert_tagged(v: &Value, tag: &str, inner: &str) {
    match v {
        Value::Tagged(t) => {
            assert_eq!(t.tag(), &Tag::new(tag));
            assert_eq!(t.value().as_str(), Some(inner));
        }
        other => panic!("expected Tagged({tag}, {inner:?}), got {other:?}"),
    }
}

#[test]
fn mapping_target_keeps_the_tag() {
    let m: Mapping = from_str("k: !vault ABC\n").unwrap();
    let v = m.get("k").expect("key k present");
    assert_tagged(v, "!vault", "ABC");
}

#[test]
fn struct_field_of_type_value_keeps_the_tag() {
    #[derive(Debug, Deserialize)]
    struct S {
        k: Value,
    }
    let s: S = from_str("k: !vault ABC\n").unwrap();
    assert_tagged(&s.k, "!vault", "ABC");
}

#[test]
fn vec_of_value_keeps_the_tag() {
    let v: Vec<Value> = from_str("- !vault ABC\n- plain\n").unwrap();
    assert_eq!(v.len(), 2);
    assert_tagged(&v[0], "!vault", "ABC");
    assert_eq!(v[1], Value::String("plain".to_string()));
}

#[test]
fn untagged_scalar_is_unaffected() {
    let m: Mapping = from_str("k: ABC\n").unwrap();
    assert_eq!(m.get("k"), Some(&Value::String("ABC".to_string())));

    let v: Vec<Value> = from_str("- ABC\n- 1\n").unwrap();
    assert_eq!(v[0], Value::String("ABC".to_string()));
    assert_eq!(v[1], Value::from(1_i64));
}

#[test]
fn value_target_still_keeps_the_tag_directly() {
    // Unaffected control: the top-level `Value` target already worked
    // via the `parse_one_value` bypass, and must keep working.
    let v: Value = from_str("k: !vault ABC\n").unwrap();
    let Value::Mapping(m) = v else {
        panic!("expected a mapping")
    };
    assert_tagged(m.get("k").unwrap(), "!vault", "ABC");
}
