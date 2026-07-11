// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for `value/serde_impl.rs` — the tag-preserving map
//! deserialization path (`$__noyalib_*` magic fields), the enum-from-
//! string deserializer, and the lossless-u64 visitor arm.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::{Value, from_value, to_value};
use serde::Deserialize;

#[test]
fn tag_preserving_magic_map_reconstructs_tagged_value() {
    // When a `Value::Tagged` is serialized to a *foreign* serde format
    // (one with no native tag concept), it becomes the tag-preserving
    // magic map `{$__noyalib_tag, $__noyalib_value}`. Reading that map
    // back through `Value::deserialize` (here via serde_json, so the
    // `Value`-clone fast path in `from_value` is bypassed) exercises the
    // visit_map tag-reconstruction branch and rebuilds the tagged node.
    let json = serde_json::json!({
        "$__noyalib_tag": "!Celsius",
        "$__noyalib_value": 42_i64,
    });
    let back: Value = serde_json::from_value(json).unwrap();
    let t = back
        .as_tagged()
        .expect("magic map must rebuild a tagged value");
    assert_eq!(t.tag().as_str(), "!Celsius");
    assert_eq!(t.value().as_i64(), Some(42));

    // A magic map missing the `$__noyalib_value` entry is a hard error.
    let bad = serde_json::json!({ "$__noyalib_tag": "!x", "other": 1 });
    let res: Result<Value, _> = serde_json::from_value(bad);
    assert!(res.is_err(), "missing value field must error");
}

#[test]
fn enum_deserializes_from_bare_string() {
    // A unit enum variant deserialized from a bare `Value::String`
    // exercises the string-enum deserializer arm.
    #[derive(Debug, Deserialize, PartialEq)]
    enum Level {
        Info,
        Warn,
    }
    let v = Value::String("Warn".into());
    let got: Level = from_value(&v).unwrap();
    assert_eq!(got, Level::Warn);
}

#[cfg(feature = "lossless-u64")]
#[test]
fn unsigned_value_visits_u64() {
    // A `Value` holding an `Unsigned` number deserialized into a u64
    // drives the lossless-u64 `visit_u64` arm.
    let v = to_value(&u64::MAX).unwrap();
    let n: u64 = from_value(&v).unwrap();
    assert_eq!(n, u64::MAX);
}
