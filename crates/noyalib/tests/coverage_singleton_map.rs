// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for the `with/singleton_map*` serde adapters: the empty-word
//! edge of `to_pascal_case`, plus behavioural regression tests that a
//! custom tag survives the recursive / key-transform serialize walks.
//!
//! Note: the `Value::Tagged` arms of `transform_to_singleton_map`
//! (recursive.rs:51-54) and `transform_value_keys` (with.rs:234-237) are
//! **defensive-unreachable via the serialize path** — `to_value` never
//! emits a `Value::Tagged` from a Rust type (Value's own serializer
//! flattens a tagged node into a `{tag, value}` mapping), so the walk
//! never encounters that variant. They are effective-100% exclusions.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::{Tag, TaggedValue, Value, to_string};
use serde::Serialize;

/// A value tree containing a tagged node, used to assert that a custom
/// tag survives the singleton-map serialize walks end-to-end.
fn tagged_seq() -> Value {
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        Value::from(7_i64),
    )));
    Value::Sequence(vec![tagged])
}

#[test]
fn recursive_serialize_transforms_tagged_node() {
    #[derive(Serialize)]
    struct W {
        #[serde(with = "noyalib::with::singleton_map_recursive")]
        v: Value,
    }
    let out = to_string(&W { v: tagged_seq() }).unwrap();
    // The custom tag survives the recursive transform.
    assert!(out.contains("!custom"), "{out}");
    assert!(out.contains('7'), "{out}");
}

#[test]
fn with_serialize_transforms_tagged_node() {
    // `serialize_with` walks the value tree applying the key transform;
    // the walk must recurse through a `Value::Tagged` node too.
    struct KeyWrap;
    impl Serialize for KeyWrap {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            noyalib::with::singleton_map_with::serialize_with(
                &tagged_seq(),
                s,
                noyalib::with::singleton_map_with::to_lowercase,
            )
        }
    }
    let out = to_string(&KeyWrap).unwrap();
    // The custom tag survives the key-transform walk.
    assert!(out.contains("!custom"), "{out}");
    assert!(out.contains('7'), "{out}");
}

#[test]
fn to_pascal_case_handles_empty_words() {
    use noyalib::with::singleton_map_with::to_pascal_case;
    // Leading / doubled / trailing underscores yield empty split words,
    // exercising the `chars.next() == None` branch.
    assert_eq!(to_pascal_case("_leading"), "Leading");
    assert_eq!(to_pascal_case("a__b"), "AB");
    assert_eq!(to_pascal_case("trailing_"), "Trailing");
    assert_eq!(to_pascal_case(""), "");
}
