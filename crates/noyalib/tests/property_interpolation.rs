// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Value::interpolate_properties` — `${name}` substitution inside
//! string scalars, with strict and lossy variants.

#![allow(missing_docs)]

use noyalib::{Mapping, Tag, TaggedValue, Value, from_str};
use std::collections::HashMap;

fn props(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn interpolates_top_level_string() {
    let mut v: Value = from_str("name: ${APP}").unwrap();
    v.interpolate_properties(&props(&[("APP", "noyalib")]))
        .unwrap();
    assert_eq!(v["name"].as_str(), Some("noyalib"));
}

#[test]
fn interpolates_nested_mapping() {
    let mut v: Value =
        from_str("service:\n  name: ${APP}\n  url: https://${HOST}:${PORT}/api\n").unwrap();
    let map = props(&[
        ("APP", "noyalib"),
        ("HOST", "api.example.com"),
        ("PORT", "8443"),
    ]);
    v.interpolate_properties(&map).unwrap();
    assert_eq!(v["service"]["name"].as_str(), Some("noyalib"));
    assert_eq!(
        v["service"]["url"].as_str(),
        Some("https://api.example.com:8443/api"),
    );
}

#[test]
fn interpolates_inside_sequence() {
    let mut v: Value = from_str("items:\n  - hello ${WHO}\n  - bye ${WHO}\n").unwrap();
    v.interpolate_properties(&props(&[("WHO", "world")]))
        .unwrap();
    assert_eq!(v["items"][0].as_str(), Some("hello world"));
    assert_eq!(v["items"][1].as_str(), Some("bye world"));
}

#[test]
fn does_not_touch_keys() {
    // The mapping key `${APP}` is left as-is — only values are
    // interpolated. This avoids surprising key-rename interactions.
    let mut v: Value = from_str("\"${APP}\": me\nname: ${APP}\n").unwrap();
    v.interpolate_properties(&props(&[("APP", "noyalib")]))
        .unwrap();
    let map = match &v {
        Value::Mapping(m) => m,
        _ => panic!("expected mapping"),
    };
    assert!(
        map.contains_key("${APP}"),
        "literal placeholder key must survive"
    );
    assert!(!map.contains_key("noyalib"), "value-only interpolation");
    assert_eq!(v["name"].as_str(), Some("noyalib"));
}

#[test]
fn does_not_touch_non_string_scalars() {
    let mut v: Value = from_str("port: 8080\nflag: true\nnull_val: null\n").unwrap();
    v.interpolate_properties(&props(&[("X", "y")])).unwrap();
    // None of these had string content; tree unchanged.
    assert_eq!(v["port"].as_i64(), Some(8080));
    assert_eq!(v["flag"].as_bool(), Some(true));
    assert!(v["null_val"].is_null());
}

#[test]
fn unknown_placeholder_errors_in_strict_mode() {
    let mut v: Value = from_str("name: ${MISSING}").unwrap();
    let err = v
        .interpolate_properties::<String>(&HashMap::new())
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("MISSING"),
        "error must name the placeholder: {msg}"
    );
}

#[test]
fn unknown_placeholder_substitutes_empty_in_lossy_mode() {
    let mut v: Value = from_str("greeting: hello ${WHO} and ${OTHER}").unwrap();
    v.interpolate_properties_lossy(&props(&[("WHO", "world")]));
    assert_eq!(v["greeting"].as_str(), Some("hello world and "));
}

#[test]
fn escape_double_brace_is_literal_dollar_brace() {
    let mut v: Value = from_str("template: \"price: ${{USD}\"").unwrap();
    v.interpolate_properties(&props(&[("USD", "ignored")]))
        .unwrap();
    assert_eq!(v["template"].as_str(), Some("price: ${USD}"));
}

#[test]
fn escape_double_close_brace_is_literal() {
    let mut v: Value = from_str("template: \"a}}b\"").unwrap();
    v.interpolate_properties(&props(&[])).unwrap();
    assert_eq!(v["template"].as_str(), Some("a}b"));
}

#[test]
fn dotted_placeholder_names_supported() {
    let mut v: Value = from_str("url: ${db.host}:${db.port}").unwrap();
    v.interpolate_properties(&props(&[("db.host", "db1"), ("db.port", "5432")]))
        .unwrap();
    assert_eq!(v["url"].as_str(), Some("db1:5432"));
}

#[test]
fn unterminated_placeholder_errors() {
    let mut v: Value = from_str("name: \"${UNCLOSED\"").unwrap();
    let err = v
        .interpolate_properties::<String>(&HashMap::new())
        .unwrap_err();
    assert!(err.to_string().contains("unterminated"));
}

#[test]
fn empty_placeholder_errors() {
    let mut v: Value = from_str("name: \"${}\"").unwrap();
    let err = v
        .interpolate_properties::<String>(&HashMap::new())
        .unwrap_err();
    assert!(err.to_string().contains("empty placeholder"));
}

#[test]
fn invalid_placeholder_character_errors() {
    let mut v: Value = from_str("name: \"${A B}\"").unwrap();
    let err = v
        .interpolate_properties::<String>(&HashMap::new())
        .unwrap_err();
    assert!(err.to_string().contains("invalid character"));
}

#[test]
fn no_dollar_means_no_walk_no_change() {
    // Strings without `${` or `}` should not allocate.
    let mut v: Value = from_str("a: hello\nb: world\nc: 42\n").unwrap();
    let before = v.clone();
    v.interpolate_properties(&props(&[("APP", "noyalib")]))
        .unwrap();
    assert_eq!(v, before);
}

#[test]
fn unicode_around_placeholder_preserved() {
    let mut v: Value = from_str("greeting: \"héllo ${WHO} 🌍\"").unwrap();
    v.interpolate_properties(&props(&[("WHO", "wörld")]))
        .unwrap();
    assert_eq!(v["greeting"].as_str(), Some("héllo wörld 🌍"));
}

#[test]
fn descends_into_tagged_value() {
    // Build a `Value::Tagged` directly — the typed Value loader
    // resolves most user tags into their target shape, so we
    // construct the wrapper programmatically to exercise the
    // interpolate-walks-into-Tagged path.
    let inner = Value::String("v=${V}".into());
    let tagged = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!Version"), inner)));

    let mut map = Mapping::new();
    let _ = map.insert("tagged", tagged);
    let mut v = Value::Mapping(map);

    v.interpolate_properties(&props(&[("V", "1.2.3")])).unwrap();

    let inner_after = match &v["tagged"] {
        Value::Tagged(boxed) => boxed.value(),
        other => panic!("expected tagged, got {other:?}"),
    };
    assert_eq!(inner_after.as_str(), Some("v=1.2.3"));
}

#[test]
fn nested_lossy_substitution_in_complex_doc() {
    let mut v: Value = from_str(
        "config:
  servers:
    - name: ${A}
      url: https://${HOST_A}/
    - name: ${B}
      url: https://${HOST_B}/
",
    )
    .unwrap();

    let map = props(&[
        ("A", "alpha"),
        ("HOST_A", "alpha.example.com"),
        ("B", "beta"),
    ]);
    v.interpolate_properties_lossy(&map);

    let s0 = &v["config"]["servers"][0];
    assert_eq!(s0["name"].as_str(), Some("alpha"));
    assert_eq!(s0["url"].as_str(), Some("https://alpha.example.com/"));

    let s1 = &v["config"]["servers"][1];
    assert_eq!(s1["name"].as_str(), Some("beta"));
    // HOST_B missing → empty in lossy mode.
    assert_eq!(s1["url"].as_str(), Some("https:///"));
}
