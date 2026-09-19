//! A single-entry mapping whose only key starts with `!` is a mapping,
//! not a tag (Refs #377).
//!
//! `TaggedValue::serialize` uses a single-entry map keyed by the tag
//! string as its wire form, the shape a serializer with no YAML-tag
//! concept needs (#350). The serializer used to reconstruct
//! `Value::Tagged` from *any* map of that shape, so `{"!important": red}`
//! was written `!important red` and read back as a tagged scalar. The
//! wire form now travels inside a marker newtype, and only the marker
//! rebuilds the tag.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Mapping, Tag, TaggedValue, Value, from_str, to_string, to_value};

fn round_trip(input: &str) -> (Value, String, Value) {
    let original: Value = from_str(input).unwrap();
    let out = to_string(&original).unwrap();
    let back: Value = from_str(&out).unwrap();
    (original, out, back)
}

#[test]
fn nested_single_entry_mapping_keyed_by_a_tag_indicator_string_stays_a_mapping() {
    let (original, out, back) = round_trip("rules:\n  '!important': red\n");
    assert_eq!(out, "rules:\n  \"!important\": red");
    assert_eq!(back, original);
}

#[test]
fn root_single_entry_mapping_keyed_by_a_tag_indicator_string_stays_a_mapping() {
    let (original, out, back) = round_trip("'!important': red\n");
    assert_eq!(out, "\"!important\": red");
    assert_eq!(back, original);
}

#[test]
fn sequence_item_single_entry_mapping_keyed_by_a_tag_indicator_string_stays_a_mapping() {
    let (original, out, back) = round_trip("- '!important': red\n");
    assert_eq!(out, "- \"!important\": red");
    assert_eq!(back, original);
}

#[test]
fn null_and_mapping_values_under_a_tag_indicator_key_stay_mappings() {
    for input in ["'!x':\n", "'!x':\n  a: 1\n", "'!!str': v\n", "'! x': v\n"] {
        let (original, out, back) = round_trip(input);
        assert!(
            matches!(back, Value::Mapping(_)),
            "{input:?} emitted {out:?}"
        );
        assert_eq!(back, original, "{input:?} emitted {out:?}");
    }
}

#[test]
fn to_value_keeps_a_tag_indicator_key_as_a_mapping() {
    let mut m = Mapping::new();
    let _ = m.insert("!important", Value::from("red"));
    let v = to_value(&m).unwrap();
    assert_eq!(v, Value::Mapping(m));
}

#[test]
fn a_genuine_tagged_value_still_round_trips_as_a_tag() {
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!important"),
        Value::from("red"),
    )));
    let mut m = Mapping::new();
    let _ = m.insert("rules", tagged);
    let original = Value::Mapping(m);

    assert_eq!(to_value(&original).unwrap(), original);
    let out = to_string(&original).unwrap();
    assert_eq!(out, "rules: !important red");
    let back: Value = from_str(&out).unwrap();
    assert_eq!(back, original);
}

#[test]
fn serde_json_still_sees_the_single_entry_map_wire_form() {
    let tagged = TaggedValue::new(Tag::new("!important"), Value::from("red"));
    let json = serde_json::to_string(&tagged).unwrap();
    assert_eq!(json, r#"{"!important":"red"}"#);

    let value = Value::Tagged(Box::new(tagged));
    assert_eq!(serde_json::to_string(&value).unwrap(), json);
}
