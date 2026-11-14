// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! YAML 1.2.2 §6.8.2.2: a local tag (`!int`) is application-specific.
//! Only `!!int` (`tag:yaml.org,2002:int`) names the core schema type,
//! so `!int 5` stays a tagged node like any other local tag. serde_yaml
//! 0.9 reads `- !int 5` as `[{"!int": 5}]` too.

use noyalib::{Tag, TaggedValue, Value};

#[test]
fn local_tags_named_like_core_types_stay_tagged() {
    for (source, tag, payload) in [
        ("!int 5", "!int", "5"),
        ("!str 5", "!str", "5"),
        ("!float 1", "!float", "1"),
        ("!bool true", "!bool", "true"),
        ("!null ~", "!null", "~"),
        ("!int x", "!int", "x"),
    ] {
        let value: Value = noyalib::from_str(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
        let expected = Value::Tagged(Box::new(TaggedValue::new(
            Tag::new(tag),
            Value::String(payload.to_owned()),
        )));
        assert_eq!(value, expected, "{source:?}");
    }
}

#[test]
fn core_schema_tags_still_resolve() {
    for (source, expected) in [
        ("!!int 5", Value::from(5)),
        ("!<tag:yaml.org,2002:int> 5", Value::from(5)),
        ("!!str 5", Value::from("5")),
        ("%TAG !e! tag:yaml.org,2002:\n--- !e!int 5", Value::from(5)),
    ] {
        let value: Value = noyalib::from_str(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
        assert_eq!(value, expected, "{source:?}");
    }
}
